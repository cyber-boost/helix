//! AMQP/RabbitMQ Messaging Operator for Helix Rust SDK
use crate::error::{HlxError,ExecutionError};
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use futures_util::stream::StreamExt;
use lapin::{
    options::*,
    types::FieldTable,
    BasicProperties,
    Channel,
    Connection,
    ConnectionProperties,
    Consumer,
    ExchangeKind,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMQPConfig {
    pub connection_string: String,
    pub vhost: String,
    pub heartbeat: u16,
    pub channel_max: u16,
    pub frame_max: u32,
    pub connection_timeout: u64,
    pub enable_ssl: bool,
    pub ssl_verify: bool,
    pub prefetch_count: u16,
}

impl Default for AMQPConfig {
    fn default() -> Self {
        Self {
            connection_string: "amqp://guest:guest@localhost:5672".to_string(),
            vhost: "/".to_string(),
            heartbeat: 60,
            channel_max: 2047,
            frame_max: 131072,
            connection_timeout: 30,
            enable_ssl: false,
            ssl_verify: true,
            prefetch_count: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub exchange_type: ExchangeType,
    pub durable: bool,
    pub auto_delete: bool,
    pub internal: bool,
    pub arguments: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExchangeType {
    Direct,
    Fanout,
    Topic,
    Headers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub name: String,
    pub durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
    pub arguments: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub body: Vec<u8>,
    pub properties: MessageProperties,
    pub routing_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageProperties {
    pub content_type: Option<String>,
    pub content_encoding: Option<String>,
    pub delivery_mode: Option<u8>,
    pub priority: Option<u8>,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
    pub expiration: Option<String>,
    pub message_id: Option<String>,
    pub timestamp: Option<u64>,
    pub user_id: Option<String>,
    pub app_id: Option<String>,
    pub headers: HashMap<String, JsonValue>,
}

#[derive(Debug, Default)]
struct AMQPMetrics {
    messages_published: u64,
    messages_consumed: u64,
    exchanges_declared: u64,
    queues_declared: u64,
    queue_bindings: u64,
    connections_opened: u64,
    channels_opened: u64,
    avg_publish_time: f64,
}

#[derive(Debug, Clone)]
struct QueueStats {
    message_count: u32,
    consumer_count: u32,
    cached_at: Instant,
}

#[derive(Debug)]
struct QueueStatsCache {
    stats: HashMap<String, QueueStats>,
    ttl: Duration,
}

pub struct AMQPOperator {
    config: AMQPConfig,
    metrics: Arc<Mutex<AMQPMetrics>>,
    connection: Arc<Mutex<Option<Connection>>>,
    channel: Arc<Mutex<Option<Channel>>>,
    exchanges: Arc<RwLock<HashMap<String, ExchangeConfig>>>,
    queues: Arc<RwLock<HashMap<String, QueueConfig>>>,
    consumers: Arc<RwLock<HashMap<String, Consumer>>>,
    queue_stats_cache: Arc<Mutex<QueueStatsCache>>,
}

impl AMQPOperator {
    pub async fn new(config: AMQPConfig) -> Result<Self, HlxError> {
        // Establish real AMQP connection
        let connection = Connection::connect(
            &config.connection_string,
            ConnectionProperties::default()
        ).await
        .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to connect to AMQP broker: {}", e))))?;

        // Create a channel
        let channel = connection.create_channel().await
        .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to create AMQP channel: {}", e))))?;

        // Update metrics
        let mut metrics = AMQPMetrics::default();
        metrics.connections_opened = 1;
        metrics.channels_opened = 1;

        let queue_stats_cache = QueueStatsCache {
            stats: HashMap::new(),
            ttl: Duration::from_secs(30), // 30 second cache TTL
        };

        Ok(Self {
            config,
            metrics: Arc::new(Mutex::new(metrics)),
            connection: Arc::new(Mutex::new(Some(connection))),
            channel: Arc::new(Mutex::new(Some(channel))),
            exchanges: Arc::new(RwLock::new(HashMap::new())),
            queues: Arc::new(RwLock::new(HashMap::new())),
            consumers: Arc::new(RwLock::new(HashMap::new())),
            queue_stats_cache: Arc::new(Mutex::new(queue_stats_cache)),
        })
    }

    pub async fn declare_exchange(&self, exchange: ExchangeConfig) -> Result<(), HlxError> {
        let channel = self.channel.lock().unwrap();
        if let Some(ref channel) = *channel {
            // Convert ExchangeType to ExchangeKind
            let exchange_kind = match exchange.exchange_type {
                ExchangeType::Direct => ExchangeKind::Direct,
                ExchangeType::Fanout => ExchangeKind::Fanout,
                ExchangeType::Topic => ExchangeKind::Topic,
                ExchangeType::Headers => ExchangeKind::Headers,
            };

            // Convert arguments HashMap to FieldTable
            let arguments = FieldTable::default(); // For now, skip complex argument handling

            // Declare the exchange
            channel
                .exchange_declare(
                    &exchange.name,
                    exchange_kind,
                    ExchangeDeclareOptions {
                        durable: exchange.durable,
                        auto_delete: exchange.auto_delete,
                        internal: exchange.internal,
                        ..Default::default()
                    },
                    arguments,
                )
                .await
                .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to declare exchange: {}", e))))?;
        } else {
            return Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available")));
        }

        // Store exchange config locally for tracking
        {
            let mut exchanges = self.exchanges.write().await;
            exchanges.insert(exchange.name.clone(), exchange);
        }

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.exchanges_declared += 1;
        }

        Ok(())
    }

    pub async fn declare_queue(&self, queue: QueueConfig) -> Result<String, HlxError> {
        let channel = self.channel.lock().unwrap();
        if let Some(ref channel) = *channel {
            // Convert arguments HashMap to FieldTable
            let arguments = FieldTable::default(); // For now, skip complex argument handling

            // Declare the queue
            let queue_name = if queue.name.is_empty() {
                // Empty name means server-generated queue name
                String::new()
            } else {
                queue.name.clone()
            };

            let declare_options = QueueDeclareOptions {
                durable: queue.durable,
                exclusive: queue.exclusive,
                auto_delete: queue.auto_delete,
                ..Default::default()
            };

            let result = channel
                .queue_declare(&queue_name, declare_options, arguments)
                .await
                .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to declare queue: {}", e))))?;

            let actual_queue_name = result.name().to_string();

            // Store queue config locally for tracking
            {
                let mut queues = self.queues.write().await;
                let mut queue_config = queue;
                queue_config.name = actual_queue_name.clone();
                queues.insert(actual_queue_name.clone(), queue_config);
            }

            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.queues_declared += 1;
            }

            Ok(actual_queue_name)
        } else {
            Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available")))
        }
    }

    pub async fn bind_queue(&self, queue: &str, exchange: &str, routing_key: &str) -> Result<(), HlxError> {
        let channel = self.channel.lock().unwrap();
        if let Some(ref channel) = *channel {
            // Bind the queue to the exchange
            channel
                .queue_bind(queue, exchange, routing_key, QueueBindOptions::default(), FieldTable::default())
                .await
                .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to bind queue to exchange: {}", e))))?;

            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.queue_bindings += 1;
            }

            Ok(())
        } else {
            Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available")))
        }
    }

    pub async fn publish_message(&self, exchange: &str, routing_key: &str, message: Message) -> Result<(), HlxError> {
        let start_time = Instant::now();
        let channel = self.channel.lock().unwrap();

        if let Some(ref channel) = *channel {
            // Build AMQP BasicProperties from MessageProperties
            let mut properties = BasicProperties::default();

            if let Some(ref content_type) = message.properties.content_type {
                properties = properties.with_content_type(content_type.clone().into());
            }

            if let Some(ref content_encoding) = message.properties.content_encoding {
                properties = properties.with_content_encoding(content_encoding.clone().into());
            }

            if let Some(delivery_mode) = message.properties.delivery_mode {
                properties = properties.with_delivery_mode(delivery_mode);
            }

            if let Some(priority) = message.properties.priority {
                properties = properties.with_priority(priority);
            }

            if let Some(ref correlation_id) = message.properties.correlation_id {
                properties = properties.with_correlation_id(correlation_id.clone().into());
            }

            if let Some(ref reply_to) = message.properties.reply_to {
                properties = properties.with_reply_to(reply_to.clone().into());
            }

            if let Some(ref expiration) = message.properties.expiration {
                properties = properties.with_expiration(expiration.clone().into());
            }

            if let Some(ref message_id) = message.properties.message_id {
                properties = properties.with_message_id(message_id.clone().into());
            }

            if let Some(timestamp) = message.properties.timestamp {
                properties = properties.with_timestamp(timestamp);
            }

            if let Some(ref user_id) = message.properties.user_id {
                properties = properties.with_user_id(user_id.clone().into());
            }

            if let Some(ref app_id) = message.properties.app_id {
                properties = properties.with_app_id(app_id.clone().into());
            }

            // Convert headers to FieldTable
            let mut headers = FieldTable::default();
            for (key, value) in &message.properties.headers {
                // Simple conversion - in production you'd want proper type conversion
                if let Some(s) = value.as_str() {
                    headers.insert(key.clone().into(), s.to_string().into());
                }
            }
            properties = properties.with_headers(headers);

            // Publish the message
            channel
                .basic_publish(
                    exchange,
                    routing_key,
                    BasicPublishOptions::default(),
                    &message.body,
                    properties,
                )
                .await
                .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to publish message: {}", e))))?;

            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.messages_published += 1;
                let publish_time = start_time.elapsed().as_millis() as f64;
                metrics.avg_publish_time = (metrics.avg_publish_time * (metrics.messages_published - 1) as f64 + publish_time) / metrics.messages_published as f64;
            }

            Ok(())
        } else {
            Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available")))
        }
    }

    pub async fn consume_message(&self, queue: &str) -> Result<Option<Message>, HlxError> {
        let channel = self.channel.lock().unwrap();

        if let Some(ref channel) = *channel {
            // Check if we already have a consumer for this queue
            let consumer_key = format!("consumer_{}", queue);
            let mut consumers = self.consumers.write().await;

            if !consumers.contains_key(&consumer_key) {
                // Create a new consumer for this queue
                let consumer = channel
                    .basic_consume(
                        queue,
                        &consumer_key,
                        BasicConsumeOptions::default(),
                        FieldTable::default(),
                    )
                    .await
                    .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to create consumer for queue {}: {}", queue, e))))?;

                consumers.insert(consumer_key.clone(), consumer);
            }

            if let Some(ref mut consumer) = consumers.get_mut(&consumer_key) {
                // Try to get a message with a short timeout
                match tokio::time::timeout(
                    tokio::time::Duration::from_millis(100),
                    consumer.next()
                ).await {
                    Ok(Some(delivery_result)) => {
                        match delivery_result {
                            Ok(delivery) => {
                                // Convert AMQP delivery to our Message struct
                                let properties = delivery.properties;

                                let message_properties = MessageProperties {
                                    content_type: properties.content_type().map(|ct| ct.to_string()),
                                    content_encoding: properties.content_encoding().map(|ce| ce.to_string()),
                                    delivery_mode: properties.delivery_mode(),
                                    priority: properties.priority(),
                                    correlation_id: properties.correlation_id().map(|ci| ci.to_string()),
                                    reply_to: properties.reply_to().map(|rt| rt.to_string()),
                                    expiration: properties.expiration().map(|exp| exp.to_string()),
                                    message_id: properties.message_id().map(|mi| mi.to_string()),
                                    timestamp: properties.timestamp(),
                                    user_id: properties.user_id().map(|ui| ui.to_string()),
                                    app_id: properties.app_id().map(|ai| ai.to_string()),
                                    headers: HashMap::new(), // TODO: Convert FieldTable to HashMap
                                };

                                let message = Message {
                                    body: delivery.data,
                                    properties: message_properties,
                                    routing_key: delivery.routing_key.to_string(),
                                };

                                // Acknowledge the message
                                delivery
                                    .ack(BasicAckOptions::default())
                                    .await
                                    .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to acknowledge message: {}", e))))?;

                                // Update metrics
                                {
                                    let mut metrics = self.metrics.lock().unwrap();
                                    metrics.messages_consumed += 1;
                                }

                                Ok(Some(message))
                            },
                            Err(e) => {
                                Err(HlxError::ExecutionError(ExecutionError::new(&format!("Failed to receive message: {}", e))))
                            }
                        }
                    },
                    Ok(None) => {
                        // No message available
                        Ok(None)
                    },
                    Err(_) => {
                        // Timeout - no message available
                        Ok(None)
                    }
                }
            } else {
                Err(HlxError::ExecutionError(ExecutionError::new("Failed to access consumer")))
            }
        } else {
            Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available")))
        }
    }

    pub async fn delete_queue(&self, queue: &str) -> Result<u32, HlxError> {
        let channel = self.channel.lock().unwrap();

        if let Some(ref channel) = *channel {
            // Delete the queue and get the message count
            let delete_result = channel
                .queue_delete(queue, QueueDeleteOptions::default())
                .await
                .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to delete queue {}: {}", queue, e))))?;

            let message_count = delete_result.message_count();

            // Remove from local tracking
            {
                let mut queues = self.queues.write().await;
                queues.remove(queue);
            }

            // Remove any associated consumer
            let consumer_key = format!("consumer_{}", queue);
            {
                let mut consumers = self.consumers.write().await;
                consumers.remove(&consumer_key);
            }

            Ok(message_count)
        } else {
            Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available")))
        }
    }

    pub async fn delete_exchange(&self, exchange: &str) -> Result<(), HlxError> {
        let channel = self.channel.lock().unwrap();

        if let Some(ref channel) = *channel {
            // Delete the exchange
            channel
                .exchange_delete(exchange, ExchangeDeleteOptions::default())
                .await
                .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to delete exchange {}: {}", exchange, e))))?;

            // Remove from local tracking
            {
                let mut exchanges = self.exchanges.write().await;
                exchanges.remove(exchange);
            }

            Ok(())
        } else {
            Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available")))
        }
    }

    pub async fn purge_queue(&self, queue: &str) -> Result<u32, HlxError> {
        let channel = self.channel.lock().unwrap();

        if let Some(ref channel) = *channel {
            // Purge the queue and get the message count
            let purge_result = channel
                .queue_purge(queue, QueuePurgeOptions::default())
                .await
                .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!("Failed to purge queue {}: {}", queue, e))))?;

            let message_count = purge_result.message_count();

            Ok(message_count)
        } else {
            Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available")))
        }
    }

    /// Get real-time message count for a queue with caching
    pub async fn get_queue_message_count(&self, queue: &str) -> Result<u32, HlxError> {
        // Check cache first
        {
            let mut cache = self.queue_stats_cache.lock().unwrap();
            if let Some(stats) = cache.stats.get(queue) {
                if stats.cached_at.elapsed() < cache.ttl {
                    return Ok(stats.message_count);
                }
            }
        }

        // Cache miss or expired - fetch from broker
        let channel = self.channel.lock().unwrap();
        if let Some(ref channel) = *channel {
            // Use queue_declare with passive=true to get current queue info without creating/modifying
            let declare_result = channel
                .queue_declare(queue, QueueDeclareOptions {
                    passive: true, // Don't create, just inspect existing queue
                    ..Default::default()
                }, FieldTable::default())
                .await
                .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!(
                    "Failed to get queue stats for {}: {}. Ensure queue exists.", queue, e
                ))))?;

            let message_count = declare_result.message_count();
            let consumer_count = declare_result.consumer_count();

            // Update cache
            {
                let mut cache = self.queue_stats_cache.lock().unwrap();
                cache.stats.insert(queue.to_string(), QueueStats {
                    message_count,
                    consumer_count,
                    cached_at: Instant::now(),
                });
            }

            Ok(message_count)
        } else {
            Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available - broker may be unreachable")))
        }
    }

    /// Get comprehensive queue statistics
    pub async fn get_queue_stats(&self, queue: &str) -> Result<HashMap<String, Value>, HlxError> {
        let channel = self.channel.lock().unwrap();
        if let Some(ref channel) = *channel {
            // Use queue_declare with passive=true to inspect queue
            let declare_result = channel
                .queue_declare(queue, QueueDeclareOptions {
                    passive: true,
                    ..Default::default()
                }, FieldTable::default())
                .await
                .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!(
                    "Failed to get queue stats for {}: {}. Ensure queue exists.", queue, e
                ))))?;

            let mut stats = HashMap::new();
            stats.insert("queue_name".to_string(), Value::String(queue.to_string()));
            stats.insert("message_count".to_string(), Value::Number(declare_result.message_count() as f64));
            stats.insert("consumer_count".to_string(), Value::Number(declare_result.consumer_count() as f64));

            // Update cache with fresh data
            {
                let mut cache = self.queue_stats_cache.lock().unwrap();
                cache.stats.insert(queue.to_string(), QueueStats {
                    message_count: declare_result.message_count(),
                    consumer_count: declare_result.consumer_count(),
                    cached_at: Instant::now(),
                });
            }

            Ok(stats)
        } else {
            Err(HlxError::ExecutionError(ExecutionError::new("AMQP channel not available - cannot retrieve queue statistics")))
        }
    }

    /// Clear queue statistics cache (useful for testing or forcing fresh data)
    pub fn clear_queue_stats_cache(&self) {
        let mut cache = self.queue_stats_cache.lock().unwrap();
        cache.stats.clear();
    }

    /// Set cache TTL for queue statistics
    pub fn set_queue_stats_cache_ttl(&self, ttl_seconds: u64) {
        let mut cache = self.queue_stats_cache.lock().unwrap();
        cache.ttl = Duration::from_secs(ttl_seconds);
    }

    /// Enhanced purge_queue with validation and better error handling
    pub async fn purge_queue_enhanced(&self, queue: &str) -> Result<HashMap<String, Value>, HlxError> {
        // First check if queue exists and get current count
        let pre_purge_count = self.get_queue_message_count(queue).await
            .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!(
                "Cannot purge queue {}: {}. Queue may not exist.", queue, e
            ))))?;

        // Perform purge operation
        let purged_count = self.purge_queue(queue).await?;

        // Invalidate cache for this queue
        {
            let mut cache = self.queue_stats_cache.lock().unwrap();
            cache.stats.remove(queue);
        }

        let mut result = HashMap::new();
        result.insert("queue_name".to_string(), Value::String(queue.to_string()));
        result.insert("messages_purged".to_string(), Value::Number(purged_count as f64));
        result.insert("pre_purge_count".to_string(), Value::Number(pre_purge_count as f64));
        result.insert("success".to_string(), Value::Boolean(true));

        Ok(result)
    }

    /// Enhanced delete_queue with validation and statistics
    pub async fn delete_queue_enhanced(&self, queue: &str) -> Result<HashMap<String, Value>, HlxError> {
        // Get final statistics before deletion
        let stats = self.get_queue_stats(queue).await
            .map_err(|e| HlxError::ExecutionError(ExecutionError::new(&format!(
                "Cannot delete queue {}: {}. Queue may not exist.", queue, e
            ))))?;

        // Perform delete operation
        let deleted_message_count = self.delete_queue(queue).await?;

        // Clean up cache
        {
            let mut cache = self.queue_stats_cache.lock().unwrap();
            cache.stats.remove(queue);
        }

        let mut result = HashMap::new();
        result.insert("queue_name".to_string(), Value::String(queue.to_string()));
        result.insert("messages_deleted".to_string(), Value::Number(deleted_message_count as f64));
        result.insert("final_consumer_count".to_string(), stats.get("consumer_count").unwrap().clone());
        result.insert("success".to_string(), Value::Boolean(true));

        Ok(result)
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("messages_published".to_string(), Value::Number(metrics.messages_published as f64));
        result.insert("messages_consumed".to_string(), Value::Number(metrics.messages_consumed as f64));
        result.insert("exchanges_declared".to_string(), Value::Number(metrics.exchanges_declared as f64));
        result.insert("queues_declared".to_string(), Value::Number(metrics.queues_declared as f64));
        result.insert("queue_bindings".to_string(), Value::Number(metrics.queue_bindings as f64));
        result.insert("connections_opened".to_string(), Value::Number(metrics.connections_opened as f64));
        result.insert("channels_opened".to_string(), Value::Number(metrics.channels_opened as f64));
        result.insert("avg_publish_time_ms".to_string(), Value::Number(metrics.avg_publish_time));

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for AMQPOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "declare_exchange" => {
                let name = params_map.get("name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("name".to_string()),
                        message: "Missing exchange name".to_string(),
                    })?;

                let exchange_type = params_map.get("type").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "direct".to_string());

                let durable = params_map.get("durable").and_then(|v| v.as_boolean()).unwrap_or(true);
                let auto_delete = params_map.get("auto_delete").and_then(|v| v.as_boolean()).unwrap_or(false);

                let exchange_type_enum = match exchange_type.as_str() {
                    "direct" => ExchangeType::Direct,
                    "fanout" => ExchangeType::Fanout,
                    "topic" => ExchangeType::Topic,
                    "headers" => ExchangeType::Headers,
                    _ => ExchangeType::Direct,
                };

                let exchange = ExchangeConfig {
                    name: name.clone(),
                    exchange_type: exchange_type_enum,
                    durable,
                    auto_delete,
                    internal: false,
                    arguments: HashMap::new(),
                };

                self.declare_exchange(exchange).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("exchange_name".to_string(), Value::String(name.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "declare_queue" => {
                let name = params_map.get("name").and_then(|v| v.as_string()).unwrap_or_default();
                let durable = params_map.get("durable").and_then(|v| v.as_boolean()).unwrap_or(true);
                let exclusive = params_map.get("exclusive").and_then(|v| v.as_boolean()).unwrap_or(false);
                let auto_delete = params_map.get("auto_delete").and_then(|v| v.as_boolean()).unwrap_or(false);

                let queue = QueueConfig {
                    name,
                    durable,
                    exclusive,
                    auto_delete,
                    arguments: HashMap::new(),
                };

                let queue_name = self.declare_queue(queue).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("queue_name".to_string(), Value::String(queue_name.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "bind_queue" => {
                let queue = params_map.get("queue").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("queue".to_string()),
                        message: "Missing queue name".to_string(),
                    })?;

                let exchange = params_map.get("exchange").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("exchange".to_string()),
                        message: "Missing exchange name".to_string(),
                    })?;

                let routing_key = params_map.get("routing_key").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "".to_string());

                self.bind_queue(&queue, &exchange, &routing_key).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("queue".to_string(), Value::String(queue.to_string()));
                    map.insert("exchange".to_string(), Value::String(exchange.to_string()));
                    map.insert("routing_key".to_string(), Value::String(routing_key.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "publish" => {
                let exchange = params_map.get("exchange").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "".to_string());

                let routing_key = params_map.get("routing_key").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "".to_string());

                let body = params_map.get("body").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("body".to_string()),
                        message: "Missing message body".to_string(),
                    })?;

                let message = Message {
                    body: body.into_bytes(),
                    properties: MessageProperties {
                        content_type: Some("text/plain".to_string()),
                        content_encoding: None,
                        delivery_mode: Some(2),
                        priority: None,
                        correlation_id: None,
                        reply_to: None,
                        expiration: None,
                        message_id: Some(format!("msg-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())),
                        timestamp: Some(chrono::Utc::now().timestamp() as u64),
                        user_id: None,
                        app_id: Some("helix-rust-sdk".to_string()),
                        headers: HashMap::new(),
                    },
                    routing_key: routing_key.clone(),
                };

                self.publish_message(&exchange, &routing_key, message).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("exchange".to_string(), Value::String(exchange.to_string()));
                    map.insert("routing_key".to_string(), Value::String(routing_key.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "consume" => {
                let queue = params_map.get("queue").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("queue".to_string()),
                        message: "Missing queue name".to_string(),
                    })?;

                let message = self.consume_message(&queue).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    if let Some(msg) = message {
                        map.insert("message".to_string(), Value::String(String::from_utf8_lossy(&msg.body.to_string()).to_string()));
                        map.insert("routing_key".to_string(), Value::String(msg.routing_key.to_string()));
                        map.insert("message_id".to_string(),
                            Value::String(msg.properties.message_id.unwrap_or_else(|| "unknown".to_string())));
                    } else {
                        map.insert("message".to_string(), Value::Null);
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete_queue" => {
                let queue = params_map.get("queue").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("queue".to_string()),
                        message: "Missing queue name".to_string(),
                    })?;

                let result = self.delete_queue_enhanced(&queue).await?;
                Ok(Value::Object(result))
            }

            "delete_exchange" => {
                let exchange = params_map.get("exchange").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("exchange".to_string()),
                        message: "Missing exchange name".to_string(),
                    })?;

                self.delete_exchange(&exchange).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("exchange".to_string(), Value::String(exchange.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "purge_queue" => {
                let queue = params_map.get("queue").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("queue".to_string()),
                        message: "Missing queue name".to_string(),
                    })?;

                let result = self.purge_queue_enhanced(&queue).await?;
                Ok(Value::Object(result))
            }

            "get_queue_message_count" => {
                let queue = params_map.get("queue").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("queue".to_string()),
                        message: "Missing queue name".to_string(),
                    })?;

                let message_count = self.get_queue_message_count(&queue).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("queue".to_string(), Value::String(queue.to_string()));
                    map.insert("message_count".to_string(), Value::Number(message_count as f64));
                    map.insert("cached".to_string(), Value::Boolean(true)); // Indicates result may be from cache
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_queue_stats" => {
                let queue = params_map.get("queue").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("queue".to_string()),
                        message: "Missing queue name".to_string(),
                    })?;

                let stats = self.get_queue_stats(&queue).await?;
                Ok(Value::Object(stats))
            }

            "clear_queue_cache" => {
                self.clear_queue_stats_cache();
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("operation".to_string(), Value::String("clear_queue_cache".to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "set_cache_ttl" => {
                let ttl_seconds = params_map.get("ttl_seconds").and_then(|v| v.as_number())
                    .unwrap_or(30.0) as u64;

                self.set_queue_stats_cache_ttl(ttl_seconds);
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("operation".to_string(), Value::String("set_cache_ttl".to_string()));
                    map.insert("ttl_seconds".to_string(), Value::Number(ttl_seconds as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "amqp".to_string(),
                params: format!("Unknown AMQP operation: {}", operator),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_stats_cache_structure() {
        // Test the cache data structures
        let cache = QueueStatsCache {
            stats: HashMap::new(),
            ttl: Duration::from_secs(30),
        };

        let stats = QueueStats {
            message_count: 42,
            consumer_count: 2,
            cached_at: Instant::now(),
        };

        assert_eq!(stats.message_count, 42);
        assert_eq!(stats.consumer_count, 2);
        assert_eq!(cache.ttl, Duration::from_secs(30));
    }

    #[test]
    fn test_amqp_config_defaults() {
        let config = AMQPConfig::default();
        assert_eq!(config.connection_string, "amqp://guest:guest@localhost:5672");
        assert_eq!(config.vhost, "/");
        assert_eq!(config.heartbeat, 60);
        assert_eq!(config.prefetch_count, 10);
        assert_eq!(config.connection_timeout, 30);
        assert!(!config.enable_ssl);
        assert!(config.ssl_verify);
    }

    #[test]
    fn test_cache_operations() {
        let cache = QueueStatsCache {
            stats: HashMap::new(),
            ttl: Duration::from_secs(30),
        };

        // Test cache TTL setting
        let mut test_cache = cache;
        test_cache.ttl = Duration::from_secs(60);
        assert_eq!(test_cache.ttl, Duration::from_secs(60));

        // Test clearing cache
        test_cache.stats.insert("test_queue".to_string(), QueueStats {
            message_count: 10,
            consumer_count: 1,
            cached_at: Instant::now(),
        });
        assert_eq!(test_cache.stats.len(), 1);
        test_cache.stats.clear();
        assert_eq!(test_cache.stats.len(), 0);
    }
} 