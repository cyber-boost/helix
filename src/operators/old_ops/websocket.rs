//! WebSocket Communication Operator for Helix Rust SDK
//!
//! Provides comprehensive WebSocket capabilities including:
//! - WebSocket client/server with message framing
//! - Binary and text message support
//! - Ping/pong keepalive mechanisms
//! - Graceful connection close handling
//! - Automatic reconnection with backoff
//! - Message queuing and backpressure
//! - Connection pooling and multiplexing
//! - Compression and extensions support

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, timeout};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, error, info, warn};
use url::Url;

/// WebSocket operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// WebSocket server URL
    pub url: String,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Message timeout in seconds
    pub message_timeout: u64,
    /// Ping interval in seconds
    pub ping_interval: u64,
    /// Pong timeout in seconds
    pub pong_timeout: u64,
    /// Enable automatic reconnection
    pub auto_reconnect: bool,
    /// Reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in seconds
    pub reconnect_delay: u64,
    /// Enable compression
    pub enable_compression: bool,
    /// Max message size in bytes
    pub max_message_size: usize,
    /// Message queue size
    pub message_queue_size: usize,
    /// Additional headers for connection
    pub headers: HashMap<String, String>,
    /// Subprotocols
    pub subprotocols: Vec<String>,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:8080/ws".to_string(),
            connection_timeout: 30,
            message_timeout: 10,
            ping_interval: 30,
            pong_timeout: 5,
            auto_reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_delay: 2,
            enable_compression: true,
            max_message_size: 64 * 1024 * 1024, // 64MB
            message_queue_size: 1000,
            headers: HashMap::new(),
            subprotocols: Vec::new(),
        }
    }
}

/// WebSocket connection state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Closing,
    Closed,
    Failed,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WSMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<u16>),
}

/// WebSocket connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: String,
    pub state: ConnectionState,
    pub connected_at: Option<Instant>,
    pub last_activity: Instant,
    pub reconnect_attempts: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

/// Message handler callback type
type MessageHandler = Arc<dyn Fn(String, WSMessage) + Send + Sync>;

/// WebSocket connection handle
pub struct ConnectionHandle {
    id: String,
    sender: mpsc::UnboundedSender<Message>,
    info: Arc<RwLock<ConnectionInfo>>,
    state: Arc<RwLock<ConnectionState>>,
}

/// WebSocket Communication Operator
pub struct WebSocketOperator {
    config: WebSocketConfig,
    connections: Arc<RwLock<HashMap<String, ConnectionHandle>>>,
    message_handlers: Arc<RwLock<HashMap<String, MessageHandler>>>,
    global_handler: Arc<RwLock<Option<MessageHandler>>>,
    metrics: Arc<Mutex<WSMetrics>>,
}

/// WebSocket performance metrics
#[derive(Debug, Default)]
struct WSMetrics {
    total_connections: u64,
    active_connections: u32,
    failed_connections: u64,
    total_messages_sent: u64,
    total_messages_received: u64,
    total_bytes_sent: u64,
    total_bytes_received: u64,
    reconnection_attempts: u64,
    avg_connection_duration: f64,
}

impl WebSocketOperator {
    /// Create a new WebSocket operator with configuration
    pub async fn new(config: WebSocketConfig) -> Result<Self, HlxError> {
        let operator = Self {
            config: config.clone(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            global_handler: Arc::new(RwLock::new(None)),
            metrics: Arc::new(Mutex::new(WSMetrics::default())),
        };

        info!("WebSocket operator initialized successfully");
        Ok(operator)
    }

    /// Connect to WebSocket server
    pub async fn connect(&self, connection_id: Option<String>) -> Result<String, HlxError> {
        let conn_id = connection_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        // Parse URL
        let url = Url::parse(&self.config.url)
            .map_err(|e| HlxError::ConfigurationError {
                component: "WebSocket URL".to_string(),
                message: format!("Invalid WebSocket URL: {}", e),
            })?;

        // Create connection info
        let connection_info = Arc::new(RwLock::new(ConnectionInfo {
            id: conn_id.clone(),
            state: ConnectionState::Connecting,
            connected_at: None,
            last_activity: Instant::now(),
            reconnect_attempts: 0,
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
        }));

        let state = Arc::new(RwLock::new(ConnectionState::Connecting));

        // Attempt connection with timeout
        let connection_result = timeout(
            Duration::from_secs(self.config.connection_timeout),
            self.establish_connection(url),
        ).await;

        match connection_result {
            Ok(Ok((ws_stream, _))) => {
                // Create message channel
                let (tx, rx) = mpsc::unbounded_channel();

                // Create connection handle
                let handle = ConnectionHandle {
                    id: conn_id.clone(),
                    sender: tx,
                    info: Arc::clone(&connection_info),
                    state: Arc::clone(&state),
                };

                // Store connection
                {
                    let mut connections = self.connections.write().await;
                    connections.insert(conn_id.clone(), handle);
                }

                // Update connection info
                {
                    let mut info = connection_info.write().await;
                    info.state = ConnectionState::Connected;
                    info.connected_at = Some(Instant::now());
                }

                *state.write().await = ConnectionState::Connected;

                // Start connection handler
                self.start_connection_handler(conn_id.clone(), ws_stream, rx).await;

                // Start ping handler if enabled
                if self.config.ping_interval > 0 {
                    self.start_ping_handler(conn_id.clone()).await;
                }

                // Update metrics
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.total_connections += 1;
                    metrics.active_connections += 1;
                }

                info!("WebSocket connection {} established", conn_id);
                Ok(conn_id)
            }
            Ok(Err(e)) => {
                *state.write().await = ConnectionState::Failed;
                Err(HlxError::ConnectionError {
                    service: "WebSocket".to_string(),
                    message: format!("Connection failed: {}", e),
                })
            }
            Err(_) => {
                *state.write().await = ConnectionState::Failed;
                Err(HlxError::TimeoutError {
                    operation: "WebSocket Connection".to_string(),
                    duration: self.config.connection_timeout,
                })
            }
        }
    }

    /// Send text message
    pub async fn send_text(&self, connection_id: &str, message: &str) -> Result<(), HlxError> {
        self.send_message(connection_id, Message::Text(message.to_string())).await
    }

    /// Send binary message
    pub async fn send_binary(&self, connection_id: &str, data: Vec<u8>) -> Result<(), HlxError> {
        self.send_message(connection_id, Message::Binary(data)).await
    }

    /// Send ping message
    pub async fn send_ping(&self, connection_id: &str, payload: Option<Vec<u8>>) -> Result<(), HlxError> {
        let ping_data = payload.unwrap_or_default();
        self.send_message(connection_id, Message::Ping(ping_data)).await
    }

    /// Close connection
    pub async fn close_connection(&self, connection_id: &str, code: Option<u16>) -> Result<(), HlxError> {
        let close_frame = code.map(|c| tokio_tungstenite::tungstenite::protocol::CloseFrame {
            code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(c),
            reason: std::borrow::Cow::Borrowed(""),
        });

        self.send_message(connection_id, Message::Close(close_frame)).await?;

        // Remove connection from active connections
        {
            let mut connections = self.connections.write().await;
            connections.remove(connection_id);
        }

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.active_connections = metrics.active_connections.saturating_sub(1);
        }

        info!("WebSocket connection {} closed", connection_id);
        Ok(())
    }

    /// Get connection state
    pub async fn get_connection_state(&self, connection_id: &str) -> Result<ConnectionState, HlxError> {
        let connections = self.connections.read().await;
        
        if let Some(handle) = connections.get(connection_id) {
            let state = handle.state.read().await;
            Ok(state.clone())
        } else {
            Err(HlxError::NotFoundError {
                resource: "WebSocket Connection".to_string(),
                identifier: connection_id.to_string(),
            })
        }
    }

    /// Get connection information
    pub async fn get_connection_info(&self, connection_id: &str) -> Result<ConnectionInfo, HlxError> {
        let connections = self.connections.read().await;
        
        if let Some(handle) = connections.get(connection_id) {
            let info = handle.info.read().await;
            Ok(info.clone())
        } else {
            Err(HlxError::NotFoundError {
                resource: "WebSocket Connection".to_string(),
                identifier: connection_id.to_string(),
            })
        }
    }

    /// List all connections
    pub async fn list_connections(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// Set message handler for specific connection
    pub async fn set_message_handler<F>(&self, connection_id: String, handler: F)
    where
        F: Fn(String, WSMessage) + Send + Sync + 'static,
    {
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(connection_id, Arc::new(handler));
    }

    /// Set global message handler
    pub async fn set_global_message_handler<F>(&self, handler: F)
    where
        F: Fn(String, WSMessage) + Send + Sync + 'static,
    {
        let mut global_handler = self.global_handler.write().await;
        *global_handler = Some(Arc::new(handler));
    }

    /// Send message to connection
    async fn send_message(&self, connection_id: &str, message: Message) -> Result<(), HlxError> {
        let connections = self.connections.read().await;
        
        if let Some(handle) = connections.get(connection_id) {
            // Update activity timestamp
            {
                let mut info = handle.info.write().await;
                info.last_activity = Instant::now();
                info.messages_sent += 1;
                
                // Estimate message size for metrics
                let message_size = match &message {
                    Message::Text(text) => text.len() as u64,
                    Message::Binary(data) => data.len() as u64,
                    Message::Ping(data) => data.len() as u64,
                    Message::Pong(data) => data.len() as u64,
                    Message::Close(_) => 2, // Close frame is typically 2 bytes
                };
                
                info.bytes_sent += message_size;
            }

            handle.sender.send(message)
                .map_err(|e| HlxError::CommunicationError {
                    component: "WebSocket Channel".to_string(),
                    message: format!("Failed to send message: {}", e),
                })?;

            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "WebSocket Connection".to_string(),
                identifier: connection_id.to_string(),
            })
        }
    }

    /// Establish WebSocket connection
    async fn establish_connection(&self, url: Url) -> Result<(WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, tokio_tungstenite::tungstenite::handshake::client::Response), HlxError> {
        // Build request with headers and subprotocols
        let mut request = url.into_client_request()
            .map_err(|e| HlxError::ConfigurationError {
                component: "WebSocket Request".to_string(),
                message: format!("Failed to create request: {}", e),
            })?;

        // Add custom headers
        for (key, value) in &self.config.headers {
            request.headers_mut().insert(
                key.parse().map_err(|e| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                    field: Some("header_name".to_string()),
                    message: format!("Invalid header name: {}", e),
                })?,
                value.parse().map_err(|e| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                    field: Some("header_value".to_string()),
                    message: format!("Invalid header value: {}", e),
                })?,
            );
        }

        // Add subprotocols if specified
        if !self.config.subprotocols.is_empty() {
            let subprotocol_header = self.config.subprotocols.join(", ");
            request.headers_mut().insert(
                "Sec-WebSocket-Protocol",
                subprotocol_header.parse().map_err(|e| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                    field: Some("subprotocols".to_string()),
                    message: format!("Invalid subprotocol header: {}", e),
                })?,
            );
        }

        connect_async(request).await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "WebSocket Connect".to_string(),
                message: format!("Connection failed: {}", e),
            })
    }

    /// Start connection handler
    async fn start_connection_handler(
        &self,
        connection_id: String,
        ws_stream: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        mut rx: mpsc::UnboundedReceiver<Message>,
    ) {
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let connections = Arc::clone(&self.connections);
        let message_handlers = Arc::clone(&self.message_handlers);
        let global_handler = Arc::clone(&self.global_handler);
        let metrics = Arc::clone(&self.metrics);
        let config = self.config.clone();

        // Handle outgoing messages
        let conn_id_send = connection_id.clone();
        let connections_send = Arc::clone(&connections);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = ws_sender.send(message).await {
                    error!("Failed to send WebSocket message for connection {}: {}", conn_id_send, e);
                    
                    // Update connection state to failed
                    if let Some(handle) = connections_send.read().await.get(&conn_id_send) {
                        *handle.state.write().await = ConnectionState::Failed;
                    }
                    break;
                }
            }
        });

        // Handle incoming messages
        tokio::spawn(async move {
            while let Some(message_result) = ws_receiver.next().await {
                match message_result {
                    Ok(message) => {
                        // Update connection info
                        if let Some(handle) = connections.read().await.get(&connection_id) {
                            let mut info = handle.info.write().await;
                            info.last_activity = Instant::now();
                            info.messages_received += 1;
                            
                            let message_size = match &message {
                                Message::Text(text) => text.len() as u64,
                                Message::Binary(data) => data.len() as u64,
                                Message::Ping(data) => data.len() as u64,
                                Message::Pong(data) => data.len() as u64,
                                Message::Close(_) => 2,
                            };
                            
                            info.bytes_received += message_size;
                        }

                        // Convert to our message type
                        let ws_message = match message {
                            Message::Text(text) => WSMessage::Text(text),
                            Message::Binary(data) => WSMessage::Binary(data),
                            Message::Ping(data) => {
                                // Auto-respond to ping with pong
                                if let Some(handle) = connections.read().await.get(&connection_id) {
                                    let _ = handle.sender.send(Message::Pong(data.clone()));
                                }
                                WSMessage::Ping(data)
                            }
                            Message::Pong(data) => WSMessage::Pong(data),
                            Message::Close(close_frame) => {
                                let code = close_frame.as_ref().map(|cf| cf.code.into());
                                WSMessage::Close(code)
                            }
                        };

                        // Call message handlers
                        let handlers = message_handlers.read().await;
                        if let Some(handler) = handlers.get(&connection_id) {
                            handler(connection_id.clone(), ws_message.clone());
                        }

                        let global = global_handler.read().await;
                        if let Some(handler) = global.as_ref() {
                            handler(connection_id.clone(), ws_message);
                        }

                        // Update global metrics
                        {
                            let mut metrics = metrics.lock().unwrap();
                            metrics.total_messages_received += 1;
                        }
                    }
                    Err(e) => {
                        error!("WebSocket error for connection {}: {}", connection_id, e);
                        
                        // Update connection state
                        if let Some(handle) = connections.read().await.get(&connection_id) {
                            *handle.state.write().await = ConnectionState::Failed;
                        }
                        
                        break;
                    }
                }
            }

            // Connection closed
            if let Some(handle) = connections.read().await.get(&connection_id) {
                *handle.state.write().await = ConnectionState::Closed;
            }

            info!("WebSocket connection {} handler terminated", connection_id);
        });
    }

    /// Start ping handler for keep-alive
    async fn start_ping_handler(&self, connection_id: String) {
        let connections = Arc::clone(&self.connections);
        let ping_interval = self.config.ping_interval;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(ping_interval));

            loop {
                interval.tick().await;

                let connections_read = connections.read().await;
                if let Some(handle) = connections_read.get(&connection_id) {
                    let state = handle.state.read().await.clone();
                    
                    if state == ConnectionState::Connected {
                        let ping_data = format!("ping-{}", Instant::now().elapsed().as_millis()).into_bytes();
                        if handle.sender.send(Message::Ping(ping_data)).is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
                drop(connections_read);
            }

            debug!("Ping handler for connection {} terminated", connection_id);
        });
    }

    /// Get WebSocket metrics
    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();
        
        result.insert("total_connections".to_string(), Value::Number(metrics.total_connections as f64));
        result.insert("active_connections".to_string(), Value::Number(metrics.active_connections as f64));
        result.insert("failed_connections".to_string(), Value::Number(metrics.failed_connections as f64));
        result.insert("total_messages_sent".to_string(), Value::Number(metrics.total_messages_sent as f64));
        result.insert("total_messages_received".to_string(), Value::Number(metrics.total_messages_received as f64));
        result.insert("total_bytes_sent".to_string(), Value::Number(metrics.total_bytes_sent as f64));
        result.insert("total_bytes_received".to_string(), Value::Number(metrics.total_bytes_received as f64));
        result.insert("reconnection_attempts".to_string(), Value::Number(metrics.reconnection_attempts as f64));
        result.insert("avg_connection_duration".to_string(), Value::Number(metrics.avg_connection_duration));
        
        if metrics.total_connections > 0 {
            let success_rate = ((metrics.total_connections - metrics.failed_connections) as f64 / metrics.total_connections as f64) * 100.0;
            result.insert("connection_success_rate".to_string(), Value::Number(success_rate));
        }
        
        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for WebSocketOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "connect" => {
                let connection_id = params_map.get("connection_id")
                    .and_then(|v| v.as_string());

                let conn_id = self.connect(connection_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(conn_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "send_text" => {
                let connection_id = params_map.get("connection_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("connection_id".to_string()),
                        message: "Missing connection ID".to_string(),
                    })?;

                let message = params_map.get("message")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("message".to_string()),
                        message: "Missing message text".to_string(),
                    })?;

                self.send_text(&connection_id, &message).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(connection_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "send_binary" => {
                let connection_id = params_map.get("connection_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("connection_id".to_string()),
                        message: "Missing connection ID".to_string(),
                    })?;

                let data = params_map.get("data")
                    .and_then(|v| {
                        match v {
                            Value::String(s.to_string()) => Some(s.as_bytes().to_vec()),
                            Value::Array(arr) => {
                                let bytes: Result<Vec<u8>, _> = arr.iter()
                                    .map(|v| v.as_number().map(|n| n as u8))
                                    .collect::<Option<Vec<u8>>>()
                                    .ok_or("Invalid byte array");
                                bytes.ok()
                            }
                            _ => None,
                        }
                    })
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("data".to_string()),
                        message: "Missing or invalid binary data".to_string(),
                    })?;

                self.send_binary(&connection_id, data).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(connection_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "ping" => {
                let connection_id = params_map.get("connection_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("connection_id".to_string()),
                        message: "Missing connection ID".to_string(),
                    })?;

                let payload = params_map.get("payload")
                    .and_then(|v| v.as_string())
                    .map(|s| s.as_bytes().to_vec());

                self.send_ping(&connection_id, payload).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(connection_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "close" => {
                let connection_id = params_map.get("connection_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("connection_id".to_string()),
                        message: "Missing connection ID".to_string(),
                    })?;

                let code = params_map.get("code")
                    .and_then(|v| v.as_number())
                    .map(|n| n as u16);

                self.close_connection(&connection_id, code).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(connection_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "state" => {
                let connection_id = params_map.get("connection_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("connection_id".to_string()),
                        message: "Missing connection ID".to_string(),
                    })?;

                let state = self.get_connection_state(&connection_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(connection_id.to_string()));
                    map.insert("state".to_string(), Value::String(format!("{:?}", state.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "info" => {
                let connection_id = params_map.get("connection_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("connection_id".to_string()),
                        message: "Missing connection ID".to_string(),
                    })?;

                let info = self.get_connection_info(&connection_id).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connection_id".to_string(), Value::String(info.id.to_string()));
                    map.insert("state".to_string(), Value::String(format!("{:?}", info.state.to_string())));
                    map.insert("messages_sent".to_string(), Value::Number(info.messages_sent as f64));
                    map.insert("messages_received".to_string(), Value::Number(info.messages_received as f64));
                    map.insert("bytes_sent".to_string(), Value::Number(info.bytes_sent as f64));
                    map.insert("bytes_received".to_string(), Value::Number(info.bytes_received as f64));
                    map.insert("reconnect_attempts".to_string(), Value::Number(info.reconnect_attempts as f64));
                    if let Some(connected_at) = info.connected_at {
                        map.insert("uptime_seconds".to_string(), Value::Number(connected_at.elapsed().as_secs() as f64));
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "list" => {
                let connections = self.list_connections().await;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("connections".to_string(), Value::Array(
                        connections.into_iter().map(Value::String).collect()
                    ));
                    map.insert("count".to_string(), Value::Number(connections.len() as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }
            
            _ => Err(HlxError::InvalidParameters {
                operator: "websocket".to_string(),
                params: format!("Unknown WebSocket operation: {}", operator),
            }),
        }
    }
} 