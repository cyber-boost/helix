//! SQLite Database Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use rusqlite::{Connection, OpenFlags, Result as SqliteResult, Row, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    pub database_path: String,
    pub enable_wal_mode: bool,
    pub enable_foreign_keys: bool,
    pub cache_size: i32,
    pub auto_vacuum: AutoVacuum,
    pub busy_timeout: u32,
    pub journal_mode: JournalMode,
    pub synchronous_mode: SynchronousMode,
    pub temp_store: TempStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutoVacuum { None, Full, Incremental }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalMode { Delete, Truncate, Persist, Memory, Wal, Off }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SynchronousMode { Off, Normal, Full, Extra }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TempStore { Default, File, Memory }

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            database_path: ":memory:".to_string(),
            enable_wal_mode: true,
            enable_foreign_keys: true,
            cache_size: -2000, // 2MB
            auto_vacuum: AutoVacuum::Incremental,
            busy_timeout: 30000,
            journal_mode: JournalMode::Wal,
            synchronous_mode: SynchronousMode::Normal,
            temp_store: TempStore::Memory,
        }
    }
}

#[derive(Debug, Default)]
struct SqliteMetrics {
    queries_executed: u64,
    transactions_started: u64,
    transactions_committed: u64,
    transactions_rolled_back: u64,
    rows_affected: u64,
    avg_query_time: f64,
    database_size: u64,
    page_count: u64,
    freelist_count: u64,
}

pub struct SqliteOperator {
    config: SqliteConfig,
    connection: Arc<Mutex<Connection>>,
    metrics: Arc<Mutex<SqliteMetrics>>,
    active_transactions: Arc<RwLock<HashMap<String, bool>>>,
}

impl SqliteOperator {
    pub async fn new(config: SqliteConfig) -> Result<Self, HlxError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE 
            | OpenFlags::SQLITE_OPEN_CREATE 
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;

        let conn = Connection::open_with_flags(&config.database_path, flags)
            .map_err(|e| HlxError::DatabaseError {
                operation: "Open Database".to_string(),
                message: format!("Failed to open SQLite database: {}", e),
            })?;

        // Configure SQLite settings
        Self::configure_database(&conn, &config)?;

        let operator = Self {
            config: config.clone(),
            connection: Arc::new(Mutex::new(conn)),
            metrics: Arc::new(Mutex::new(SqliteMetrics::default())),
            active_transactions: Arc::new(RwLock::new(HashMap::new())),
        };

        info!("SQLite operator initialized with database: {}", config.database_path);
        Ok(operator)
    }

    fn configure_database(conn: &Connection, config: &SqliteConfig) -> Result<(), HlxError> {
        // Set busy timeout
        conn.busy_timeout(Duration::from_millis(config.busy_timeout as u64))
            .map_err(|e| HlxError::ConfigurationError {
                component: "SQLite Busy Timeout".to_string(),
                message: format!("Failed to set busy timeout: {}", e),
            })?;

        // Enable foreign keys
        if config.enable_foreign_keys {
            conn.execute("PRAGMA foreign_keys = ON", [])
                .map_err(|e| HlxError::ConfigurationError {
                    component: "SQLite Foreign Keys".to_string(),
                    message: format!("Failed to enable foreign keys: {}", e),
                })?;
        }

        // Set cache size
        conn.execute(&format!("PRAGMA cache_size = {}", config.cache_size), [])
            .map_err(|e| HlxError::ConfigurationError {
                component: "SQLite Cache Size".to_string(),
                message: format!("Failed to set cache size: {}", e),
            })?;

        // Set journal mode
        let journal_mode = match config.journal_mode {
            JournalMode::Delete => "DELETE",
            JournalMode::Truncate => "TRUNCATE",
            JournalMode::Persist => "PERSIST",
            JournalMode::Memory => "MEMORY",
            JournalMode::Wal => "WAL",
            JournalMode::Off => "OFF",
        };
        conn.execute(&format!("PRAGMA journal_mode = {}", journal_mode), [])
            .map_err(|e| HlxError::ConfigurationError {
                component: "SQLite Journal Mode".to_string(),
                message: format!("Failed to set journal mode: {}", e),
            })?;

        // Set synchronous mode
        let sync_mode = match config.synchronous_mode {
            SynchronousMode::Off => "OFF",
            SynchronousMode::Normal => "NORMAL",
            SynchronousMode::Full => "FULL",
            SynchronousMode::Extra => "EXTRA",
        };
        conn.execute(&format!("PRAGMA synchronous = {}", sync_mode), [])
            .map_err(|e| HlxError::ConfigurationError {
                component: "SQLite Synchronous Mode".to_string(),
                message: format!("Failed to set synchronous mode: {}", e),
            })?;

        // Set auto vacuum
        let auto_vacuum = match config.auto_vacuum {
            AutoVacuum::None => "NONE",
            AutoVacuum::Full => "FULL",
            AutoVacuum::Incremental => "INCREMENTAL",
        };
        conn.execute(&format!("PRAGMA auto_vacuum = {}", auto_vacuum), [])
            .map_err(|e| HlxError::ConfigurationError {
                component: "SQLite Auto Vacuum".to_string(),
                message: format!("Failed to set auto vacuum: {}", e),
            })?;

        Ok(())
    }

    pub async fn execute_query(&self, sql: &str, params: Option<Vec<JsonValue>>) -> Result<Vec<HashMap<String, JsonValue>>, HlxError> {
        let start_time = Instant::now();
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection.prepare(sql)
            .map_err(|e| HlxError::DatabaseError {
                operation: "Prepare Query".to_string(),
                message: format!("Failed to prepare query: {}", e),
            })?;

        // Bind parameters if provided
        if let Some(parameters) = params {
            for (i, param) in parameters.iter().enumerate() {
                self.bind_parameter(&mut stmt, i + 1, param)?;
            }
        }

        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let rows = stmt.query_map([], |row| {
            let mut result = HashMap::new();
            for (i, column_name) in column_names.iter().enumerate() {
                let value = self.extract_value(row, i)?;
                result.insert(column_name.clone(), value);
            }
            Ok(result)
        }).map_err(|e| HlxError::DatabaseError {
            operation: "Execute Query".to_string(),
            message: format!("Failed to execute query: {}", e),
        })?;

        let mut results = Vec::new();
        for row_result in rows {
            results.push(row_result.map_err(|e| HlxError::DatabaseError {
                operation: "Process Row".to_string(),
                message: format!("Failed to process row: {}", e),
            })?);
        }

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.queries_executed += 1;
            let execution_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_query_time = (metrics.avg_query_time * (metrics.queries_executed - 1) as f64 + execution_time) / metrics.queries_executed as f64;
        }

        debug!("Executed SQLite query, returned {} rows", results.len());
        Ok(results)
    }

    pub async fn execute_command(&self, sql: &str, params: Option<Vec<JsonValue>>) -> Result<u64, HlxError> {
        let start_time = Instant::now();
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection.prepare(sql)
            .map_err(|e| HlxError::DatabaseError {
                operation: "Prepare Command".to_string(),
                message: format!("Failed to prepare command: {}", e),
            })?;

        // Bind parameters if provided
        if let Some(parameters) = params {
            for (i, param) in parameters.iter().enumerate() {
                self.bind_parameter(&mut stmt, i + 1, param)?;
            }
        }

        let rows_affected = stmt.execute([])
            .map_err(|e| HlxError::DatabaseError {
                operation: "Execute Command".to_string(),
                message: format!("Failed to execute command: {}", e),
            })? as u64;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.queries_executed += 1;
            metrics.rows_affected += rows_affected;
            let execution_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_query_time = (metrics.avg_query_time * (metrics.queries_executed - 1) as f64 + execution_time) / metrics.queries_executed as f64;
        }

        debug!("Executed SQLite command, affected {} rows", rows_affected);
        Ok(rows_affected)
    }

    pub async fn begin_transaction(&self) -> Result<String, HlxError> {
        let transaction_id = uuid::Uuid::new_v4().to_string();
        let connection = self.connection.lock().unwrap();

        connection.execute("BEGIN TRANSACTION", [])
            .map_err(|e| HlxError::DatabaseError {
                operation: "Begin Transaction".to_string(),
                message: format!("Failed to begin transaction: {}", e),
            })?;

        {
            let mut transactions = self.active_transactions.write().await;
            transactions.insert(transaction_id.clone(), true);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.transactions_started += 1;
        }

        info!("Started SQLite transaction: {}", transaction_id);
        Ok(transaction_id)
    }

    pub async fn commit_transaction(&self, transaction_id: &str) -> Result<(), HlxError> {
        {
            let transactions = self.active_transactions.read().await;
            if !transactions.contains_key(transaction_id) {
                return Err(HlxError::NotFoundError {
                    resource: "Transaction".to_string(),
                    identifier: transaction_id.to_string(),
                });
            }
        }

        let connection = self.connection.lock().unwrap();
        connection.execute("COMMIT", [])
            .map_err(|e| HlxError::DatabaseError {
                operation: "Commit Transaction".to_string(),
                message: format!("Failed to commit transaction: {}", e),
            })?;

        {
            let mut transactions = self.active_transactions.write().await;
            transactions.remove(transaction_id);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.transactions_committed += 1;
        }

        info!("Committed SQLite transaction: {}", transaction_id);
        Ok(())
    }

    pub async fn rollback_transaction(&self, transaction_id: &str) -> Result<(), HlxError> {
        {
            let transactions = self.active_transactions.read().await;
            if !transactions.contains_key(transaction_id) {
                return Err(HlxError::NotFoundError {
                    resource: "Transaction".to_string(),
                    identifier: transaction_id.to_string(),
                });
            }
        }

        let connection = self.connection.lock().unwrap();
        connection.execute("ROLLBACK", [])
            .map_err(|e| HlxError::DatabaseError {
                operation: "Rollback Transaction".to_string(),
                message: format!("Failed to rollback transaction: {}", e),
            })?;

        {
            let mut transactions = self.active_transactions.write().await;
            transactions.remove(transaction_id);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.transactions_rolled_back += 1;
        }

        info!("Rolled back SQLite transaction: {}", transaction_id);
        Ok(())
    }

    pub async fn vacuum(&self, incremental: bool) -> Result<(), HlxError> {
        let connection = self.connection.lock().unwrap();

        let vacuum_sql = if incremental {
            "PRAGMA incremental_vacuum"
        } else {
            "VACUUM"
        };

        connection.execute(vacuum_sql, [])
            .map_err(|e| HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Vacuum Database".to_string(),
                message: format!("Failed to vacuum database: {}", e),
            })?;

        info!("SQLite vacuum completed (incremental: {})", incremental);
        Ok(())
    }

    pub async fn analyze(&self, table: Option<&str>) -> Result<(), HlxError> {
        let connection = self.connection.lock().unwrap();

        let analyze_sql = if let Some(table_name) = table {
            format!("ANALYZE {}", table_name)
        } else {
            "ANALYZE".to_string()
        };

        connection.execute(&analyze_sql, [])
            .map_err(|e| HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Analyze Database".to_string(),
                message: format!("Failed to analyze database: {}", e),
            })?;

        info!("SQLite analyze completed for: {}", table.unwrap_or("all tables"));
        Ok(())
    }

    pub async fn get_database_info(&self) -> Result<HashMap<String, Value>, HlxError> {
        let connection = self.connection.lock().unwrap();
        let mut info = HashMap::new();

        // Get database version
        let version: String = connection.query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .unwrap_or_else(|_| "unknown".to_string());
        info.insert("sqlite_version".to_string(), Value::String(version.to_string()));

        // Get database size information
        let page_count: i64 = connection.pragma_query_value(None, "page_count", |row| row.get(0))
            .unwrap_or(0);
        let page_size: i64 = connection.pragma_query_value(None, "page_size", |row| row.get(0))
            .unwrap_or(0);
        let freelist_count: i64 = connection.pragma_query_value(None, "freelist_count", |row| row.get(0))
            .unwrap_or(0);

        info.insert("page_count".to_string(), Value::Number(page_count as f64));
        info.insert("page_size".to_string(), Value::Number(page_size as f64));
        info.insert("database_size_bytes".to_string(), Value::Number((page_count * page_size) as f64));
        info.insert("freelist_count".to_string(), Value::Number(freelist_count as f64));

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.page_count = page_count as u64;
            metrics.database_size = (page_count * page_size) as u64;
            metrics.freelist_count = freelist_count as u64;
        }

        Ok(info)
    }

    fn bind_parameter(&self, stmt: &mut rusqlite::Statement, index: usize, value: &JsonValue) -> Result<(), HlxError> {
        use rusqlite::types::Value as SqliteValue;

        let sqlite_value = match value {
            JsonValue::String(s.to_string()) => SqliteValue::Text(s.clone()),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    SqliteValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    SqliteValue::Real(f)
                } else {
                    SqliteValue::Text(n.to_string())
                }
            }
            JsonValue::Bool(b) => SqliteValue::Integer(if *b { 1 } else { 0 }),
            JsonValue::Null => SqliteValue::Null,
            JsonValue::Array(_) | JsonValue::Object(_) => SqliteValue::Text(value.to_string()),
        };

        stmt.bind_parameter(index, sqlite_value)
            .map_err(|e| HlxError::DatabaseError {
                operation: "Bind Parameter".to_string(),
                message: format!("Failed to bind parameter {}: {}", index, e),
            })?;

        Ok(())
    }

    fn extract_value(&self, row: &Row, index: usize) -> SqliteResult<JsonValue> {
        use rusqlite::types::ValueRef;

        match row.get_ref(index)? {
            ValueRef::Null => Ok(JsonValue::Null),
            ValueRef::Integer(i) => Ok(JsonValue::Number(serde_json::Number::from(i))),
            ValueRef::Real(f) => Ok(JsonValue::Number(serde_json::Number::from_f64(f).unwrap())),
            ValueRef::Text(s) => {
                let text = std::str::from_utf8(s).unwrap_or("");
                // Try to parse as JSON first
                match serde_json::from_str(text) {
                    Ok(json_val) => Ok(json_val),
                    Err(_) => Ok(JsonValue::String(text.to_string(.to_string()))),
                }
            }
            ValueRef::Blob(b) => {
                // Try to convert blob to text first, then base64 if needed
                match std::str::from_utf8(b) {
                    Ok(text) => Ok(JsonValue::String(text.to_string(.to_string()))),
                    Err(_) => Ok(JsonValue::String(base64::encode(b.to_string()))),
                }
            }
        }
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("queries_executed".to_string(), Value::Number(metrics.queries_executed as f64));
        result.insert("transactions_started".to_string(), Value::Number(metrics.transactions_started as f64));
        result.insert("transactions_committed".to_string(), Value::Number(metrics.transactions_committed as f64));
        result.insert("transactions_rolled_back".to_string(), Value::Number(metrics.transactions_rolled_back as f64));
        result.insert("rows_affected".to_string(), Value::Number(metrics.rows_affected as f64));
        result.insert("avg_query_time_ms".to_string(), Value::Number(metrics.avg_query_time));
        result.insert("database_size_bytes".to_string(), Value::Number(metrics.database_size as f64));
        result.insert("page_count".to_string(), Value::Number(metrics.page_count as f64));
        result.insert("freelist_count".to_string(), Value::Number(metrics.freelist_count as f64));

        if metrics.transactions_started > 0 {
            let commit_rate = (metrics.transactions_committed as f64 / metrics.transactions_started as f64) * 100.0;
            result.insert("transaction_commit_rate_percent".to_string(), Value::Number(commit_rate));
        }

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for SqliteOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "query" => {
                let sql = params_map.get("sql").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("sql".to_string()),
                        message: "Missing SQL query".to_string(),
                    })?;

                let query_params = params_map.get("params").and_then(|v| {
                    if let Value::Array(arr) = v {
                        Some(arr.iter().map(|v| utils::value_to_json_value(v)).collect())
                    } else { None }
                });

                let rows = self.execute_query(&sql, query_params).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("rows".to_string(), Value::Array(
                        rows.into_iter().map(|row| {
                            Value::Object(row.into_iter().map(|(k, v)| (k, utils::json_value_to_value(&v))).collect())
                        }).collect()
                    ));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "execute" => {
                let sql = params_map.get("sql").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("sql".to_string()),
                        message: "Missing SQL command".to_string(),
                    })?;

                let query_params = params_map.get("params").and_then(|v| {
                    if let Value::Array(arr) = v {
                        Some(arr.iter().map(|v| utils::value_to_json_value(v)).collect())
                    } else { None }
                });

                let affected_rows = self.execute_command(&sql, query_params).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("affected_rows".to_string(), Value::Number(affected_rows as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "begin_transaction" => {
                let transaction_id = self.begin_transaction().await?;
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("transaction_id".to_string(), Value::String(transaction_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "commit_transaction" => {
                let transaction_id = params_map.get("transaction_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("transaction_id".to_string()),
                        message: "Missing transaction ID".to_string(),
                    })?;

                self.commit_transaction(&transaction_id).await?;
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("transaction_id".to_string(), Value::String(transaction_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "rollback_transaction" => {
                let transaction_id = params_map.get("transaction_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("transaction_id".to_string()),
                        message: "Missing transaction ID".to_string(),
                    })?;

                self.rollback_transaction(&transaction_id).await?;
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("transaction_id".to_string(), Value::String(transaction_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "vacuum" => {
                let incremental = params_map.get("incremental").and_then(|v| v.as_boolean()).unwrap_or(false);
                self.vacuum(incremental).await?;
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("vacuum_completed".to_string(), Value::Boolean(true));
                    map.insert("incremental".to_string(), Value::Boolean(incremental));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "analyze" => {
                let table = params_map.get("table").and_then(|v| v.as_string());
                self.analyze(table.as_deref()).await?;
                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("analyze_completed".to_string(), Value::Boolean(true));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "database_info" => {
                let info = self.get_database_info().await?;
                Ok(Value::Object(info))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "sqlite".to_string(),
                params: format!("Unknown SQLite operation: {}", operator),
            }),
        }
    }
} 