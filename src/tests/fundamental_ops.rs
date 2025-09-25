//! Tests for fundamental operators

#[cfg(test)]
mod tests {
    use super::super::operators::{OperatorRegistry, ExecutionContext, RequestData};
    use super::super::value::Value;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_var_operator_set_and_get() {
        let mut cookies = HashMap::new();
        cookies.insert("session_id".to_string(), "abc123".to_string());

        let mut params = HashMap::new();
        params.insert("user_id".to_string(), "12345".to_string());

        let mut query = HashMap::new();
        query.insert("q".to_string(), "test".to_string());

        let context = ExecutionContext {
            request: None,
            session: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            cookies,
            params,
            query,
        };

        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        // Set a variable
        let result = registry.execute("@var", r#"name="test_var" value="test_value""#).await.unwrap();
        assert!(matches!(result, Value::Object(_)));

        // Get the variable
        let result = registry.execute("@var", r#"name="test_var""#).await.unwrap();
        if let Value::Object(map) = result {
            assert_eq!(map.get("operation").unwrap(), &Value::String("get".to_string()));
            assert_eq!(map.get("value").unwrap(), &Value::String("test_value".to_string()));
        } else {
            panic!("Expected object result");
        }
    }

    #[tokio::test]
    async fn test_env_operator_with_default() {
        // Set an environment variable for testing
        std::env::set_var("HELIX_test_VAR", "test_value");

        let context = ExecutionContext::default();
        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        // Read existing env var
        let result = registry.execute("@env", r#"var="HELIX_test_VAR""#).await.unwrap();
        assert_eq!(result, Value::String("test_value".to_string()));

        // Test with default fallback
        let result = registry.execute("@env", r#"var="NONEXISTENT_VAR" default="fallback""#).await.unwrap();
        assert_eq!(result, Value::String("fallback".to_string()));

        // Clean up
        std::env::remove_var("HELIX_test_VAR");
    }

    #[tokio::test]
    async fn test_request_operator() {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "Test/1.0".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let request = RequestData {
            method: "GET".to_string(),
            url: "http://example.com/api/test".to_string(),
            headers,
            body: r#"{"key": "value"}"#.to_string(),
        };

        let context = ExecutionContext {
            request: Some(request),
            session: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            cookies: HashMap::new(),
            params: HashMap::new(),
            query: HashMap::new(),
        };

        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        // Test getting method
        let result = registry.execute("@request", r#"field="method""#).await.unwrap();
        assert_eq!(result, Value::String("GET".to_string()));

        // Test getting url
        let result = registry.execute("@request", r#"field="url""#).await.unwrap();
        assert_eq!(result, Value::String("http://example.com/api/test".to_string()));

        // Test getting headers
        let result = registry.execute("@request", r#"field="headers""#).await.unwrap();
        assert!(matches!(result, Value::Object(_)));
    }

    #[tokio::test]
    async fn test_session_operator() {
        let context = ExecutionContext::default();
        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        // Set a session value
        let result = registry.execute("@session", r#"action="set" key="user_id" value="12345""#).await.unwrap();
        assert!(matches!(result, Value::Object(_)));

        // Get the session value
        let result = registry.execute("@session", r#"action="get" key="user_id""#).await.unwrap();
        if let Value::Object(map) = result {
            assert_eq!(map.get("value").unwrap(), &Value::String("12345".to_string()));
        } else {
            panic!("Expected object result");
        }
    }

    #[tokio::test]
    async fn test_cookie_operator() {
        let mut cookies = HashMap::new();
        cookies.insert("theme".to_string(), "dark".to_string());
        cookies.insert("lang".to_string(), "en".to_string());

        let context = ExecutionContext {
            request: None,
            session: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            cookies,
            params: HashMap::new(),
            query: HashMap::new(),
        };

        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        let result = registry.execute("@cookie", r#"name="theme""#).await.unwrap();
        if let Value::Object(map) = result {
            assert_eq!(map.get("value").unwrap(), &Value::String("dark".to_string()));
        } else {
            panic!("Expected object result");
        }
    }

    #[tokio::test]
    async fn test_param_operator() {
        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());
        params.insert("action".to_string(), "edit".to_string());

        let context = ExecutionContext {
            request: None,
            session: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            cookies: HashMap::new(),
            params,
            query: HashMap::new(),
        };

        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        let result = registry.execute("@param", r#"name="id""#).await.unwrap();
        if let Value::Object(map) = result {
            assert_eq!(map.get("value").unwrap(), &Value::String("123".to_string()));
        } else {
            panic!("Expected object result");
        }
    }

    #[tokio::test]
    async fn test_query_operator() {
        let mut query = HashMap::new();
        query.insert("page".to_string(), "1".to_string());
        query.insert("limit".to_string(), "10".to_string());

        let context = ExecutionContext {
            request: None,
            session: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            cookies: HashMap::new(),
            params: HashMap::new(),
            query,
        };

        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        let result = registry.execute("@query", r#"name="page""#).await.unwrap();
        if let Value::Object(map) = result {
            assert_eq!(map.get("value").unwrap(), &Value::String("1".to_string()));
        } else {
            panic!("Expected object result");
        }
    }

    #[tokio::test]
    async fn test_if_operator() {
        let context = ExecutionContext::default();
        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        // True condition
        let result = registry.execute("@if", r#"condition=true then="yes" else="no""#).await.unwrap();
        assert_eq!(result, Value::String("yes".to_string()));

        // False condition
        let result = registry.execute("@if", r#"condition=false then="yes" else="no""#).await.unwrap();
        assert_eq!(result, Value::String("no".to_string()));
    }

    #[tokio::test]
    async fn test_math_operator() {
        let context = ExecutionContext::default();
        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        // Addition
        let result = registry.execute("@math", r#"operation="add" a=5 b=3"#).await.unwrap();
        assert_eq!(result, Value::Number(8.0));

        // Subtraction
        let result = registry.execute("@math", r#"operation="sub" a=10 b=4"#).await.unwrap();
        assert_eq!(result, Value::Number(6.0));

        // Division by zero should error
        let result = registry.execute("@math", r#"operation="div" a=1 b=0"#).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_hash_operator() {
        let context = ExecutionContext::default();
        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        // SHA256 hash
        let result = registry.execute("@hash", r#"input="hello world" algorithm="sha256""#).await.unwrap();
        assert_eq!(result, Value::String("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string()));

        // MD5 hash
        let result = registry.execute("@hash", r#"input="hello world" algorithm="md5""#).await.unwrap();
        assert_eq!(result, Value::String("5d41402abc4b2a76b9719d911017c592".to_string()));
    }

    #[tokio::test]
    async fn test_base64_operator() {
        let context = ExecutionContext::default();
        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        // Encode
        let result = registry.execute("@base64", r#"input="hello world" operation="encode""#).await.unwrap();
        assert_eq!(result, Value::String("aGVsbG8gd29ybGQ=".to_string()));

        // Decode
        let result = registry.execute("@base64", r#"input="aGVsbG8gd29ybGQ=" operation="decode""#).await.unwrap();
        assert_eq!(result, Value::String("hello world".to_string()));
    }

    #[tokio::test]
    async fn test_url_operator() {
        let context = ExecutionContext::default();
        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        // Encode
        let result = registry.execute("@url", r#"input="hello world" operation="encode""#).await.unwrap();
        assert_eq!(result, Value::String("hello%20world".to_string()));

        // Decode
        let result = registry.execute("@url", r#"input="hello%20world" operation="decode""#).await.unwrap();
        assert_eq!(result, Value::String("hello world".to_string()));
    }

    #[tokio::test]
    async fn test_uuid_operator() {
        let context = ExecutionContext::default();
        let registry = OperatorRegistry::new_with_context(context).await.unwrap();

        let result = registry.execute("@uuid", r#"version="v4""#).await.unwrap();
        assert!(matches!(result, Value::String(_)));

        // Check that it's a valid UUID format
        let uuid_str = result.as_string().unwrap();
        assert!(uuid::Uuid::parse_str(&uuid_str).is_ok());
    }
}
