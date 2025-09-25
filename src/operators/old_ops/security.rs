//! Security & Encryption - 6 operators
//! 
//! This module implements operators for security and encryption:
//! - @encrypt: Data encryption
//! - @decrypt: Data decryption
//! - @jwt: JWT tokens
//! - @oauth: OAuth authentication
//! - @saml: SAML authentication
//! - @ldap: LDAP authentication

use crate::{HelixResult, HlxError, value::Value};
use crate::security::{SecurityManager, EncryptionAlgorithm, SecurityLevel, KeyDerivationFunction};
use crate::operators::{OperatorTrait, utils};
use serde_json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};
use base64::{Engine as _, engine::general_purpose};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
    iss: String,
    aud: String,
}

/// OAuth2 Token structure
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OAuth2Token {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: Option<String>,
    scope: Option<String>,
}

/// SAML Assertion structure
#[derive(Debug, Serialize, Deserialize, Clone)]
struct SamlAssertion {
    assertion_id: String,
    issuer: String,
    subject: String,
    audience: String,
    not_before: String,
    not_on_or_after: String,
    attributes: HashMap<String, String>,
}

/// LDAP Entry structure
#[derive(Debug, Serialize, Deserialize, Clone)]
struct LdapEntry {
    dn: String,
    attributes: HashMap<String, Vec<String>>,
}

/// RBAC Role structure
#[derive(Debug, Serialize, Deserialize, Clone)]
struct RbacRole {
    name: String,
    permissions: Vec<String>,
    description: Option<String>,
}

/// RBAC User structure
#[derive(Debug, Serialize, Deserialize, Clone)]
struct RbacUser {
    username: String,
    roles: Vec<String>,
    permissions: Vec<String>,
}

/// Security operator context
#[derive(Clone)]
pub struct SecurityOperatorContext {
    security_manager: SecurityManager,
    jwt_secret: String,
    oauth_config: HashMap<String, String>,
    ldap_config: HashMap<String, String>,
    rbac_roles: HashMap<String, RbacRole>,
    rbac_users: HashMap<String, RbacUser>,
}

impl SecurityOperatorContext {
    pub fn new() -> Self {
        Self {
            security_manager: SecurityManager::new(SecurityLevel::High),
            jwt_secret: "default-secret-key-change-in-production".to_string(),
            oauth_config: HashMap::new(),
            ldap_config: HashMap::new(),
            rbac_roles: HashMap::new(),
            rbac_users: HashMap::new(),
        }
    }

    pub fn with_jwt_secret(mut self, secret: String) -> Self {
        self.jwt_secret = secret;
        self
    }

    pub fn with_oauth_config(mut self, config: HashMap<String, String>) -> Self {
        self.oauth_config = config;
        self
    }

    pub fn with_ldap_config(mut self, config: HashMap<String, String>) -> Self {
        self.ldap_config = config;
        self
    }
}

/// @encrypt operator - Data encryption operations
pub fn encrypt_operator(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    if args.len() < 2 {
        return Err(HlxError::Generic {
            message: "@encrypt requires at least 2 arguments: key_id and algorithm".to_string(),
            context: None,
            code: None,
        });
    }

    let key_id = args[0].as_string().ok_or_else(|| HlxError::Generic {
        message: "First argument must be a string (key_id)".to_string(),
        context: None,
        code: None,
    })?;

    let algorithm_str = args[1].as_string().ok_or_else(|| HlxError::Generic {
        message: "Second argument must be a string (algorithm)".to_string(),
        context: None,
        code: None,
    })?;

    let algorithm = match algorithm_str.to_lowercase().as_str() {
        "aes256gcm" => EncryptionAlgorithm::Aes256Gcm,
        "chacha20poly1305" => EncryptionAlgorithm::ChaCha20Poly1305,
        "xchacha20poly1305" => EncryptionAlgorithm::XChaCha20Poly1305,
        _ => {
            return Err(HlxError::Generic {
                message: format!("Unsupported algorithm: {}", algorithm_str),
                context: None,
                code: None,
            });
        }
    };

    // Generate key if it doesn't exist
    if !context.security_manager.list_keys().contains(&key_id.to_string()) {
        context.security_manager.generate_key(&key_id, algorithm.clone())?;
    }

    // Serialize input to bytes
    let data = serde_json::to_vec(input).map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
        format: "json".to_string(),
        message: e.to_string(),
    })?;

    // Encrypt data
    let encrypted_data = context.security_manager.encrypt(&key_id, &data)?;

    // Return encrypted data as JSON
    let result = serde_json::to_value(encrypted_data).map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
        format: "json".to_string(),
        message: e.to_string(),
    })?;

    Ok(Value::from(result))
}

/// @decrypt operator - Data decryption operations
pub fn decrypt_operator(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    if args.is_empty() {
        return Err(HlxError::Generic {
            message: "@decrypt requires at least 1 argument: key_id".to_string(),
            context: None,
            code: None,
        });
    }

    let key_id = args[0].as_string().ok_or_else(|| HlxError::Generic {
        message: "First argument must be a string (key_id)".to_string(),
        context: None,
        code: None,
    })?;

    // Parse encrypted data from input
    let encrypted_data: crate::security::EncryptedData = serde_json::from_value(input.clone().into())
        .map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
            format: "json".to_string(),
            message: e.to_string(),
        })?;

    // Decrypt data
    let decrypted_data = context.security_manager.decrypt(&encrypted_data)?;

    // Deserialize back to Value
    let result = serde_json::from_slice(&decrypted_data).map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
        format: "json".to_string(),
        message: e.to_string(),
    })?;

    Ok(Value::from(result))
}

/// @jwt operator - JWT token management
pub fn jwt_operator(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    if args.is_empty() {
        return Err(HlxError::Generic {
            message: "@jwt requires at least 1 argument: operation".to_string(),
            context: None,
            code: None,
        });
    }

    let operation = args[0].as_string().ok_or_else(|| HlxError::Generic {
        message: "First argument must be a string (operation)".to_string(),
        context: None,
        code: None,
    })?;

    match operation.to_lowercase().as_str() {
        "encode" => jwt_encode(input, context, &args[1..]),
        "decode" => jwt_decode(input, context, &args[1..]),
        "verify" => jwt_verify(input, context, &args[1..]),
        _ => {
            Err(HlxError::Generic {
                message: format!("Unknown JWT operation: {}", operation),
                context: None,
                code: None,
            })
        }
    }
}

fn jwt_encode(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as usize;
    
    let claims = Claims {
        sub: input.as_string().unwrap_or_else(|| "default".to_string()),
        exp: now + 3600, // 1 hour expiration
        iat: now,
        iss: "helix-security".to_string(),
        aud: "helix-users".to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(context.jwt_secret.as_ref())
    ).map_err(|e| HlxError::Generic {
        message: format!("JWT encoding failed: {}", e),
        context: None,
        code: None,
    })?;

    Ok(Value::String(token.to_string()))
}

fn jwt_decode(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let token = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a JWT token string".to_string(),
        context: None,
        code: None,
    })?;

    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(context.jwt_secret.as_ref()),
        &Validation::default()
    ).map_err(|e| HlxError::Generic {
        message: format!("JWT decoding failed: {}", e),
        context: None,
        code: None,
    })?;

    let claims_json = serde_json::to_value(token_data.claims).map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
        format: "json".to_string(),
        message: e.to_string(),
    })?;

    Ok(Value::from(claims_json))
}

fn jwt_verify(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let token = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a JWT token string".to_string(),
        context: None,
        code: None,
    })?;

    let validation = Validation::new(Algorithm::HS256);
    let result = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(context.jwt_secret.as_ref()),
        &validation
    );

    match result {
        Ok(_) => Ok(Value::Bool(true)),
        Err(_) => Ok(Value::Bool(false)),
    }
}

/// @oauth operator - OAuth2 authentication protocols
pub fn oauth_operator(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    if args.is_empty() {
        return Err(HlxError::Generic {
            message: "@oauth requires at least 1 argument: operation".to_string(),
            context: None,
            code: None,
        });
    }

    let operation = args[0].as_string().ok_or_else(|| HlxError::Generic {
        message: "First argument must be a string (operation)".to_string(),
        context: None,
        code: None,
    })?;

    match operation.to_lowercase().as_str() {
        "authorize" => oauth_authorize(input, context, &args[1..]),
        "token" => oauth_token(input, context, &args[1..]),
        "refresh" => oauth_refresh(input, context, &args[1..]),
        "validate" => oauth_validate(input, context, &args[1..]),
        _ => {
            Err(HlxError::Generic {
                message: format!("Unknown OAuth operation: {}", operation),
                context: None,
                code: None,
            })
        }
    }
}

fn oauth_authorize(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let client_id = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a client_id string".to_string(),
        context: None,
        code: None,
    })?;

    // Generate authorization URL
    let auth_url = format!(
        "https://auth.example.com/oauth/authorize?client_id={}&response_type=code&scope=read&redirect_uri={}",
        client_id,
        context.oauth_config.get("redirect_uri").unwrap_or(&"http://localhost/callback".to_string())
    );

    Ok(Value::String(auth_url.to_string()))
}

fn oauth_token(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let auth_code = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be an authorization code string".to_string(),
        context: None,
        code: None,
    })?;

    // Simulate token exchange
    let token = OAuth2Token {
        access_token: format!("access_token_{}", auth_code),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: Some(format!("refresh_token_{}", auth_code)),
        scope: Some("read".to_string()),
    };

    let token_json = serde_json::to_value(token).map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
        format: "json".to_string(),
        message: e.to_string(),
    })?;

    Ok(Value::from(token_json))
}

fn oauth_refresh(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let refresh_token = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a refresh token string".to_string(),
        context: None,
        code: None,
    })?;

    // Simulate token refresh
    let token = OAuth2Token {
        access_token: format!("new_access_token_{}", refresh_token),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: Some(refresh_token),
        scope: Some("read".to_string()),
    };

    let token_json = serde_json::to_value(token).map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
        format: "json".to_string(),
        message: e.to_string(),
    })?;

    Ok(Value::from(token_json))
}

fn oauth_validate(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let access_token = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be an access token string".to_string(),
        context: None,
        code: None,
    })?;

    // Simulate token validation
    let is_valid = access_token.starts_with("access_token_");
    Ok(Value::Bool(is_valid))
}

/// @saml operator - SAML authentication
pub fn saml_operator(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    if args.is_empty() {
        return Err(HlxError::Generic {
            message: "@saml requires at least 1 argument: operation".to_string(),
            context: None,
            code: None,
        });
    }

    let operation = args[0].as_string().ok_or_else(|| HlxError::Generic {
        message: "First argument must be a string (operation)".to_string(),
        context: None,
        code: None,
    })?;

    match operation.to_lowercase().as_str() {
        "create_assertion" => saml_create_assertion(input, context, &args[1..]),
        "validate_assertion" => saml_validate_assertion(input, context, &args[1..]),
        "parse_response" => saml_parse_response(input, context, &args[1..]),
        _ => {
            Err(HlxError::Generic {
                message: format!("Unknown SAML operation: {}", operation),
                context: None,
                code: None,
            })
        }
    }
}

fn saml_create_assertion(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let subject = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a subject string".to_string(),
        context: None,
        code: None,
    })?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let assertion = SamlAssertion {
        assertion_id: format!("_assertion_{}", now),
        issuer: "helix-security".to_string(),
        subject,
        audience: "helix-users".to_string(),
        not_before: now.to_string(),
        not_on_or_after: (now + 3600).to_string(),
        attributes: HashMap::new(),
    };

    let assertion_json = serde_json::to_value(assertion).map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
        format: "json".to_string(),
        message: e.to_string(),
    })?;

    Ok(Value::from(assertion_json))
}

fn saml_validate_assertion(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let assertion: SamlAssertion = serde_json::from_value(input.clone().into())
        .map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
            format: "json".to_string(),
            message: e.to_string(),
        })?;

    // Basic validation
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let not_before: u64 = assertion.not_before.parse().unwrap_or(0);
    let not_on_or_after: u64 = assertion.not_on_or_after.parse().unwrap_or(0);

    let is_valid = now >= not_before && now < not_on_or_after;
    Ok(Value::Bool(is_valid))
}

fn saml_parse_response(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let response = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a SAML response string".to_string(),
        context: None,
        code: None,
    })?;

    // Simulate SAML response parsing
    let parsed_data = serde_json::json!({
        "status": "success",
        "assertion": {
            "subject": "user@example.com",
            "attributes": {
                "email": "user@example.com",
                "name": "Test User"
            }
        }
    });

    Ok(Value::from(parsed_data))
}

/// @ldap operator - LDAP directory operations
pub fn ldap_operator(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    if args.is_empty() {
        return Err(HlxError::Generic {
            message: "@ldap requires at least 1 argument: operation".to_string(),
            context: None,
            code: None,
        });
    }

    let operation = args[0].as_string().ok_or_else(|| HlxError::Generic {
        message: "First argument must be a string (operation)".to_string(),
        context: None,
        code: None,
    })?;

    match operation.to_lowercase().as_str() {
        "search" => ldap_search(input, context, &args[1..]),
        "authenticate" => ldap_authenticate(input, context, &args[1..]),
        "add" => ldap_add(input, context, &args[1..]),
        "modify" => ldap_modify(input, context, &args[1..]),
        "delete" => ldap_delete(input, context, &args[1..]),
        _ => {
            Err(HlxError::Generic {
                message: format!("Unknown LDAP operation: {}", operation),
                context: None,
                code: None,
            })
        }
    }
}

fn ldap_search(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let search_filter = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a search filter string".to_string(),
        context: None,
        code: None,
    })?;

    // Simulate LDAP search
    let entries = vec![
        LdapEntry {
            dn: "cn=testuser,dc=example,dc=com".to_string(),
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("cn".to_string(), vec!["testuser".to_string()]);
                attrs.insert("mail".to_string(), vec!["testuser@example.com".to_string()]);
                attrs
            },
        }
    ];

    let entries_json = serde_json::to_value(entries).map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
        format: "json".to_string(),
        message: e.to_string(),
    })?;

    Ok(Value::from(entries_json))
}

fn ldap_authenticate(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let credentials = input.as_object().ok_or_else(|| HlxError::Generic {
        message: "Input must be an object with username and password".to_string(),
        context: None,
        code: None,
    })?;

    let username = credentials.get("username").and_then(|v| v.as_string()).unwrap_or("");
    let password = credentials.get("password").and_then(|v| v.as_string()).unwrap_or("");

    // Simulate LDAP authentication
    let is_authenticated = !username.is_empty() && !password.is_empty();
    Ok(Value::Bool(is_authenticated))
}

fn ldap_add(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let entry: LdapEntry = serde_json::from_value(input.clone().into())
        .map_err(|e| HlxError::SerializationError {
        format: "json".to_string(),
            format: "json".to_string(),
            message: e.to_string(),
        })?;

    // Simulate LDAP add operation
    let result = serde_json::json!({
        "success": true,
        "dn": entry.dn,
        "message": "Entry added successfully"
    });

    Ok(Value::from(result))
}

fn ldap_modify(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let modifications = input.as_object().ok_or_else(|| HlxError::Generic {
        message: "Input must be an object with modifications".to_string(),
        context: None,
        code: None,
    })?;

    // Simulate LDAP modify operation
    let result = serde_json::json!({
        "success": true,
        "message": "Entry modified successfully",
        "modifications": modifications
    });

    Ok(Value::from(result))
}

fn ldap_delete(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let dn = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a DN string".to_string(),
        context: None,
        code: None,
    })?;

    // Simulate LDAP delete operation
    let result = serde_json::json!({
        "success": true,
        "dn": dn,
        "message": "Entry deleted successfully"
    });

    Ok(Value::from(result))
}

/// @rbac operator - Role-based access control
pub fn rbac_operator(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    if args.is_empty() {
        return Err(HlxError::Generic {
            message: "@rbac requires at least 1 argument: operation".to_string(),
            context: None,
            code: None,
        });
    }

    let operation = args[0].as_string().ok_or_else(|| HlxError::Generic {
        message: "First argument must be a string (operation)".to_string(),
        context: None,
        code: None,
    })?;

    match operation.to_lowercase().as_str() {
        "check_permission" => rbac_check_permission(input, context, &args[1..]),
        "add_role" => rbac_add_role(input, context, &args[1..]),
        "remove_role" => rbac_remove_role(input, context, &args[1..]),
        "assign_role" => rbac_assign_role(input, context, &args[1..]),
        "list_roles" => rbac_list_roles(input, context, &args[1..]),
        _ => {
            Err(HlxError::Generic {
                message: format!("Unknown RBAC operation: {}", operation),
                context: None,
                code: None,
            })
        }
    }
}

fn rbac_check_permission(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    if args.len() < 2 {
        return Err(HlxError::Generic {
            message: "@rbac check_permission requires username and permission".to_string(),
            context: None,
            code: None,
        });
    }

    let username = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a username string".to_string(),
        context: None,
        code: None,
    })?;

    let permission = args[1].as_string().ok_or_else(|| HlxError::Generic {
        message: "Second argument must be a permission string".to_string(),
        context: None,
        code: None,
    })?;

    // Check if user has the permission
    if let Some(user) = context.rbac_users.get(&username) {
        let has_permission = user.permissions.contains(&permission);
        Ok(Value::Bool(has_permission))
    } else {
        Ok(Value::Bool(false))
    }
}

fn rbac_add_role(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    let role_name = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a role name string".to_string(),
        context: None,
        code: None,
    })?;

    let permissions = if args.len() > 1 {
        args[1].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_string())
                .collect::<Vec<String>>()
        }).unwrap_or_default()
    } else {
        Vec::new()
    };

    let role = RbacRole {
        name: role_name.clone(),
        permissions,
        description: None,
    };

    context.rbac_roles.insert(role_name.clone(), role);

    let result = serde_json::json!({
        "success": true,
        "role": role_name,
        "message": "Role added successfully"
    });

    Ok(Value::from(result))
}

fn rbac_remove_role(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let role_name = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a role name string".to_string(),
        context: None,
        code: None,
    })?;

    let removed = context.rbac_roles.remove(&role_name).is_some();

    let result = serde_json::json!({
        "success": removed,
        "role": role_name,
        "message": if removed { "Role removed successfully" } else { "Role not found" }
    });

    Ok(Value::from(result))
}

fn rbac_assign_role(input: &Value, context: &mut SecurityOperatorContext, args: &[Value]) -> HelixResult<Value> {
    if args.is_empty() {
        return Err(HlxError::Generic {
            message: "@rbac assign_role requires role name".to_string(),
            context: None,
            code: None,
        });
    }

    let username = input.as_string().ok_or_else(|| HlxError::Generic {
        message: "Input must be a username string".to_string(),
        context: None,
        code: None,
    })?;

    let role_name = args[0].as_string().ok_or_else(|| HlxError::Generic {
        message: "First argument must be a role name string".to_string(),
        context: None,
        code: None,
    })?;

    // Check if role exists
    if !context.rbac_roles.contains_key(&role_name) {
        return Err(HlxError::Generic {
            message: format!("Role '{}' does not exist", role_name),
            context: None,
            code: None,
        });
    }

    // Get role permissions
    let role_permissions = context.rbac_roles.get(&role_name).unwrap().permissions.clone();

    // Create or update user
    let user = RbacUser {
        username: username.clone(),
        roles: vec![role_name.clone()],
        permissions: role_permissions,
    };

    context.rbac_users.insert(username.clone(), user);

    let result = serde_json::json!({
        "success": true,
        "username": username,
        "role": role_name,
        "message": "Role assigned successfully"
    });

    Ok(Value::from(result))
}

fn rbac_list_roles(input: &Value, context: &mut SecurityOperatorContext, _args: &[Value]) -> HelixResult<Value> {
    let roles: Vec<serde_json::Value> = context.rbac_roles
        .values()
        .map(|role| serde_json::to_value(role).unwrap())
        .collect();

    Ok(Value::from(serde_json::json!({
        "roles": roles,
        "count": roles.len()
    })))
}

/// SecurityOperators struct that implements OperatorTrait
pub struct SecurityOperators {
    context: SecurityOperatorContext,
}

impl SecurityOperators {
    pub fn new() -> Self {
        Self {
            context: SecurityOperatorContext::new(),
        }
    }

    pub fn with_jwt_secret(mut self, secret: String) -> Self {
        self.context = self.context.with_jwt_secret(secret);
        self
    }

    pub fn with_oauth_config(mut self, config: HashMap<String, String>) -> Self {
        self.context = self.context.with_oauth_config(config);
        self
    }

    pub fn with_ldap_config(mut self, config: HashMap<String, String>) -> Self {
        self.context = self.context.with_ldap_config(config);
        self
    }
}

#[async_trait]
impl OperatorTrait for SecurityOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;
        
        // Extract input and arguments from params
        let input = params_map.get("input").cloned().unwrap_or(Value::String("".to_string(.to_string())));
        let args = params_map.get("args")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        match operator {
            "encrypt" => {
                let mut context = self.context.clone();
                encrypt_operator(&input, &mut context, &args)
            }
            "decrypt" => {
                let mut context = self.context.clone();
                decrypt_operator(&input, &mut context, &args)
            }
            "jwt" => {
                let mut context = self.context.clone();
                jwt_operator(&input, &mut context, &args)
            }
            "oauth" => {
                let mut context = self.context.clone();
                oauth_operator(&input, &mut context, &args)
            }
            "saml" => {
                let mut context = self.context.clone();
                saml_operator(&input, &mut context, &args)
            }
            "ldap" => {
                let mut context = self.context.clone();
                ldap_operator(&input, &mut context, &args)
            }
            "rbac" => {
                let mut context = self.context.clone();
                rbac_operator(&input, &mut context, &args)
            }
            _ => Err(HlxError::Generic {
                message: format!("Unknown security operator: {}", operator),
                context: None,
                code: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_operator() {
        let mut context = SecurityOperatorContext::new();
        let input = Value::String("secret data".to_string(.to_string()));
        let args = vec![
            Value::String("test_key".to_string(.to_string())),
            Value::String("aes256gcm".to_string(.to_string())),
        ];

        let result = encrypt_operator(&input, &mut context, &args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_jwt_operator() {
        let mut context = SecurityOperatorContext::new();
        let input = Value::String("testuser".to_string(.to_string()));
        let args = vec![Value::String("encode".to_string(.to_string()))];

        let result = jwt_operator(&input, &mut context, &args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rbac_operator() {
        let mut context = SecurityOperatorContext::new();
        let input = Value::String("admin".to_string(.to_string()));
        let args = vec![Value::String("add_role".to_string(.to_string()))];

        let result = rbac_operator(&input, &mut context, &args);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_security_operators_struct() {
        let operators = SecurityOperators::new();
        let params = r#"{"input": "testuser", "args": ["encode"]}"#;
        
        let result = operators.execute("jwt", params).await;
        assert!(result.is_ok());
    }
} 