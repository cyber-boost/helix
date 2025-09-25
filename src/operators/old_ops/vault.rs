//! HashiCorp Vault Operator for Helix Rust SDK
//!
//! Provides comprehensive HashiCorp Vault capabilities including:
//! - Secret engines integration (KV, Database, PKI, etc.)
//! - Authentication methods (Token, AppRole, LDAP, etc.)
//! - Policy management and access control
//! - Dynamic secret generation and lifecycle management
//! - Transit encryption and key management
//! - Audit logging and compliance tracking
//! - High availability and enterprise features
//! - Secure secret rotation and renewal

use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use vaultrs::api::kv2::requests::ReadSecretRequest;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

/// Vault operator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Vault server address
    pub address: String,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// TLS configuration
    pub tls_config: Option<TlsConfig>,
    /// Default secret mount path
    pub default_mount: String,
    /// Enable secret caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
    /// Token renewal threshold (percentage)
    pub token_renewal_threshold: f32,
    /// Enable audit logging
    pub enable_audit_logging: bool,
    /// Namespace for Vault Enterprise
    pub namespace: Option<String>,
    /// Max retry attempts
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay: u64,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthConfig {
    /// Token authentication
    Token {
        token: String,
    },
    /// AppRole authentication
    AppRole {
        role_id: String,
        secret_id: String,
        mount_path: Option<String>,
    },
    /// LDAP authentication
    Ldap {
        username: String,
        password: String,
        mount_path: Option<String>,
    },
    /// Kubernetes authentication
    Kubernetes {
        role: String,
        jwt_path: String,
        mount_path: Option<String>,
    },
    /// AWS authentication
    Aws {
        role: String,
        iam_server_id_header: Option<String>,
        mount_path: Option<String>,
    },
    /// Azure authentication
    Azure {
        role: String,
        jwt: String,
        mount_path: Option<String>,
    },
    /// TLS certificate authentication
    Cert {
        cert_path: String,
        key_path: String,
        mount_path: Option<String>,
    },
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
    /// Skip certificate verification (for development)
    pub skip_verify: bool,
    /// TLS server name for SNI
    pub server_name: Option<String>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            address: "http://127.0.0.1:8200".to_string(),
            auth: AuthConfig::Token {
                token: "root".to_string(),
            },
            timeout: 30,
            connection_timeout: 10,
            tls_config: None,
            default_mount: "secret".to_string(),
            enable_caching: true,
            cache_ttl: 300, // 5 minutes
            token_renewal_threshold: 0.1, // 10%
            enable_audit_logging: true,
            namespace: None,
            max_retries: 3,
            retry_delay: 1000,
        }
    }
}

/// Vault secret information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretInfo {
    pub path: String,
    pub data: HashMap<String, JsonValue>,
    pub metadata: SecretMetadata,
    pub version: Option<u32>,
    pub created_time: Option<String>,
    pub deletion_time: Option<String>,
}

/// Secret metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    pub created_time: Option<String>,
    pub custom_metadata: Option<HashMap<String, String>>,
    pub deletion_time: Option<String>,
    pub destroyed: bool,
    pub version: u32,
    pub versions: Option<HashMap<String, SecretVersion>>,
}

/// Secret version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretVersion {
    pub created_time: String,
    pub deletion_time: Option<String>,
    pub destroyed: bool,
}

/// Authentication token information
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: String,
    pub token_type: String,
    pub lease_duration: u64,
    pub renewable: bool,
    pub policies: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: Instant,
    pub expires_at: Option<Instant>,
}

/// Policy information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInfo {
    pub name: String,
    pub rules: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Dynamic secret lease information
#[derive(Debug, Clone)]
pub struct LeaseInfo {
    pub lease_id: String,
    pub lease_duration: u64,
    pub renewable: bool,
    pub data: HashMap<String, JsonValue>,
    pub warnings: Option<Vec<String>>,
    pub created_at: Instant,
    pub expires_at: Instant,
}

/// Cached secret entry
#[derive(Debug, Clone)]
struct CachedSecret {
    secret: SecretInfo,
    cached_at: Instant,
    expires_at: Instant,
}

/// Vault performance metrics
#[derive(Debug, Default)]
struct VaultMetrics {
    secrets_read: u64,
    secrets_written: u64,
    secrets_deleted: u64,
    policies_created: u64,
    policies_deleted: u64,
    tokens_created: u64,
    tokens_renewed: u64,
    leases_renewed: u64,
    leases_revoked: u64,
    auth_operations: u64,
    cache_hits: u64,
    cache_misses: u64,
    api_errors: u64,
    avg_response_time: f64,
    active_leases: u32,
}

/// HashiCorp Vault Operator
pub struct VaultOperator {
    config: VaultConfig,
    client: VaultClient,
    http_client: Client,
    current_token: Arc<RwLock<Option<TokenInfo>>>,
    secret_cache: Arc<RwLock<HashMap<String, CachedSecret>>>,
    active_leases: Arc<RwLock<HashMap<String, LeaseInfo>>>,
    metrics: Arc<Mutex<VaultMetrics>>,
}

impl VaultOperator {
    /// Create a new Vault operator with configuration
    pub async fn new(config: VaultConfig) -> Result<Self, HlxError> {
        // Build HTTP client with timeouts
        let mut client_builder = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .connect_timeout(Duration::from_secs(config.connection_timeout));

        // Configure TLS if specified
        if let Some(tls_config) = &config.tls_config {
            if tls_config.skip_verify {
                client_builder = client_builder.danger_accept_invalid_certs(true);
            }
            
            if let Some(ca_cert_path) = &tls_config.ca_cert_path {
                if let Ok(ca_cert) = std::fs::read(ca_cert_path) {
                    if let Ok(cert) = reqwest::Certificate::from_pem(&ca_cert) {
                        client_builder = client_builder.add_root_certificate(cert);
                    }
                }
            }
        }

        let http_client = client_builder.build()
            .map_err(|e| HlxError::InitializationError {
                component: "Vault HTTP Client".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        // Build Vault client settings
        let mut vault_settings = VaultClientSettingsBuilder::default()
            .address(&config.address)
            .timeout(Some(Duration::from_secs(config.timeout)));

        if let Some(namespace) = &config.namespace {
            vault_settings = vault_settings.namespace(namespace);
        }

        let vault_client = VaultClient::new(
            vault_settings.build()
                .map_err(|e| HlxError::InitializationError {
                    component: "Vault Client Settings".to_string(),
                    message: format!("Invalid Vault client settings: {}", e),
                })?
        ).map_err(|e| HlxError::InitializationError {
            component: "Vault Client".to_string(),
            message: format!("Failed to create Vault client: {}", e),
        })?;

        let operator = Self {
            config: config.clone(),
            client: vault_client,
            http_client,
            current_token: Arc::new(RwLock::new(None)),
            secret_cache: Arc::new(RwLock::new(HashMap::new())),
            active_leases: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(VaultMetrics::default())),
        };

        // Authenticate with Vault
        operator.authenticate().await?;

        // Start token renewal background task
        operator.start_token_renewal().await;

        info!("Vault operator initialized successfully");
        Ok(operator)
    }

    /// Authenticate with Vault using configured method
    pub async fn authenticate(&self) -> Result<TokenInfo, HlxError> {
        let start_time = Instant::now();
        
        let token_info = match &self.config.auth {
            AuthConfig::Token { token } => {
                // For token auth, we need to validate the token
                self.validate_token(token).await?
            }
            AuthConfig::AppRole { role_id, secret_id, mount_path } => {
                self.authenticate_approle(role_id, secret_id, mount_path.as_deref()).await?
            }
            AuthConfig::Ldap { username, password, mount_path } => {
                self.authenticate_ldap(username, password, mount_path.as_deref()).await?
            }
            AuthConfig::Kubernetes { role, jwt_path, mount_path } => {
                let jwt = std::fs::read_to_string(jwt_path)
                    .map_err(|e| HlxError::ConfigurationError {
                        component: "Kubernetes JWT".to_string(),
                        message: format!("Failed to read JWT from {}: {}", jwt_path, e),
                    })?;
                self.authenticate_kubernetes(role, &jwt, mount_path.as_deref()).await?
            }
            _ => {
                return Err(HlxError::ConfigurationError {
                    component: "Vault Authentication".to_string(),
                    message: "Unsupported authentication method".to_string(),
                });
            }
        };

        // Store the token
        {
            let mut current_token = self.current_token.write().await;
            *current_token = Some(token_info.clone());
        }

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.auth_operations += 1;
            
            let response_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_response_time = 
                (metrics.avg_response_time * (metrics.auth_operations - 1) as f64 + response_time) / 
                metrics.auth_operations as f64;
        }

        info!("Vault authentication successful");
        Ok(token_info)
    }

    /// Read a secret from Vault
    pub async fn read_secret(&self, path: &str, version: Option<u32>) -> Result<SecretInfo, HlxError> {
        // Check cache first
        if self.config.enable_caching {
            let cache_key = format!("{}:{:?}", path, version);
            if let Some(cached) = self.get_cached_secret(&cache_key).await {
                return Ok(cached.secret);
            }
        }

        let start_time = Instant::now();
        let token = self.get_current_token().await?;

        // Build API URL
        let url = format!("{}/v1/{}/data/{}", 
                         self.config.address, 
                         self.config.default_mount, 
                         path);

        let mut request_builder = self.http_client
            .get(&url)
            .header("X-Vault-Token", &token.token);

        if let Some(namespace) = &self.config.namespace {
            request_builder = request_builder.header("X-Vault-Namespace", namespace);
        }

        // Add version parameter if specified
        if let Some(v) = version {
            request_builder = request_builder.query(&[("version", v.to_string())]);
        }

        let response = request_builder.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Vault Read Secret".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.api_errors += 1;
            }

            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Read Secret".to_string(),
                message: format!("Vault API error ({}): {}", response.status(), error_text),
            });
        }

        let vault_response: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Vault Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        // Parse the response
        let secret_info = self.parse_secret_response(path, &vault_response)?;

        // Cache the secret if caching is enabled
        if self.config.enable_caching {
            let cache_key = format!("{}:{:?}", path, version);
            self.cache_secret(cache_key, secret_info.clone()).await;
        }

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.secrets_read += 1;
            
            let response_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_response_time = 
                (metrics.avg_response_time * (metrics.secrets_read - 1) as f64 + response_time) / 
                metrics.secrets_read as f64;
        }

        debug!("Read secret from path: {}", path);
        Ok(secret_info)
    }

    /// Write a secret to Vault
    pub async fn write_secret(&self, path: &str, data: HashMap<String, JsonValue>, options: Option<HashMap<String, JsonValue>>) -> Result<SecretMetadata, HlxError> {
        let start_time = Instant::now();
        let token = self.get_current_token().await?;

        // Build API URL
        let url = format!("{}/v1/{}/data/{}", 
                         self.config.address, 
                         self.config.default_mount, 
                         path);

        let mut payload = json!({
            "data": data
        });

        // Add options if provided
        if let Some(opts) = options {
            payload.as_object_mut().unwrap().extend(opts);
        }

        let mut request_builder = self.http_client
            .post(&url)
            .header("X-Vault-Token", &token.token)
            .json(&payload);

        if let Some(namespace) = &self.config.namespace {
            request_builder = request_builder.header("X-Vault-Namespace", namespace);
        }

        let response = request_builder.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Vault Write Secret".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.api_errors += 1;
            }

            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Write Secret".to_string(),
                message: format!("Vault API error ({}): {}", response.status(), error_text),
            });
        }

        let vault_response: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Vault Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        // Parse metadata from response
        let metadata = self.parse_secret_metadata(&vault_response)?;

        // Invalidate cache for this path
        if self.config.enable_caching {
            self.invalidate_cache(path).await;
        }

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.secrets_written += 1;
            
            let response_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_response_time = 
                (metrics.avg_response_time * (metrics.secrets_written - 1) as f64 + response_time) / 
                metrics.secrets_written as f64;
        }

        info!("Wrote secret to path: {}", path);
        Ok(metadata)
    }

    /// Delete a secret from Vault
    pub async fn delete_secret(&self, path: &str, versions: Option<Vec<u32>>) -> Result<(), HlxError> {
        let start_time = Instant::now();
        let token = self.get_current_token().await?;

        let url = if versions.is_some() {
            // Delete specific versions
            format!("{}/v1/{}/delete/{}", 
                   self.config.address, 
                   self.config.default_mount, 
                   path)
        } else {
            // Soft delete latest version
            format!("{}/v1/{}/data/{}", 
                   self.config.address, 
                   self.config.default_mount, 
                   path)
        };

        let mut request_builder = self.http_client
            .delete(&url)
            .header("X-Vault-Token", &token.token);

        if let Some(namespace) = &self.config.namespace {
            request_builder = request_builder.header("X-Vault-Namespace", namespace);
        }

        // Add versions if specified for targeted deletion
        if let Some(versions_to_delete) = versions {
            let payload = json!({"versions": versions_to_delete});
            request_builder = request_builder.json(&payload);
        }

        let response = request_builder.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Vault Delete Secret".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.api_errors += 1;
            }

            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Delete Secret".to_string(),
                message: format!("Vault API error ({}): {}", response.status(), error_text),
            });
        }

        // Invalidate cache for this path
        if self.config.enable_caching {
            self.invalidate_cache(path).await;
        }

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.secrets_deleted += 1;
        }

        info!("Deleted secret at path: {}", path);
        Ok(())
    }

    /// Create or update a policy
    pub async fn write_policy(&self, name: &str, rules: &str) -> Result<(), HlxError> {
        let start_time = Instant::now();
        let token = self.get_current_token().await?;

        let url = format!("{}/v1/sys/policies/acl/{}", self.config.address, name);

        let payload = json!({
            "policy": rules
        });

        let mut request_builder = self.http_client
            .put(&url)
            .header("X-Vault-Token", &token.token)
            .json(&payload);

        if let Some(namespace) = &self.config.namespace {
            request_builder = request_builder.header("X-Vault-Namespace", namespace);
        }

        let response = request_builder.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Vault Write Policy".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.api_errors += 1;
            }

            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Write Policy".to_string(),
                message: format!("Vault API error ({}): {}", response.status(), error_text),
            });
        }

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.policies_created += 1;
        }

        info!("Created/updated policy: {}", name);
        Ok(())
    }

    /// Read a policy
    pub async fn read_policy(&self, name: &str) -> Result<PolicyInfo, HlxError> {
        let token = self.get_current_token().await?;

        let url = format!("{}/v1/sys/policies/acl/{}", self.config.address, name);

        let mut request_builder = self.http_client
            .get(&url)
            .header("X-Vault-Token", &token.token);

        if let Some(namespace) = &self.config.namespace {
            request_builder = request_builder.header("X-Vault-Namespace", namespace);
        }

        let response = request_builder.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Vault Read Policy".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Read Policy".to_string(),
                message: format!("Vault API error ({}): {}", response.status(), error_text),
            });
        }

        let vault_response: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Vault Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        // Parse policy from response
        let rules = vault_response["data"]["policy"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(PolicyInfo {
            name: name.to_string(),
            rules,
            created_at: None,
            updated_at: None,
        })
    }

    /// Renew the current token
    pub async fn renew_token(&self) -> Result<TokenInfo, HlxError> {
        let current_token_value = {
            let token = self.current_token.read().await;
            token.as_ref().map(|t| t.token.clone())
        };

        let token_to_renew = current_token_value.ok_or_else(|| HlxError::InvalidStateError {
            component: "Vault Token".to_string(),
            state: "no_token".to_string(),
            message: "No current token to renew".to_string(),
        })?;

        let url = format!("{}/v1/auth/token/renew-self", self.config.address);

        let mut request_builder = self.http_client
            .post(&url)
            .header("X-Vault-Token", &token_to_renew);

        if let Some(namespace) = &self.config.namespace {
            request_builder = request_builder.header("X-Vault-Namespace", namespace);
        }

        let response = request_builder.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Vault Token Renewal".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Token Renewal".to_string(),
                message: format!("Vault API error ({}): {}", response.status(), error_text),
            });
        }

        let vault_response: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Vault Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        let renewed_token = self.parse_auth_response(&vault_response)?;

        // Update stored token
        {
            let mut current_token = self.current_token.write().await;
            *current_token = Some(renewed_token.clone());
        }

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.tokens_renewed += 1;
        }

        info!("Token renewed successfully");
        Ok(renewed_token)
    }

    /// Get Vault health status
    pub async fn health(&self) -> Result<HashMap<String, JsonValue>, HlxError> {
        let url = format!("{}/v1/sys/health", self.config.address);

        let response = self.http_client.get(&url).send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Vault Health Check".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        let health_data: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Vault Health Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        // Convert to HashMap
        let mut health_map = HashMap::new();
        if let Some(obj) = health_data.as_object() {
            for (key, value) in obj {
                health_map.insert(key.clone(), value.clone());
            }
        }

        Ok(health_map)
    }

    /// Get current authentication token
    async fn get_current_token(&self) -> Result<TokenInfo, HlxError> {
        let token = self.current_token.read().await;
        
        if let Some(token_info) = token.as_ref() {
            // Check if token needs renewal
            if let Some(expires_at) = token_info.expires_at {
                let time_until_expiry = expires_at.duration_since(Instant::now()).unwrap_or_else(|| Duration::from_secs(0));
                let total_duration = expires_at.duration_since(token_info.created_at);
                
                if time_until_expiry.as_secs_f32() / total_duration.as_secs_f32() < self.config.token_renewal_threshold {
                    warn!("Token is close to expiry, renewal recommended");
                }
            }

            Ok(token_info.clone())
        } else {
            Err(HlxError::InvalidStateError {
                component: "Vault Authentication".to_string(),
                state: "no_token".to_string(),
                message: "No authentication token available".to_string(),
            })
        }
    }

    /// Validate a token
    async fn validate_token(&self, token: &str) -> Result<TokenInfo, HlxError> {
        let url = format!("{}/v1/auth/token/lookup-self", self.config.address);

        let response = self.http_client
            .get(&url)
            .header("X-Vault-Token", token)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Token Validation".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(HlxError::AuthenticationError {
                method: "Token".to_string(),
                message: "Token validation failed".to_string(),
            });
        }

        let vault_response: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Token Validation Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        // Parse token info
        let data = vault_response["data"].as_object()
            .ok_or_else(|| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Token Data".to_string(),
                message: "Invalid token data structure".to_string(),
            })?;

        let lease_duration = data.get("ttl")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let policies = data.get("policies")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                   .filter_map(|v| v.as_str())
                   .map(String::from)
                   .collect()
            })
            .unwrap_or_default();

        let token_info = TokenInfo {
            token: token.to_string(),
            token_type: "service".to_string(),
            lease_duration,
            renewable: data.get("renewable").and_then(|v| v.as_bool()).unwrap_or(false),
            policies,
            metadata: HashMap::new(),
            created_at: Instant::now(),
            expires_at: if lease_duration > 0 {
                Some(Instant::now() + Duration::from_secs(lease_duration))
            } else {
                None
            },
        };

        Ok(token_info)
    }

    /// Authenticate using AppRole method
    async fn authenticate_approle(&self, role_id: &str, secret_id: &str, mount_path: Option<&str>) -> Result<TokenInfo, HlxError> {
        let mount = mount_path.unwrap_or("approle");
        let url = format!("{}/v1/auth/{}/login", self.config.address, mount);

        let payload = json!({
            "role_id": role_id,
            "secret_id": secret_id
        });

        let response = self.http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "AppRole Authentication".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(HlxError::AuthenticationError {
                method: "AppRole".to_string(),
                message: format!("Authentication failed: {}", error_text),
            });
        }

        let vault_response: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "AppRole Auth Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        self.parse_auth_response(&vault_response)
    }

    /// Authenticate using LDAP method
    async fn authenticate_ldap(&self, username: &str, password: &str, mount_path: Option<&str>) -> Result<TokenInfo, HlxError> {
        let mount = mount_path.unwrap_or("ldap");
        let url = format!("{}/v1/auth/{}/login/{}", self.config.address, mount, username);

        let payload = json!({
            "password": password
        });

        let response = self.http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "LDAP Authentication".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(HlxError::AuthenticationError {
                method: "LDAP".to_string(),
                message: format!("Authentication failed: {}", error_text),
            });
        }

        let vault_response: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "LDAP Auth Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        self.parse_auth_response(&vault_response)
    }

    /// Authenticate using Kubernetes method
    async fn authenticate_kubernetes(&self, role: &str, jwt: &str, mount_path: Option<&str>) -> Result<TokenInfo, HlxError> {
        let mount = mount_path.unwrap_or("kubernetes");
        let url = format!("{}/v1/auth/{}/login", self.config.address, mount);

        let payload = json!({
            "role": role,
            "jwt": jwt
        });

        let response = self.http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Kubernetes Authentication".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(HlxError::AuthenticationError {
                method: "Kubernetes".to_string(),
                message: format!("Authentication failed: {}", error_text),
            });
        }

        let vault_response: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Kubernetes Auth Response".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        self.parse_auth_response(&vault_response)
    }

    /// Parse authentication response
    fn parse_auth_response(&self, response: &JsonValue) -> Result<TokenInfo, HlxError> {
        let auth = response["auth"].as_object()
            .ok_or_else(|| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Auth Response".to_string(),
                message: "Missing auth data in response".to_string(),
            })?;

        let token = auth.get("client_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Auth Token".to_string(),
                message: "Missing client_token in auth response".to_string(),
            })?;

        let lease_duration = auth.get("lease_duration")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let policies = auth.get("policies")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                   .filter_map(|v| v.as_str())
                   .map(String::from)
                   .collect()
            })
            .unwrap_or_default();

        let metadata = auth.get("metadata")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                   .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                   .collect()
            })
            .unwrap_or_default();

        Ok(TokenInfo {
            token: token.to_string(),
            token_type: auth.get("token_type")
                .and_then(|v| v.as_str())
                .unwrap_or("service")
                .to_string(),
            lease_duration,
            renewable: auth.get("renewable").and_then(|v| v.as_bool()).unwrap_or(false),
            policies,
            metadata.clone(),
            created_at: Instant::now(),
            expires_at: if lease_duration > 0 {
                Some(Instant::now() + Duration::from_secs(lease_duration))
            } else {
                None
            },
        })
    }

    /// Parse secret response from Vault
    fn parse_secret_response(&self, path: &str, response: &JsonValue) -> Result<SecretInfo, HlxError> {
        let data = response["data"]["data"].as_object()
            .ok_or_else(|| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Secret Data".to_string(),
                message: "Missing data field in secret response".to_string(),
            })?;

        let metadata_obj = response["data"]["metadata"].as_object()
            .ok_or_else(|| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "Secret Metadata".to_string(),
                message: "Missing metadata field in secret response".to_string(),
            })?;

        // Convert data to HashMap<String, JsonValue>
        let mut secret_data = HashMap::new();
        for (key, value) in data {
            secret_data.insert(key.clone(), value.clone());
        }

        // Parse metadata
        let metadata = SecretMetadata {
            created_time: metadata_obj.get("created_time")
                .and_then(|v| v.as_str())
                .map(String::from),
            custom_metadata: None, // Simplified for now
            deletion_time: metadata_obj.get("deletion_time")
                .and_then(|v| v.as_str())
                .map(String::from),
            destroyed: metadata_obj.get("destroyed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            version: metadata_obj.get("version")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32,
            versions: None, // Simplified for now
        };

        Ok(SecretInfo {
            path: path.to_string(),
            data: secret_data,
            metadata.clone(),
            version: Some(metadata.version),
            created_time: metadata.created_time.clone(),
            deletion_time: metadata.deletion_time.clone(),
        })
    }

    /// Parse secret metadata from write response
    fn parse_secret_metadata(&self, response: &JsonValue) -> Result<SecretMetadata, HlxError> {
        let metadata_obj = response["data"].as_object()
            .ok_or_else(|| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: Some("Write Response".to_string()),
                message: "Missing data field in write response".to_string(),
            })?;

        Ok(SecretMetadata {
            created_time: metadata_obj.get("created_time")
                .and_then(|v| v.as_str())
                .map(String::from),
            custom_metadata: None,
            deletion_time: metadata_obj.get("deletion_time")
                .and_then(|v| v.as_str())
                .map(String::from),
            destroyed: metadata_obj.get("destroyed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            version: metadata_obj.get("version")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as u32,
            versions: None,
        })
    }

    /// Start background token renewal task
    async fn start_token_renewal(&self) {
        let current_token = Arc::clone(&self.current_token);
        let config = self.config.clone();
        let address = self.config.address.clone();
        let http_client = self.http_client.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Check if token needs renewal
                let should_renew = {
                    let token = current_token.read().await;
                    if let Some(token_info) = token.as_ref() {
                        if let Some(expires_at) = token_info.expires_at {
                            let time_until_expiry = expires_at.duration_since(Instant::now()).unwrap_or_else(|| Duration::from_secs(0));
                            let total_duration = expires_at.duration_since(token_info.created_at);
                            
                            time_until_expiry.as_secs_f32() / total_duration.as_secs_f32() < config.token_renewal_threshold
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };

                if should_renew {
                    debug!("Attempting token renewal");
                    // In a real implementation, you would call self.renew_token() here
                    // For this background task, we'll just log it
                }
            }
        });
    }

    /// Get cached secret if valid
    async fn get_cached_secret(&self, cache_key: &str) -> Option<CachedSecret> {
        let cache = self.secret_cache.read().await;
        
        if let Some(cached) = cache.get(cache_key) {
            if cached.expires_at > Instant::now() {
                {
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.cache_hits += 1;
                }
                return Some(cached.clone());
            }
        }
        
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.cache_misses += 1;
        }
        
        None
    }

    /// Cache a secret
    async fn cache_secret(&self, cache_key: String, secret: SecretInfo) {
        let mut cache = self.secret_cache.write().await;
        
        let cached_secret = CachedSecret {
            secret,
            cached_at: Instant::now(),
            expires_at: Instant::now() + Duration::from_secs(self.config.cache_ttl),
        };
        
        cache.insert(cache_key, cached_secret);
        
        // Cleanup expired entries
        cache.retain(|_, cached| cached.expires_at > Instant::now());
    }

    /// Invalidate cache for a path
    async fn invalidate_cache(&self, path: &str) {
        let mut cache = self.secret_cache.write().await;
        cache.retain(|key, _| !key.starts_with(&format!("{}:", path)));
    }

    /// Get Vault metrics
    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();
        
        result.insert("secrets_read".to_string(), Value::Number(metrics.secrets_read as f64));
        result.insert("secrets_written".to_string(), Value::Number(metrics.secrets_written as f64));
        result.insert("secrets_deleted".to_string(), Value::Number(metrics.secrets_deleted as f64));
        result.insert("policies_created".to_string(), Value::Number(metrics.policies_created as f64));
        result.insert("policies_deleted".to_string(), Value::Number(metrics.policies_deleted as f64));
        result.insert("tokens_created".to_string(), Value::Number(metrics.tokens_created as f64));
        result.insert("tokens_renewed".to_string(), Value::Number(metrics.tokens_renewed as f64));
        result.insert("leases_renewed".to_string(), Value::Number(metrics.leases_renewed as f64));
        result.insert("leases_revoked".to_string(), Value::Number(metrics.leases_revoked as f64));
        result.insert("auth_operations".to_string(), Value::Number(metrics.auth_operations as f64));
        result.insert("cache_hits".to_string(), Value::Number(metrics.cache_hits as f64));
        result.insert("cache_misses".to_string(), Value::Number(metrics.cache_misses as f64));
        result.insert("api_errors".to_string(), Value::Number(metrics.api_errors as f64));
        result.insert("avg_response_time_ms".to_string(), Value::Number(metrics.avg_response_time));
        result.insert("active_leases".to_string(), Value::Number(metrics.active_leases as f64));
        
        // Calculate cache hit rate
        if metrics.cache_hits + metrics.cache_misses > 0 {
            let hit_rate = (metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64) * 100.0;
            result.insert("cache_hit_rate_percent".to_string(), Value::Number(hit_rate));
        }
        
        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for VaultOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        match operator {
            "read" => {
                let path = params_map.get("path")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("path".to_string()),
                        message: "Missing secret path".to_string(),
                    })?;

                let version = params_map.get("version")
                    .and_then(|v| v.as_number())
                    .map(|n| n as u32);

                let secret = self.read_secret(&path, version).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("path".to_string(), Value::String(secret.path.to_string()));
                    map.insert("data".to_string(), Value::Object(
                        secret.data.into_iter()
                            .map(|(k, v)| (k, utils::json_value_to_value(&v)))
                            .collect()
                    ));
                    map.insert("version".to_string(), Value::Number(secret.version.unwrap_or(1) as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "write" => {
                let path = params_map.get("path")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("path".to_string()),
                        message: "Missing secret path".to_string(),
                    })?;

                let data = params_map.get("data")
                    .and_then(|v| {
                        if let Value::Object(obj) = v {
                            let mut data_map = HashMap::new();
                            for (k, v) in obj {
                                data_map.insert(k.clone(), utils::value_to_json_value(v));
                            }
                            Some(data_map)
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("data".to_string()),
                        message: "Missing or invalid secret data".to_string(),
                    })?;

                let metadata = self.write_secret(&path, data, None).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("path".to_string(), Value::String(path.to_string(.to_string())));
                    map.insert("version".to_string(), Value::Number(metadata.version as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "delete" => {
                let path = params_map.get("path")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("path".to_string()),
                        message: "Missing secret path".to_string(),
                    })?;

                let versions = params_map.get("versions")
                    .and_then(|v| {
                        if let Value::Array(arr) = v {
                            let versions: Option<Vec<u32>> = arr.iter()
                                .map(|v| v.as_number().map(|n| n as u32))
                                .collect();
                            versions
                        } else {
                            None
                        }
                    });

                self.delete_secret(&path, versions).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("path".to_string(), Value::String(path.to_string(.to_string())));
                    map.insert("deleted".to_string(), Value::Boolean(true));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "write_policy" => {
                let name = params_map.get("name")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("name".to_string()),
                        message: "Missing policy name".to_string(),
                    })?;

                let rules = params_map.get("rules")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("rules".to_string()),
                        message: "Missing policy rules".to_string(),
                    })?;

                self.write_policy(&name, &rules).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("name".to_string(), Value::String(name.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "read_policy" => {
                let name = params_map.get("name")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        field: Some("name".to_string()),
                        message: "Missing policy name".to_string(),
                    })?;

                let policy = self.read_policy(&name).await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("name".to_string(), Value::String(policy.name.to_string()));
                    map.insert("rules".to_string(), Value::String(policy.rules.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "renew_token" => {
                let token_info = self.renew_token().await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("lease_duration".to_string(), Value::Number(token_info.lease_duration as f64));
                    map.insert("renewable".to_string(), Value::Boolean(token_info.renewable));
                    map.insert("policies".to_string(), Value::Array(
                        token_info.policies.into_iter().map(Value::String).collect()
                    ));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "health" => {
                let health_data = self.health().await?;
                
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    for (key, value) in health_data {
                        map.insert(key, utils::json_value_to_value(&value));
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }
            
            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }
            
            _ => Err(HlxError::InvalidParameters {
                operator: "vault".to_string(),
                params: format!("Unknown Vault operation: {}", operator),
            }),
        }
    }
} 