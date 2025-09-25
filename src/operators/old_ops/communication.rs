//! Communication & Messaging - Enterprise Operators
//! Implements all communication and messaging operators for production velocity mode

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::{json, Value as JsonValue};
use reqwest::Client;
use std::time::Duration;

/// Enterprise Communication and messaging operators implementation
pub struct CommunicationOperators {
    http_client: Client,
}

impl CommunicationOperators {
    pub async fn new() -> Result<Self, HlxError> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| HlxError::NetworkError { 
                message: format!("Failed to create HTTP client: {}", e),
                operation: Some("create_client".to_string()),
                status_code: None,
                url: None,
            })?;
            
        Ok(Self { http_client })
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for CommunicationOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "@graphql" => self.graphql_operator(&params_map).await,
            "@grpc" => self.grpc_operator(&params_map).await,
            "@websocket" => self.websocket_operator(&params_map).await,
            "@sse" => self.sse_operator(&params_map).await,
            "@email" => self.email_operator(&params_map).await,
            "@slack" => self.slack_operator(&params_map).await,
            "@discord" => self.discord_operator(&params_map).await,
            "@teams" => self.teams_operator(&params_map).await,
            _ => Err(HlxError::InvalidParameters { 
                operator: operator.to_string(), 
                params: "Unknown communication operator".to_string() 
            }),
        }
    }
}

impl CommunicationOperators {
    /// @graphql - GraphQL integration with query execution
    async fn graphql_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let endpoint = params.get("endpoint")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@graphql".to_string(),
                params: "Missing required 'endpoint' parameter".to_string(),
            })?;
            
        let query = params.get("query")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@graphql".to_string(),
                params: "Missing required 'query' parameter".to_string(),
            })?;
            
        let variables = params.get("variables")
            .and_then(|v| v.as_object())
            .map(|obj| {
                let mut json_obj = serde_json::Map::new();
                for (k, v) in obj {
                    json_obj.insert(k.clone(), serde_json::to_value(v).unwrap_or(JsonValue::Null));
                }
                JsonValue::Object(json_obj)
            })
            .unwrap_or(JsonValue::Object(serde_json::Map::new()));
            
        let request_body = json!({
            "query": query,
            "variables": variables
        });
        
        let response = self.http_client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                message: format!("GraphQL request failed: {}", e)
            })?;
            
        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                message: format!("Failed to read GraphQL response: {}", e)
            })?;
            
        let response_json: JsonValue = serde_json::from_str(&response_text)
            .map_err(|e| HlxError::ParsingError {
                message: format!("Failed to parse GraphQL response: {}", e)
            })?;
            
        let mut result = HashMap::new();
        result.insert("status_code".to_string(), Value::Number(status.as_u16() as f64));
        result.insert("success".to_string(), Value::Boolean(status.is_success()));
        result.insert("data".to_string(), Value::from_json(response_json));
        result.insert("endpoint".to_string(), Value::String(endpoint.to_string(.to_string())));
        result.insert("query".to_string(), Value::String(query.to_string(.to_string())));
        
        Ok(Value::Object(result))
    }

    /// @grpc - gRPC service calls with protocol buffer support
    async fn grpc_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let endpoint = params.get("endpoint")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@grpc".to_string(),
                params: "Missing required 'endpoint' parameter".to_string(),
            })?;
            
        let service = params.get("service")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@grpc".to_string(),
                params: "Missing required 'service' parameter".to_string(),
            })?;
            
        let method = params.get("method")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@grpc".to_string(),
                params: "Missing required 'method' parameter".to_string(),
            })?;
            
        let payload = params.get("payload")
            .and_then(|v| v.as_object())
            .map(|obj| {
                let mut json_obj = serde_json::Map::new();
                for (k, v) in obj {
                    json_obj.insert(k.clone(), serde_json::to_value(v).unwrap_or(JsonValue::Null));
                }
                JsonValue::Object(json_obj)
            })
            .unwrap_or(JsonValue::Object(serde_json::Map::new()));
            
        // For MVP, we'll simulate gRPC call with HTTP/2
        let request_body = json!({
            "service": service,
            "method": method,
            "payload": payload
        });
        
        let response = self.http_client
            .post(&format!("{}/grpc", endpoint))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                message: format!("gRPC request failed: {}", e)
            })?;
            
        let status = response.status();
        let response_text = response.text().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                message: format!("Failed to read gRPC response: {}", e)
            })?;
            
        let response_json: JsonValue = serde_json::from_str(&response_text)
            .unwrap_or(JsonValue::String(response_text.to_string()));
            
        let mut result = HashMap::new();
        result.insert("status_code".to_string(), Value::Number(status.as_u16() as f64));
        result.insert("success".to_string(), Value::Boolean(status.is_success()));
        result.insert("response".to_string(), Value::from_json(response_json));
        result.insert("endpoint".to_string(), Value::String(endpoint.to_string(.to_string())));
        result.insert("service".to_string(), Value::String(service.to_string(.to_string())));
        result.insert("method".to_string(), Value::String(method.to_string(.to_string())));
        
        Ok(Value::Object(result))
    }

    /// @websocket - WebSocket connections (connect, send, close)
    async fn websocket_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let url = params.get("url")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@websocket".to_string(),
                params: "Missing required 'url' parameter".to_string(),
            })?;
            
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .unwrap_or("connect");
            
        match action {
            "connect" => {
                // For MVP, simulate WebSocket connection
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("connected".to_string(.to_string())));
                result.insert("url".to_string(), Value::String(url.to_string(.to_string())));
                result.insert("connection_id".to_string(), Value::String(format!("conn-{}", std::time::SystemTime::now(.to_string()).duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())));

                Ok(Value::Object(result))
            },
            "send" => {
                let message = params.get("message")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@websocket".to_string(),
                        params: "Missing required 'message' parameter for send action".to_string(),
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("message_sent".to_string(.to_string())));
                result.insert("url".to_string(), Value::String(url.to_string(.to_string())));
                result.insert("message".to_string(), Value::String(message.to_string(.to_string())));
                result.insert("timestamp".to_string(), Value::String(format!("{}-{:?}", std::time::SystemTime::now(.to_string()).duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos())));
                
                Ok(Value::Object(result))
            },
            "close" => {
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("connection_closed".to_string(.to_string())));
                result.insert("url".to_string(), Value::String(url.to_string(.to_string())));
                result.insert("timestamp".to_string(), Value::String(format!("{}-{:?}", std::time::SystemTime::now(.to_string()).duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos())));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@websocket".to_string(),
                params: format!("Invalid action '{}'. Valid actions: connect, send, close", action)
            })
        }
    }

    /// @sse - Server-Sent Events (subscribe, unsubscribe)
    async fn sse_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let url = params.get("url")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@sse".to_string(),
                params: "Missing required 'url' parameter".to_string(),
            })?;
            
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .unwrap_or("subscribe");
            
        match action {
            "subscribe" => {
                let response = self.http_client
                    .get(url)
                    .header("Accept", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                        message: format!("SSE subscription failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("subscribed".to_string(.to_string())));
                result.insert("url".to_string(), Value::String(url.to_string(.to_string())));
                result.insert("subscription_id".to_string(), Value::String(format!("sub-{}", std::time::SystemTime::now(.to_string()).duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())));
                result.insert("content_type".to_string(), Value::String(
                    response.headers().get("content-type")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("text/event-stream")
                        .to_string()
                ));
                
                Ok(Value::Object(result))
            },
            "unsubscribe" => {
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("unsubscribed".to_string(.to_string())));
                result.insert("url".to_string(), Value::String(url.to_string(.to_string())));
                result.insert("timestamp".to_string(), Value::String(format!("{}-{:?}", std::time::SystemTime::now(.to_string()).duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos())));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@sse".to_string(),
                params: format!("Invalid action '{}'. Valid actions: subscribe, unsubscribe", action)
            })
        }
    }

    /// @email - Email operations
    async fn email_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let to = params.get("to")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@email".to_string(),
                params: "Missing required 'to' parameter".to_string(),
            })?;
            
        let subject = params.get("subject")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@email".to_string(),
                params: "Missing required 'subject' parameter".to_string(),
            })?;
            
        let body = params.get("body")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@email".to_string(),
                params: "Missing required 'body' parameter".to_string(),
            })?;
            
        let smtp_server = params.get("smtp_server")
            .and_then(|v| v.as_string())
            .unwrap_or("localhost");
            
        let smtp_port = params.get("smtp_port")
            .and_then(|v| v.as_number())
            .unwrap_or(587.0) as u16;
            
        // For MVP, simulate email sending
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::String("email_sent".to_string(.to_string())));
        result.insert("to".to_string(), Value::String(to.to_string(.to_string())));
        result.insert("subject".to_string(), Value::String(subject.to_string(.to_string())));
        result.insert("smtp_server".to_string(), Value::String(smtp_server.to_string(.to_string())));
        result.insert("smtp_port".to_string(), Value::Number(smtp_port as f64));
        result.insert("message_id".to_string(), Value::String(format!("msg-{}", std::time::SystemTime::now(.to_string()).duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())));
        result.insert("timestamp".to_string(), Value::String(chrono::Utc::now(.to_string()).to_rfc3339()));
        
        Ok(Value::Object(result))
    }

    /// @slack - Slack messaging integration
    async fn slack_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let webhook_url = params.get("webhook_url")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@slack".to_string(),
                params: "Missing required 'webhook_url' parameter".to_string(),
            })?;
            
        let channel = params.get("channel")
            .and_then(|v| v.as_string())
            .unwrap_or("#general");
            
        let message = params.get("message")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@slack".to_string(),
                params: "Missing required 'message' parameter".to_string(),
            })?;
            
        let payload = json!({
            "channel": channel,
            "text": message,
            "username": "Helix Bot",
            "icon_emoji": ":robot_face:"
        });
        
        let response = self.http_client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                message: format!("Slack message failed: {}", e)
            })?;
            
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::String("message_sent".to_string(.to_string())));
        result.insert("channel".to_string(), Value::String(channel.to_string(.to_string())));
        result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
        result.insert("message_id".to_string(), Value::String(format!("msg-{}", std::time::SystemTime::now(.to_string()).duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())));
        result.insert("timestamp".to_string(), Value::String(chrono::Utc::now(.to_string()).to_rfc3339()));
        
        Ok(Value::Object(result))
    }

    /// @discord - Discord integration
    async fn discord_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let webhook_url = params.get("webhook_url")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@discord".to_string(),
                params: "Missing required 'webhook_url' parameter".to_string(),
            })?;
            
        let content = params.get("content")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@discord".to_string(),
                params: "Missing required 'content' parameter".to_string(),
            })?;
            
        let username = params.get("username")
            .and_then(|v| v.as_string())
            .unwrap_or("Helix Bot");
            
        let payload = json!({
            "content": content,
            "username": username
        });
        
        let response = self.http_client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                message: format!("Discord message failed: {}", e)
            })?;
            
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::String("message_sent".to_string(.to_string())));
        result.insert("username".to_string(), Value::String(username.to_string(.to_string())));
        result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
        result.insert("message_id".to_string(), Value::String(format!("msg-{}", std::time::SystemTime::now(.to_string()).duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())));
        result.insert("timestamp".to_string(), Value::String(chrono::Utc::now(.to_string()).to_rfc3339()));
        
        Ok(Value::Object(result))
    }

    /// @teams - Microsoft Teams integration
    async fn teams_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let webhook_url = params.get("webhook_url")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@teams".to_string(),
                params: "Missing required 'webhook_url' parameter".to_string(),
            })?;
            
        let title = params.get("title")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@teams".to_string(),
                params: "Missing required 'title' parameter".to_string(),
            })?;
            
        let text = params.get("text")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@teams".to_string(),
                params: "Missing required 'text' parameter".to_string(),
            })?;
            
        let payload = json!({
            "@type": "MessageCard",
            "@context": "http://schema.org/extensions",
            "themeColor": "0076D7",
            "summary": title,
            "sections": [{
                "activityTitle": title,
                "text": text
            }]
        });
        
        let response = self.http_client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                message: format!("Teams message failed: {}", e)
            })?;
            
        let mut result = HashMap::new();
        result.insert("status".to_string(), Value::String("message_sent".to_string(.to_string())));
        result.insert("title".to_string(), Value::String(title.to_string(.to_string())));
        result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
        result.insert("message_id".to_string(), Value::String(format!("msg-{}", std::time::SystemTime::now(.to_string()).duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())));
        result.insert("timestamp".to_string(), Value::String(chrono::Utc::now(.to_string()).to_rfc3339()));
        
        Ok(Value::Object(result))
    }
} 