//! Zipkin Tracing Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipkinConfig {
    pub collector_url: String,
    pub service_name: String,
    pub service_version: String,
    pub local_endpoint: ZipkinEndpoint,
    pub sample_rate: f64,
    pub batch_size: usize,
    pub flush_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipkinEndpoint {
    pub service_name: String,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub port: Option<u16>,
}

impl Default for ZipkinConfig {
    fn default() -> Self {
        Self {
            collector_url: "http://localhost:9411/api/v2/spans".to_string(),
            service_name: "helix-service".to_string(),
            service_version: "1.0.0".to_string(),
            local_endpoint: ZipkinEndpoint {
                service_name: "helix-service".to_string(),
                ipv4: Some("127.0.0.1".to_string()),
                ipv6: None,
                port: Some(8080),
            },
            sample_rate: 1.0,
            batch_size: 100,
            flush_interval_seconds: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipkinSpan {
    pub trace_id: String,
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub kind: ZipkinSpanKind,
    pub timestamp: u64, // microseconds
    pub duration: Option<u64>, // microseconds
    pub local_endpoint: ZipkinEndpoint,
    pub remote_endpoint: Option<ZipkinEndpoint>,
    pub annotations: Vec<ZipkinAnnotation>,
    pub tags: HashMap<String, String>,
    pub debug: bool,
    pub shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZipkinSpanKind {
    Client,
    Server,
    Producer,
    Consumer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipkinAnnotation {
    pub timestamp: u64, // microseconds
    pub value: String,
}

#[derive(Debug, Default)]
struct ZipkinMetrics {
    spans_created: u64,
    spans_sent: u64,
    batches_sent: u64,
    send_errors: u64,
    avg_batch_size: f64,
    total_duration: u64,
}

pub struct ZipkinOperator {
    config: ZipkinConfig,
    http_client: reqwest::Client,
    metrics: Arc<Mutex<ZipkinMetrics>>,
    pending_spans: Arc<RwLock<Vec<ZipkinSpan>>>,
    active_spans: Arc<RwLock<HashMap<String, ZipkinSpan>>>,
}

impl ZipkinOperator {
    pub async fn new(config: ZipkinConfig) -> Result<Self, HlxError> {
        let client = reqwest::Client::new();

        info!("Zipkin operator initialized for service: {} endpoint: {}", 
              config.service_name, config.collector_url);

        let operator = Self {
            config,
            http_client: client,
            metrics: Arc::new(Mutex::new(ZipkinMetrics::default())),
            pending_spans: Arc::new(RwLock::new(Vec::new())),
            active_spans: Arc::new(RwLock::new(HashMap::new())),
        };

        // Start background flush task
        let flush_operator = operator.clone();
        tokio::spawn(async move {
            flush_operator.background_flush_task().await;
        });

        Ok(operator)
    }

    pub async fn start_span(&self, name: &str, kind: ZipkinSpanKind, trace_id: Option<&str>, parent_id: Option<&str>) -> Result<ZipkinSpan, HlxError> {
        let span_id = Self::generate_span_id();
        let trace_id = trace_id.unwrap_or(&Self::generate_trace_id()).to_string();

        debug!("Starting Zipkin span {} for trace: {}", span_id, trace_id);

        let span = ZipkinSpan {
            trace_id: trace_id.clone(),
            id: span_id.clone(),
            parent_id: parent_id.map(|s| s.to_string()),
            name: name.to_string(),
            kind,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64,
            duration: None,
            local_endpoint: self.config.local_endpoint.clone(),
            remote_endpoint: None,
            annotations: Vec::new(),
            tags: HashMap::new(),
            debug: false,
            shared: false,
        };

        {
            let mut active_spans = self.active_spans.write().await;
            active_spans.insert(span_id.clone(), span.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.spans_created += 1;
        }

        Ok(span)
    }

    pub async fn finish_span(&self, span_id: &str) -> Result<(), HlxError> {
        debug!("Finishing Zipkin span: {}", span_id);

        let mut active_spans = self.active_spans.write().await;
        if let Some(mut span) = active_spans.remove(span_id) {
            let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64;
            span.duration = Some(end_time - span.timestamp);

            {
                let mut pending_spans = self.pending_spans.write().await;
                pending_spans.push(span);
            }

            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.total_duration += span.duration.unwrap_or(0);
            }

            info!("Zipkin span {} finished with duration: {} μs", span_id, span.duration.unwrap_or(0));
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Span".to_string(),
                identifier: span_id.to_string(),
            })
        }
    }

    pub async fn add_span_tag(&self, span_id: &str, key: &str, value: &str) -> Result<(), HlxError> {
        let mut active_spans = self.active_spans.write().await;
        if let Some(span) = active_spans.get_mut(span_id) {
            span.tags.insert(key.to_string(), value.to_string());
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Span".to_string(),
                identifier: span_id.to_string(),
            })
        }
    }

    pub async fn add_annotation(&self, span_id: &str, value: &str) -> Result<(), HlxError> {
        let mut active_spans = self.active_spans.write().await;
        if let Some(span) = active_spans.get_mut(span_id) {
            let annotation = ZipkinAnnotation {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64,
                value: value.to_string(),
            };
            span.annotations.push(annotation);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Span".to_string(),
                identifier: span_id.to_string(),
            })
        }
    }

    pub async fn set_remote_endpoint(&self, span_id: &str, endpoint: ZipkinEndpoint) -> Result<(), HlxError> {
        let mut active_spans = self.active_spans.write().await;
        if let Some(span) = active_spans.get_mut(span_id) {
            span.remote_endpoint = Some(endpoint);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Span".to_string(),
                identifier: span_id.to_string(),
            })
        }
    }

    pub async fn flush_spans(&self) -> Result<u64, HlxError> {
        let mut pending_spans = self.pending_spans.write().await;
        if pending_spans.is_empty() {
            return Ok(0);
        }

        let spans_to_send = pending_spans.clone();
        pending_spans.clear();

        drop(pending_spans); // Release the lock

        let batch_count = (spans_to_send.len() + self.config.batch_size - 1) / self.config.batch_size;
        let mut total_sent = 0u64;

        for batch in spans_to_send.chunks(self.config.batch_size) {
            match self.send_batch(batch).await {
                Ok(sent) => total_sent += sent,
                Err(e) => {
                    debug!("Failed to send batch: {}", e);
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.send_errors += 1;
                }
            }
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.spans_sent += total_sent;
            metrics.batches_sent += batch_count as u64;
            if batch_count > 0 {
                metrics.avg_batch_size = (metrics.avg_batch_size * (metrics.batches_sent - batch_count as u64) as f64 + total_sent as f64) / metrics.batches_sent as f64;
            }
        }

        info!("Flushed {} spans to Zipkin in {} batches", total_sent, batch_count);
        Ok(total_sent)
    }

    async fn send_batch(&self, spans: &[ZipkinSpan]) -> Result<u64, HlxError> {
        let json_spans: Vec<JsonValue> = spans.iter().map(|span| {
            json!({
                "traceId": span.trace_id,
                "id": span.id,
                "parentId": span.parent_id,
                "name": span.name,
                "kind": format!("{:?}", span.kind).to_uppercase(),
                "timestamp": span.timestamp,
                "duration": span.duration,
                "localEndpoint": {
                    "serviceName": span.local_endpoint.service_name,
                    "ipv4": span.local_endpoint.ipv4,
                    "ipv6": span.local_endpoint.ipv6,
                    "port": span.local_endpoint.port
                },
                "remoteEndpoint": span.remote_endpoint.as_ref().map(|ep| json!({
                    "serviceName": ep.service_name,
                    "ipv4": ep.ipv4,
                    "ipv6": ep.ipv6,
                    "port": ep.port
                })),
                "annotations": span.annotations.iter().map(|ann| json!({
                    "timestamp": ann.timestamp,
                    "value": ann.value
                })).collect::<Vec<_>>(),
                "tags": span.tags,
                "debug": span.debug,
                "shared": span.shared
            })
        }).collect();

        let response = self.http_client
            .post(&self.config.collector_url)
            .header("Content-Type", "application/json")
            .json(&json_spans)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Send Zipkin Batch".to_string(),
                message: format!("Failed to send batch: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Send Zipkin Batch".to_string(),
                message: format!("Zipkin returned status: {}", response.status()),
            });
        }

        Ok(spans.len() as u64)
    }

    async fn background_flush_task(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(self.config.flush_interval_seconds));

        loop {
            interval.tick().await;
            if let Err(e) = self.flush_spans().await {
                debug!("Background flush failed: {}", e);
            }
        }
    }

    fn generate_trace_id() -> String {
        format!("{:032x}", uuid::Uuid::new_v4().as_u128())
    }

    fn generate_span_id() -> String {
        format!("{:016x}", uuid::Uuid::new_v4().as_u128() as u64)
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("spans_created".to_string(), Value::Number(metrics.spans_created as f64));
        result.insert("spans_sent".to_string(), Value::Number(metrics.spans_sent as f64));
        result.insert("batches_sent".to_string(), Value::Number(metrics.batches_sent as f64));
        result.insert("send_errors".to_string(), Value::Number(metrics.send_errors as f64));
        result.insert("avg_batch_size".to_string(), Value::Number(metrics.avg_batch_size));
        result.insert("total_duration_micros".to_string(), Value::Number(metrics.total_duration as f64));

        if metrics.spans_created > 0 {
            let send_rate = (metrics.spans_sent as f64 / metrics.spans_created as f64) * 100.0;
            result.insert("send_rate_percent".to_string(), Value::Number(send_rate));
        }

        result
    }
}

impl Clone for ZipkinOperator {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            http_client: self.http_client.clone(),
            metrics: Arc::clone(&self.metrics),
            pending_spans: Arc::clone(&self.pending_spans),
            active_spans: Arc::clone(&self.active_spans),
        }
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for ZipkinOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "start_span" => {
                let name = params_map.get("name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("name".to_string()),
                        message: "Missing span name".to_string(),
                    })?;

                let kind_str = params_map.get("kind").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "server");

                let kind = match kind_str {
                    "client" => ZipkinSpanKind::Client,
                    "server" => ZipkinSpanKind::Server,
                    "producer" => ZipkinSpanKind::Producer,
                    "consumer" => ZipkinSpanKind::Consumer,
                    _ => ZipkinSpanKind::Server,
                };

                let trace_id = params_map.get("trace_id").and_then(|v| v.as_string());
                let parent_id = params_map.get("parent_id").and_then(|v| v.as_string());

                let span = self.start_span(&name, kind, trace_id.as_deref(), parent_id.as_deref()).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("span_id".to_string(), Value::String(span.id.to_string()));
                    map.insert("trace_id".to_string(), Value::String(span.trace_id.to_string()));
                    map.insert("name".to_string(), Value::String(span.name.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "finish_span" => {
                let span_id = params_map.get("span_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("span_id".to_string()),
                        message: "Missing span ID".to_string(),
                    })?;

                self.finish_span(&span_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("span_id".to_string(), Value::String(span_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "add_tag" => {
                let span_id = params_map.get("span_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("span_id".to_string()),
                        message: "Missing span ID".to_string(),
                    })?;

                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing tag key".to_string(),
                    })?;

                let value = params_map.get("value").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("value".to_string()),
                        message: "Missing tag value".to_string(),
                    })?;

                self.add_span_tag(&span_id, &key, &value).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("span_id".to_string(), Value::String(span_id.to_string(.to_string())));
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("value".to_string(), Value::String(value.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "add_annotation" => {
                let span_id = params_map.get("span_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("span_id".to_string()),
                        message: "Missing span ID".to_string(),
                    })?;

                let value = params_map.get("value").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("value".to_string()),
                        message: "Missing annotation value".to_string(),
                    })?;

                self.add_annotation(&span_id, &value).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("span_id".to_string(), Value::String(span_id.to_string(.to_string())));
                    map.insert("annotation".to_string(), Value::String(value.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "flush" => {
                let spans_flushed = self.flush_spans().await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("spans_flushed".to_string(), Value::Number(spans_flushed as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "zipkin".to_string(),
                params: format!("Unknown Zipkin operation: {}", operator),
            }),
        }
    }
} 