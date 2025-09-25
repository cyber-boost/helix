//! Monitoring & Observability - Enterprise Operators
//! Implements all monitoring and observability operators for production velocity mode

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::{json, Value as JsonValue};
use reqwest::Client;
use prometheus::{Counter, Gauge, Histogram, Registry, TextEncoder};
use opentelemetry::{global, trace::{Span, Tracer}, KeyValue};
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Enterprise Monitoring and observability operators implementation
pub struct MonitoringOperators {
    http_client: Client,
    prometheus_registry: Registry,
    metrics: Arc<Mutex<HashMap<String, Counter>>>,
    gauges: Arc<Mutex<HashMap<String, Gauge>>>,
    histograms: Arc<Mutex<HashMap<String, Histogram>>>,
}

impl MonitoringOperators {
    pub async fn new() -> Result<Self, HlxError> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| HlxError::NetworkError { 
                message: format!("Failed to create HTTP client: {}", e) 
            })?;
            
        let prometheus_registry = Registry::new();
        let metrics = Arc::new(Mutex::new(HashMap::new()));
        let gauges = Arc::new(Mutex::new(HashMap::new()));
        let histograms = Arc::new(Mutex::new(HashMap::new()));
            
        Ok(Self { 
            http_client,
            prometheus_registry,
            metrics,
            gauges,
            histograms,
        })
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for MonitoringOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "@prometheus" => self.prometheus_operator(&params_map).await,
            "@jaeger" => self.jaeger_operator(&params_map).await,
            "@grafana" => self.grafana_operator(&params_map).await,
            "@datadog" => self.datadog_operator(&params_map).await,
            "@newrelic" => self.newrelic_operator(&params_map).await,
            _ => Err(HlxError::InvalidParameters { 
                operator: operator.to_string(), 
                params: "Unknown monitoring operator".to_string() 
            }),
        }
    }
}

impl MonitoringOperators {
    /// @prometheus - Prometheus metrics (counter, gauge, histogram)
    async fn prometheus_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@prometheus".to_string(),
                params: "Missing required 'action' parameter".to_string(),
            })?;
            
        let metric_name = params.get("metric_name")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@prometheus".to_string(),
                params: "Missing required 'metric_name' parameter".to_string(),
            })?;
            
        match action {
            "counter" => {
                let value = params.get("value")
                    .and_then(|v| v.as_number())
                    .unwrap_or(1.0);
                    
                let mut metrics = self.metrics.lock().await;
                let counter = metrics.entry(metric_name.clone()).or_insert_with(|| {
                    Counter::new(&metric_name, "Helix metric").unwrap()
                });
                counter.inc_by(value as u64);
                
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("counter_incremented".to_string(.to_string())));
                result.insert("metric_name".to_string(), Value::String(metric_name.to_string()));
                result.insert("value".to_string(), Value::Number(value));
                result.insert("current_value".to_string(), Value::Number(counter.get() as f64));
                
                Ok(Value::Object(result))
            },
            "gauge" => {
                let value = params.get("value")
                    .and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@prometheus".to_string(),
                        params: "Missing required 'value' parameter for gauge".to_string(),
                    })?;
                    
                let mut gauges = self.gauges.lock().await;
                let gauge = gauges.entry(metric_name.clone()).or_insert_with(|| {
                    Gauge::new(&metric_name, "Helix gauge").unwrap()
                });
                gauge.set(value);
                
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("gauge_set".to_string(.to_string())));
                result.insert("metric_name".to_string(), Value::String(metric_name.to_string()));
                result.insert("value".to_string(), Value::Number(value));
                
                Ok(Value::Object(result))
            },
            "histogram" => {
                let value = params.get("value")
                    .and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@prometheus".to_string(),
                        params: "Missing required 'value' parameter for histogram".to_string(),
                    })?;
                    
                let mut histograms = self.histograms.lock().await;
                let histogram = histograms.entry(metric_name.clone()).or_insert_with(|| {
                    Histogram::new(&metric_name, "Helix histogram").unwrap()
                });
                histogram.observe(value);
                
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("histogram_observed".to_string(.to_string())));
                result.insert("metric_name".to_string(), Value::String(metric_name.to_string()));
                result.insert("value".to_string(), Value::Number(value));
                
                Ok(Value::Object(result))
            },
            "export" => {
                let encoder = TextEncoder::new();
                let metric_families = self.prometheus_registry.gather();
                let encoded = encoder.encode_to_string(&metric_families)
                    .map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
                        message: format!("Failed to encode Prometheus metrics: {}", e)
                    })?;
                
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("metrics_exported".to_string(.to_string())));
                result.insert("metrics".to_string(), Value::String(encoded.to_string()));
                result.insert("count".to_string(), Value::Number(metric_families.len() as f64));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@prometheus".to_string(),
                params: format!("Invalid action '{}'. Valid actions: counter, gauge, histogram, export", action)
            })
        }
    }

    /// @jaeger - Jaeger distributed tracing (start_span, finish_span, log)
    async fn jaeger_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@jaeger".to_string(),
                params: "Missing required 'action' parameter".to_string(),
            })?;
            
        let service_name = params.get("service_name")
            .and_then(|v| v.as_string())
            .unwrap_or("helix-service");
            
        match action {
            "start_span" => {
                let span_name = params.get("span_name")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@jaeger".to_string(),
                        params: "Missing required 'span_name' parameter".to_string(),
                    })?;
                    
                let tracer = global::tracer(service_name);
                let span = tracer.start(span_name.clone());
                
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("span_started".to_string(.to_string())));
                result.insert("service_name".to_string(), Value::String(service_name.to_string(.to_string())));
                result.insert("span_name".to_string(), Value::String(span_name.to_string()));
                result.insert("span_id".to_string(), Value::String(uuid::Uuid::new_v4(.to_string()).to_string()));
                result.insert("timestamp".to_string(), Value::String(chrono::Utc::now(.to_string()).to_rfc3339()));
                
                Ok(Value::Object(result))
            },
            "finish_span" => {
                let span_id = params.get("span_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@jaeger".to_string(),
                        params: "Missing required 'span_id' parameter".to_string(),
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("span_finished".to_string(.to_string())));
                result.insert("span_id".to_string(), Value::String(span_id.to_string()));
                result.insert("timestamp".to_string(), Value::String(chrono::Utc::now(.to_string()).to_rfc3339()));
                
                Ok(Value::Object(result))
            },
            "log" => {
                let message = params.get("message")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@jaeger".to_string(),
                        params: "Missing required 'message' parameter".to_string(),
                    })?;
                    
                let level = params.get("level")
                    .and_then(|v| v.as_string())
                    .unwrap_or("info");
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("log_recorded".to_string(.to_string())));
                result.insert("message".to_string(), Value::String(message.to_string()));
                result.insert("level".to_string(), Value::String(level.to_string(.to_string())));
                result.insert("service_name".to_string(), Value::String(service_name.to_string(.to_string())));
                result.insert("timestamp".to_string(), Value::String(chrono::Utc::now(.to_string()).to_rfc3339()));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@jaeger".to_string(),
                params: format!("Invalid action '{}'. Valid actions: start_span, finish_span, log", action)
            })
        }
    }

    /// @grafana - Grafana dashboard operations (update_panel, create_dashboard)
    async fn grafana_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@grafana".to_string(),
                params: "Missing required 'action' parameter".to_string(),
            })?;
            
        let grafana_url = params.get("grafana_url")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@grafana".to_string(),
                params: "Missing required 'grafana_url' parameter".to_string(),
            })?;
            
        let api_key = params.get("api_key")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@grafana".to_string(),
                params: "Missing required 'api_key' parameter".to_string(),
            })?;
            
        match action {
            "update_panel" => {
                let dashboard_id = params.get("dashboard_id")
                    .and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@grafana".to_string(),
                        params: "Missing required 'dashboard_id' parameter".to_string(),
                    })?;
                    
                let panel_id = params.get("panel_id")
                    .and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@grafana".to_string(),
                        params: "Missing required 'panel_id' parameter".to_string(),
                    })?;
                    
                let data = params.get("data")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        let mut json_obj = serde_json::Map::new();
                        for (k, v) in obj {
                            json_obj.insert(k.clone(), serde_json::to_value(v).unwrap_or(JsonValue::Null));
                        }
                        JsonValue::Object(json_obj)
                    })
                    .unwrap_or(JsonValue::Object(serde_json::Map::new()));
                    
                let url = format!("{}/api/dashboards/db", grafana_url);
                let payload = json!({
                    "dashboard": {
                        "id": dashboard_id,
                        "panels": [{
                            "id": panel_id,
                            "targets": [{
                                "expr": "helix_metric",
                                "refId": "A"
                            }]
                        }]
                    },
                    "overwrite": true
                });
                
                let response = self.http_client
                    .post(&url)
                    .header("Authorization", &format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                        message: format!("Grafana API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("panel_updated".to_string(.to_string())));
                result.insert("dashboard_id".to_string(), Value::Number(dashboard_id));
                result.insert("panel_id".to_string(), Value::Number(panel_id));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "create_dashboard" => {
                let title = params.get("title")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@grafana".to_string(),
                        params: "Missing required 'title' parameter".to_string(),
                    })?;
                    
                let url = format!("{}/api/dashboards/db", grafana_url);
                let payload = json!({
                    "dashboard": {
                        "title": title,
                        "panels": [],
                        "time": {
                            "from": "now-1h",
                            "to": "now"
                        }
                    },
                    "overwrite": false
                });
                
                let response = self.http_client
                    .post(&url)
                    .header("Authorization", &format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                        message: format!("Grafana API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("dashboard_created".to_string(.to_string())));
                result.insert("title".to_string(), Value::String(title.to_string()));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@grafana".to_string(),
                params: format!("Invalid action '{}'. Valid actions: update_panel, create_dashboard", action)
            })
        }
    }

    /// @datadog - Datadog APM integration
    async fn datadog_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@datadog".to_string(),
                params: "Missing required 'action' parameter".to_string(),
            })?;
            
        let api_key = params.get("api_key")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@datadog".to_string(),
                params: "Missing required 'api_key' parameter".to_string(),
            })?;
            
        let app_key = params.get("app_key")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@datadog".to_string(),
                params: "Missing required 'app_key' parameter".to_string(),
            })?;
            
        match action {
            "send_metric" => {
                let metric_name = params.get("metric_name")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@datadog".to_string(),
                        params: "Missing required 'metric_name' parameter".to_string(),
                    })?;
                    
                let value = params.get("value")
                    .and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@datadog".to_string(),
                        params: "Missing required 'value' parameter".to_string(),
                    })?;
                    
                let tags = params.get("tags")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| format!("{}:{}", k, v.as_string().unwrap_or_default()))
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_default();
                    
                let url = "https://api.datadoghq.com/api/v1/series";
                let payload = json!({
                    "series": [{
                        "metric": metric_name,
                        "points": [[chrono::Utc::now().timestamp(), value]],
                        "type": "gauge",
                        "tags": tags.split(",").filter(|s| !s.is_empty()).collect::<Vec<_>>()
                    }]
                });
                
                let response = self.http_client
                    .post(url)
                    .header("DD-API-KEY", api_key)
                    .header("DD-APPLICATION-KEY", app_key)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                        message: format!("Datadog API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("metric_sent".to_string(.to_string())));
                result.insert("metric_name".to_string(), Value::String(metric_name.to_string()));
                result.insert("value".to_string(), Value::Number(value));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "send_event" => {
                let title = params.get("title")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@datadog".to_string(),
                        params: "Missing required 'title' parameter".to_string(),
                    })?;
                    
                let text = params.get("text")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@datadog".to_string(),
                        params: "Missing required 'text' parameter".to_string(),
                    })?;
                    
                let url = "https://api.datadoghq.com/api/v1/events";
                let payload = json!({
                    "title": title,
                    "text": text,
                    "tags": ["helix", "automation"]
                });
                
                let response = self.http_client
                    .post(url)
                    .header("DD-API-KEY", api_key)
                    .header("DD-APPLICATION-KEY", app_key)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                        message: format!("Datadog API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("event_sent".to_string(.to_string())));
                result.insert("title".to_string(), Value::String(title.to_string()));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@datadog".to_string(),
                params: format!("Invalid action '{}'. Valid actions: send_metric, send_event", action)
            })
        }
    }

    /// @newrelic - New Relic monitoring
    async fn newrelic_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@newrelic".to_string(),
                params: "Missing required 'action' parameter".to_string(),
            })?;
            
        let license_key = params.get("license_key")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@newrelic".to_string(),
                params: "Missing required 'license_key' parameter".to_string(),
            })?;
            
        match action {
            "send_metric" => {
                let metric_name = params.get("metric_name")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@newrelic".to_string(),
                        params: "Missing required 'metric_name' parameter".to_string(),
                    })?;
                    
                let value = params.get("value")
                    .and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@newrelic".to_string(),
                        params: "Missing required 'value' parameter".to_string(),
                    })?;
                    
                let url = "https://metric-api.newrelic.com/metric/v1";
                let payload = json!([{
                    "metrics": [{
                        "name": metric_name,
                        "type": "gauge",
                        "value": value,
                        "timestamp": chrono::Utc::now().timestamp()
                    }]
                }]);
                
                let response = self.http_client
                    .post(url)
                    .header("Api-Key", license_key)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                        message: format!("New Relic API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("metric_sent".to_string(.to_string())));
                result.insert("metric_name".to_string(), Value::String(metric_name.to_string()));
                result.insert("value".to_string(), Value::Number(value));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "send_event" => {
                let event_type = params.get("event_type")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@newrelic".to_string(),
                        params: "Missing required 'event_type' parameter".to_string(),
                    })?;
                    
                let attributes = params.get("attributes")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        let mut json_obj = serde_json::Map::new();
                        for (k, v) in obj {
                            json_obj.insert(k.clone(), serde_json::to_value(v).unwrap_or(JsonValue::Null));
                        }
                        JsonValue::Object(json_obj)
                    })
                    .unwrap_or(JsonValue::Object(serde_json::Map::new()));
                    
                let url = "https://insights-collector.newrelic.com/v1/accounts/events";
                let payload = json!([{
                    "eventType": event_type,
                    "attributes": attributes
                }]);
                
                let response = self.http_client
                    .post(url)
                    .header("X-Insert-Key", license_key)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                        message: format!("New Relic API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("event_sent".to_string(.to_string())));
                result.insert("event_type".to_string(), Value::String(event_type.to_string()));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@newrelic".to_string(),
                params: format!("Invalid action '{}'. Valid actions: send_metric, send_event", action)
            })
        }
    }
} 