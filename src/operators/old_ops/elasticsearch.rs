//! Elasticsearch Search Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElasticsearchConfig {
    pub hosts: Vec<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_key: Option<String>,
    pub cloud_id: Option<String>,
    pub timeout: u64,
    pub max_retries: u32,
    pub enable_ssl: bool,
    pub verify_ssl: bool,
}

impl Default for ElasticsearchConfig {
    fn default() -> Self {
        Self {
            hosts: vec!["http://localhost:9200".to_string()],
            username: None,
            password: None,
            api_key: None,
            cloud_id: None,
            timeout: 30,
            max_retries: 3,
            enable_ssl: false,
            verify_ssl: true,
        }
    }
}

#[derive(Debug, Default)]
struct ElasticsearchMetrics {
    queries_executed: u64,
    documents_indexed: u64,
    documents_deleted: u64,
    bulk_operations: u64,
    avg_query_time: f64,
    index_operations: u64,
    search_operations: u64,
}

pub struct ElasticsearchOperator {
    config: ElasticsearchConfig,
    http_client: reqwest::Client,
    metrics: Arc<Mutex<ElasticsearchMetrics>>,
    indices: Arc<RwLock<HashMap<String, bool>>>, // Track known indices
}

impl ElasticsearchOperator {
    pub async fn new(config: ElasticsearchConfig) -> Result<Self, HlxError> {
        let client_builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .danger_accept_invalid_certs(!config.verify_ssl);

        let client = client_builder.build()
            .map_err(|e| HlxError::InitializationError {
                component: "Elasticsearch HTTP Client".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;


        Ok(Self {
            config,
            http_client: client,
            metrics: Arc::new(Mutex::new(ElasticsearchMetrics::default())),
            indices: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn search(&self, index: &str, query: &JsonValue) -> Result<JsonValue, HlxError> {
        let start_time = Instant::now();
        let url = format!("{}/{}/_search", self.get_primary_host(), index);

        let mut request = self.http_client.post(&url).json(query);
        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Elasticsearch Search".to_string(),
                message: format!("Search request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Elasticsearch Search".to_string(),
                message: format!("Search failed with status: {}", response.status()),
            });
        }

        let result: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Elasticsearch Response".to_string(),
                message: format!("Failed to parse search response: {}", e),
            })?;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.search_operations += 1;
            metrics.queries_executed += 1;
            let query_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_query_time = (metrics.avg_query_time * (metrics.queries_executed - 1) as f64 + query_time) / metrics.queries_executed as f64;
        }

        Ok(result)
    }

    pub async fn index_document(&self, index: &str, document_id: Option<&str>, document: &JsonValue) -> Result<JsonValue, HlxError> {
        let url = if let Some(id) = document_id {
            format!("{}/{}/_doc/{}", self.get_primary_host(), index, id)
        } else {
            format!("{}/{}/_doc", self.get_primary_host(), index)
        };

        let mut request = if document_id.is_some() {
            self.http_client.put(&url).json(document)
        } else {
            self.http_client.post(&url).json(document)
        };

        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Elasticsearch Index".to_string(),
                message: format!("Index request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Elasticsearch Index".to_string(),
                message: format!("Index failed with status: {}", response.status()),
            });
        }

        let result: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Elasticsearch Response".to_string(),
                message: format!("Failed to parse index response: {}", e),
            })?;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.documents_indexed += 1;
            metrics.index_operations += 1;
        }

        // Track index
        {
            let mut indices = self.indices.write().await;
            indices.insert(index.to_string(), true);
        }

        Ok(result)
    }

    pub async fn delete_document(&self, index: &str, document_id: &str) -> Result<JsonValue, HlxError> {
        let url = format!("{}/{}/_doc/{}", self.get_primary_host(), index, document_id);

        let mut request = self.http_client.delete(&url);
        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Elasticsearch Delete".to_string(),
                message: format!("Delete request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Elasticsearch Delete".to_string(),
                message: format!("Delete failed with status: {}", response.status()),
            });
        }

        let result: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Elasticsearch Response".to_string(),
                message: format!("Failed to parse delete response: {}", e),
            })?;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.documents_deleted += 1;
        }

        Ok(result)
    }

    pub async fn bulk_operation(&self, operations: &JsonValue) -> Result<JsonValue, HlxError> {
        let url = format!("{}/_bulk", self.get_primary_host());

        // Convert JSON to NDJSON format for bulk API
        let bulk_data = self.prepare_bulk_data(operations)?;

        let mut request = self.http_client.post(&url)
            .header("Content-Type", "application/x-ndjson")
            .body(bulk_data);

        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Elasticsearch Bulk".to_string(),
                message: format!("Bulk request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Elasticsearch Bulk".to_string(),
                message: format!("Bulk operation failed with status: {}", response.status()),
            });
        }

        let result: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Elasticsearch Response".to_string(),
                message: format!("Failed to parse bulk response: {}", e),
            })?;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.bulk_operations += 1;
        }

        Ok(result)
    }

    pub async fn create_index(&self, index: &str, mapping: Option<&JsonValue>) -> Result<JsonValue, HlxError> {
        let url = format!("{}/{}", self.get_primary_host(), index);

        let body = mapping.cloned().unwrap_or_else(|| json!({}));
        let mut request = self.http_client.put(&url).json(&body);
        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Elasticsearch Create Index".to_string(),
                message: format!("Create index request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Elasticsearch Create Index".to_string(),
                message: format!("Create index failed with status: {}", response.status()),
            });
        }

        let result: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Elasticsearch Response".to_string(),
                message: format!("Failed to parse create index response: {}", e),
            })?;

        // Track index
        {
            let mut indices = self.indices.write().await;
            indices.insert(index.to_string(), true);
        }

        Ok(result)
    }

    pub async fn delete_index(&self, index: &str) -> Result<JsonValue, HlxError> {
        let url = format!("{}/{}", self.get_primary_host(), index);

        let mut request = self.http_client.delete(&url);
        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Elasticsearch Delete Index".to_string(),
                message: format!("Delete index request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Elasticsearch Delete Index".to_string(),
                message: format!("Delete index failed with status: {}", response.status()),
            });
        }

        let result: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Elasticsearch Response".to_string(),
                message: format!("Failed to parse delete index response: {}", e),
            })?;

        // Remove from tracked indices
        {
            let mut indices = self.indices.write().await;
            indices.remove(index);
        }

        Ok(result)
    }

    pub async fn get_cluster_health(&self) -> Result<JsonValue, HlxError> {
        let url = format!("{}/_cluster/health", self.get_primary_host());

        let mut request = self.http_client.get(&url);
        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Elasticsearch Cluster Health".to_string(),
                message: format!("Cluster health request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Elasticsearch Cluster Health".to_string(),
                message: format!("Cluster health failed with status: {}", response.status()),
            });
        }

        let result: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Elasticsearch Response".to_string(),
                message: format!("Failed to parse cluster health response: {}", e),
            })?;

        Ok(result)
    }

    fn get_primary_host(&self) -> &str {
        self.config.hosts.first().unwrap()
    }

    fn add_auth_headers(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("ApiKey {}", api_key));
        } else if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
            request = request.basic_auth(username, Some(password));
        }
        request
    }

    fn prepare_bulk_data(&self, operations: &JsonValue) -> Result<String, HlxError> {
        let mut bulk_data = String::new();

        if let JsonValue::Array(ops) = operations {
            for op in ops {
                bulk_data.push_str(&serde_json::to_string(op).map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                    format: Some("JSON".to_string()),
                    message: format!("Failed to serialize bulk operation: {}", e),
                })?);
                bulk_data.push('\n');
            }
        }

        Ok(bulk_data)
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("queries_executed".to_string(), Value::Number(metrics.queries_executed as f64));
        result.insert("documents_indexed".to_string(), Value::Number(metrics.documents_indexed as f64));
        result.insert("documents_deleted".to_string(), Value::Number(metrics.documents_deleted as f64));
        result.insert("bulk_operations".to_string(), Value::Number(metrics.bulk_operations as f64));
        result.insert("avg_query_time_ms".to_string(), Value::Number(metrics.avg_query_time));
        result.insert("index_operations".to_string(), Value::Number(metrics.index_operations as f64));
        result.insert("search_operations".to_string(), Value::Number(metrics.search_operations as f64));

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for ElasticsearchOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "search" => {
                let index = params_map.get("index").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("index".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        message: "Missing index name".to_string(),
                    })?;

                let query = params_map.get("query").map(|v| utils::value_to_json_value(v))
                    .unwrap_or_else(|| json!({"query": {"match_all": {}}}));

                let result = self.search(&index, &query).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "index" => {
                let index = params_map.get("index").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("index".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        message: "Missing index name".to_string(),
                    })?;

                let document = params_map.get("document")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("document".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        message: "Missing document".to_string(),
                    })?;

                let document_id = params_map.get("id").and_then(|v| v.as_string());
                let doc_json = utils::value_to_json_value(document);

                let result = self.index_document(&index, document_id.as_deref(), &doc_json).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete" => {
                let index = params_map.get("index").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("index".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        message: "Missing index name".to_string(),
                    })?;

                let document_id = params_map.get("id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("id".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        message: "Missing document ID".to_string(),
                    })?;

                let result = self.delete_document(&index, &document_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "bulk" => {
                let operations = params_map.get("operations")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("operations".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        message: "Missing bulk operations".to_string(),
                    })?;

                let ops_json = utils::value_to_json_value(operations);
                let result = self.bulk_operation(&ops_json).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "create_index" => {
                let index = params_map.get("index").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("index".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        message: "Missing index name".to_string(),
                    })?;

                let mapping = params_map.get("mapping").map(|v| utils::value_to_json_value(v));

                let result = self.create_index(&index, mapping.as_ref()).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete_index" => {
                let index = params_map.get("index").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("index".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        message: "Missing index name".to_string(),
                    })?;

                let result = self.delete_index(&index).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "cluster_health" => {
                let result = self.get_cluster_health().await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "elasticsearch".to_string(),
                params: format!("Unknown Elasticsearch operation: {}", operator),
            }),
        }
    }
} 