//! Server-Sent Events Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use hyper::body::Bytes;
use hyper::http::{Response, StatusCode};
use futures_util::stream::StreamExt;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSEConfig {
    pub retry_interval: u64,
    pub keep_alive_interval: u64,
    pub max_connections: usize,
    pub buffer_size: usize,
    pub enable_compression: bool,
}

impl Default for SSEConfig {
    fn default() -> Self {
        Self {
            retry_interval: 3000,
            keep_alive_interval: 30000,
            max_connections: 1000,
            buffer_size: 1024,
            enable_compression: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SSEConnection {
    pub id: String,
    pub created_at: Instant,
    pub last_event_id: Option<String>,
    pub is_active: bool,
    pub events_sent: u64,
}

pub struct SSEOperator {
    config: SSEConfig,
    connections: Arc<RwLock<HashMap<String, SSEConnection>>>,
    event_channels: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<String>>>>,
    metrics: Arc<Mutex<SSEMetrics>>,
}

#[derive(Debug, Default)]
struct SSEMetrics {
    total_connections: u64,
    active_connections: u32,
    events_sent: u64,
    bytes_sent: u64,
}

impl SSEOperator {
    pub async fn new(config: SSEConfig) -> Result<Self, HlxError> {
        Ok(Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_channels: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(SSEMetrics::default())),
        })
    }

    pub async fn create_connection(&self, connection_id: Option<String>) -> Result<String, HlxError> {
        let conn_id = connection_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let (tx, _rx) = mpsc::unbounded_channel();

        {
            let mut connections = self.connections.write().await;
            connections.insert(conn_id.clone(), SSEConnection {
                id: conn_id.clone(),
                created_at: Instant::now(),
                last_event_id: None,
                is_active: true,
                events_sent: 0,
            });
        }

        {
            let mut channels = self.event_channels.write().await;
            channels.insert(conn_id.clone(), tx);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.total_connections += 1;
            metrics.active_connections += 1;
        }

        info!("SSE connection created: {}", conn_id);
        Ok(conn_id)
    }

    pub async fn send_event(&self, connection_id: &str, event_type: Option<&str>, data: &str, event_id: Option<&str>) -> Result<(), HlxError> {
        let channels = self.event_channels.read().await;
        
        if let Some(channel) = channels.get(connection_id) {
            let mut event_data = String::new();
            
            if let Some(id) = event_id {
                event_data.push_str(&format!("id: {}\n", id));
            }
            
            if let Some(evt_type) = event_type {
                event_data.push_str(&format!("event: {}\n", evt_type));
            }
            
            event_data.push_str(&format!("data: {}\n\n", data));
            event_data.push_str(&format!("retry: {}\n\n", self.config.retry_interval));

            channel.send(event_data).map_err(|_| HlxError::CommunicationError {
                component: "SSE Channel".to_string(),
                message: "Failed to send event".to_string(),
            })?;

            // Update connection metrics
            {
                let mut connections = self.connections.write().await;
                if let Some(conn) = connections.get_mut(connection_id) {
                    conn.events_sent += 1;
                    if let Some(id) = event_id {
                        conn.last_event_id = Some(id.to_string());
                    }
                }
            }

            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.events_sent += 1;
                metrics.bytes_sent += event_data.len() as u64;
            }

            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "SSE Connection".to_string(),
                identifier: connection_id.to_string(),
            })
        }
    }

    pub async fn broadcast(&self, event_type: Option<&str>, data: &str, event_id: Option<&str>) -> Result<u32, HlxError> {
        let connections: Vec<String> = {
            let conns = self.connections.read().await;
            conns.keys().cloned().collect()
        };

        let mut sent_count = 0;
        for conn_id in connections {
            if self.send_event(&conn_id, event_type, data, event_id).await.is_ok() {
                sent_count += 1;
            }
        }

        Ok(sent_count)
    }

    pub async fn close_connection(&self, connection_id: &str) -> Result<(), HlxError> {
        {
            let mut connections = self.connections.write().await;
            connections.remove(connection_id);
        }

        {
            let mut channels = self.event_channels.write().await;
            channels.remove(connection_id);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.active_connections = metrics.active_connections.saturating_sub(1);
        }

        info!("SSE connection closed: {}", connection_id);
        Ok(())
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();
        
        result.insert("total_connections".to_string(), Value::Number(metrics.total_connections as f64));
        result.insert("active_connections".to_string(), Value::Number(metrics.active_connections as f64));
        result.insert("events_sent".to_string(), Value::Number(metrics.events_sent as f64));
        result.insert("bytes_sent".to_string(), Value::Number(metrics.bytes_sent as f64));
        
        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for SSEOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "create_connection" => {
                let connection_id = params_map.get("connection_id").and_then(|v| v.as_string());
                let conn_id = self.create_connection(connection_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(conn_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "send_event" => {
                let connection_id = params_map.get("connection_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("connection_id".to_string()),
                        message: "Missing connection ID".to_string(),
                    })?;

                let data = params_map.get("data")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("data".to_string()),
                        message: "Missing event data".to_string(),
                    })?;

                let event_type = params_map.get("event").and_then(|v| v.as_string());
                let event_id = params_map.get("id").and_then(|v| v.as_string());

                self.send_event(&connection_id, event_type.as_deref(), &data, event_id.as_deref()).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(connection_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "broadcast" => {
                let data = params_map.get("data")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("data".to_string()),
                        message: "Missing event data".to_string(),
                    })?;

                let event_type = params_map.get("event").and_then(|v| v.as_string());
                let event_id = params_map.get("id").and_then(|v| v.as_string());

                let sent_count = self.broadcast(event_type.as_deref(), &data, event_id.as_deref()).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("sent_to_connections".to_string(), Value::Number(sent_count as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "close_connection" => {
                let connection_id = params_map.get("connection_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("connection_id".to_string()),
                        message: "Missing connection ID".to_string(),
                    })?;

                self.close_connection(&connection_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(connection_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }
            
            _ => Err(HlxError::InvalidParameters {
                operator: "sse".to_string(),
                params: format!("Unknown SSE operation: {}", operator),
            }),
        }
    }
} 