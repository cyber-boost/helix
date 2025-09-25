use crate::error::{HlxError, HelixResult};
use crate::database::{DatabaseAdapter, DatabaseConfig, DatabaseType, QueryResult, Value};
use crate::parser::Operator;
use std::collections::HashMap;
use async_trait::async_trait;

/// Database operator trait
#[async_trait]
pub trait DatabaseOperator: Send + Sync {
    async fn execute(&self, query: &str, params: &[Value]) -> HelixResult<QueryResult>;
    async fn get_status(&self) -> HelixResult<String>;
    async fn ping(&self) -> HelixResult<bool>;
}

/// SQLite Database Operator
pub struct SqliteOperator {
    adapter: crate::database::SqliteAdapter,
    config: DatabaseConfig,
}

impl SqliteOperator {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            adapter: crate::database::SqliteAdapter::new(),
            config,
        }
    }
}

#[async_trait]
impl DatabaseOperator for SqliteOperator {
    async fn execute(&self, query: &str, params: &[Value]) -> HelixResult<QueryResult> {
        self.adapter.execute_query(query, params).await
    }

    async fn get_status(&self) -> HelixResult<String> {
        let status = self.adapter.get_status().await;
        Ok(format!("SQLite: {:?}", status))
    }

    async fn ping(&self) -> HelixResult<bool> {
        self.adapter.ping().await
    }
}

/// PostgreSQL Database Operator
pub struct PostgresOperator {
    adapter: crate::database::PostgresAdapter,
    config: DatabaseConfig,
}

impl PostgresOperator {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            adapter: crate::database::PostgresAdapter::new(),
            config,
        }
    }
}

#[async_trait]
impl DatabaseOperator for PostgresOperator {
    async fn execute(&self, query: &str, params: &[Value]) -> HelixResult<QueryResult> {
        self.adapter.execute_query(query, params).await
    }

    async fn get_status(&self) -> HelixResult<String> {
        let status = self.adapter.get_status().await;
        Ok(format!("PostgreSQL: {:?}", status))
    }

    async fn ping(&self) -> HelixResult<bool> {
        self.adapter.ping().await
    }
}

/// MySQL Database Operator
pub struct MySqlOperator {
    adapter: crate::database::MySqlAdapter,
    config: DatabaseConfig,
}

impl MySqlOperator {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            adapter: crate::database::MySqlAdapter::new(),
            config,
        }
    }
}

#[async_trait]
impl DatabaseOperator for MySqlOperator {
    async fn execute(&self, query: &str, params: &[Value]) -> HelixResult<QueryResult> {
        self.adapter.execute_query(query, params).await
    }

    async fn get_status(&self) -> HelixResult<String> {
        let status = self.adapter.get_status().await;
        Ok(format!("MySQL: {:?}", status))
    }

    async fn ping(&self) -> HelixResult<bool> {
        self.adapter.ping().await
    }
}

#[cfg(feature = "mongodb")]
/// MongoDB Database Operator
#[cfg(feature = "mongodb")]
pub struct MongoOperator {
    adapter: crate::database::MongoAdapter,
    config: DatabaseConfig,
}

#[cfg(feature = "mongodb")]
impl MongoOperator {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            adapter: crate::database::MongoAdapter::new(),
            config,
        }
    }
}

#[async_trait]
#[cfg(feature = "mongodb")]
impl DatabaseOperator for MongoOperator {
    async fn execute(&self, query: &str, params: &[Value]) -> HelixResult<QueryResult> {
        self.adapter.execute_query(query, params).await
    }

    async fn get_status(&self) -> HelixResult<String> {
        let status = self.adapter.get_status().await;
        Ok(format!("MongoDB: {:?}", status))
    }

    async fn ping(&self) -> HelixResult<bool> {
        self.adapter.ping().await
    }
}

/// Redis Database Operator
pub struct RedisOperator {
    adapter: crate::database::RedisAdapter,
    config: DatabaseConfig,
}

impl RedisOperator {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            adapter: crate::database::RedisAdapter::new(),
            config,
        }
    }
}

#[async_trait]
impl DatabaseOperator for RedisOperator {
    async fn execute(&self, query: &str, params: &[Value]) -> HelixResult<QueryResult> {
        self.adapter.execute_query(query, params).await
    }

    async fn get_status(&self) -> HelixResult<String> {
        let status = self.adapter.get_status().await;
        Ok(format!("Redis: {:?}", status))
    }

    async fn ping(&self) -> HelixResult<bool> {
        self.adapter.ping().await
    }
}

/// Database operator registry
pub struct DatabaseOperatorRegistry {
    operators: HashMap<String, Box<dyn DatabaseOperator>>,
}

impl DatabaseOperatorRegistry {
    pub fn new() -> Self {
        Self {
            operators: HashMap::new(),
        }
    }

    pub fn register_sqlite(&mut self, config: DatabaseConfig) {
        let operator = SqliteOperator::new(config);
        self.operators.insert("@sqlite".to_string(), Box::new(operator));
    }

    pub fn register_postgresql(&mut self, config: DatabaseConfig) {
        let operator = PostgresOperator::new(config);
        self.operators.insert("@postgresql".to_string(), Box::new(operator));
    }

    pub fn register_mysql(&mut self, config: DatabaseConfig) {
        let operator = MySqlOperator::new(config);
        self.operators.insert("@mysql".to_string(), Box::new(operator));
    }

    pub fn register_mongodb(&mut self, config: DatabaseConfig) {
        let operator = MongoOperator::new(config);
        self.operators.insert("@mongodb".to_string(), Box::new(operator));
    }

    pub fn register_redis(&mut self, config: DatabaseConfig) {
        let operator = RedisOperator::new(config);
        self.operators.insert("@redis".to_string(), Box::new(operator));
    }

    pub async fn execute_operator(&self, operator_name: &str, query: &str, params: &[Value]) -> HelixResult<QueryResult> {
        if let Some(operator) = self.operators.get(operator_name) {
            operator.execute(query, params).await
        } else {
            Err(HlxError::Generic {
                message: format!("Unknown database operator: {}", operator_name),
                context: None,
                code: None,
            })
        }
    }

    pub async fn get_operator_status(&self, operator_name: &str) -> HelixResult<String> {
        if let Some(operator) = self.operators.get(operator_name) {
            operator.get_status().await
        } else {
            Err(HlxError::Generic {
                message: format!("Unknown database operator: {}", operator_name),
                context: None,
                code: None,
            })
        }
    }

    pub async fn ping_operator(&self, operator_name: &str) -> HelixResult<bool> {
        if let Some(operator) = self.operators.get(operator_name) {
            operator.ping().await
        } else {
            Err(HlxError::Generic {
                message: format!("Unknown database operator: {}", operator_name),
                context: None,
                code: None,
            })
        }
    }

    pub fn list_operators(&self) -> Vec<String> {
        self.operators.keys().cloned().collect()
    }
}

/// Database operator parser
pub struct DatabaseOperatorParser;

impl DatabaseOperatorParser {
    pub fn parse_operator(input: &str) -> HelixResult<(String, String, Vec<Value>)> {
        // Parse operator syntax: @database_name query params
        let parts: Vec<&str> = input.splitn(3, ' ').collect();
        
        if parts.len() < 2 {
            return Err(HlxError::Generic {
                message: "Invalid database operator syntax. Expected: @database_name query [params]".to_string(),
                context: None,
                code: None,
            });
        }

        let operator_name = parts[0].to_string();
        let query = parts[1].to_string();
        let params = if parts.len() > 2 {
            Self::parse_params(parts[2])?
        } else {
            Vec::new()
        };

        Ok((operator_name, query, params))
    }

    fn parse_params(params_str: &str) -> HelixResult<Vec<Value>> {
        // Simple parameter parsing - can be enhanced
        let params: Vec<Value> = params_str
            .split(',')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| {
                if p.starts_with('"') && p.ends_with('"') {
                    Value::String(p[1..p.len(.to_string())-1].to_string())
                } else if p.parse::<i64>().is_ok() {
                    Value::Integer(p.parse().unwrap())
                } else if p.parse::<f64>().is_ok() {
                    Value::Float(p.parse().unwrap())
                } else if p == "true" || p == "false" {
                    Value::Boolean(p == "true")
                } else if p == "null" {
                    Value::Null
                } else {
                    Value::String(p.to_string(.to_string()))
                }
            })
            .collect();

        Ok(params)
    }
}

/// Database operator examples and documentation
pub struct DatabaseOperatorExamples;

impl DatabaseOperatorExamples {
    pub fn get_examples() -> HashMap<String, Vec<String>> {
        let mut examples = HashMap::new();

        examples.insert("@sqlite".to_string(), vec![
            "@sqlite SELECT * FROM users WHERE age > 18",
            "@sqlite INSERT INTO users (name, email) VALUES (\"John\", \"john@example.com\")",
            "@sqlite UPDATE users SET status = \"active\" WHERE id = 1",
            "@sqlite DELETE FROM users WHERE id = 1",
        ]);

        examples.insert("@postgresql".to_string(), vec![
            "@postgresql SELECT * FROM users WHERE age > $1 18",
            "@postgresql INSERT INTO users (name, email) VALUES ($1, $2) \"John\", \"john@example.com\"",
            "@postgresql UPDATE users SET status = $1 WHERE id = $2 \"active\", 1",
        ]);

        examples.insert("@mysql".to_string(), vec![
            "@mysql SELECT * FROM users WHERE age > ? 18",
            "@mysql INSERT INTO users (name, email) VALUES (?, ?) \"John\", \"john@example.com\"",
            "@mysql UPDATE users SET status = ? WHERE id = ? \"active\", 1",
        ]);

        examples.insert("@mongodb".to_string(), vec![
            "@mongodb find users {\"age\": {\"$gt\": 18}}",
            "@mongodb insert users {\"name\": \"John\", \"email\": \"john@example.com\"}",
            "@mongodb update users {\"id\": 1} {\"$set\": {\"status\": \"active\"}}",
        ]);

        examples.insert("@redis".to_string(), vec![
            "@redis GET user:1",
            "@redis SET user:1 \"{\\\"name\\\": \\\"John\\\", \\\"email\\\": \\\"john@example.com\\\"}\"",
            "@redis DEL user:1",
            "@redis HGETALL user:1",
        ]);

        examples
    }

    pub fn get_syntax_help() -> HashMap<String, String> {
        let mut help = HashMap::new();

        help.insert("@sqlite".to_string(), 
            "SQLite operator for file-based database operations.\n\
             Syntax: @sqlite <sql_query> [parameters]\n\
             Example: @sqlite SELECT * FROM users WHERE age > 18".to_string());

        help.insert("@postgresql".to_string(), 
            "PostgreSQL operator for advanced SQL operations.\n\
             Syntax: @postgresql <sql_query> [parameters]\n\
             Example: @postgresql SELECT * FROM users WHERE age > $1 18".to_string());

        help.insert("@mysql".to_string(), 
            "MySQL operator for traditional SQL operations.\n\
             Syntax: @mysql <sql_query> [parameters]\n\
             Example: @mysql SELECT * FROM users WHERE age > ? 18".to_string());

        help.insert("@mongodb".to_string(), 
            "MongoDB operator for document operations.\n\
             Syntax: @mongodb <operation> <collection> <query> [options]\n\
             Example: @mongodb find users {\"age\": {\"$gt\": 18}}".to_string());

        help.insert("@redis".to_string(), 
            "Redis operator for key-value operations.\n\
             Syntax: @redis <command> <key> [value]\n\
             Example: @redis GET user:1".to_string());

        help
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sqlite_operator() {
        let config = DatabaseConfig {
            database_type: DatabaseType::SQLite,
            host: "localhost".to_string(),
            port: 0,
            database: "test.db".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            max_connections: 10,
            connection_timeout: std::time::Duration::from_secs(30),
            idle_timeout: std::time::Duration::from_secs(300),
            ssl_mode: false,
            connection_string: None,
        };

        let operator = SqliteOperator::new(config);
        let status = operator.get_status().await.unwrap();
        assert!(status.contains("SQLite"));
    }

    #[test]
    fn test_operator_parsing() {
        let (operator, query, params) = DatabaseOperatorParser::parse_operator("@sqlite SELECT * FROM users WHERE age > 18").unwrap();
        assert_eq!(operator, "@sqlite");
        assert_eq!(query, "SELECT * FROM users WHERE age > 18");
        assert_eq!(params.len(), 0);

        let (operator, query, params) = DatabaseOperatorParser::parse_operator("@postgresql SELECT * FROM users WHERE age > $1 18").unwrap();
        assert_eq!(operator, "@postgresql");
        assert_eq!(query, "SELECT * FROM users WHERE age > $1");
        assert_eq!(params.len(), 1);
        assert!(matches!(params[0], Value::Integer(18)));
    }

    #[test]
    fn test_operator_examples() {
        let examples = DatabaseOperatorExamples::get_examples();
        assert!(examples.contains_key("@sqlite"));
        assert!(examples.contains_key("@postgresql"));
        assert!(examples.contains_key("@mysql"));
        assert!(examples.contains_key("@mongodb"));
        assert!(examples.contains_key("@redis"));
    }
} 