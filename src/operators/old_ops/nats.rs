//! NATS Messaging Operator for Helix Rust SDK
//!
//! Provides comprehensive NATS capabilities including:
//! - Publish/Subscribe messaging with subject wildcards
//! - Request/Reply messaging patterns
//! - Queue groups for load balancing
//! - JetStream persistent messaging
//! - Consumer management and acknowledgments
//! - Stream management and configuration
//! - Key-Value store operations
//! - Object store functionality
//! - Connection pooling and fault tolerance

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// NATS operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    /// NATS server URLs
    pub servers: Vec<String>,
    /// Client name
    pub name: Option<String>,
    /// Username for authentication
    pub username: Option<String>,
    /// Password for authentication
    pub password: Option<String>,
    /// Token for authentication
    pub token: Option<String>,
    /// JWT credentials
    pub jwt: Option<String>,
    /// NKey seed for authentication
    pub nkey_seed: Option<String>,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Enable TLS
    pub enable_tls: bool,
    /// TLS certificate path
    pub tls_cert_path: Option<String>,
    /// TLS key path
    pub tls_key_path: Option<String>,
    /// TLS CA certificate path
    pub tls_ca_path: Option<String>,
    /// Max reconnect attempts
    pub max_reconnect_attempts: u32,
    /// Reconnect delay in seconds
    pub reconnect_delay: u64,
    /// Ping interval in seconds
    pub ping_interval: u64,
    /// Enable JetStream
    pub enable_jetstream: bool,
    /// JetStream domain
    pub jetstream_domain: Option<String>,
    /// Connection pool size
    pub pool_size: usize,
    /// Enable drain on close
    pub drain_on_close: bool,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            servers: vec!["nats://127.0.0.1:4222".to_string()],
            name: Some("helix-nats-client".to_string()),
            username: None,
            password: None,
            token: None,
            jwt: None,
            nkey_seed: None,
            connection_timeout: 30,
            request_timeout: 30,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            max_reconnect_attempts: 10,
            reconnect_delay: 2,
            ping_interval: 30,
            enable_jetstream: true,
            jetstream_domain: None,
            pool_size: 10,
            drain_on_close: true,
        }
    }
}

/// NATS message information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsMessage {
    pub subject: String,
    pub payload: Vec<u8>,
    pub reply: Option<String>,
    pub headers: HashMap<String, String>,
    pub timestamp: u64,
}

/// NATS subscription information
#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub id: String,
    pub subject: String,
    pub queue: Option<String>,
    pub message_count: u64,
    pub bytes_received: u64,
    pub created_at: Instant,
    pub is_active: bool,
}

/// JetStream stream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub name: String,
    pub subjects: Vec<String>,
    pub retention_policy: RetentionPolicy,
    pub max_consumers: Option<i64>,
    pub max_msgs: Option<i64>,
    pub max_bytes: Option<i64>,
    pub max_age: Option<Duration>,
    pub storage: StorageType,
    pub replicas: Option<usize>,
    pub no_ack: bool,
    pub template_owner: Option<String>,
    pub discard: DiscardPolicy,
    pub duplicate_window: Option<Duration>,
}

/// JetStream retention policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    Limits,
    Interest,
    WorkQueue,
}

/// JetStream storage types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    File,
    Memory,
}

/// JetStream discard policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscardPolicy {
    Old,
    New,
}

/// JetStream consumer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    pub name: Option<String>,
    pub durable_name: Option<String>,
    pub description: Option<String>,
    pub deliver_policy: DeliverPolicy,
    pub ack_policy: AckPolicy,
    pub ack_wait: Duration,
    pub max_deliver: Option<i64>,
    pub filter_subject: Option<String>,
    pub replay_policy: ReplayPolicy,
    pub rate_limit_bps: Option<u64>,
    pub sample_freq: Option<String>,
    pub max_ack_pending: Option<i64>,
    pub idle_heartbeat: Option<Duration>,
    pub flow_control: bool,
}

/// JetStream delivery policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliverPolicy {
    All,
    Last,
    New,
    ByStartSequence { seq: u64 },
    ByStartTime { time: std::time::SystemTime },
    LastPerSubject,
}

/// JetStream acknowledgment policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AckPolicy {
    None,
    All,
    Explicit,
}

/// JetStream replay policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplayPolicy {
    Instant,
    Original,
}

/// Connection pool entry
#[derive(Debug, Clone)]
struct PooledConnection {
    client: Client,
    jetstream: Option<async_nats::jetstream::Context>,
    created_at: Instant,
    last_used: Instant,
    is_healthy: bool,
    connection_id: String,
}

/// NATS Messaging Operator
pub struct NatsOperator {
    config: NatsConfig,
    connection_pool: Arc<RwLock<Vec<PooledConnection>>>,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
    metrics: Arc<Mutex<NatsMetrics>>,
    message_handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(NatsMessage) + Send + Sync>>>>,
}

/// NATS performance metrics
#[derive(Debug, Default)]
struct NatsMetrics {
    total_connections: u64,
    active_connections: u32,
    failed_connections: u64,
    messages_published: u64,
    messages_received: u64,
    bytes_published: u64,
    bytes_received: u64,
    requests_sent: u64,
    replies_received: u64,
    subscriptions_created: u64,
    avg_publish_latency: f64,
    avg_request_latency: f64,
}

impl NatsOperator {
    /// Create a new NATS operator with configuration
    pub async fn new(_config: NatsConfig) -> Result<Self, HlxError> {
        // NATS functionality is not available without async_nats dependency
        Err(HlxError::InitializationError {
            component: "NATS Operator".to_string(),
            message: "NATS operator requires async_nats dependency which is not available".to_string(),
        })
    }

    /// Publish a message to a subject
    pub async fn publish(&self, _subject: &str, _payload: &[u8], _reply: Option<&str>) -> Result<(), HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "nats".to_string(),
            details: None,
            operation: "publish".to_string(),
            message: "NATS functionality not available - async_nats dependency missing".to_string(),
        })
    }

    /// Subscribe to a subject
    pub async fn subscribe(&self, _subject: &str, _queue_group: Option<&str>) -> Result<String, HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "nats".to_string(),
            details: None,
            operation: "subscribe".to_string(),
            message: "NATS functionality not available - async_nats dependency missing".to_string(),
        })
    }

    /// Send request and wait for reply
    pub async fn request(&self, _subject: &str, _payload: &[u8], _timeout_secs: Option<u64>) -> Result<Vec<u8>, HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "nats".to_string(),
            details: None,
            operation: "request".to_string(),
            message: "NATS functionality not available - async_nats dependency missing".to_string(),
        })
    }

    /// Create JetStream stream
    pub async fn create_stream(&self, _config: &StreamConfig) -> Result<(), HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "nats".to_string(),
            details: None,
            operation: "create_stream".to_string(),
            message: "NATS functionality not available - async_nats dependency missing".to_string(),
        })
    }

    /// Publish to JetStream
    pub async fn jetstream_publish(&self, _subject: &str, _payload: &[u8]) -> Result<String, HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "nats".to_string(),
            details: None,
            operation: "jetstream_publish".to_string(),
            message: "NATS functionality not available - async_nats dependency missing".to_string(),
        })
    }

    /// Get JetStream consumer
    pub async fn get_jetstream_consumer(&self, _stream_name: &str, _consumer_config: &ConsumerConfig) -> Result<String, HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "nats".to_string(),
            details: None,
            operation: "get_jetstream_consumer".to_string(),
            message: "NATS functionality not available - async_nats dependency missing".to_string(),
        })
    }

    /// Get subscription information
    pub async fn get_subscription_info(&self, subscription_id: &str) -> Result<SubscriptionInfo, HlxError> {
        let subscriptions = self.subscriptions.read().await;
        
        subscriptions.get(subscription_id)
            .cloned()
            .ok_or_else(|| HlxError::NotFoundError {
                resource: "NATS Subscription".to_string(),
                identifier: subscription_id.to_string(),
            })
    }

    /// List all subscriptions
    pub async fn list_subscriptions(&self) -> Vec<SubscriptionInfo> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions.values().cloned().collect()
    }

    /// Set message handler for subscription
    pub async fn set_message_handler<F>(&self, subscription_id: String, handler: F)
    where
        F: Fn(NatsMessage) + Send + Sync + 'static,
    {
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(subscription_id, Box::new(handler));
    }


    /// Get NATS metrics
    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();
        
        result.insert("total_connections".to_string(), Value::Number(metrics.total_connections as f64));
        result.insert("active_connections".to_string(), Value::Number(metrics.active_connections as f64));
        result.insert("failed_connections".to_string(), Value::Number(metrics.failed_connections as f64));
        result.insert("messages_published".to_string(), Value::Number(metrics.messages_published as f64));
        result.insert("messages_received".to_string(), Value::Number(metrics.messages_received as f64));
        result.insert("bytes_published".to_string(), Value::Number(metrics.bytes_published as f64));
        result.insert("bytes_received".to_string(), Value::Number(metrics.bytes_received as f64));
        result.insert("requests_sent".to_string(), Value::Number(metrics.requests_sent as f64));
        result.insert("replies_received".to_string(), Value::Number(metrics.replies_received as f64));
        result.insert("subscriptions_created".to_string(), Value::Number(metrics.subscriptions_created as f64));
        result.insert("avg_publish_latency_ms".to_string(), Value::Number(metrics.avg_publish_latency));
        result.insert("avg_request_latency_ms".to_string(), Value::Number(metrics.avg_request_latency));
        
        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for NatsOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "publish" => {
                let subject = params_map.get("subject")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("subject".to_string()),
                        message: "Missing NATS subject".to_string(),
                    })?;

                let payload = params_map.get("payload")
                    .and_then(|v| match v {
                        Value::String(s.to_string()) => Some(s.as_bytes().to_vec()),
                        Value::Array(arr) => {
                            let bytes: Option<Vec<u8>> = arr.iter()
                                .map(|v| v.as_number().map(|n| n as u8))
                                .collect();
                            bytes
                        }
                        _ => None,
                    })
                    .unwrap_or_default();

                let reply = params_map.get("reply")
                    .and_then(|v| v.as_string());

                self.publish(&subject, &payload, reply.as_deref()).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("subject".to_string(), Value::String(subject.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "subscribe" => {
                let subject = params_map.get("subject")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("subject".to_string()),
                        message: "Missing NATS subject".to_string(),
                    })?;

                let queue = params_map.get("queue")
                    .and_then(|v| v.as_string());

                let subscription_id = self.subscribe(&subject, queue.as_deref()).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("subscription_id".to_string(), Value::String(subscription_id.to_string()));
                    map.insert("subject".to_string(), Value::String(subject.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "request" => {
                let subject = params_map.get("subject")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("subject".to_string()),
                        message: "Missing NATS subject".to_string(),
                    })?;

                let payload = params_map.get("payload")
                    .and_then(|v| match v {
                        Value::String(s.to_string()) => Some(s.as_bytes().to_vec()),
                        Value::Array(arr) => {
                            let bytes: Option<Vec<u8>> = arr.iter()
                                .map(|v| v.as_number().map(|n| n as u8))
                                .collect();
                            bytes
                        }
                        _ => None,
                    })
                    .unwrap_or_default();

                let timeout = params_map.get("timeout")
                    .and_then(|v| v.as_number())
                    .map(|n| n as u64);

                let response = self.request(&subject, &payload, timeout).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("subject".to_string(), Value::String(subject.to_string(.to_string())));
                    map.insert("response".to_string(), Value::String(
                        String::from_utf8_lossy(&response).to_string()
                    ));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "jetstream_publish" => {
                let subject = params_map.get("subject")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("subject".to_string()),
                        message: "Missing JetStream subject".to_string(),
                    })?;

                let payload = params_map.get("payload")
                    .and_then(|v| match v {
                        Value::String(s.to_string()) => Some(s.as_bytes().to_vec()),
                        Value::Array(arr) => {
                            let bytes: Option<Vec<u8>> = arr.iter()
                                .map(|v| v.as_number().map(|n| n as u8))
                                .collect();
                            bytes
                        }
                        _ => None,
                    })
                    .unwrap_or_default();

                let ack_info = self.jetstream_publish(&subject, &payload).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("subject".to_string(), Value::String(subject.to_string(.to_string())));
                    map.insert("ack".to_string(), Value::String(ack_info.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "subscription_info" => {
                let subscription_id = params_map.get("subscription_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("subscription_id".to_string()),
                        message: "Missing subscription ID".to_string(),
                    })?;

                let info = self.get_subscription_info(&subscription_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("subscription_id".to_string(), Value::String(info.id.to_string()));
                    map.insert("subject".to_string(), Value::String(info.subject.to_string()));
                    map.insert("message_count".to_string(), Value::Number(info.message_count as f64));
                    map.insert("bytes_received".to_string(), Value::Number(info.bytes_received as f64));
                    map.insert("is_active".to_string(), Value::Boolean(info.is_active));
                    map.insert("uptime_seconds".to_string(), Value::Number(info.created_at.elapsed().as_secs() as f64));
                    if let Some(queue) = info.queue {
                        map.insert("queue".to_string(), Value::String(queue.to_string()));
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "list_subscriptions" => {
                let subscriptions = self.list_subscriptions().await;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("subscriptions".to_string(), Value::Array(
                        subscriptions.clone().into_iter().map(|info| Value::String(info.id.to_string())).collect()
                    ));
                    map.insert("count".to_string(), Value::Number(subscriptions.len() as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }
            
            _ => Err(HlxError::InvalidParameters {
                operator: "nats".to_string(),
                params: format!("Unknown NATS operation: {}", operator),
            }),
        }
    }
} 