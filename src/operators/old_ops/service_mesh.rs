//! Service Mesh & Infrastructure - Enterprise Operators
//! Implements all service mesh and infrastructure operators for production velocity mode

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::{json, Value as JsonValue};
use reqwest::Client;
use std::time::Duration;
use std::process::Command;

/// Enterprise Service Mesh and infrastructure operators implementation
pub struct ServiceMeshOperators {
    http_client: Client,
}

impl ServiceMeshOperators {
    pub async fn new() -> Result<Self, HlxError> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| HlxError::NetworkError {
                message: format!("Failed to create HTTP client: {}", e),
                operation: Some("http_client_creation".to_string()),
                status_code: None,
                url: Some("".to_string())
            })?;
            
        Ok(Self { http_client })
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for ServiceMeshOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "@istio" => self.istio_operator(&params_map).await,
            "@consul" => self.consul_operator(&params_map).await,
            "@vault" => self.vault_operator(&params_map).await,
            "@temporal" => self.temporal_operator(&params_map).await,
            _ => Err(HlxError::InvalidParameters { 
                operator: operator.to_string(), 
                params: "Unknown service mesh operator".to_string() 
            }),
        }
    }
}

impl ServiceMeshOperators {
    /// @istio - Istio service mesh operations (route, policy)
    async fn istio_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@istio".to_string(),
                params: "Missing required 'action' parameter".to_string(),
            })?;
            
        let namespace = params.get("namespace")
            .and_then(|v| v.as_string())
            .unwrap_or("default");
            
        match action {
            "get-virtual-services" => {
                let output = Command::new("kubectl")
                    .args(&["get", "virtualservices", "-n", &namespace, "-o", "json"])
                    .output()
                    .map_err(|e| HlxError::OperationError { operator: "unknown".to_string(),
                        operator: "@istio".to_string(),
                        details: None,
                        operation: "get_virtualservices".to_string(),
                        message: format!("kubectl command failed: {}", e)
                    })?;
                    
                let virtual_services_json = String::from_utf8_lossy(&output.stdout);
                let virtual_services: JsonValue = serde_json::from_str(&virtual_services_json)
                    .map_err(|e| HlxError::ParsingError {
                        message: format!("Failed to parse Istio virtualservices response: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("virtual_services_listed".to_string(.to_string())));
                result.insert("namespace".to_string(), Value::String(namespace.to_string()));
                result.insert("virtual_services".to_string(), Value::from_json(virtual_services));
                
                Ok(Value::Object(result))
            },
            "get-destination-rules" => {
                let output = Command::new("kubectl")
                    .args(&["get", "destinationrules", "-n", &namespace, "-o", "json"])
                    .output()
                    .map_err(|e| HlxError::OperationError { operator: "unknown".to_string(),
                        operator: "@istio".to_string(),
                        details: None,
                        operation: "get_destinationrules".to_string(),
                        message: format!("kubectl command failed: {}", e)
                    })?;
                    
                let destination_rules_json = String::from_utf8_lossy(&output.stdout);
                let destination_rules: JsonValue = serde_json::from_str(&destination_rules_json)
                    .map_err(|e| HlxError::ParsingError {
                        message: format!("Failed to parse Istio destinationrules response: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("destination_rules_listed".to_string(.to_string())));
                result.insert("namespace".to_string(), Value::String(namespace.to_string()));
                result.insert("destination_rules".to_string(), Value::from_json(destination_rules));
                
                Ok(Value::Object(result))
            },
            "get-gateways" => {
                let output = Command::new("kubectl")
                    .args(&["get", "gateways", "-n", &namespace, "-o", "json"])
                    .output()
                    .map_err(|e| HlxError::OperationError { operator: "unknown".to_string(),
                        operator: "@istio".to_string(),
                        details: None,
                        operation: "get_gateways".to_string(),
                        message: format!("kubectl command failed: {}", e)
                    })?;

                let gateways_json = String::from_utf8_lossy(&output.stdout);
                let gateways: JsonValue = serde_json::from_str(&gateways_json)
                    .map_err(|e| HlxError::ParsingError {
                        message: format!("Failed to parse Istio gateways response: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("gateways_listed".to_string(.to_string())));
                result.insert("namespace".to_string(), Value::String(namespace.to_string()));
                result.insert("gateways".to_string(), Value::from_json(gateways));
                
                Ok(Value::Object(result))
            },
            "create-virtual-service" => {
                let name = params.get("name")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@istio".to_string(),
                        params: "Missing required 'name' parameter".to_string(),
                    })?;
                    
                let host = params.get("host")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@istio".to_string(),
                        params: "Missing required 'host' parameter".to_string(),
                    })?;
                    
                let virtual_service_yaml = format!(
                    r#"apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: {}
  namespace: {}
spec:
  hosts:
  - {}
  http:
  - route:
    - destination:
        host: {}
        port:
          number: 80
"#,
                    name, namespace, host, host
                );
                
                let output = Command::new("kubectl")
                    .args(&["apply", "-f", "-"])
                    .stdin(std::process::Stdio::piped())
                    .output()
                    .map_err(|e| HlxError::OperationError { operator: "unknown".to_string(),
                        operator: "@istio".to_string(),
                        details: None,
                        operation: "create_virtual_service".to_string(),
                        message: format!("kubectl apply failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("virtual_service_created".to_string(.to_string())));
                result.insert("name".to_string(), Value::String(name.to_string()));
                result.insert("host".to_string(), Value::String(host.to_string()));
                result.insert("namespace".to_string(), Value::String(namespace.to_string()));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@istio".to_string(),
                params: format!("Invalid Istio action '{}'. Valid actions: get-virtual-services, get-destination-rules, get-gateways, create-virtual-service", action)
            })
        }
    }

    /// @consul - HashiCorp Consul integration (register, deregister, get, put)
    async fn consul_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@consul".to_string(),
                params: "Missing required 'action' parameter".to_string(),
            })?;
            
        let consul_url = params.get("consul_url")
            .and_then(|v| v.as_string())
            .unwrap_or("http://localhost:8500");
            
        match action {
            "register-service" => {
                let service_name = params.get("service_name")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@consul".to_string(),
                        params: "Missing required 'service_name' parameter".to_string(),
                    })?;
                    
                let service_id = params.get("service_id")
                    .and_then(|v| v.as_string())
                    .unwrap_or(&service_name);
                    
                let service_address = params.get("service_address")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@consul".to_string(),
                        params: "Missing required 'service_address' parameter".to_string(),
                    })?;
                    
                let service_port = params.get("service_port")
                    .and_then(|v| v.as_number())
                    .unwrap_or(8080.0) as u16;
                    
                let payload = json!({
                    "ID": service_id,
                    "Name": service_name,
                    "Address": service_address,
                    "Port": service_port,
                    "Check": {
                        "HTTP": format!("http://{}:{}", service_address, service_port),
                        "Interval": "10s"
                    }
                });
                
                let response = self.http_client
                    .put(&format!("{}/v1/agent/service/register", consul_url))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("consul_register_service".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/agent/service/register", consul_url)),
                        message: format!("Consul API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("service_registered".to_string(.to_string())));
                result.insert("service_name".to_string(), Value::String(service_name.to_string()));
                result.insert("service_id".to_string(), Value::String(service_id.to_string(.to_string())));
                result.insert("service_address".to_string(), Value::String(service_address.to_string()));
                result.insert("service_port".to_string(), Value::Number(service_port as f64));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "deregister-service" => {
                let service_id = params.get("service_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@consul".to_string(),
                        params: "Missing required 'service_id' parameter".to_string(),
                    })?;
                    
                let response = self.http_client
                    .put(&format!("{}/v1/agent/service/deregister/{}", consul_url, service_id))
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("consul_deregister_service".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/agent/service/deregister/{}", consul_url, service_id)),
                        message: format!("Consul API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("service_deregistered".to_string(.to_string())));
                result.insert("service_id".to_string(), Value::String(service_id.to_string()));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "get-services" => {
                let response = self.http_client
                    .get(&format!("{}/v1/agent/services", consul_url))
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("consul_get_services".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/agent/services", consul_url)),
                        message: format!("Consul API request failed: {}", e)
                    })?;
                    
                let services_text = response.text().await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("consul_read_services_response".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/agent/services", consul_url)),
                        message: format!("Failed to read Consul response: {}", e)
                    })?;
                    
                let services: JsonValue = serde_json::from_str(&services_text)
                    .map_err(|e| HlxError::ParsingError {
                        message: format!("Failed to parse Consul services response: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("services_retrieved".to_string(.to_string())));
                result.insert("services".to_string(), Value::from_json(services));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "put-key" => {
                let key = params.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@consul".to_string(),
                        params: "Missing required 'key' parameter".to_string(),
                    })?;
                    
                let value = params.get("value")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@consul".to_string(),
                        params: "Missing required 'value' parameter".to_string(),
                    })?;
                    
                let response = self.http_client
                    .put(&format!("{}/v1/kv/{}", consul_url, key))
                    .body(value)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("consul_put_key".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/kv/{}", consul_url, key)),
                        message: format!("Consul API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("key_put".to_string(.to_string())));
                result.insert("key".to_string(), Value::String(key.to_string()));
                result.insert("value".to_string(), Value::String(value.to_string()));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "get-key" => {
                let key = params.get("key")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@consul".to_string(),
                        params: "Missing required 'key' parameter".to_string(),
                    })?;
                    
                let response = self.http_client
                    .get(&format!("{}/v1/kv/{}", consul_url, key))
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("consul_get_key".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/kv/{}", consul_url, key)),
                        message: format!("Consul API request failed: {}", e)
                    })?;

                let key_data_text = response.text().await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("consul_read_key_response".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/kv/{}", consul_url, key)),
                        message: format!("Failed to read Consul response: {}", e)
                    })?;
                
                let key_data: JsonValue = serde_json::from_str(&key_data_text)
                    .map_err(|e| HlxError::ParsingError {
                        message: format!("Failed to parse Consul key response: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("key_retrieved".to_string(.to_string())));
                result.insert("key".to_string(), Value::String(key.to_string()));
                result.insert("data".to_string(), Value::from_json(key_data));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@consul".to_string(),
                params: format!("Invalid Consul action '{}'. Valid actions: register-service, deregister-service, get-services, put-key, get-key", action)
            })
        }
    }

    /// @vault - HashiCorp Vault secret management (read, write, delete)
    async fn vault_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@vault".to_string(),
                params: "Missing required 'action' parameter".to_string(),
            })?;
            
        let vault_url = params.get("vault_url")
            .and_then(|v| v.as_string())
            .unwrap_or("http://localhost:8200");
            
        let token = params.get("token")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@vault".to_string(),
                params: "Missing required 'token' parameter".to_string(),
            })?;
            
        match action {
            "read-secret" => {
                let path = params.get("path")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@vault".to_string(),
                        params: "Missing required 'path' parameter".to_string(),
                    })?;
                    
                let response = self.http_client
                    .get(&format!("{}/v1/{}", vault_url, path))
                    .header("X-Vault-Token", token)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("vault_read_secret".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/{}", vault_url, path)),
                        message: format!("Vault API request failed: {}", e)
                    })?;

                let secret_text = response.text().await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("vault_read_secret_response".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/{}", vault_url, path)),
                        message: format!("Failed to read Vault response: {}", e)
                    })?;
                
                let secret: JsonValue = serde_json::from_str(&secret_text)
                    .map_err(|e| HlxError::ParsingError {
                        message: format!("Failed to parse Vault secret response: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("secret_read".to_string(.to_string())));
                result.insert("path".to_string(), Value::String(path.to_string()));
                result.insert("secret".to_string(), Value::from_json(secret));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "write-secret" => {
                let path = params.get("path")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@vault".to_string(),
                        params: "Missing required 'path' parameter".to_string(),
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
                    
                let payload = json!({
                    "data": data
                });
                
                let response = self.http_client
                    .post(&format!("{}/v1/{}", vault_url, path))
                    .header("X-Vault-Token", token)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("vault_write_secret".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/{}", vault_url, path)),
                        message: format!("Vault API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("secret_written".to_string(.to_string())));
                result.insert("path".to_string(), Value::String(path.to_string()));
                result.insert("data".to_string(), Value::from_json(data));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "delete-secret" => {
                let path = params.get("path")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@vault".to_string(),
                        params: "Missing required 'path' parameter".to_string(),
                    })?;
                    
                let response = self.http_client
                    .delete(&format!("{}/v1/{}", vault_url, path))
                    .header("X-Vault-Token", token)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("vault_delete_secret".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/{}", vault_url, path)),
                        message: format!("Vault API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("secret_deleted".to_string(.to_string())));
                result.insert("path".to_string(), Value::String(path.to_string()));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "list-secrets" => {
                let path = params.get("path")
                    .and_then(|v| v.as_string())
                    .unwrap_or("secret");
                    
                let response = self.http_client
                    .get(&format!("{}/v1/{}/metadata", vault_url, path))
                    .header("X-Vault-Token", token)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("vault_list_secrets".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/{}/metadata", vault_url, path)),
                        message: format!("Vault API request failed: {}", e)
                    })?;

                let secrets_text = response.text().await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("vault_read_secrets_response".to_string()),
                        status_code: None,
                        url: Some(format!("{}/v1/{}/metadata", vault_url, path)),
                        message: format!("Failed to read Vault response: {}", e)
                    })?;
                
                let secrets: JsonValue = serde_json::from_str(&secrets_text)
                    .map_err(|e| HlxError::ParsingError {
                        message: format!("Failed to parse Vault secrets response: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("secrets_listed".to_string(.to_string())));
                result.insert("path".to_string(), Value::String(path.to_string()));
                result.insert("secrets".to_string(), Value::from_json(secrets));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@vault".to_string(),
                params: format!("Invalid Vault action '{}'. Valid actions: read-secret, write-secret, delete-secret, list-secrets", action)
            })
        }
    }

    /// @temporal - Temporal workflow management (start_workflow, signal_workflow, query_workflow)
    async fn temporal_operator(&self, params: &HashMap<String, Value>) -> Result<Value, HlxError> {
        let action = params.get("action")
            .and_then(|v| v.as_string())
            .ok_or_else(|| HlxError::InvalidParameters {
                operator: "@temporal".to_string(),
                params: "Missing required 'action' parameter".to_string(),
            })?;
            
        let temporal_url = params.get("temporal_url")
            .and_then(|v| v.as_string())
            .unwrap_or("http://localhost:8088");
            
        match action {
            "start-workflow" => {
                let workflow_id = params.get("workflow_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@temporal".to_string(),
                        params: "Missing required 'workflow_id' parameter".to_string(),
                    })?;
                    
                let workflow_type = params.get("workflow_type")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@temporal".to_string(),
                        params: "Missing required 'workflow_type' parameter".to_string(),
                    })?;
                    
                let task_queue = params.get("task_queue")
                    .and_then(|v| v.as_string())
                    .unwrap_or("default");
                    
                let input = params.get("input")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        let mut json_obj = serde_json::Map::new();
                        for (k, v) in obj {
                            json_obj.insert(k.clone(), serde_json::to_value(v).unwrap_or(JsonValue::Null));
                        }
                        JsonValue::Object(json_obj)
                    })
                    .unwrap_or(JsonValue::Object(serde_json::Map::new()));
                    
                let payload = json!({
                    "workflowId": workflow_id,
                    "workflowType": workflow_type,
                    "taskQueue": task_queue,
                    "input": input
                });
                
                let response = self.http_client
                    .post(&format!("{}/api/v1/namespaces/default/workflows", temporal_url))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("temporal_start_workflow".to_string()),
                        status_code: None,
                        url: Some(format!("{}/api/v1/namespaces/default/workflows", temporal_url)),
                        message: format!("Temporal API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("workflow_started".to_string(.to_string())));
                result.insert("workflow_id".to_string(), Value::String(workflow_id.to_string()));
                result.insert("workflow_type".to_string(), Value::String(workflow_type.to_string()));
                result.insert("task_queue".to_string(), Value::String(task_queue.to_string()));
                result.insert("input".to_string(), Value::from_json(input));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "signal-workflow" => {
                let workflow_id = params.get("workflow_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@temporal".to_string(),
                        params: "Missing required 'workflow_id' parameter".to_string(),
                    })?;
                    
                let signal_name = params.get("signal_name")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@temporal".to_string(),
                        params: "Missing required 'signal_name' parameter".to_string(),
                    })?;
                    
                let input = params.get("input")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        let mut json_obj = serde_json::Map::new();
                        for (k, v) in obj {
                            json_obj.insert(k.clone(), serde_json::to_value(v).unwrap_or(JsonValue::Null));
                        }
                        JsonValue::Object(json_obj)
                    })
                    .unwrap_or(JsonValue::Object(serde_json::Map::new()));
                    
                let payload = json!({
                    "signalName": signal_name,
                    "input": input
                });
                
                let response = self.http_client
                    .post(&format!("{}/api/v1/namespaces/default/workflows/{}/signal", temporal_url, workflow_id))
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("temporal_signal_workflow".to_string()),
                        status_code: None,
                        url: Some(format!("{}/api/v1/namespaces/default/workflows/{}/signal", temporal_url, workflow_id)),
                        message: format!("Temporal API request failed: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("workflow_signaled".to_string(.to_string())));
                result.insert("workflow_id".to_string(), Value::String(workflow_id.to_string()));
                result.insert("signal_name".to_string(), Value::String(signal_name.to_string()));
                result.insert("input".to_string(), Value::from_json(input));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "query-workflow" => {
                let workflow_id = params.get("workflow_id")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@temporal".to_string(),
                        params: "Missing required 'workflow_id' parameter".to_string(),
                    })?;
                    
                let query_type = params.get("query_type")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::InvalidParameters {
                        operator: "@temporal".to_string(),
                        params: "Missing required 'query_type' parameter".to_string(),
                    })?;
                    
                let response = self.http_client
                    .get(&format!("{}/api/v1/namespaces/default/workflows/{}/query?queryType={}", temporal_url, workflow_id, query_type))
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("temporal_query_workflow".to_string()),
                        status_code: None,
                        url: Some(format!("{}/api/v1/namespaces/default/workflows/{}/query?queryType={}", temporal_url, workflow_id, query_type)),
                        message: format!("Temporal API request failed: {}", e)
                    })?;

                let query_result_text = response.text().await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("temporal_read_query_response".to_string()),
                        status_code: None,
                        url: Some(format!("{}/api/v1/namespaces/default/workflows/{}/query?queryType={}", temporal_url, workflow_id, query_type)),
                        message: format!("Failed to read Temporal response: {}", e)
                    })?;
                
                let query_result: JsonValue = serde_json::from_str(&query_result_text)
                    .map_err(|e| HlxError::ParsingError {
                        message: format!("Failed to parse Temporal query response: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("workflow_queried".to_string(.to_string())));
                result.insert("workflow_id".to_string(), Value::String(workflow_id.to_string()));
                result.insert("query_type".to_string(), Value::String(query_type.to_string()));
                result.insert("result".to_string(), Value::from_json(query_result));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            "list-workflows" => {
                let response = self.http_client
                    .get(&format!("{}/api/v1/namespaces/default/workflows", temporal_url))
                    .send()
                    .await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("temporal_list_workflows".to_string()),
                        status_code: None,
                        url: Some(format!("{}/api/v1/namespaces/default/workflows", temporal_url)),
                        message: format!("Temporal API request failed: {}", e)
                    })?;

                let workflows_text = response.text().await
                    .map_err(|e| HlxError::NetworkError {
                        operation: Some("temporal_read_workflows_response".to_string()),
                        status_code: None,
                        url: Some(format!("{}/api/v1/namespaces/default/workflows", temporal_url)),
                        message: format!("Failed to read Temporal response: {}", e)
                    })?;
                
                let workflows: JsonValue = serde_json::from_str(&workflows_text)
                    .map_err(|e| HlxError::ParsingError {
                        message: format!("Failed to parse Temporal workflows response: {}", e)
                    })?;
                    
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::String("workflows_listed".to_string(.to_string())));
                result.insert("workflows".to_string(), Value::from_json(workflows));
                result.insert("response_status".to_string(), Value::Number(response.status().as_u16() as f64));
                
                Ok(Value::Object(result))
            },
            _ => Err(HlxError::InvalidParameters {
                operator: "@temporal".to_string(),
                params: format!("Invalid Temporal action '{}'. Valid actions: start-workflow, signal-workflow, query-workflow, list-workflows", action)
            })
        }
    }
} 