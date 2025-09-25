//! Apache Kafka Operator for Helix Rust SDK
//!
//! Provides comprehensive Apache Kafka capabilities including:
//! - High-throughput producer with batching and compression
//! - Consumer groups with partition assignment and rebalancing
//! - Offset management and exactly-once semantics
//! - Schema registry integration
//! - Transactional messaging support
//! - Dead letter queue handling
//! - Performance monitoring and metrics
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

/// Kafka operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    /// Kafka bootstrap servers
    pub bootstrap_servers: String,
    /// Client ID
    pub client_id: String,
    /// Security protocol
    pub security_protocol: SecurityProtocol,
    /// SASL mechanism
    pub sasl_mechanism: Option<SaslMechanism>,
    /// SASL username
    pub sasl_username: Option<String>,
    /// SASL password
    pub sasl_password: Option<String>,
    /// SSL CA certificate location
    pub ssl_ca_location: Option<String>,
    /// SSL certificate location
    pub ssl_certificate_location: Option<String>,
    /// SSL key location
    pub ssl_key_location: Option<String>,
    /// Enable SSL certificate verification
    pub enable_ssl_certificate_verification: bool,
    /// Producer configuration
    pub producer_config: ProducerConfig,
    /// Consumer configuration
    pub consumer_config: ConsumerConfig,
    /// Schema registry URL
    pub schema_registry_url: Option<String>,
    /// Schema registry authentication
    pub schema_registry_auth: Option<SchemaRegistryAuth>,
    /// Enable transactions
    pub enable_transactions: bool,
    /// Transaction timeout in milliseconds
    pub transaction_timeout_ms: u64,
    /// Enable idempotence
    pub enable_idempotence: bool,
    /// Debug logging contexts
    pub debug_contexts: Vec<String>,
}

/// Security protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityProtocol {
    Plaintext,
    Ssl,
    SaslPlaintext,
    SaslSsl,
}

/// SASL mechanisms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SaslMechanism {
    Plain,
    ScramSha256,
    ScramSha512,
    Gssapi,
    OAuthBearer,
}

/// Producer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerConfig {
    /// Acknowledgment configuration
    pub acks: AckConfig,
    /// Retries
    pub retries: i32,
    /// Retry backoff in milliseconds
    pub retry_backoff_ms: u64,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Delivery timeout in milliseconds
    pub delivery_timeout_ms: u64,
    /// Batch size in bytes
    pub batch_size: i32,
    /// Linger time in milliseconds
    pub linger_ms: u64,
    /// Buffer memory in bytes
    pub buffer_memory: u64,
    /// Compression type
    pub compression_type: CompressionType,
    /// Max in flight requests per connection
    pub max_in_flight_requests_per_connection: i32,
    /// Enable batching
    pub enable_batching: bool,
}

/// Consumer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    /// Group ID
    pub group_id: String,
    /// Auto offset reset
    pub auto_offset_reset: OffsetResetPolicy,
    /// Enable auto commit
    pub enable_auto_commit: bool,
    /// Auto commit interval in milliseconds
    pub auto_commit_interval_ms: u64,
    /// Session timeout in milliseconds
    pub session_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Max poll interval in milliseconds
    pub max_poll_interval_ms: u64,
    /// Max poll records
    pub max_poll_records: i32,
    /// Fetch min bytes
    pub fetch_min_bytes: i32,
    /// Fetch max wait in milliseconds
    pub fetch_max_wait_ms: u64,
    /// Max partition fetch bytes
    pub max_partition_fetch_bytes: i32,
}

/// Acknowledgment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AckConfig {
    None,        // 0
    Leader,      // 1
    All,         // -1 or "all"
}

/// Compression types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}

/// Offset reset policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OffsetResetPolicy {
    Earliest,
    Latest,
    None,
}

/// Schema registry authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistryAuth {
    pub username: String,
    pub password: String,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            bootstrap_servers: "localhost:9092".to_string(),
            client_id: "helix-kafka-client".to_string(),
            security_protocol: SecurityProtocol::Plaintext,
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
            ssl_ca_location: None,
            ssl_certificate_location: None,
            ssl_key_location: None,
            enable_ssl_certificate_verification: true,
            producer_config: ProducerConfig::default(),
            consumer_config: ConsumerConfig::default(),
            schema_registry_url: None,
            schema_registry_auth: None,
            enable_transactions: false,
            transaction_timeout_ms: 60000,
            enable_idempotence: true,
            debug_contexts: vec!["broker".to_string(), "topic".to_string()],
        }
    }
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            acks: AckConfig::All,
            retries: 2147483647, // Max retries
            retry_backoff_ms: 100,
            request_timeout_ms: 30000,
            delivery_timeout_ms: 120000,
            batch_size: 16384,
            linger_ms: 0,
            buffer_memory: 33554432, // 32MB
            compression_type: CompressionType::None,
            max_in_flight_requests_per_connection: 5,
            enable_batching: true,
        }
    }
}

impl Default for ConsumerConfig {
    fn default() -> Self {
        Self {
            group_id: "helix-consumer-group".to_string(),
            auto_offset_reset: OffsetResetPolicy::Latest,
            enable_auto_commit: true,
            auto_commit_interval_ms: 5000,
            session_timeout_ms: 45000,
            heartbeat_interval_ms: 3000,
            max_poll_interval_ms: 300000,
            max_poll_records: 500,
            fetch_min_bytes: 1,
            fetch_max_wait_ms: 500,
            max_partition_fetch_bytes: 1048576, // 1MB
        }
    }
}

/// Kafka message information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaMessage {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<Vec<u8>>,
    pub payload: Vec<u8>,
    pub headers: HashMap<String, Vec<u8>>,
    pub timestamp: Option<i64>,
    pub timestamp_type: Option<String>,
}

/// Producer result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProduceResult {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub timestamp: Option<i64>,
}

/// Consumer group information
#[derive(Debug, Clone)]
pub struct ConsumerInfo {
    pub consumer_id: String,
    pub group_id: String,
    pub topics: Vec<String>,
    pub assigned_partitions: Vec<(String, i32)>,
    pub messages_consumed: u64,
    pub bytes_consumed: u64,
    pub last_commit_time: Option<Instant>,
    pub is_active: bool,
}

/// Context for custom callbacks
struct HelixKafkaContext {
    metrics: Arc<Mutex<KafkaMetrics>>,
}

/// Kafka performance metrics
#[derive(Debug, Default)]
struct KafkaMetrics {
    // Producer metrics
    messages_produced: u64,
    bytes_produced: u64,
    produce_errors: u64,
    avg_produce_latency: f64,
    
    // Consumer metrics
    messages_consumed: u64,
    bytes_consumed: u64,
    consume_errors: u64,
    consumer_lag: i64,
    
    // Connection metrics
    active_producers: u32,
    active_consumers: u32,
    connection_failures: u64,
    
    // Topic metrics
    topic_metadata_updates: u64,
    partition_assignments: u64,
}

impl KafkaMetrics {
    fn update_from_statistics(&mut self, _stats: &()) {
        // Kafka statistics not available without rdkafka dependency
        self.topic_metadata_updates += 1;
    }
}

/// Apache Kafka Operator
pub struct KafkaOperator {
    config: KafkaConfig,
    consumers: Arc<RwLock<HashMap<String, ConsumerInfo>>>,
    metrics: Arc<Mutex<KafkaMetrics>>,
    message_handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(KafkaMessage) + Send + Sync>>>>,
}

impl KafkaOperator {
    /// Create a new Kafka operator with configuration
    pub async fn new(config: KafkaConfig) -> Result<Self, HlxError> {
        let metrics = Arc::new(Mutex::new(KafkaMetrics::default()));

        let operator = Self {
            config: config.clone(),
            consumers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::clone(&metrics),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
        };

        // Kafka functionality is not available without rdkafka dependency
        Err(HlxError::InitializationError {
            component: "Kafka Operator".to_string(),
            message: "Kafka operator requires rdkafka dependency which is not available".to_string(),
        })
    }

    /// Produce a message to a topic
    pub async fn produce(
        &self,
        _topic: &str,
        _key: Option<&[u8]>,
        _payload: &[u8],
        _headers: Option<HashMap<String, Vec<u8>>>,
        _partition: Option<i32>,
    ) -> Result<ProduceResult, HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "kafka".to_string(),
            details: None,
            operation: "produce".to_string(),
            message: "Kafka functionality not available - rdkafka dependency missing".to_string(),
        })
    }

    /// Start a consumer for the given topics
    pub async fn start_consumer(&self, _consumer_id: String, _topics: Vec<String>, _group_id: Option<String>) -> Result<String, HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "kafka".to_string(),
            details: None,
            operation: "start_consumer".to_string(),
            message: "Kafka functionality not available - rdkafka dependency missing".to_string(),
        })
    }

    /// Create a topic
    pub async fn create_topic(
        &self,
        _topic_name: &str,
        _partitions: i32,
        _replication_factor: i32,
        _config: Option<HashMap<String, String>>,
    ) -> Result<(), HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "kafka".to_string(),
            details: None,
            operation: "create_topic".to_string(),
            message: "Kafka functionality not available - rdkafka dependency missing".to_string(),
        })
    }

    /// Delete a topic
    pub async fn delete_topic(&self, _topic_name: &str) -> Result<(), HlxError> {
        Err(HlxError::OperationError { operator: "unknown".to_string(),
            operator: "kafka".to_string(),
            details: None,
            operation: "delete_topic".to_string(),
            message: "Kafka functionality not available - rdkafka dependency missing".to_string(),
        })
    }

    /// Stop a consumer
    pub async fn stop_consumer(&self, consumer_id: &str) -> Result<(), HlxError> {
        let mut consumers = self.consumers.write().await;
        
        if let Some(mut consumer_info) = consumers.get_mut(consumer_id) {
            consumer_info.is_active = false;
            
            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.active_consumers = metrics.active_consumers.saturating_sub(1);
            }

            info!("Stopped Kafka consumer: {}", consumer_id);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Kafka Consumer".to_string(),
                identifier: consumer_id.to_string(),
            })
        }
    }

    /// Get consumer information
    pub async fn get_consumer_info(&self, consumer_id: &str) -> Result<ConsumerInfo, HlxError> {
        let consumers = self.consumers.read().await;
        
        consumers.get(consumer_id)
            .cloned()
            .ok_or_else(|| HlxError::NotFoundError {
                resource: "Kafka Consumer".to_string(),
                identifier: consumer_id.to_string(),
            })
    }

    /// List all consumers
    pub async fn list_consumers(&self) -> Vec<ConsumerInfo> {
        let consumers = self.consumers.read().await;
        consumers.values().cloned().collect()
    }

    /// Set message handler for consumer
    pub async fn set_message_handler<F>(&self, consumer_id: String, handler: F)
    where
        F: Fn(KafkaMessage) + Send + Sync + 'static,
    {
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(consumer_id, Box::new(handler));
    }


    /// Get Kafka metrics
    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();
        
        // Producer metrics
        result.insert("messages_produced".to_string(), Value::Number(metrics.messages_produced as f64));
        result.insert("bytes_produced".to_string(), Value::Number(metrics.bytes_produced as f64));
        result.insert("produce_errors".to_string(), Value::Number(metrics.produce_errors as f64));
        result.insert("avg_produce_latency_ms".to_string(), Value::Number(metrics.avg_produce_latency));
        
        // Consumer metrics
        result.insert("messages_consumed".to_string(), Value::Number(metrics.messages_consumed as f64));
        result.insert("bytes_consumed".to_string(), Value::Number(metrics.bytes_consumed as f64));
        result.insert("consume_errors".to_string(), Value::Number(metrics.consume_errors as f64));
        result.insert("consumer_lag".to_string(), Value::Number(metrics.consumer_lag as f64));
        
        // Connection metrics
        result.insert("active_producers".to_string(), Value::Number(metrics.active_producers as f64));
        result.insert("active_consumers".to_string(), Value::Number(metrics.active_consumers as f64));
        result.insert("connection_failures".to_string(), Value::Number(metrics.connection_failures as f64));
        
        // Topic metrics
        result.insert("topic_metadata_updates".to_string(), Value::Number(metrics.topic_metadata_updates as f64));
        result.insert("partition_assignments".to_string(), Value::Number(metrics.partition_assignments as f64));
        
        // Calculate throughput rates
        if metrics.messages_produced > 0 {
            let produce_rate = metrics.bytes_produced as f64 / metrics.messages_produced as f64;
            result.insert("avg_message_size_produced".to_string(), Value::Number(produce_rate));
        }
        
        if metrics.messages_consumed > 0 {
            let consume_rate = metrics.bytes_consumed as f64 / metrics.messages_consumed as f64;
            result.insert("avg_message_size_consumed".to_string(), Value::Number(consume_rate));
        }
        
        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for KafkaOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "produce" => {
                let topic = params_map.get("topic")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("topic".to_string()),
                        message: "Missing Kafka topic".to_string(),
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

                let key = params_map.get("key")
                    .and_then(|v| v.as_string())
                    .map(|s| s.as_bytes().to_vec());

                let partition = params_map.get("partition")
                    .and_then(|v| v.as_number())
                    .map(|n| n as i32);

                let headers = params_map.get("headers")
                    .and_then(|v| {
                        if let Value::Object(obj) = v {
                            let mut header_map = HashMap::new();
                            for (k, v) in obj {
                                if let Some(val_str) = v.as_string() {
                                    header_map.insert(k.clone(), val_str.as_bytes().to_vec());
                                }
                            }
                            Some(header_map)
                        } else {
                            None
                        }
                    });

                let result = self.produce(&topic, key.as_deref(), &payload, headers, partition).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("topic".to_string(), Value::String(result.topic.to_string()));
                    map.insert("partition".to_string(), Value::Number(result.partition as f64));
                    map.insert("offset".to_string(), Value::Number(result.offset as f64));
                    if let Some(timestamp) = result.timestamp {
                        map.insert("timestamp".to_string(), Value::Number(timestamp as f64));
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "start_consumer" => {
                let consumer_id = params_map.get("consumer_id")
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| format!("consumer-{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis()));

                let topics = params_map.get("topics")
                    .and_then(|v| {
                        if let Value::Array(arr) = v {
                            let topics: Option<Vec<String>> = arr.iter()
                                .map(|v| v.as_string())
                                .collect();
                            topics
                        } else if let Some(topic) = v.as_string() {
                            Some(vec![topic])
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("topics".to_string()),
                        message: "Missing topics list".to_string(),
                    })?;

                let group_id = params_map.get("group_id")
                    .and_then(|v| v.as_string());

                let actual_consumer_id = self.start_consumer(consumer_id.clone(), topics, group_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("consumer_id".to_string(), Value::String(actual_consumer_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "stop_consumer" => {
                let consumer_id = params_map.get("consumer_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("consumer_id".to_string()),
                        message: "Missing consumer ID".to_string(),
                    })?;

                self.stop_consumer(&consumer_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("consumer_id".to_string(), Value::String(consumer_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "consumer_info" => {
                let consumer_id = params_map.get("consumer_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("consumer_id".to_string()),
                        message: "Missing consumer ID".to_string(),
                    })?;

                let info = self.get_consumer_info(&consumer_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("consumer_id".to_string(), Value::String(info.consumer_id.to_string()));
                    map.insert("group_id".to_string(), Value::String(info.group_id.to_string()));
                    map.insert("topics".to_string(), Value::Array(
                        info.topics.into_iter().map(Value::String).collect()
                    ));
                    map.insert("messages_consumed".to_string(), Value::Number(info.messages_consumed as f64));
                    map.insert("bytes_consumed".to_string(), Value::Number(info.bytes_consumed as f64));
                    map.insert("is_active".to_string(), Value::Boolean(info.is_active));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "list_consumers" => {
                let consumers = self.list_consumers().await;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("consumers".to_string(), Value::Array(
                        consumers.into_iter().map(|info| Value::String(info.consumer_id.to_string())).collect()
                    ));
                    map.insert("count".to_string(), Value::Number(consumers.len() as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "create_topic" => {
                let topic_name = params_map.get("topic")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("topic".to_string()),
                        message: "Missing topic name".to_string(),
                    })?;

                let partitions = params_map.get("partitions")
                    .and_then(|v| v.as_number())
                    .unwrap_or(1.0) as i32;

                let replication_factor = params_map.get("replication_factor")
                    .and_then(|v| v.as_number())
                    .unwrap_or(1.0) as i32;

                let config = params_map.get("config")
                    .and_then(|v| {
                        if let Value::Object(obj) = v {
                            let mut cfg = HashMap::new();
                            for (k, v) in obj {
                                if let Some(val_str) = v.as_string() {
                                    cfg.insert(k.clone(), val_str);
                                }
                            }
                            Some(cfg)
                        } else {
                            None
                        }
                    });

                self.create_topic(&topic_name, partitions, replication_factor, config).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("topic".to_string(), Value::String(topic_name.to_string()));
                    map.insert("partitions".to_string(), Value::Number(partitions as f64));
                    map.insert("replication_factor".to_string(), Value::Number(replication_factor as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "delete_topic" => {
                let topic_name = params_map.get("topic")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("topic".to_string()),
                        message: "Missing topic name".to_string(),
                    })?;

                self.delete_topic(&topic_name).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("topic".to_string(), Value::String(topic_name.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }
            
            _ => Err(HlxError::InvalidParameters {
                operator: "kafka".to_string(),
                params: format!("Unknown Kafka operation: {}", operator),
            }),
        }
    }
} 