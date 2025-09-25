//! Prometheus Metrics Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    pub server_url: String,
    pub basic_auth: Option<BasicAuth>,
    pub query_timeout: u64,
    pub scrape_interval: u64,
    pub enable_remote_write: bool,
    pub remote_write_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:9090".to_string(),
            basic_auth: None,
            query_timeout: 30,
            scrape_interval: 15,
            enable_remote_write: false,
            remote_write_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    pub metric_name: String,
    pub labels: HashMap<String, String>,
    pub value: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub result_type: String,
    pub result: Vec<MetricResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricResult {
    pub metric: HashMap<String, String>,
    pub value: Option<(u64, String)>,
    pub values: Option<Vec<(u64, String)>>,
}

#[derive(Debug, Default)]
struct PrometheusMetrics {
    queries_executed: u64,
    metrics_scraped: u64,
    remote_writes: u64,
    avg_query_time: f64,
    active_scrapes: u32,
}

pub struct PrometheusOperator {
    config: PrometheusConfig,
    http_client: reqwest::Client,
    metrics: Arc<Mutex<PrometheusMetrics>>,
    local_metrics: Arc<RwLock<HashMap<String, MetricSample>>>,
}

impl PrometheusOperator {
    pub async fn new(config: PrometheusConfig) -> Result<Self, HlxError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.query_timeout))
            .build()
            .map_err(|e| HlxError::InitializationError {
                component: "Prometheus HTTP Client".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        info!("Prometheus operator initialized for server: {}", config.server_url);

        Ok(Self {
            config,
            http_client: client,
            metrics: Arc::new(Mutex::new(PrometheusMetrics::default())),
            local_metrics: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn query(&self, query: &str, time: Option<u64>) -> Result<QueryResult, HlxError> {
        let start_time = Instant::now();
        let mut url = format!("{}/api/v1/query", self.config.server_url);
        
        let mut request = self.http_client.get(&url).query(&[("query", query)]);
        
        if let Some(timestamp) = time {
            request = request.query(&[("time", timestamp.to_string())]);
        }

        if let Some(auth) = &self.config.basic_auth {
            request = request.basic_auth(&auth.username, Some(&auth.password));
        }

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Prometheus Query".to_string(),
                message: format!("Query request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Prometheus Query".to_string(),
                message: format!("Query failed with status: {}", response.status()),
            });
        }

        let response_body: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Prometheus Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        let query_result = self.parse_query_response(&response_body)?;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.queries_executed += 1;
            let query_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_query_time = (metrics.avg_query_time * (metrics.queries_executed - 1) as f64 + query_time) / metrics.queries_executed as f64;
        }

        Ok(query_result)
    }

    pub async fn query_range(&self, query: &str, start: u64, end: u64, step: &str) -> Result<QueryResult, HlxError> {
        let start_time = Instant::now();
        let url = format!("{}/api/v1/query_range", self.config.server_url);
        
        let mut request = self.http_client.get(&url)
            .query(&[
                ("query", query),
                ("start", &start.to_string()),
                ("end", &end.to_string()),
                ("step", step),
            ]);

        if let Some(auth) = &self.config.basic_auth {
            request = request.basic_auth(&auth.username, Some(&auth.password));
        }

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Prometheus Range Query".to_string(),
                message: format!("Range query request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Prometheus Range Query".to_string(),
                message: format!("Range query failed with status: {}", response.status()),
            });
        }

        let response_body: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Prometheus Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        let query_result = self.parse_query_response(&response_body)?;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.queries_executed += 1;
            let query_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_query_time = (metrics.avg_query_time * (metrics.queries_executed - 1) as f64 + query_time) / metrics.queries_executed as f64;
        }

        Ok(query_result)
    }

    pub async fn add_metric(&self, metric: MetricSample) -> Result<(), HlxError> {
        let metric_key = format!("{}:{}", metric.metric_name, 
            metric.labels.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(",")
        );

        {
            let mut local_metrics = self.local_metrics.write().await;
            local_metrics.insert(metric_key, metric);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.metrics_scraped += 1;
        }

        Ok(())
    }

    pub async fn get_local_metrics(&self) -> Vec<MetricSample> {
        let local_metrics = self.local_metrics.read().await;
        local_metrics.values().cloned().collect()
    }

    pub async fn clear_local_metrics(&self) -> Result<(), HlxError> {
        let mut local_metrics = self.local_metrics.write().await;
        local_metrics.clear();
        info!("Cleared local Prometheus metrics");
        Ok(())
    }

    pub async fn get_targets(&self) -> Result<JsonValue, HlxError> {
        let url = format!("{}/api/v1/targets", self.config.server_url);
        
        let mut request = self.http_client.get(&url);

        if let Some(auth) = &self.config.basic_auth {
            request = request.basic_auth(&auth.username, Some(&auth.password));
        }

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Prometheus Targets".to_string(),
                message: format!("Targets request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Prometheus Targets".to_string(),
                message: format!("Targets request failed with status: {}", response.status()),
            });
        }

        let targets: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Prometheus Response".to_string(),
                message: format!("Failed to parse targets response: {}", e),
            })?;

        Ok(targets)
    }

    fn parse_query_response(&self, response: &JsonValue) -> Result<QueryResult, HlxError> {
        let status = response["status"].as_str()
            .ok_or_else(|| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Prometheus Response".to_string(),
                message: "Missing status field".to_string(),
            })?;

        if status != "success" {
            let error_msg = response["error"].as_str().unwrap_or("Unknown error");
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operation: "Prometheus Query".to_string(),
                message: format!("Query failed: {}", error_msg),
                details: None,
            });
        }

        let data = &response["data"];
        let result_type = data["resultType"].as_str()
            .ok_or_else(|| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Prometheus Response".to_string(),
                message: "Missing resultType field".to_string(),
            })?;

        let result_array = data["result"].as_array()
            .ok_or_else(|| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Prometheus Response".to_string(),
                message: "Missing result array".to_string(),
            })?;

        let mut results = Vec::new();
        for result_item in result_array {
            let metric = result_item["metric"].as_object()
                .map(|obj| {
                    obj.iter()
                       .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                       .collect::<HashMap<String, String>>()
                })
                .unwrap_or_default();

            let mut metric_result = MetricResult {
                metric,
                value: None,
                values: None,
            };

            // Handle instant query results
            if let Some(value_array) = result_item["value"].as_array() {
                if value_array.len() == 2 {
                    if let (Some(timestamp), Some(value)) = (value_array[0].as_f64(), value_array[1].as_str()) {
                        metric_result.value = Some((timestamp as u64, value.to_string()));
                    }
                }
            }

            // Handle range query results
            if let Some(values_array) = result_item["values"].as_array() {
                let mut values = Vec::new();
                for value_pair in values_array {
                    if let Some(pair_array) = value_pair.as_array() {
                        if pair_array.len() == 2 {
                            if let (Some(timestamp), Some(value)) = (pair_array[0].as_f64(), pair_array[1].as_str()) {
                                values.push((timestamp as u64, value.to_string()));
                            }
                        }
                    }
                }
                metric_result.values = Some(values);
            }

            results.push(metric_result);
        }

        Ok(QueryResult {
            result_type: result_type.to_string(),
            result: results,
        })
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("queries_executed".to_string(), Value::Number(metrics.queries_executed as f64));
        result.insert("metrics_scraped".to_string(), Value::Number(metrics.metrics_scraped as f64));
        result.insert("remote_writes".to_string(), Value::Number(metrics.remote_writes as f64));
        result.insert("avg_query_time_ms".to_string(), Value::Number(metrics.avg_query_time));
        result.insert("active_scrapes".to_string(), Value::Number(metrics.active_scrapes as f64));

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for PrometheusOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "query" => {
                let query = params_map.get("query").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        message: "Missing PromQL query".to_string(),
                        field: Some("query".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                    })?;

                let time = params_map.get("time").and_then(|v| v.as_number()).map(|n| n as u64);

                let result = self.query(&query, time).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result_type".to_string(), Value::String(result.result_type.to_string()));
                    map.insert("result_count".to_string(), Value::Number(result.result.len() as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "query_range" => {
                let query = params_map.get("query").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        message: "Missing PromQL query".to_string(),
                        field: Some("query".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                    })?;

                let start = params_map.get("start").and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        message: "Missing start timestamp".to_string(),
                        field: Some("start".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                    })? as u64;

                let end = params_map.get("end").and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        message: "Missing end timestamp".to_string(),
                        field: Some("end".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                    })? as u64;

                let step = params_map.get("step").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "15s");

                let result = self.query_range(&query, start, end, &step).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result_type".to_string(), Value::String(result.result_type.to_string()));
                    map.insert("result_count".to_string(), Value::Number(result.result.len() as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "add_metric" => {
                let metric_name = params_map.get("name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        message: "Missing metric name".to_string(),
                        field: Some("name".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                    })?;

                let value = params_map.get("value").and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        message: "Missing metric value".to_string(),
                        field: Some("value".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                    })?;

                let labels = params_map.get("labels").and_then(|v| {
                    if let Value::Object(obj) = v {
                        let mut labels_map = HashMap::new();
                        for (k, v) in obj {
                            if let Some(val_str) = v.as_string() {
                                labels_map.insert(k.clone(), val_str);
                            }
                        }
                        Some(labels_map)
                    } else { None }
                }).unwrap_or_default();

                let timestamp = params_map.get("timestamp").and_then(|v| v.as_number()).map(|n| n as u64)
                    .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());

                let metric = MetricSample {
                    metric_name: metric_name.to_string(),
                    labels: labels.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                    value,
                    timestamp,
                };

                self.add_metric(metric).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("metric_added".to_string(), Value::Boolean(true));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_local_metrics" => {
                let metrics = self.get_local_metrics().await;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("metric_count".to_string(), Value::Number(metrics.len() as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "clear_local_metrics" => {
                self.clear_local_metrics().await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("metrics_cleared".to_string(), Value::Boolean(true));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "targets" => {
                let targets = self.get_targets().await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("targets".to_string(), utils::json_value_to_value(&targets));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "prometheus".to_string(),
                params: format!("Unknown Prometheus operation: {}", operator),
            }),
        }
    }
} 