//! Jaeger Tracing Operator for Helix Rust SDK
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
pub struct JaegerConfig {
    pub agent_endpoint: String,
    pub collector_endpoint: Option<String>,
    pub service_name: String,
    pub service_version: String,
    pub environment: String,
    pub sampling_rate: f64,
    pub max_spans_per_trace: u32,
    pub enable_debug: bool,
}

impl Default for JaegerConfig {
    fn default() -> Self {
        Self {
            agent_endpoint: "http://localhost:14268".to_string(),
            collector_endpoint: Some("http://localhost:14250".to_string()),
            service_name: "helix-service".to_string(),
            service_version: "1.0.0".to_string(),
            environment: "development".to_string(),
            sampling_rate: 1.0,
            max_spans_per_trace: 1000,
            enable_debug: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_micros: Option<u64>,
    pub tags: HashMap<String, String>,
    pub logs: Vec<SpanLog>,
    pub status: SpanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLog {
    pub timestamp: u64,
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    Ok,
    Error,
    Timeout,
    Cancelled,
}

#[derive(Debug, Default)]
struct JaegerMetrics {
    spans_created: u64,
    spans_finished: u64,
    traces_sent: u64,
    spans_dropped: u64,
    avg_span_duration: f64,
    total_traces: u64,
}

pub struct JaegerOperator {
    config: JaegerConfig,
    http_client: reqwest::Client,
    metrics: Arc<Mutex<JaegerMetrics>>,
    active_spans: Arc<RwLock<HashMap<String, Span>>>,
    traces: Arc<RwLock<HashMap<String, Vec<String>>>>, // trace_id -> span_ids
}

impl JaegerOperator {
    pub async fn new(config: JaegerConfig) -> Result<Self, HlxError> {
        let client = reqwest::Client::new();

        info!("Jaeger operator initialized for service: {} environment: {}", 
              config.service_name, config.environment);

        Ok(Self {
            config,
            http_client: client,
            metrics: Arc::new(Mutex::new(JaegerMetrics::default())),
            active_spans: Arc::new(RwLock::new(HashMap::new())),
            traces: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn start_span(&self, operation_name: &str, trace_id: Option<&str>, parent_span_id: Option<&str>) -> Result<Span, HlxError> {
        let span_id = uuid::Uuid::new_v4().to_string();
        let trace_id = trace_id.unwrap_or(&uuid::Uuid::new_v4().to_string()).to_string();

        debug!("Starting span {} for operation: {}", span_id, operation_name);

        let span = Span {
            trace_id: trace_id.clone(),
            span_id: span_id.clone(),
            parent_span_id: parent_span_id.map(|s| s.to_string()),
            operation_name: operation_name.to_string(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64,
            end_time: None,
            duration_micros: None,
            tags: HashMap::new(),
            logs: Vec::new(),
            status: SpanStatus::Ok,
        };

        {
            let mut spans = self.active_spans.write().await;
            spans.insert(span_id.clone(), span.clone());
        }

        {
            let mut traces = self.traces.write().await;
            traces.entry(trace_id.clone()).or_insert_with(Vec::new).push(span_id);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.spans_created += 1;
        }

        Ok(span)
    }

    pub async fn finish_span(&self, span_id: &str) -> Result<(), HlxError> {
        debug!("Finishing span: {}", span_id);

        let mut spans = self.active_spans.write().await;
        if let Some(span) = spans.get_mut(span_id) {
            let end_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64;
            span.end_time = Some(end_time);
            span.duration_micros = Some(end_time - span.start_time);

            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.spans_finished += 1;
                let duration = span.duration_micros.unwrap_or(0) as f64;
                metrics.avg_span_duration = (metrics.avg_span_duration * (metrics.spans_finished - 1) as f64 + duration) / metrics.spans_finished as f64;
            }

            info!("Span {} finished with duration: {} μs", span_id, span.duration_micros.unwrap_or(0));
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Span".to_string(),
                identifier: span_id.to_string(),
            })
        }
    }

    pub async fn add_span_tag(&self, span_id: &str, key: &str, value: &str) -> Result<(), HlxError> {
        debug!("Adding tag {}={} to span: {}", key, value, span_id);

        let mut spans = self.active_spans.write().await;
        if let Some(span) = spans.get_mut(span_id) {
            span.tags.insert(key.to_string(), value.to_string());
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Span".to_string(),
                identifier: span_id.to_string(),
            })
        }
    }

    pub async fn add_span_log(&self, span_id: &str, fields: HashMap<String, String>) -> Result<(), HlxError> {
        debug!("Adding log to span: {} with fields: {:?}", span_id, fields);

        let mut spans = self.active_spans.write().await;
        if let Some(span) = spans.get_mut(span_id) {
            let log = SpanLog {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64,
                fields,
            };
            span.logs.push(log);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Span".to_string(),
                identifier: span_id.to_string(),
            })
        }
    }

    pub async fn set_span_status(&self, span_id: &str, status: SpanStatus) -> Result<(), HlxError> {
        debug!("Setting span {} status to: {:?}", span_id, status);

        let mut spans = self.active_spans.write().await;
        if let Some(span) = spans.get_mut(span_id) {
            span.status = status;
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Span".to_string(),
                identifier: span_id.to_string(),
            })
        }
    }

    pub async fn get_span(&self, span_id: &str) -> Result<Option<Span>, HlxError> {
        let spans = self.active_spans.read().await;
        Ok(spans.get(span_id).cloned())
    }

    pub async fn get_trace_spans(&self, trace_id: &str) -> Result<Vec<Span>, HlxError> {
        let traces = self.traces.read().await;
        let spans = self.active_spans.read().await;

        if let Some(span_ids) = traces.get(trace_id) {
            let mut trace_spans = Vec::new();
            for span_id in span_ids {
                if let Some(span) = spans.get(span_id) {
                    trace_spans.push(span.clone());
                }
            }
            Ok(trace_spans)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn send_traces(&self) -> Result<u64, HlxError> {
        debug!("Sending traces to Jaeger collector");

        let spans = self.active_spans.read().await;
        let finished_spans: Vec<_> = spans.values()
            .filter(|span| span.end_time.is_some())
            .cloned()
            .collect();

        let traces_count = finished_spans.len() as u64;

        if !finished_spans.is_empty() {
            // In a real implementation, you would send spans to Jaeger
            // For this mock, we'll just simulate the sending
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.traces_sent += traces_count;
                metrics.total_traces += 1;
            }

            info!("Sent {} spans to Jaeger", traces_count);
        }

        Ok(traces_count)
    }

    pub async fn flush(&self) -> Result<(), HlxError> {
        debug!("Flushing all pending traces");

        self.send_traces().await?;

        // Clear finished spans
        let mut spans = self.active_spans.write().await;
        spans.retain(|_, span| span.end_time.is_none());

        info!("Jaeger tracer flushed successfully");
        Ok(())
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("spans_created".to_string(), Value::Number(metrics.spans_created as f64));
        result.insert("spans_finished".to_string(), Value::Number(metrics.spans_finished as f64));
        result.insert("traces_sent".to_string(), Value::Number(metrics.traces_sent as f64));
        result.insert("spans_dropped".to_string(), Value::Number(metrics.spans_dropped as f64));
        result.insert("avg_span_duration_micros".to_string(), Value::Number(metrics.avg_span_duration));
        result.insert("total_traces".to_string(), Value::Number(metrics.total_traces as f64));

        if metrics.spans_created > 0 {
            let finish_rate = (metrics.spans_finished as f64 / metrics.spans_created as f64) * 100.0;
            result.insert("span_finish_rate_percent".to_string(), Value::Number(finish_rate));
        }

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for JaegerOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "start_span" => {
                let operation_name = params_map.get("operation_name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("operation_name".to_string()),
                        message: "Missing operation name".to_string(),
                    })?;

                let trace_id = params_map.get("trace_id").and_then(|v| v.as_string());
                let parent_span_id = params_map.get("parent_span_id").and_then(|v| v.as_string());

                let span = self.start_span(&operation_name, trace_id.as_deref(), parent_span_id.as_deref()).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("span_id".to_string(), Value::String(span.span_id.to_string()));
                    map.insert("trace_id".to_string(), Value::String(span.trace_id.to_string()));
                    map.insert("operation_name".to_string(), Value::String(span.operation_name.to_string()));
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

            "add_log" => {
                let span_id = params_map.get("span_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("span_id".to_string()),
                        message: "Missing span ID".to_string(),
                    })?;

                let fields = params_map.get("fields").and_then(|v| {
                    if let Value::Object(obj) = v {
                        let mut fields_map = HashMap::new();
                        for (k, v) in obj {
                            if let Some(val_str) = v.as_string() {
                                fields_map.insert(k.clone(), val_str);
                            }
                        }
                        Some(fields_map)
                    } else { None }
                }).unwrap_or_default();

                let string_fields: HashMap<String, String> = fields.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
                self.add_span_log(&span_id, string_fields).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("span_id".to_string(), Value::String(span_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "set_status" => {
                let span_id = params_map.get("span_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("span_id".to_string()),
                        message: "Missing span ID".to_string(),
                    })?;

                let status_str = params_map.get("status").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "ok");

                let status = match status_str {
                    "ok" => SpanStatus::Ok,
                    "error" => SpanStatus::Error,
                    "timeout" => SpanStatus::Timeout,
                    "cancelled" => SpanStatus::Cancelled,
                    _ => SpanStatus::Ok,
                };

                self.set_span_status(&span_id, status).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("span_id".to_string(), Value::String(span_id.to_string(.to_string())));
                    map.insert("status".to_string(), Value::String(status_str.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_span" => {
                let span_id = params_map.get("span_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("span_id".to_string()),
                        message: "Missing span ID".to_string(),
                    })?;

                let span = self.get_span(&span_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    if let Some(sp) = span {
                        map.insert("span_id".to_string(), Value::String(sp.span_id.to_string()));
                        map.insert("trace_id".to_string(), Value::String(sp.trace_id.to_string()));
                        map.insert("operation_name".to_string(), Value::String(sp.operation_name.to_string()));
                        map.insert("start_time".to_string(), Value::Number(sp.start_time as f64));
                        map.insert("duration_micros".to_string(), 
                            Value::Number(sp.duration_micros.unwrap_or(0) as f64));
                        map.insert("status".to_string(), Value::String(format!("{:?}", sp.status.to_string())));
                    } else {
                        map.insert("span".to_string(), Value::Null);
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_trace" => {
                let trace_id = params_map.get("trace_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("trace_id".to_string()),
                        message: "Missing trace ID".to_string(),
                    })?;

                let spans = self.get_trace_spans(&trace_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("trace_id".to_string(), Value::String(trace_id.to_string(.to_string())));
                    map.insert("span_count".to_string(), Value::Number(spans.len() as f64));
                    map.insert("spans".to_string(), Value::Array(
                        spans.into_iter().map(|span| {
                            let mut span_map = HashMap::new();
                            span_map.insert("span_id".to_string(), Value::String(span.span_id.to_string()));
                            span_map.insert("operation_name".to_string(), Value::String(span.operation_name.to_string()));
                            span_map.insert("duration_micros".to_string(), 
                                Value::Number(span.duration_micros.unwrap_or(0) as f64));
                            Value::Object(span_map)
                        }).collect()
                    ));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "send_traces" => {
                let traces_sent = self.send_traces().await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("traces_sent".to_string(), Value::Number(traces_sent as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "flush" => {
                self.flush().await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("flushed".to_string(), Value::Boolean(true));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "jaeger".to_string(),
                params: format!("Unknown Jaeger operation: {}", operator),
            }),
        }
    }
} 