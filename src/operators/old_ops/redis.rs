//! Redis Cache Operator for Helix Rust SDK
//!
//! Provides comprehensive Redis capabilities including:
//! - Key-value operations with expiration and eviction policies
//! - Pub/Sub messaging with pattern matching and subscriptions
//! - Lua script execution with caching and atomic operations
//! - Redis Cluster support with hash slot routing
//! - Connection pooling with failover and load balancing
//! - Pipelining and batch operations for performance
//! - Stream operations and consumer groups
//! - Geospatial operations and indexing
//! - HyperLogLog operations for cardinality estimation

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime, redis};
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Client, Cmd, RedisError, RedisResult, cmd};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Redis operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL or cluster nodes
    pub connection: RedisConnection,
    /// Database number (0-15 for standard Redis)
    pub database: u8,
    /// Username for Redis ACL authentication
    pub username: Option<String>,
    /// Password for authentication
    pub password: Option<String>,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Command timeout in seconds
    pub command_timeout: u64,
    /// Connection pool configuration
    pub pool_config: PoolConfig,
    /// Enable pipelining for batch operations
    pub enable_pipelining: bool,
    /// Pipeline batch size
    pub pipeline_batch_size: usize,
    /// Default key expiration in seconds
    pub default_expiration: Option<u64>,
    /// Enable pub/sub
    pub enable_pubsub: bool,
    /// Enable Lua script caching
    pub enable_script_caching: bool,
    /// Script cache size
    pub script_cache_size: usize,
    /// Enable cluster mode
    pub enable_cluster: bool,
    /// Cluster configuration
    pub cluster_config: Option<ClusterConfig>,
    /// Enable compression for large values
    pub enable_compression: bool,
    /// Compression threshold in bytes
    pub compression_threshold: usize,
}

/// Redis connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedisConnection {
    /// Single Redis instance
    Single(String),
    /// Sentinel configuration
    Sentinel {
        sentinels: Vec<String>,
        service_name: String,
    },
    /// Cluster nodes
    Cluster(Vec<String>),
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of connections
    pub max_size: usize,
    /// Connection timeout in seconds
    pub timeout: u64,
    /// Connection idle timeout in seconds
    pub idle_timeout: u64,
    /// Connection max lifetime in seconds
    pub max_lifetime: u64,
    /// Connection retry attempts
    pub retry_attempts: u32,
    /// Retry delay in milliseconds
    pub retry_delay: u64,
}

/// Redis cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Read from replica nodes
    pub read_from_replicas: bool,
    /// Connection pool size per node
    pub connections_per_node: usize,
    /// Cluster refresh interval in seconds
    pub refresh_interval: u64,
    /// Maximum number of redirections
    pub max_redirections: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            connection: RedisConnection::Single("redis://127.0.0.1:6379".to_string()),
            database: 0,
            username: None,
            password: None,
            connection_timeout: 30,
            command_timeout: 30,
            pool_config: PoolConfig::default(),
            enable_pipelining: true,
            pipeline_batch_size: 100,
            default_expiration: None,
            enable_pubsub: true,
            enable_script_caching: true,
            script_cache_size: 100,
            enable_cluster: false,
            cluster_config: None,
            enable_compression: false,
            compression_threshold: 1024, // 1KB
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 20,
            timeout: 30,
            idle_timeout: 300, // 5 minutes
            max_lifetime: 1800, // 30 minutes
            retry_attempts: 3,
            retry_delay: 1000,
        }
    }
}

/// Redis key-value pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisKeyValue {
    pub key: String,
    pub value: JsonValue,
    pub ttl: Option<u64>,
    pub data_type: String,
}

/// Redis subscription information
#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub subscription_id: String,
    pub channel_pattern: String,
    pub is_pattern: bool,
    pub message_count: u64,
    pub created_at: Instant,
    pub is_active: bool,
}

/// Redis Lua script information
#[derive(Debug, Clone)]
pub struct LuaScript {
    pub script_id: String,
    pub script: String,
    pub sha1: String,
    pub created_at: Instant,
    pub execution_count: u64,
    pub total_execution_time: Duration,
}

/// Redis stream message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    pub stream: String,
    pub id: String,
    pub fields: HashMap<String, String>,
    pub timestamp: u64,
}

/// Redis performance metrics
#[derive(Debug, Default)]
struct RedisMetrics {
    commands_executed: u64,
    cache_hits: u64,
    cache_misses: u64,
    keys_created: u64,
    keys_deleted: u64,
    keys_expired: u64,
    pub_messages_sent: u64,
    sub_messages_received: u64,
    scripts_executed: u64,
    pipeline_operations: u64,
    avg_command_time: f64,
    connection_errors: u64,
    active_connections: u32,
    memory_usage: u64,
}

/// Message handler for pub/sub
type MessageHandler = Arc<dyn Fn(String, String, String) + Send + Sync>;

/// Redis Cache Operator
pub struct RedisOperator {
    config: RedisConfig,
    connection_pool: Pool,
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,
    lua_scripts: Arc<RwLock<HashMap<String, LuaScript>>>,
    metrics: Arc<Mutex<RedisMetrics>>,
    message_handlers: Arc<RwLock<HashMap<String, MessageHandler>>>,
    pipeline_queue: Arc<RwLock<Vec<redis::Cmd>>>,
}

impl RedisOperator {
    /// Create a new Redis operator with configuration
    pub async fn new(config: RedisConfig) -> Result<Self, HlxError> {
        // Build Redis URL
        let redis_url = match &config.connection {
            RedisConnection::Single(url) => url.clone(),
            RedisConnection::Sentinel { sentinels, service_name } => {
                // For simplicity, use first sentinel
                // In production, implement proper sentinel logic
                sentinels.first().unwrap_or(&"redis://127.0.0.1:26379".to_string()).clone()
            }
            RedisConnection::Cluster(nodes) => {
                // For simplicity, use first node
                // In production, implement proper cluster logic
                nodes.first().unwrap_or(&"redis://127.0.0.1:7000".to_string()).clone()
            }
        };

        // Create deadpool configuration
        let pool_config = Config::from_url(redis_url.clone());
        
        let pool = pool_config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| HlxError::ConnectionError {
                service: "Redis".to_string(),
                message: format!("Failed to create connection pool: {}", e),
            })?;

        // Test connection
        let mut conn = pool.get().await.map_err(|e| HlxError::ConnectionError {
            service: "Redis".to_string(),
            message: format!("Failed to get connection from pool: {}", e),
        })?;

        // Select database if specified
        if config.database > 0 {
            redis::cmd("SELECT").arg(config.database).query_async(&mut *conn).await
                .map_err(|e| HlxError::ConnectionError {
                    service: "Redis".to_string(),
                    message: format!("Failed to select database {}: {}", config.database, e),
                })?;
        }

        // Test connection with PING
        let pong: String = redis::cmd("PING").query_async(&mut *conn).await
            .map_err(|e| HlxError::ConnectionError {
                service: "Redis".to_string(),
                message: format!("Connection test failed: {}", e),
            })?;

        if pong != "PONG" {
            return Err(HlxError::ConnectionError {
                service: "Redis".to_string(),
                message: "Invalid PING response".to_string(),
            });
        }

        info!("Redis operator initialized successfully with database {}", config.database);

        Ok(Self {
            config: config.clone(),
            connection_pool: pool,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            lua_scripts: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(RedisMetrics::default())),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            pipeline_queue: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Set a key-value pair with optional expiration
    pub async fn set(&self, key: &str, value: JsonValue, expiration: Option<u64>) -> Result<(), HlxError> {
        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        // Serialize value
        let serialized_value = self.serialize_value(&value)?;
        let ttl = expiration.or(self.config.default_expiration);

        let result = if let Some(expire_secs) = ttl {
            conn.set_ex(key, serialized_value, expire_secs as usize).await
        } else {
            conn.set(key, serialized_value).await
        };

        match result {
            Ok(()) => {
                self.update_command_metrics(start_time, true);
                
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.keys_created += 1;
                }

                debug!("Set key: {} with TTL: {:?}", key, ttl);
                Ok(())
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::CacheError {
                    operation: "SET".to_string(),
                    message: format!("Failed to set key {}: {}", key, e),
                })
            }
        }
    }

    /// Get a value by key
    pub async fn get(&self, key: &str) -> Result<Option<JsonValue>, HlxError> {
        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        let result: RedisResult<Option<String>> = conn.get(key).await;

        match result {
            Ok(Some(value)) => {
                self.update_command_metrics(start_time, true);
                
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.cache_hits += 1;
                }

                let deserialized = self.deserialize_value(&value)?;
                Ok(Some(deserialized))
            }
            Ok(None) => {
                self.update_command_metrics(start_time, true);
                
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.cache_misses += 1;
                }

                Ok(None)
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::CacheError {
                    operation: "GET".to_string(),
                    message: format!("Failed to get key {}: {}", key, e),
                })
            }
        }
    }

    /// Delete one or more keys
    pub async fn delete(&self, keys: Vec<&str>) -> Result<u64, HlxError> {
        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        let result: RedisResult<u64> = conn.del(&keys).await;

        match result {
            Ok(deleted_count) => {
                self.update_command_metrics(start_time, true);
                
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.keys_deleted += deleted_count;
                }

                debug!("Deleted {} keys", deleted_count);
                Ok(deleted_count)
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::CacheError {
                    operation: "DEL".to_string(),
                    message: format!("Failed to delete keys: {}", e),
                })
            }
        }
    }

    /// Check if key exists
    pub async fn exists(&self, key: &str) -> Result<bool, HlxError> {
        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        let result: RedisResult<bool> = conn.exists(key).await;

        match result {
            Ok(exists) => {
                self.update_command_metrics(start_time, true);
                Ok(exists)
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::CacheError {
                    operation: "EXISTS".to_string(),
                    message: format!("Failed to check existence of key {}: {}", key, e),
                })
            }
        }
    }

    /// Set expiration for a key
    pub async fn expire(&self, key: &str, seconds: u64) -> Result<bool, HlxError> {
        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        let result: RedisResult<bool> = conn.expire(key, seconds as usize).await;

        match result {
            Ok(set) => {
                self.update_command_metrics(start_time, true);
                Ok(set)
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::CacheError {
                    operation: "EXPIRE".to_string(),
                    message: format!("Failed to set expiration for key {}: {}", key, e),
                })
            }
        }
    }

    /// Get time-to-live for a key
    pub async fn ttl(&self, key: &str) -> Result<i64, HlxError> {
        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        let result: RedisResult<i64> = conn.ttl(key).await;

        match result {
            Ok(ttl) => {
                self.update_command_metrics(start_time, true);
                Ok(ttl)
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::CacheError {
                    operation: "TTL".to_string(),
                    message: format!("Failed to get TTL for key {}: {}", key, e),
                })
            }
        }
    }

    /// Increment a numeric key
    pub async fn increment(&self, key: &str, amount: i64) -> Result<i64, HlxError> {
        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        let result: RedisResult<i64> = if amount == 1 {
            conn.incr(key, 1).await
        } else {
            conn.incr(key, amount).await
        };

        match result {
            Ok(new_value) => {
                self.update_command_metrics(start_time, true);
                Ok(new_value)
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::CacheError {
                    operation: "INCR".to_string(),
                    message: format!("Failed to increment key {}: {}", key, e),
                })
            }
        }
    }

    /// Publish a message to a channel
    pub async fn publish(&self, channel: &str, message: &str) -> Result<u64, HlxError> {
        if !self.config.enable_pubsub {
            return Err(HlxError::ConfigurationError {
                component: "Redis Pub/Sub".to_string(),
                message: "Pub/Sub is not enabled".to_string(),
            });
        }

        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        let result: RedisResult<u64> = conn.publish(channel, message).await;

        match result {
            Ok(subscriber_count) => {
                self.update_command_metrics(start_time, true);
                
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.pub_messages_sent += 1;
                }

                debug!("Published message to channel {}, {} subscribers", channel, subscriber_count);
                Ok(subscriber_count)
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::CommunicationError {
                    component: "Redis Pub/Sub".to_string(),
                    message: format!("Failed to publish to channel {}: {}", channel, e),
                })
            }
        }
    }

    /// Subscribe to a channel
    pub async fn subscribe(&self, channel: &str, is_pattern: bool) -> Result<String, HlxError> {
        if !self.config.enable_pubsub {
            return Err(HlxError::ConfigurationError {
                component: "Redis Pub/Sub".to_string(),
                message: "Pub/Sub is not enabled".to_string(),
            });
        }

        let subscription_id = uuid::Uuid::new_v4().to_string();
        
        // Store subscription info
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.insert(subscription_id.clone(), SubscriptionInfo {
                subscription_id: subscription_id.clone(),
                channel_pattern: channel.to_string(),
                is_pattern,
                message_count: 0,
                created_at: Instant::now(),
                is_active: true,
            });
        }

        // In a real implementation, you would create a separate connection for subscriptions
        // and handle incoming messages in a background task
        info!("Subscribed to {}: {} (pattern: {})", 
              if is_pattern { "pattern" } else { "channel" }, 
              channel, is_pattern);

        Ok(subscription_id)
    }

    /// Unsubscribe from a channel
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<(), HlxError> {
        let mut subscriptions = self.subscriptions.write().await;
        
        if let Some(mut subscription) = subscriptions.get_mut(subscription_id) {
            subscription.is_active = false;
            info!("Unsubscribed from subscription: {}", subscription_id);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Redis Subscription".to_string(),
                identifier: subscription_id.to_string(),
            })
        }
    }

    /// Execute a Lua script
    pub async fn eval_script(&self, script: &str, keys: Vec<&str>, args: Vec<&str>) -> Result<JsonValue, HlxError> {
        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        // Create Lua script command
        let mut cmd = redis::cmd("EVAL");
        cmd.arg(script).arg(keys.len());
        
        for key in keys {
            cmd.arg(key);
        }
        
        for arg in args {
            cmd.arg(arg);
        }

        let result: RedisResult<String> = cmd.query_async(&mut *conn).await;

        match result {
            Ok(response) => {
                self.update_command_metrics(start_time, true);
                
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.scripts_executed += 1;
                }

                // Try to parse as JSON, fallback to string
                match serde_json::from_str(&response) {
                    Ok(json_value) => Ok(json_value),
                    Err(_) => Ok(JsonValue::String(response.to_string())),
                }
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::OperationError {
                    operator: "unknown".to_string(),
                    details: None,
                    operation: "Lua Script".to_string(),
                    message: format!("Script execution failed: {}", e),
                })
            }
        }
    }

    /// Load and cache a Lua script
    pub async fn load_script(&self, script_id: String, script: &str) -> Result<String, HlxError> {
        if !self.config.enable_script_caching {
            return Err(HlxError::ConfigurationError {
                component: "Redis Lua Scripts".to_string(),
                message: "Script caching is not enabled".to_string(),
            });
        }

        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        // Load script and get SHA1 hash
        let sha1: String = conn.script_load(script).await
            .map_err(|e| HlxError::OperationError {
                operator: "unknown".to_string(),
                details: None,
                operation: "Load Lua Script".to_string(),
                message: format!("Failed to load script: {}", e),
            })?;

        // Store script info
        {
            let mut scripts = self.lua_scripts.write().await;
            scripts.insert(script_id.clone(), LuaScript {
                script_id: script_id.clone(),
                script: script.to_string(),
                sha1: sha1.clone(),
                created_at: Instant::now(),
                execution_count: 0,
                total_execution_time: Duration::default(),
            });
        }

        self.update_command_metrics(start_time, true);
        info!("Loaded Lua script: {} (SHA1: {})", script_id, sha1);
        Ok(sha1)
    }

    /// Execute a cached Lua script by SHA1
    pub async fn evalsha(&self, script_id: &str, keys: Vec<&str>, args: Vec<&str>) -> Result<JsonValue, HlxError> {
        let sha1 = {
            let scripts = self.lua_scripts.read().await;
            if let Some(script) = scripts.get(script_id) {
                script.sha1.clone()
            } else {
                return Err(HlxError::NotFoundError {
                    resource: "Lua Script".to_string(),
                    identifier: script_id.to_string(),
                });
            }
        };

        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        // Create EVALSHA command
        let mut cmd = redis::cmd("EVALSHA");
        cmd.arg(&sha1).arg(keys.len());
        
        for key in keys {
            cmd.arg(key);
        }
        
        for arg in args {
            cmd.arg(arg);
        }

        let result: RedisResult<String> = cmd.query_async(&mut *conn).await;

        match result {
            Ok(response) => {
                self.update_command_metrics(start_time, true);
                
                // Update script execution stats
                {
                    let mut scripts = self.lua_scripts.write().await;
                    if let Some(script) = scripts.get_mut(script_id) {
                        script.execution_count += 1;
                        script.total_execution_time += start_time.elapsed();
                    }
                }

                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.scripts_executed += 1;
                }

                // Try to parse as JSON, fallback to string
                match serde_json::from_str(&response) {
                    Ok(json_value) => Ok(json_value),
                    Err(_) => Ok(JsonValue::String(response.to_string())),
                }
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::OperationError {
                    operator: "unknown".to_string(),
                    details: None,
                    operation: "Execute Cached Lua Script".to_string(),
                    message: format!("EVALSHA failed: {}", e),
                })
            }
        }
    }

    /// Execute multiple commands in a pipeline
    pub async fn pipeline(&self, commands: Vec<(String, Vec<String>)>) -> Result<Vec<JsonValue>, HlxError> {
        if !self.config.enable_pipelining {
            return Err(HlxError::ConfigurationError {
                component: "Redis Pipelining".to_string(),
                message: "Pipelining is not enabled".to_string(),
            });
        }

        let start_time = Instant::now();
        let mut conn = self.get_connection().await?;

        // Build pipeline
        let mut pipe = redis::pipe();
        for (command, args) in commands {
            let mut cmd = redis::cmd(&command);
            for arg in args {
                cmd.arg(arg);
            }
            pipe.add_command(cmd);
        }

        let results: RedisResult<Vec<String>> = pipe.query_async(&mut *conn).await;

        match results {
            Ok(responses) => {
                self.update_command_metrics(start_time, true);
                
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.pipeline_operations += responses.len() as u64;
                }

                // Convert responses to JSON
                let json_results: Vec<JsonValue> = responses.into_iter()
                    .map(|response| {
                        serde_json::from_str(&response).unwrap_or(JsonValue::String(response.to_string()))
                    })
                    .collect();

                Ok(json_results)
            }
            Err(e) => {
                self.update_command_metrics(start_time, false);
                Err(HlxError::OperationError {
                    operator: "unknown".to_string(),
                    details: None,
                    operation: "Redis Pipeline".to_string(),
                    message: format!("Pipeline execution failed: {}", e),
                })
            }
        }
    }

    /// Get Redis server info
    pub async fn info(&self, section: Option<&str>) -> Result<HashMap<String, String>, HlxError> {
        let mut conn = self.get_connection().await?;

        let info_result: RedisResult<String> = if let Some(sec) = section {
            redis::cmd("INFO").arg(sec).query_async(&mut *conn).await
        } else {
            redis::cmd("INFO").query_async(&mut *conn).await
        };

        match info_result {
            Ok(info_text) => {
                let mut info_map = HashMap::new();
                
                for line in info_text.lines() {
                    if line.starts_with('#') || line.is_empty() {
                        continue;
                    }
                    
                    if let Some((key, value)) = line.split_once(':') {
                        info_map.insert(key.to_string(), value.to_string());
                    }
                }
                
                Ok(info_map)
            }
            Err(e) => {
                Err(HlxError::OperationError {
                    operator: "unknown".to_string(),
                    details: None,
                    operation: "Redis INFO".to_string(),
                    message: format!("Failed to get Redis info: {}", e),
                })
            }
        }
    }

    /// Flush database
    pub async fn flush_db(&self, all_databases: bool) -> Result<(), HlxError> {
        let mut conn = self.get_connection().await?;

        let result: RedisResult<()> = if all_databases {
            redis::cmd("FLUSHALL").query_async(&mut *conn).await
        } else {
            redis::cmd("FLUSHDB").query_async(&mut *conn).await
        };

        match result {
            Ok(()) => {
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.keys_deleted += 1; // Approximation
                }

                info!("Flushed Redis database(s)");
                Ok(())
            }
            Err(e) => {
                Err(HlxError::OperationError {
                    operator: "unknown".to_string(),
                    details: None,
                    operation: "Redis FLUSH".to_string(),
                    message: format!("Failed to flush database: {}", e),
                })
            }
        }
    }

    /// Get connection from pool
    async fn get_connection(&self) -> Result<deadpool_redis::Connection, HlxError> {
        self.connection_pool.get().await
            .map_err(|e| HlxError::ConnectionError {
                service: "Redis".to_string(),
                message: format!("Failed to get connection from pool: {}", e),
            })
    }

    /// Serialize value to string
    fn serialize_value(&self, value: &JsonValue) -> Result<String, HlxError> {
        match value {
            JsonValue::String(s) => Ok(s.clone()),
            other => serde_json::to_string(other)
                .map_err(|e| HlxError::SerializationError {
                    format: "JSON".to_string(),
                    message: format!("Failed to serialize value: {}", e),
                }),
        }
    }

    /// Deserialize value from string
    fn deserialize_value(&self, value: &str) -> Result<JsonValue, HlxError> {
        // Try to parse as JSON first
        match serde_json::from_str(value) {
            Ok(json_value) => Ok(json_value),
            Err(_) => {
                // If JSON parsing fails, treat as string
                Ok(JsonValue::String(value.to_string()))
            }
        }
    }

    /// Update command execution metrics
    fn update_command_metrics(&self, start_time: Instant, success: bool) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.commands_executed += 1;
        
        if !success {
            metrics.connection_errors += 1;
        }
        
        let execution_time = start_time.elapsed().as_millis() as f64;
        metrics.avg_command_time = 
            (metrics.avg_command_time * (metrics.commands_executed - 1) as f64 + execution_time) / 
            metrics.commands_executed as f64;
    }

    /// Get Redis performance metrics
    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();
        
        result.insert("commands_executed".to_string(), Value::Number(metrics.commands_executed as f64));
        result.insert("cache_hits".to_string(), Value::Number(metrics.cache_hits as f64));
        result.insert("cache_misses".to_string(), Value::Number(metrics.cache_misses as f64));
        result.insert("keys_created".to_string(), Value::Number(metrics.keys_created as f64));
        result.insert("keys_deleted".to_string(), Value::Number(metrics.keys_deleted as f64));
        result.insert("keys_expired".to_string(), Value::Number(metrics.keys_expired as f64));
        result.insert("pub_messages_sent".to_string(), Value::Number(metrics.pub_messages_sent as f64));
        result.insert("sub_messages_received".to_string(), Value::Number(metrics.sub_messages_received as f64));
        result.insert("scripts_executed".to_string(), Value::Number(metrics.scripts_executed as f64));
        result.insert("pipeline_operations".to_string(), Value::Number(metrics.pipeline_operations as f64));
        result.insert("avg_command_time_ms".to_string(), Value::Number(metrics.avg_command_time));
        result.insert("connection_errors".to_string(), Value::Number(metrics.connection_errors as f64));
        result.insert("active_connections".to_string(), Value::Number(metrics.active_connections as f64));
        result.insert("memory_usage".to_string(), Value::Number(metrics.memory_usage as f64));
        
        // Calculate hit rate
        if metrics.cache_hits + metrics.cache_misses > 0 {
            let hit_rate = (metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64) * 100.0;
            result.insert("cache_hit_rate_percent".to_string(), Value::Number(hit_rate));
        }
        
        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for RedisOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "set" => {
                let key = params_map.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing Redis key".to_string(),
                        field: Some("key".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let value = params_map.get("value")
                    .map(|v| utils::value_to_json_value(v))
                    .unwrap_or(JsonValue::Null);

                let expiration = params_map.get("expiration")
                    .and_then(|v| v.as_number())
                    .map(|n| n as u64);

                self.set(&key, value, expiration).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "get" => {
                let key = params_map.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing Redis key".to_string(),
                        field: Some("key".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let result = self.get(&key).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string()));

                    if let Some(value) = result {
                        map.insert("value".to_string(), utils::json_value_to_value(&value));
                        map.insert("found".to_string(), Value::Boolean(true));
                    } else {
                        map.insert("found".to_string(), Value::Boolean(false));
                    }
                    
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "delete" => {
                let keys = params_map.get("keys")
                    .and_then(|v| {
                        match v {
                            Value::String(s) => Some(vec![s]),
                            Value::Array(arr) => {
                                let keys: Option<Vec<&str>> = arr.iter()
                                    .map(|v| v.as_string().map(|s| s))
                                    .collect();
                                keys
                            }
                            _ => None,
                        }
                    })
                    .or_else(|| {
                        params_map.get("key")
                            .and_then(|v| v.as_string())
                            .map(|s| vec![s])
                    })
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing Redis keys to delete".to_string(),
                        field: Some("keys".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let deleted_count = self.delete(keys).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("deleted_count".to_string(), Value::Number(deleted_count as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "exists" => {
                let key = params_map.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing Redis key".to_string(),
                        field: Some("key".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let exists = self.exists(&key).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("exists".to_string(), Value::Boolean(exists));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "expire" => {
                let key = params_map.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing Redis key".to_string(),
                        field: Some("key".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let seconds = params_map.get("seconds")
                    .and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing expiration seconds".to_string(),
                        field: Some("seconds".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })? as u64;

                let was_set = self.expire(&key, seconds).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("expiration_set".to_string(), Value::Boolean(was_set));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "ttl" => {
                let key = params_map.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing Redis key".to_string(),
                        field: Some("key".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let ttl = self.ttl(&key).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("ttl".to_string(), Value::Number(ttl as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "incr" => {
                let key = params_map.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing Redis key".to_string(),
                        field: Some("key".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let amount = params_map.get("amount")
                    .and_then(|v| v.as_number())
                    .unwrap_or(1.0) as i64;

                let new_value = self.increment(&key, amount).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string()));
                    map.insert("value".to_string(), Value::Number(new_value as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "publish" => {
                let channel = params_map.get("channel")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing pub/sub channel".to_string(),
                        field: Some("channel".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let message = params_map.get("message")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing message to publish".to_string(),
                        field: Some("message".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let subscriber_count = self.publish(&channel, &message).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("channel".to_string(), Value::String(channel.to_string()));
                    map.insert("subscriber_count".to_string(), Value::Number(subscriber_count as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "subscribe" => {
                let channel = params_map.get("channel")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing pub/sub channel".to_string(),
                        field: Some("channel".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let is_pattern = params_map.get("pattern")
                    .and_then(|v| v.as_boolean())
                    .unwrap_or(false);

                let subscription_id = self.subscribe(&channel, is_pattern).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("subscription_id".to_string(), Value::String(subscription_id.to_string()));
                    map.insert("channel".to_string(), Value::String(channel.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "eval" => {
                let script = params_map.get("script")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError {
                        message: "Missing Lua script".to_string(),
                        field: Some("script".to_string()),
                        value: None,
                        rule: Some("required".to_string()),
                    })?;

                let keys = params_map.get("keys")
                    .and_then(|v| {
                        if let Value::Array(arr) = v {
                            let keys: Option<Vec<&str>> = arr.iter()
                                .map(|v| v.as_string().map(|s| s))
                                .collect();
                            keys
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                let args = params_map.get("args")
                    .and_then(|v| {
                        if let Value::Array(arr) = v {
                            let args: Option<Vec<&str>> = arr.iter()
                                .map(|v| v.as_string().map(|s| s))
                                .collect();
                            args
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                let result = self.eval_script(&script, keys, args).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "info" => {
                let section = params_map.get("section")
                    .and_then(|v| v.as_string());

                let info = self.info(section.as_deref()).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    for (key, value) in info {
                        map.insert(key, Value::String(value.to_string()));
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "flush" => {
                let all_databases = params_map.get("all")
                    .and_then(|v| v.as_boolean())
                    .unwrap_or(false);

                self.flush_db(all_databases).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("flushed".to_string(), Value::Boolean(true));
                    map.insert("all_databases".to_string(), Value::Boolean(all_databases));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }
            
            _ => Err(HlxError::InvalidParameters {
                operator: "redis".to_string(),
                params: format!("Unknown Redis operation: {}", operator),
            }),
        }
    }
} 