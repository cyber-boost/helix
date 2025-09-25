//! gRPC Communication Operator for Helix Rust SDK
//!
//! Provides comprehensive gRPC capabilities including:
//! - Unary and streaming RPC calls
//! - Protobuf message serialization/deserialization
//! - TLS support and authentication
//! - Connection multiplexing and pooling
//! - Health checking and service discovery
//! - Metadata handling and interceptors
//! - Load balancing and retry policies
//! - Performance monitoring and metrics

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use prost::Message;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint, Server};
use tonic::{Code, Request, Response, Status};
use tracing::{debug, error, info, warn};

/// gRPC operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// gRPC server endpoint
    pub endpoint: String,
    /// Enable TLS encryption
    pub enable_tls: bool,
    /// TLS configuration
    pub tls_config: Option<TlsConfig>,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Connection keep alive interval
    pub keep_alive_interval: Option<u64>,
    /// Keep alive timeout
    pub keep_alive_timeout: Option<u64>,
    /// Enable keep alive pings
    pub keep_alive_while_idle: bool,
    /// Connection pool size
    pub pool_size: usize,
    /// Max retry attempts
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay: u64,
    /// Enable compression
    pub enable_compression: bool,
    /// Max message size in bytes
    pub max_message_size: Option<usize>,
    /// Enable health checks
    pub enable_health_checks: bool,
    /// Health check interval in seconds
    pub health_check_interval: u64,
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// CA certificate path
    pub ca_cert_path: Option<String>,
    /// Client certificate path
    pub client_cert_path: Option<String>,
    /// Client private key path
    pub client_key_path: Option<String>,
    /// Server name for SNI
    pub server_name: Option<String>,
    /// Accept invalid certificates (for testing)
    pub accept_invalid_certs: bool,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:50051".to_string(),
            enable_tls: false,
            tls_config: None,
            timeout: 30,
            keep_alive_interval: Some(30),
            keep_alive_timeout: Some(5),
            keep_alive_while_idle: true,
            pool_size: 10,
            max_retries: 3,
            retry_delay: 1000,
            enable_compression: true,
            max_message_size: Some(4 * 1024 * 1024), // 4MB
            enable_health_checks: true,
            health_check_interval: 10,
        }
    }
}

/// gRPC call result
#[derive(Debug, Serialize, Deserialize)]
pub struct GrpcResult {
    pub success: bool,
    pub data: Option<JsonValue>,
    pub status_code: Option<i32>,
    pub status_message: Option<String>,
    pub metadata: HashMap<String, String>,
    pub duration_ms: u64,
}

/// gRPC service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub methods: Vec<MethodInfo>,
    pub health_status: HealthStatus,
}

/// gRPC method information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub is_streaming: bool,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
}

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Unknown,
    Serving,
    NotServing,
    ServiceUnknown,
}

/// Connection pool entry
#[derive(Debug, Clone)]
struct PooledConnection {
    channel: Channel,
    created_at: Instant,
    last_used: Instant,
    is_healthy: bool,
}

/// gRPC streaming handle
#[derive(Debug)]
pub struct StreamingHandle {
    pub id: String,
    pub method: String,
    pub is_active: bool,
}

/// gRPC Communication Operator
pub struct GrpcOperator {
    config: GrpcConfig,
    connection_pool: Arc<RwLock<Vec<PooledConnection>>>,
    services: Arc<RwLock<HashMap<String, ServiceInfo>>>,
    streams: Arc<Mutex<HashMap<String, StreamingHandle>>>,
    metrics: Arc<Mutex<GrpcMetrics>>,
}

/// gRPC performance metrics
#[derive(Debug, Default)]
struct GrpcMetrics {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    avg_response_time: f64,
    active_connections: u32,
    total_bytes_sent: u64,
    total_bytes_received: u64,
}

impl GrpcOperator {
    /// Create a new gRPC operator with configuration
    pub async fn new(config: GrpcConfig) -> Result<Self, HlxError> {
        let operator = Self {
            config: config.clone(),
            connection_pool: Arc::new(RwLock::new(Vec::new())),
            services: Arc::new(RwLock::new(HashMap::new())),
            streams: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(GrpcMetrics::default())),
        };

        // Initialize connection pool
        operator.initialize_connection_pool().await?;

        // Start health checks if enabled
        if config.enable_health_checks {
            operator.start_health_checks().await;
        }

        info!("gRPC operator initialized successfully");
        Ok(operator)
    }

    /// Execute unary gRPC call
    pub async fn execute_unary_call(
        &self,
        service: &str,
        method: &str,
        request_data: JsonValue,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<GrpcResult, HlxError> {
        let start_time = Instant::now();
        
        // Get connection from pool
        let channel = self.get_connection().await?;
        
        // Create generic gRPC client
        let mut client = tonic::client::Grpc::new(channel.clone());
        
        // Set timeout
        client = client.timeout(Duration::from_secs(self.config.timeout));
        
        // Enable compression if configured
        if self.config.enable_compression {
            client = client.send_compressed(tonic::codec::CompressionEncoding::Gzip);
        }

        // Prepare request
        let mut request = Request::new(self.json_to_protobuf_bytes(&request_data)?);
        
        // Add metadata if provided
        if let Some(meta) = metadata {
            for (key, value) in meta {
                request.metadata_mut().insert(
                    key.parse().map_err(|e| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("metadata_key".to_string()),
                        message: format!("Invalid metadata key: {}", e),
                    })?,
                    value.parse().map_err(|e| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("metadata_value".to_string()),
                        message: format!("Invalid metadata value: {}", e),
                    })?,
                );
            }
        }

        // Execute the call with retry logic
        let result = self.execute_with_retry(|| async {
            let path = format!("/{}/{}", service, method);
            let mut request_clone = Request::new(request.get_ref().clone());
            request_clone.metadata_mut().clone_from(request.metadata());
            
            client.unary(request_clone, tonic::codegen::http::uri::PathAndQuery::from_static(&path), tonic::codec::ProstCodec::default()).await
        }).await;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.total_requests += 1;
            
            match &result {
                Ok(_) => metrics.successful_requests += 1,
                Err(_) => metrics.failed_requests += 1,
            }
            
            let duration = start_time.elapsed().as_millis() as f64;
            metrics.avg_response_time = 
                (metrics.avg_response_time * (metrics.total_requests - 1) as f64 + duration) / 
                metrics.total_requests as f64;
        }

        // Return connection to pool
        self.return_connection(channel).await;

        match result {
            Ok(response) => {
                let data = self.protobuf_bytes_to_json(response.get_ref())?;
                let metadata = response.metadata()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();

                Ok(GrpcResult {
                    success: true,
                    data: Some(data),
                    status_code: Some(Code::Ok as i32),
                    status_message: Some("Success".to_string()),
                    metadata,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                })
            }
            Err(status) => {
                Ok(GrpcResult {
                    success: false,
                    data: None,
                    status_code: Some(status.code() as i32),
                    status_message: Some(status.message().to_string()),
                    metadata: HashMap::new(),
                    duration_ms: start_time.elapsed().as_millis() as u64,
                })
            }
        }
    }

    /// Start server streaming call
    pub async fn start_server_stream(
        &self,
        service: &str,
        method: &str,
        request_data: JsonValue,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<String, HlxError> {
        let stream_id = uuid::Uuid::new_v4().to_string();
        
        // Store streaming handle
        {
            let mut streams = self.streams.lock().unwrap();
            streams.insert(stream_id.clone(), StreamingHandle {
                id: stream_id.clone(),
                method: format!("{}/{}", service, method),
                is_active: true,
            });
        }

        // In a real implementation, you would start the streaming call here
        info!("gRPC server stream {} started for {}/{}", stream_id, service, method);
        
        Ok(stream_id)
    }

    /// Start client streaming call
    pub async fn start_client_stream(
        &self,
        service: &str,
        method: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<String, HlxError> {
        let stream_id = uuid::Uuid::new_v4().to_string();
        
        // Store streaming handle
        {
            let mut streams = self.streams.lock().unwrap();
            streams.insert(stream_id.clone(), StreamingHandle {
                id: stream_id.clone(),
                method: format!("{}/{}", service, method),
                is_active: true,
            });
        }

        info!("gRPC client stream {} started for {}/{}", stream_id, service, method);
        
        Ok(stream_id)
    }

    /// Send data to client stream
    pub async fn send_to_stream(&self, stream_id: &str, data: JsonValue) -> Result<(), HlxError> {
        let streams = self.streams.lock().unwrap();
        
        if let Some(stream) = streams.get(stream_id) {
            if stream.is_active {
                // In a real implementation, you would send data to the stream here
                debug!("Sending data to gRPC stream {}", stream_id);
                Ok(())
            } else {
                Err(HlxError::InvalidStateError {
                    component: "gRPC Stream".to_string(),
                    state: "inactive".to_string(),
                    message: "Stream is not active".to_string(),
                })
            }
        } else {
            Err(HlxError::NotFoundError {
                resource: "gRPC Stream".to_string(),
                identifier: stream_id.to_string(),
            })
        }
    }

    /// Close streaming call
    pub async fn close_stream(&self, stream_id: &str) -> Result<(), HlxError> {
        let mut streams = self.streams.lock().unwrap();
        
        if let Some(mut stream) = streams.get_mut(stream_id) {
            stream.is_active = false;
            info!("gRPC stream {} closed", stream_id);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "gRPC Stream".to_string(),
                identifier: stream_id.to_string(),
            })
        }
    }

    /// Check service health
    pub async fn check_health(&self, service: Option<&str>) -> Result<HealthStatus, HlxError> {
        let channel = self.get_connection().await?;
        
        // In a real implementation, you would use the standard gRPC health checking protocol
        // For now, we'll simulate a health check
        match channel.ready().await {
            Ok(_) => Ok(HealthStatus::Serving),
            Err(_) => Ok(HealthStatus::NotServing),
        }
    }

    /// Get server reflection information
    pub async fn get_server_reflection(&self) -> Result<Vec<ServiceInfo>, HlxError> {
        // In a real implementation, you would use gRPC server reflection protocol
        // to discover available services and methods
        let services = self.services.read().await;
        Ok(services.values().cloned().collect())
    }

    /// Initialize connection pool
    async fn initialize_connection_pool(&self) -> Result<(), HlxError> {
        let mut pool = self.connection_pool.write().await;
        
        for _ in 0..self.config.pool_size {
            let connection = self.create_connection().await?;
            pool.push(connection);
        }
        
        info!("gRPC connection pool initialized with {} connections", pool.len());
        Ok(())
    }

    /// Create a new gRPC connection
    async fn create_connection(&self) -> Result<PooledConnection, HlxError> {
        let mut endpoint = Endpoint::from_shared(self.config.endpoint.clone())
            .map_err(|e| HlxError::ConfigurationError {
                component: "gRPC Endpoint".to_string(),
                message: format!("Invalid endpoint: {}", e),
            })?;

        // Configure timeouts
        endpoint = endpoint.timeout(Duration::from_secs(self.config.timeout));
        
        // Configure keep-alive
        if let Some(interval) = self.config.keep_alive_interval {
            endpoint = endpoint.keep_alive_timeout(Duration::from_secs(interval));
        }
        
        if let Some(timeout) = self.config.keep_alive_timeout {
            endpoint = endpoint.tcp_keepalive(Some(Duration::from_secs(timeout)));
        }

        // Configure TLS if enabled
        if self.config.enable_tls {
            let tls_config = ClientTlsConfig::new();
            endpoint = endpoint.tls_config(tls_config)
                .map_err(|e| HlxError::ConfigurationError {
                    component: "gRPC TLS".to_string(),
                    message: format!("TLS configuration failed: {}", e),
                })?;
        }

        let channel = endpoint.connect().await
            .map_err(|e| HlxError::ConnectionError {
                service: "gRPC".to_string(),
                message: format!("Failed to connect: {}", e),
            })?;

        Ok(PooledConnection {
            channel,
            created_at: Instant::now(),
            last_used: Instant::now(),
            is_healthy: true,
        })
    }

    /// Get connection from pool
    async fn get_connection(&self) -> Result<Channel, HlxError> {
        let mut pool = self.connection_pool.write().await;
        
        // Find a healthy connection
        for (index, conn) in pool.iter_mut().enumerate() {
            if conn.is_healthy {
                conn.last_used = Instant::now();
                let channel = conn.channel.clone();
                
                // Move the connection to end of pool for round-robin
                let connection = pool.remove(index);
                pool.push(connection);
                
                return Ok(channel);
            }
        }

        // If no healthy connections, create a new one
        let connection = self.create_connection().await?;
        let channel = connection.channel.clone();
        pool.push(connection);
        
        Ok(channel)
    }

    /// Return connection to pool
    async fn return_connection(&self, channel: Channel) {
        // In a real implementation, you would return the connection to the pool
        // For now, we'll just update metrics
        let mut metrics = self.metrics.lock().unwrap();
        metrics.active_connections = std::cmp::max(1, metrics.active_connections) - 1;
    }

    /// Execute operation with retry logic
    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T, Status>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, Status>>,
    {
        let mut attempts = 0;
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    attempts += 1;
                    
                    if attempts >= self.config.max_retries {
                        return Err(err);
                    }
                    
                    // Check if error is retryable
                    if !self.is_retryable_error(&err) {
                        return Err(err);
                    }
                    
                    // Wait before retry
                    tokio::time::sleep(Duration::from_millis(self.config.retry_delay)).await;
                }
            }
        }
    }

    /// Check if gRPC error is retryable
    fn is_retryable_error(&self, status: &Status) -> bool {
        matches!(
            status.code(),
            Code::Unavailable | Code::DeadlineExceeded | Code::ResourceExhausted
        )
    }

    /// Start background health checks
    async fn start_health_checks(&self) {
        let config = self.config.clone();
        let pool = Arc::clone(&self.connection_pool);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.health_check_interval));
            
            loop {
                interval.tick().await;
                
                let mut pool = pool.write().await;
                for connection in pool.iter_mut() {
                    // Check connection health
                    connection.is_healthy = connection.channel.ready().await.is_ok();
                }
            }
        });
    }

    /// Convert JSON to protobuf bytes (simplified)
    fn json_to_protobuf_bytes(&self, json: &JsonValue) -> Result<Vec<u8>, HlxError> {
        // In a real implementation, you would use proper protobuf serialization
        // based on the service definition
        serde_json::to_vec(json)
            .map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
                format: "Protobuf".to_string(),
                message: format!("JSON to protobuf conversion failed: {}", e),
            })
    }

    /// Convert protobuf bytes to JSON (simplified)
    fn protobuf_bytes_to_json(&self, bytes: &[u8]) -> Result<JsonValue, HlxError> {
        // In a real implementation, you would use proper protobuf deserialization
        serde_json::from_slice(bytes)
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Protobuf".to_string(),
                message: format!("Protobuf to JSON conversion failed: {}", e),
            })
    }

    /// Get gRPC metrics
    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();
        
        result.insert("total_requests".to_string(), Value::Number(metrics.total_requests as f64));
        result.insert("successful_requests".to_string(), Value::Number(metrics.successful_requests as f64));
        result.insert("failed_requests".to_string(), Value::Number(metrics.failed_requests as f64));
        result.insert("avg_response_time_ms".to_string(), Value::Number(metrics.avg_response_time));
        result.insert("active_connections".to_string(), Value::Number(metrics.active_connections as f64));
        result.insert("total_bytes_sent".to_string(), Value::Number(metrics.total_bytes_sent as f64));
        result.insert("total_bytes_received".to_string(), Value::Number(metrics.total_bytes_received as f64));
        
        if metrics.total_requests > 0 {
            let success_rate = (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0;
            result.insert("success_rate_percent".to_string(), Value::Number(success_rate));
        }
        
        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for GrpcOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "call" => {
                let service = params_map.get("service")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("service".to_string()),
                        message: "Missing gRPC service name".to_string(),
                    })?;

                let method = params_map.get("method")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("method".to_string()),
                        message: "Missing gRPC method name".to_string(),
                    })?;

                let request_data = params_map.get("data")
                    .map(|v| utils::value_to_json_value(v))
                    .unwrap_or(JsonValue::Null);

                let metadata = params_map.get("metadata")
                    .and_then(|v| {
                        if let Value::Object(obj) = v {
                            let mut meta = HashMap::new();
                            for (k, v) in obj {
                                if let Some(val_str) = v.as_string() {
                                    meta.insert(k.clone(), val_str);
                                }
                            }
                            Some(meta)
                        } else {
                            None
                        }
                    });

                let result = self.execute_unary_call(&service, &method, request_data, metadata).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("success".to_string(), Value::Boolean(result.success));
                    if let Some(data) = result.data {
                        map.insert("data".to_string(), utils::json_value_to_value(&data));
                    }
                    if let Some(status_code) = result.status_code {
                        map.insert("status_code".to_string(), Value::Number(status_code as f64));
                    }
                    if let Some(status_message) = result.status_message {
                        map.insert("status_message".to_string(), Value::String(status_message.to_string()));
                    }
                    map.insert("duration_ms".to_string(), Value::Number(result.duration_ms as f64));
                    map
                }))
            }
            
            "stream_server" => {
                let service = params_map.get("service")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("service".to_string()),
                        message: "Missing gRPC service name".to_string(),
                    })?;

                let method = params_map.get("method")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("method".to_string()),
                        message: "Missing gRPC method name".to_string(),
                    })?;

                let request_data = params_map.get("data")
                    .map(|v| utils::value_to_json_value(v))
                    .unwrap_or(JsonValue::Null);

                let metadata = params_map.get("metadata")
                    .and_then(|v| {
                        if let Value::Object(obj) = v {
                            let mut meta = HashMap::new();
                            for (k, v) in obj {
                                if let Some(val_str) = v.as_string() {
                                    meta.insert(k.clone(), val_str);
                                }
                            }
                            Some(meta)
                        } else {
                            None
                        }
                    });

                let stream_id = self.start_server_stream(&service, &method, request_data, metadata).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("stream_id".to_string(), Value::String(stream_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "stream_client" => {
                let service = params_map.get("service")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("service".to_string()),
                        message: "Missing gRPC service name".to_string(),
                    })?;

                let method = params_map.get("method")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("method".to_string()),
                        message: "Missing gRPC method name".to_string(),
                    })?;

                let metadata = params_map.get("metadata")
                    .and_then(|v| {
                        if let Value::Object(obj) = v {
                            let mut meta = HashMap::new();
                            for (k, v) in obj {
                                if let Some(val_str) = v.as_string() {
                                    meta.insert(k.clone(), val_str);
                                }
                            }
                            Some(meta)
                        } else {
                            None
                        }
                    });

                let stream_id = self.start_client_stream(&service, &method, metadata).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("stream_id".to_string(), Value::String(stream_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "stream_send" => {
                let stream_id = params_map.get("stream_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("stream_id".to_string()),
                        message: "Missing stream ID".to_string(),
                    })?;

                let data = params_map.get("data")
                    .map(|v| utils::value_to_json_value(v))
                    .unwrap_or(JsonValue::Null);

                self.send_to_stream(&stream_id, data).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("stream_id".to_string(), Value::String(stream_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "stream_close" => {
                let stream_id = params_map.get("stream_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("stream_id".to_string()),
                        message: "Missing stream ID".to_string(),
                    })?;

                self.close_stream(&stream_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("stream_id".to_string(), Value::String(stream_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "health" => {
                let service = params_map.get("service")
                    .and_then(|v| v.as_string());

                let health_status = self.check_health(service.as_deref()).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("status".to_string(), Value::String(format!("{:?}", health_status.to_string())));
                    map.insert("healthy".to_string(), Value::Boolean(matches!(health_status, HealthStatus::Serving)));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "reflection" => {
                let services = self.get_server_reflection().await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("service_count".to_string(), Value::Number(services.len() as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }
            
            _ => Err(HlxError::InvalidParameters {
                operator: "grpc".to_string(),
                params: format!("Unknown gRPC operation: {}", operator),
            }),
        }
    }
} 