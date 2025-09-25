//! Consul Service Discovery Operator for Helix Rust SDK  
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsulConfig {
    pub address: String,
    pub datacenter: String,
    pub token: Option<String>,
    pub namespace: Option<String>,
    pub ca_file: Option<String>,
    pub cert_file: Option<String>,
    pub key_file: Option<String>,
    pub timeout: u64,
    pub verify_ssl: bool,
}

impl Default for ConsulConfig {
    fn default() -> Self {
        Self {
            address: "http://localhost:8500".to_string(),
            datacenter: "dc1".to_string(),
            token: None,
            namespace: None,
            ca_file: None,
            cert_file: None,
            key_file: None,
            timeout: 30,
            verify_ssl: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub id: Option<String>,
    pub name: String,
    pub tags: Vec<String>,
    pub address: String,
    pub port: u16,
    pub meta: HashMap<String, String>,
    pub check: Option<HealthCheck>,
    pub checks: Vec<HealthCheck>,
    pub enable_tag_override: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub id: Option<String>,
    pub name: String,
    pub http: Option<String>,
    pub tcp: Option<String>,
    pub script: Option<String>,
    pub interval: String,
    pub timeout: Option<String>,
    pub ttl: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
    pub success_before_passing: Option<u32>,
    pub failures_before_critical: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub session: Option<String>,
    pub acquire: bool,
    pub release: bool,
}

#[derive(Debug, Default)]
struct ConsulMetrics {
    services_registered: u64,
    services_deregistered: u64,
    health_checks_performed: u64,
    key_operations: u64,
    session_operations: u64,
    api_calls: u64,
    api_errors: u64,
}

pub struct ConsulOperator {
    config: ConsulConfig,
    http_client: reqwest::Client,
    metrics: Arc<Mutex<ConsulMetrics>>,
    registered_services: Arc<RwLock<HashMap<String, ServiceDefinition>>>, // service_id -> service
    kv_store: Arc<RwLock<HashMap<String, String>>>, // Mock KV store
}

impl ConsulOperator {
    pub async fn new(config: ConsulConfig) -> Result<Self, HlxError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .map_err(|e| HlxError::InitializationError {
                component: "Consul HTTP Client".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;


        Ok(Self {
            config,
            http_client: client,
            metrics: Arc::new(Mutex::new(ConsulMetrics::default())),
            registered_services: Arc::new(RwLock::new(HashMap::new())),
            kv_store: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn register_service(&self, mut service: ServiceDefinition) -> Result<String, HlxError> {
        let service_id = service.id.clone().unwrap_or_else(|| service.name.clone());

        service.id = Some(service_id.clone());

        // Store in local registry
        {
            let mut services = self.registered_services.write().await;
            services.insert(service_id.clone(), service.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.services_registered += 1;
            metrics.api_calls += 1;
        }

        Ok(service_id)
    }

    pub async fn deregister_service(&self, service_id: &str) -> Result<(), HlxError> {

        {
            let mut services = self.registered_services.write().await;
            services.remove(service_id);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.services_deregistered += 1;
            metrics.api_calls += 1;
        }

        Ok(())
    }

    pub async fn discover_services(&self, service_name: Option<&str>, tag: Option<&str>) -> Result<Vec<ServiceDefinition>, HlxError> {

        let services = self.registered_services.read().await;
        let mut results = Vec::new();

        for service in services.values() {
            let mut matches = true;

            if let Some(name) = service_name {
                if service.name != name {
                    matches = false;
                }
            }

            if let Some(tag_filter) = tag {
                if !service.tags.contains(&tag_filter.to_string()) {
                    matches = false;
                }
            }

            if matches {
                results.push(service.clone());
            }
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.api_calls += 1;
        }

        Ok(results)
    }

    pub async fn health_check(&self, service_id: &str) -> Result<String, HlxError> {

        let services = self.registered_services.read().await;
        if let Some(service) = services.get(service_id) {
            // Mock health check - in real implementation would perform actual health checks
            let status = if service.port > 8000 { "passing" } else { "warning" };

            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.health_checks_performed += 1;
            }

            Ok(status.to_string())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Service".to_string(),
                identifier: service_id.to_string(),
            })
        }
    }

    pub async fn put_kv(&self, key: &str, value: &str) -> Result<(), HlxError> {

        {
            let mut kv = self.kv_store.write().await;
            kv.insert(key.to_string(), value.to_string());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.key_operations += 1;
            metrics.api_calls += 1;
        }

        Ok(())
    }

    pub async fn get_kv(&self, key: &str) -> Result<Option<String>, HlxError> {

        let kv = self.kv_store.read().await;
        let value = kv.get(key).cloned();

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.key_operations += 1;
            metrics.api_calls += 1;
        }

        Ok(value)
    }

    pub async fn delete_kv(&self, key: &str) -> Result<bool, HlxError> {

        let existed = {
            let mut kv = self.kv_store.write().await;
            kv.remove(key).is_some()
        };

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.key_operations += 1;
            metrics.api_calls += 1;
        }

        Ok(existed)
    }

    pub async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, HlxError> {

        let kv = self.kv_store.read().await;
        let keys: Vec<String> = kv.keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect();

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.key_operations += 1;
            metrics.api_calls += 1;
        }

        Ok(keys)
    }

    pub async fn create_session(&self, name: &str, ttl: Option<u64>) -> Result<String, HlxError> {

        let session_id = format!("session-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.session_operations += 1;
            metrics.api_calls += 1;
        }

        Ok(session_id)
    }

    pub async fn destroy_session(&self, session_id: &str) -> Result<(), HlxError> {

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.session_operations += 1;
            metrics.api_calls += 1;
        }

        Ok(())
    }

    pub async fn acquire_lock(&self, key: &str, session_id: &str) -> Result<bool, HlxError> {

        // Mock lock acquisition
        let acquired = !key.contains("locked");

        if acquired {
            let mut kv = self.kv_store.write().await;
            kv.insert(format!("{}_lock", key), session_id.to_string());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.key_operations += 1;
            metrics.api_calls += 1;
        }

        Ok(acquired)
    }

    pub async fn release_lock(&self, key: &str, session_id: &str) -> Result<bool, HlxError> {

        let released = {
            let mut kv = self.kv_store.write().await;
            let lock_key = format!("{}_lock", key);
            if let Some(current_session) = kv.get(&lock_key) {
                if current_session == session_id {
                    kv.remove(&lock_key);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.key_operations += 1;
            metrics.api_calls += 1;
        }

        Ok(released)
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("services_registered".to_string(), Value::Number(metrics.services_registered as f64));
        result.insert("services_deregistered".to_string(), Value::Number(metrics.services_deregistered as f64));
        result.insert("health_checks_performed".to_string(), Value::Number(metrics.health_checks_performed as f64));
        result.insert("key_operations".to_string(), Value::Number(metrics.key_operations as f64));
        result.insert("session_operations".to_string(), Value::Number(metrics.session_operations as f64));
        result.insert("api_calls".to_string(), Value::Number(metrics.api_calls as f64));
        result.insert("api_errors".to_string(), Value::Number(metrics.api_errors as f64));

        if metrics.api_calls > 0 {
            let success_rate = ((metrics.api_calls - metrics.api_errors) as f64 / metrics.api_calls as f64) * 100.0;
            result.insert("api_success_rate_percent".to_string(), Value::Number(success_rate));
        }

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for ConsulOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "register_service" => {
                let name = params_map.get("name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("name".to_string()),
                        message: "Missing service name".to_string(),
                    })?;

                let address = params_map.get("address").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "127.0.0.1");

                let port = params_map.get("port").and_then(|v| v.as_number())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("port".to_string()),
                        message: "Missing service port".to_string(),
                    })? as u16;

                let id = params_map.get("id").and_then(|v| v.as_string());
                let tags = params_map.get("tags").and_then(|v| {
                    if let Value::Array(arr) = v {
                        Some(arr.iter().filter_map(|item| item.as_string()).collect())
                    } else { None }
                }).unwrap_or_else(Vec::new);

                let service = ServiceDefinition {
                    id: id.map(|s| s.to_string()),
                    name: name.to_string(),
                    tags: tags.into_iter().map(|s| s.to_string()).collect(),
                    address: address.to_string(),
                    port,
                    meta: HashMap::new(),
                    check: None,
                    checks: Vec::new(),
                    enable_tag_override: false,
                };

                let service_id = self.register_service(service).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("service_id".to_string(), Value::String(service_id.to_string(.to_string())));
                    map.insert("service_name".to_string(), Value::String(name.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "deregister_service" => {
                let service_id = params_map.get("service_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("service_id".to_string()),
                        message: "Missing service ID".to_string(),
                    })?;

                self.deregister_service(&service_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("service_id".to_string(), Value::String(service_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "discover_services" => {
                let service_name = params_map.get("service_name").and_then(|v| v.as_string());
                let tag = params_map.get("tag").and_then(|v| v.as_string());

                let services = self.discover_services(service_name.as_deref(), tag.as_deref()).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("service_count".to_string(), Value::Number(services.len() as f64));
                    map.insert("services".to_string(), Value::Array(
                        services.into_iter().map(|s| {
                            let mut service_map = HashMap::new();
                            service_map.insert("id".to_string(), s.id.map(Value::String).unwrap_or(Value::Null));
                            service_map.insert("name".to_string(), Value::String(s.name.to_string()));
                            service_map.insert("address".to_string(), Value::String(s.address.to_string()));
                            service_map.insert("port".to_string(), Value::Number(s.port as f64));
                            service_map.insert("tags".to_string(), Value::Array(s.tags.into_iter().map(Value::String).collect()));
                            Value::Object(service_map)
                        }).collect()
                    ));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "health_check" => {
                let service_id = params_map.get("service_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("service_id".to_string()),
                        message: "Missing service ID".to_string(),
                    })?;

                let status = self.health_check(&service_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("service_id".to_string(), Value::String(service_id.to_string(.to_string())));
                    map.insert("status".to_string(), Value::String(status.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "put_kv" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let value = params_map.get("value").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("value".to_string()),
                        message: "Missing value".to_string(),
                    })?;

                self.put_kv(&key, &value).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("value".to_string(), Value::String(value.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_kv" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let value = self.get_kv(&key).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("value".to_string(), value.map(Value::String).unwrap_or(Value::Null));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete_kv" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let existed = self.delete_kv(&key).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("existed".to_string(), Value::Boolean(existed));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "list_keys" => {
                let prefix = params_map.get("prefix").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "");

                let keys = self.list_keys(&prefix).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("prefix".to_string(), Value::String(prefix.to_string(.to_string())));
                    map.insert("key_count".to_string(), Value::Number(keys.len() as f64));
                    map.insert("keys".to_string(), Value::Array(keys.into_iter().map(Value::String).collect()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "create_session" => {
                let name = params_map.get("name").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "default");

                let ttl = params_map.get("ttl").and_then(|v| v.as_number()).map(|n| n as u64);

                let session_id = self.create_session(&name, ttl).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("session_id".to_string(), Value::String(session_id.to_string(.to_string())));
                    map.insert("name".to_string(), Value::String(name.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "destroy_session" => {
                let session_id = params_map.get("session_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("session_id".to_string()),
                        message: "Missing session ID".to_string(),
                    })?;

                self.destroy_session(&session_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("session_id".to_string(), Value::String(session_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "acquire_lock" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let session_id = params_map.get("session_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("session_id".to_string()),
                        message: "Missing session ID".to_string(),
                    })?;

                let acquired = self.acquire_lock(&key, &session_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("session_id".to_string(), Value::String(session_id.to_string(.to_string())));
                    map.insert("acquired".to_string(), Value::Boolean(acquired));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "release_lock" => {
                let key = params_map.get("key").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("key".to_string()),
                        message: "Missing key".to_string(),
                    })?;

                let session_id = params_map.get("session_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("session_id".to_string()),
                        message: "Missing session ID".to_string(),
                    })?;

                let released = self.release_lock(&key, &session_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("key".to_string(), Value::String(key.to_string(.to_string())));
                    map.insert("session_id".to_string(), Value::String(session_id.to_string(.to_string())));
                    map.insert("released".to_string(), Value::Boolean(released));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "consul".to_string(),
                params: format!("Unknown Consul operation: {}", operator),
            }),
        }
    }
} 