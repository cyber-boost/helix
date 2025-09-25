//! MongoDB Database Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoConfig {
    pub connection_string: String,
    pub database: String,
    pub app_name: Option<String>,
    pub max_pool_size: Option<u32>,
    pub min_pool_size: Option<u32>,
    pub max_idle_time: Option<u64>,
    pub server_selection_timeout: Option<u64>,
    pub connect_timeout: Option<u64>,
    pub socket_timeout: Option<u64>,
    pub enable_ssl: bool,
    pub ssl_verify_certificate: bool,
}

impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            connection_string: "mongodb://localhost:27017".to_string(),
            database: "test".to_string(),
            app_name: Some("helix-rust-sdk".to_string()),
            max_pool_size: Some(100),
            min_pool_size: Some(5),
            max_idle_time: Some(300),
            server_selection_timeout: Some(30),
            connect_timeout: Some(10),
            socket_timeout: Some(30),
            enable_ssl: false,
            ssl_verify_certificate: true,
        }
    }
}

#[derive(Debug, Default)]
struct MongoMetrics {
    queries_executed: u64,
    documents_inserted: u64,
    documents_updated: u64,
    documents_deleted: u64,
    aggregations_performed: u64,
    collections_accessed: u64,
    avg_query_time: f64,
}

pub struct MongoOperator {
    config: MongoConfig,
    metrics: Arc<Mutex<MongoMetrics>>,
    collections: Arc<RwLock<HashMap<String, bool>>>, // Track collections
}

impl MongoOperator {
    pub async fn new(config: MongoConfig) -> Result<Self, HlxError> {
        // In a real implementation, you would create the actual MongoDB client here
        // For this example, we'll simulate the connection

        info!("MongoDB operator initialized for database: {}", config.database);
        
        Ok(Self {
            config,
            metrics: Arc::new(Mutex::new(MongoMetrics::default())),
            collections: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn find(&self, collection: &str, filter: Option<&JsonValue>, options: Option<&JsonValue>) -> Result<Vec<JsonValue>, HlxError> {
        let start_time = Instant::now();

        // Simulate find operation
        debug!("Finding documents in collection: {} with filter: {:?}", collection, filter);
        
        // Mock result set
        let mut results = Vec::new();
        let doc = json!({
            "_id": {"$oid": "64a0f8b2c3d4e5f6a7b8c9d0"},
            "name": "test document",
            "value": 42,
            "created_at": {"$date": "2023-07-01T12:00:00Z"}
        });
        results.push(doc);

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.queries_executed += 1;
            metrics.collections_accessed += 1;
            let query_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_query_time = (metrics.avg_query_time * (metrics.queries_executed - 1) as f64 + query_time) / metrics.queries_executed as f64;
        }

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(results)
    }

    pub async fn find_one(&self, collection: &str, filter: &JsonValue) -> Result<Option<JsonValue>, HlxError> {
        let results = self.find(collection, Some(filter), None).await?;
        Ok(results.into_iter().next())
    }

    pub async fn insert_one(&self, collection: &str, document: &JsonValue) -> Result<JsonValue, HlxError> {
        debug!("Inserting document into collection: {}", collection);

        // Mock insert result
        let result = json!({
            "acknowledged": true,
            "insertedId": {"$oid": "64a0f8b2c3d4e5f6a7b8c9d0"}
        });

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.documents_inserted += 1;
            metrics.collections_accessed += 1;
        }

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(result)
    }

    pub async fn insert_many(&self, collection: &str, documents: &[JsonValue]) -> Result<JsonValue, HlxError> {
        debug!("Inserting {} documents into collection: {}", documents.len(), collection);

        // Mock insert result
        let inserted_ids: Vec<JsonValue> = (0..documents.len())
            .map(|i| json!({"$oid": format!("64a0f8b2c3d4e5f6a7b8c9d{:x}", i)}))
            .collect();

        let result = json!({
            "acknowledged": true,
            "insertedIds": inserted_ids
        });

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.documents_inserted += documents.len() as u64;
            metrics.collections_accessed += 1;
        }

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(result)
    }

    pub async fn update_one(&self, collection: &str, filter: &JsonValue, update: &JsonValue) -> Result<JsonValue, HlxError> {
        debug!("Updating one document in collection: {}", collection);

        // Mock update result
        let result = json!({
            "acknowledged": true,
            "matchedCount": 1,
            "modifiedCount": 1,
            "upsertedId": null
        });

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.documents_updated += 1;
            metrics.collections_accessed += 1;
        }

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(result)
    }

    pub async fn update_many(&self, collection: &str, filter: &JsonValue, update: &JsonValue) -> Result<JsonValue, HlxError> {
        debug!("Updating many documents in collection: {}", collection);

        // Mock update result
        let result = json!({
            "acknowledged": true,
            "matchedCount": 3,
            "modifiedCount": 3,
            "upsertedId": null
        });

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.documents_updated += 3;
            metrics.collections_accessed += 1;
        }

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(result)
    }

    pub async fn delete_one(&self, collection: &str, filter: &JsonValue) -> Result<JsonValue, HlxError> {
        debug!("Deleting one document from collection: {}", collection);

        // Mock delete result
        let result = json!({
            "acknowledged": true,
            "deletedCount": 1
        });

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.documents_deleted += 1;
            metrics.collections_accessed += 1;
        }

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(result)
    }

    pub async fn delete_many(&self, collection: &str, filter: &JsonValue) -> Result<JsonValue, HlxError> {
        debug!("Deleting many documents from collection: {}", collection);

        // Mock delete result
        let result = json!({
            "acknowledged": true,
            "deletedCount": 5
        });

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.documents_deleted += 5;
            metrics.collections_accessed += 1;
        }

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(result)
    }

    pub async fn aggregate(&self, collection: &str, pipeline: &[JsonValue]) -> Result<Vec<JsonValue>, HlxError> {
        let start_time = Instant::now();

        debug!("Running aggregation pipeline on collection: {} with {} stages", collection, pipeline.len());

        // Mock aggregation result
        let result = vec![
            json!({
                "_id": "group1",
                "count": 10,
                "total": 100.0
            }),
            json!({
                "_id": "group2",
                "count": 5,
                "total": 50.0
            })
        ];

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.aggregations_performed += 1;
            metrics.collections_accessed += 1;
            let query_time = start_time.elapsed().as_millis() as f64;
            metrics.avg_query_time = (metrics.avg_query_time * (metrics.aggregations_performed - 1) as f64 + query_time) / metrics.aggregations_performed as f64;
        }

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(result)
    }

    pub async fn count_documents(&self, collection: &str, filter: Option<&JsonValue>) -> Result<u64, HlxError> {
        debug!("Counting documents in collection: {} with filter: {:?}", collection, filter);

        // Mock count result
        let count = 42u64;

        // Update metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.queries_executed += 1;
            metrics.collections_accessed += 1;
        }

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(count)
    }

    pub async fn create_index(&self, collection: &str, keys: &JsonValue, options: Option<&JsonValue>) -> Result<JsonValue, HlxError> {
        debug!("Creating index on collection: {} with keys: {:?}", collection, keys);

        // Mock index creation result
        let result = json!({
            "createdCollectionAutomatically": false,
            "numIndexesBefore": 1,
            "numIndexesAfter": 2,
            "ok": 1.0
        });

        // Track collection
        {
            let mut collections = self.collections.write().await;
            collections.insert(collection.to_string(), true);
        }

        Ok(result)
    }

    pub async fn list_collections(&self) -> Result<Vec<String>, HlxError> {
        debug!("Listing collections in database: {}", self.config.database);

        // Mock collection list
        let collections = vec![
            "users".to_string(),
            "products".to_string(),
            "orders".to_string(),
        ];

        Ok(collections)
    }

    pub async fn drop_collection(&self, collection: &str) -> Result<JsonValue, HlxError> {
        debug!("Dropping collection: {}", collection);

        // Mock drop result
        let result = json!({
            "nIndexesWas": 2,
            "ns": format!("{}.{}", self.config.database, collection),
            "ok": 1.0
        });

        // Remove from tracked collections
        {
            let mut collections = self.collections.write().await;
            collections.remove(collection);
        }

        Ok(result)
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("queries_executed".to_string(), Value::Number(metrics.queries_executed as f64));
        result.insert("documents_inserted".to_string(), Value::Number(metrics.documents_inserted as f64));
        result.insert("documents_updated".to_string(), Value::Number(metrics.documents_updated as f64));
        result.insert("documents_deleted".to_string(), Value::Number(metrics.documents_deleted as f64));
        result.insert("aggregations_performed".to_string(), Value::Number(metrics.aggregations_performed as f64));
        result.insert("collections_accessed".to_string(), Value::Number(metrics.collections_accessed as f64));
        result.insert("avg_query_time_ms".to_string(), Value::Number(metrics.avg_query_time));

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for MongoOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "find" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let filter = params_map.get("filter").map(|v| utils::value_to_json_value(v));
                let options = params_map.get("options").map(|v| utils::value_to_json_value(v));

                let results = self.find(&collection, filter.as_ref(), options.as_ref()).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("documents".to_string(), Value::Array(
                        results.into_iter().map(|doc| utils::json_value_to_value(&doc)).collect()
                    ));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "find_one" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let filter = params_map.get("filter")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("filter".to_string()),
                        message: "Missing filter".to_string(),
                    })?;

                let filter_json = utils::value_to_json_value(filter);
                let result = self.find_one(&collection, &filter_json).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("document".to_string(), result.map(|doc| utils::json_value_to_value(&doc)).unwrap_or(Value::Null));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "insert_one" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let document = params_map.get("document")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("document".to_string()),
                        message: "Missing document".to_string(),
                    })?;

                let doc_json = utils::value_to_json_value(document);
                let result = self.insert_one(&collection, &doc_json).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "insert_many" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let documents = params_map.get("documents")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("documents".to_string()),
                        message: "Missing documents array".to_string(),
                    })?;

                if let Value::Array(docs) = documents {
                    let json_docs: Vec<JsonValue> = docs.iter().map(|d| utils::value_to_json_value(d)).collect();
                    let result = self.insert_many(&collection, &json_docs).await?;

                    Ok(Value::Object({
                        let mut map = HashMap::new();
                        map.insert("result".to_string(), utils::json_value_to_value(&result));
                        map.insert("success".to_string(), Value::Boolean(true));
                        map
                    }))
                } else {
                    Err(HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("documents".to_string()),
                        message: "Documents must be an array".to_string(),
                    })
                }
            }

            "update_one" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let filter = params_map.get("filter")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("filter".to_string()),
                        message: "Missing filter".to_string(),
                    })?;

                let update = params_map.get("update")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("update".to_string()),
                        message: "Missing update".to_string(),
                    })?;

                let filter_json = utils::value_to_json_value(filter);
                let update_json = utils::value_to_json_value(update);
                let result = self.update_one(&collection, &filter_json, &update_json).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "update_many" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let filter = params_map.get("filter")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("filter".to_string()),
                        message: "Missing filter".to_string(),
                    })?;

                let update = params_map.get("update")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("update".to_string()),
                        message: "Missing update".to_string(),
                    })?;

                let filter_json = utils::value_to_json_value(filter);
                let update_json = utils::value_to_json_value(update);
                let result = self.update_many(&collection, &filter_json, &update_json).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete_one" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let filter = params_map.get("filter")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("filter".to_string()),
                        message: "Missing filter".to_string(),
                    })?;

                let filter_json = utils::value_to_json_value(filter);
                let result = self.delete_one(&collection, &filter_json).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete_many" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let filter = params_map.get("filter")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("filter".to_string()),
                        message: "Missing filter".to_string(),
                    })?;

                let filter_json = utils::value_to_json_value(filter);
                let result = self.delete_many(&collection, &filter_json).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "aggregate" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let pipeline = params_map.get("pipeline")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("pipeline".to_string()),
                        message: "Missing aggregation pipeline".to_string(),
                    })?;

                if let Value::Array(stages) = pipeline {
                    let json_stages: Vec<JsonValue> = stages.iter().map(|s| utils::value_to_json_value(s)).collect();
                    let results = self.aggregate(&collection, &json_stages).await?;

                    Ok(Value::Object({
                        let mut map = HashMap::new();
                        map.insert("results".to_string(), Value::Array(
                            results.into_iter().map(|r| utils::json_value_to_value(&r)).collect()
                        ));
                        map.insert("success".to_string(), Value::Boolean(true));
                        map
                    }))
                } else {
                    Err(HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("pipeline".to_string()),
                        message: "Pipeline must be an array".to_string(),
                    })
                }
            }

            "count_documents" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let filter = params_map.get("filter").map(|v| utils::value_to_json_value(v));
                let count = self.count_documents(&collection, filter.as_ref()).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("count".to_string(), Value::Number(count as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "create_index" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let keys = params_map.get("keys")
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("keys".to_string()),
                        message: "Missing index keys".to_string(),
                    })?;

                let options = params_map.get("options").map(|v| utils::value_to_json_value(v));
                let keys_json = utils::value_to_json_value(keys);
                let result = self.create_index(&collection, &keys_json, options.as_ref()).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "list_collections" => {
                let collections = self.list_collections().await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("collections".to_string(), Value::Array(
                        collections.into_iter().map(|c| Value::String(c.to_string())).collect()
                    ));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "drop_collection" => {
                let collection = params_map.get("collection").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("collection".to_string()),
                        message: "Missing collection name".to_string(),
                    })?;

                let result = self.drop_collection(&collection).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("result".to_string(), utils::json_value_to_value(&result));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "mongodb".to_string(),
                params: format!("Unknown MongoDB operation: {}", operator),
            }),
        }
    }
} 