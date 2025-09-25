//! Grafana Dashboard Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaConfig {
    pub base_url: String,
    pub api_token: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub organization_id: Option<u64>,
    pub timeout: u64,
    pub verify_ssl: bool,
}

impl Default for GrafanaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(),
            api_token: "".to_string(),
            username: Some("admin".to_string()),
            password: Some("admin".to_string()),
            organization_id: Some(1),
            timeout: 30,
            verify_ssl: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub id: Option<u64>,
    pub uid: Option<String>,
    pub title: String,
    pub tags: Vec<String>,
    pub timezone: String,
    pub panels: Vec<JsonValue>,
    pub time: DashboardTime,
    pub templating: JsonValue,
    pub annotations: JsonValue,
    pub refresh: String,
    pub schema_version: u32,
    pub version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardTime {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: Option<u64>,
    pub uid: String,
    pub title: String,
    pub condition: String,
    pub data: Vec<JsonValue>,
    pub no_data_state: String,
    pub exec_err_state: String,
    pub for_duration: String,
    pub annotations: HashMap<String, String>,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct GrafanaMetrics {
    dashboards_created: u64,
    dashboards_updated: u64,
    dashboards_deleted: u64,
    alerts_created: u64,
    alerts_fired: u64,
    api_calls: u64,
    api_errors: u64,
}

pub struct GrafanaOperator {
    config: GrafanaConfig,
    http_client: reqwest::Client,
    metrics: Arc<Mutex<GrafanaMetrics>>,
    dashboards: Arc<RwLock<HashMap<String, Dashboard>>>, // uid -> dashboard
    alerts: Arc<RwLock<HashMap<String, AlertRule>>>,     // uid -> alert
}

impl GrafanaOperator {
    pub async fn new(config: GrafanaConfig) -> Result<Self, HlxError> {
        let client_builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .danger_accept_invalid_certs(!config.verify_ssl);

        let client = client_builder.build()
            .map_err(|e| HlxError::InitializationError {
                component: "Grafana HTTP Client".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;


        Ok(Self {
            config,
            http_client: client,
            metrics: Arc::new(Mutex::new(GrafanaMetrics::default())),
            dashboards: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn create_dashboard(&self, dashboard: Dashboard) -> Result<Dashboard, HlxError> {

        let dashboard_json = json!({
            "dashboard": {
                "id": null,
                "uid": dashboard.uid,
                "title": dashboard.title,
                "tags": dashboard.tags,
                "timezone": dashboard.timezone,
                "panels": dashboard.panels,
                "time": dashboard.time,
                "templating": dashboard.templating,
                "annotations": dashboard.annotations,
                "refresh": dashboard.refresh,
                "schemaVersion": dashboard.schema_version,
                "version": 0
            },
            "folderId": 0,
            "overwrite": false
        });

        let mut request = self.http_client
            .post(&format!("{}/api/dashboards/db", self.config.base_url))
            .json(&dashboard_json);

        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Create Dashboard".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        self.update_api_metrics(response.status().is_success()).await;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Create Dashboard".to_string(),
                message: format!("Failed with status: {}", response.status()),
            });
        }

        let result: JsonValue = response.json().await
            .map_err(|e| HlxError::ParseError {
                line: 0,
                column: 0,
                context: String::new(),
                suggestion: None,
                format: "JSON".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        let mut created_dashboard = dashboard;
        created_dashboard.id = result["id"].as_u64();
        created_dashboard.uid = result["uid"].as_str().map(|s| s.to_string()).or(created_dashboard.uid);

        // Store in local cache
        if let Some(ref uid) = created_dashboard.uid {
            let mut dashboards = self.dashboards.write().await;
            dashboards.insert(uid.clone(), created_dashboard.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.dashboards_created += 1;
        }

        Ok(created_dashboard)
    }

    pub async fn get_dashboard(&self, uid: &str) -> Result<Option<Dashboard>, HlxError> {

        // First check local cache
        {
            let dashboards = self.dashboards.read().await;
            if let Some(dashboard) = dashboards.get(uid) {
                return Ok(Some(dashboard.clone()));
            }
        }

        // If not in cache, fetch from Grafana API
        let mut request = self.http_client
            .get(&format!("{}/api/dashboards/uid/{}", self.config.base_url, uid));

        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Get Dashboard".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        self.update_api_metrics(response.status().is_success()).await;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Get Dashboard".to_string(),
                message: format!("Failed with status: {}", response.status()),
            });
        }

        // Mock dashboard for this example
        let dashboard = Dashboard {
            id: Some(1),
            uid: Some(uid.to_string()),
            title: "Mock Dashboard".to_string(),
            tags: vec!["mock".to_string()],
            timezone: "browser".to_string(),
            panels: vec![],
            time: DashboardTime {
                from: "now-6h".to_string(),
                to: "now".to_string(),
            },
            templating: json!({"list": []}),
            annotations: json!({"list": []}),
            refresh: "5s".to_string(),
            schema_version: 30,
            version: Some(1),
        };

        // Cache the result
        {
            let mut dashboards = self.dashboards.write().await;
            dashboards.insert(uid.to_string(), dashboard.clone());
        }

        Ok(Some(dashboard))
    }

    pub async fn update_dashboard(&self, dashboard: Dashboard) -> Result<Dashboard, HlxError> {

        let dashboard_json = json!({
            "dashboard": {
                "id": dashboard.id,
                "uid": dashboard.uid,
                "title": dashboard.title,
                "tags": dashboard.tags,
                "timezone": dashboard.timezone,
                "panels": dashboard.panels,
                "time": dashboard.time,
                "templating": dashboard.templating,
                "annotations": dashboard.annotations,
                "refresh": dashboard.refresh,
                "schemaVersion": dashboard.schema_version,
                "version": dashboard.version
            },
            "folderId": 0,
            "overwrite": true
        });

        let mut request = self.http_client
            .post(&format!("{}/api/dashboards/db", self.config.base_url))
            .json(&dashboard_json);

        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Update Dashboard".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        self.update_api_metrics(response.status().is_success()).await;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Update Dashboard".to_string(),
                message: format!("Failed with status: {}", response.status()),
            });
        }

        // Update in local cache
        if let Some(ref uid) = dashboard.uid {
            let mut dashboards = self.dashboards.write().await;
            dashboards.insert(uid.clone(), dashboard.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.dashboards_updated += 1;
        }

        Ok(dashboard)
    }

    pub async fn delete_dashboard(&self, uid: &str) -> Result<(), HlxError> {

        let mut request = self.http_client
            .delete(&format!("{}/api/dashboards/uid/{}", self.config.base_url, uid));

        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Delete Dashboard".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        self.update_api_metrics(response.status().is_success()).await;

        if !response.status().is_success() {
            return Err(HlxError::OperationError { operator: "unknown".to_string(),
                operator: "unknown".to_string(),
                details: None,
                operation: "Delete Dashboard".to_string(),
                message: format!("Failed with status: {}", response.status()),
            });
        }

        // Remove from local cache
        {
            let mut dashboards = self.dashboards.write().await;
            dashboards.remove(uid);
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.dashboards_deleted += 1;
        }

        Ok(())
    }

    pub async fn create_alert_rule(&self, alert: AlertRule) -> Result<AlertRule, HlxError> {

        // Mock implementation - in real world would make API call
        let mut created_alert = alert;
        created_alert.id = Some(1);

        {
            let mut alerts = self.alerts.write().await;
            alerts.insert(created_alert.uid.clone(), created_alert.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.alerts_created += 1;
        }

        Ok(created_alert)
    }

    pub async fn get_health(&self) -> Result<JsonValue, HlxError> {

        let mut request = self.http_client
            .get(&format!("{}/api/health", self.config.base_url));

        request = self.add_auth_headers(request);

        let response = request.send().await
            .map_err(|e| HlxError::NetworkError {
                operation: Some("network_operation".to_string()),
                status_code: None,
                url: None,
                operation: "Health Check".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        self.update_api_metrics(response.status().is_success()).await;

        // Mock health response
        Ok(json!({
            "commit": "abcd123",
            "database": "ok",
            "version": "8.5.0"
        }))
    }

    fn add_auth_headers(&self, mut request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if !self.config.api_token.is_empty() {
            request = request.bearer_auth(&self.config.api_token);
        } else if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
            request = request.basic_auth(username, Some(password));
        }
        request
    }

    async fn update_api_metrics(&self, success: bool) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.api_calls += 1;
        if !success {
            metrics.api_errors += 1;
        }
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("dashboards_created".to_string(), Value::Number(metrics.dashboards_created as f64));
        result.insert("dashboards_updated".to_string(), Value::Number(metrics.dashboards_updated as f64));
        result.insert("dashboards_deleted".to_string(), Value::Number(metrics.dashboards_deleted as f64));
        result.insert("alerts_created".to_string(), Value::Number(metrics.alerts_created as f64));
        result.insert("alerts_fired".to_string(), Value::Number(metrics.alerts_fired as f64));
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
impl crate::operators::OperatorTrait for GrafanaOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "create_dashboard" => {
                let title = params_map.get("title").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("title".to_string()),
                        message: "Missing dashboard title".to_string(),
                    })?;

                let uid = params_map.get("uid").and_then(|v| v.as_string());
                let tags = params_map.get("tags").and_then(|v| {
                    if let Value::Array(arr) = v {
                        Some(arr.iter().filter_map(|item| item.as_string()).collect())
                    } else { None }
                }).unwrap_or_else(Vec::new);

                let dashboard = Dashboard {
                    id: None,
                    uid: uid.map(|s| s.to_string()),
                    title: title.to_string(),
                    tags: tags.into_iter().map(|s| s.to_string()).collect(),
                    timezone: "browser".to_string(),
                    panels: vec![],
                    time: DashboardTime {
                        from: "now-6h".to_string(),
                        to: "now".to_string(),
                    },
                    templating: json!({"list": []}),
                    annotations: json!({"list": []}),
                    refresh: "5s".to_string(),
                    schema_version: 30,
                    version: None,
                };

                let created = self.create_dashboard(dashboard).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("id".to_string(), created.id.map(|i| Value::Number(i as f64)).unwrap_or(Value::Null));
                    map.insert("uid".to_string(), created.uid.map(Value::String).unwrap_or(Value::Null));
                    map.insert("title".to_string(), Value::String(created.title.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_dashboard" => {
                let uid = params_map.get("uid").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("uid".to_string()),
                        message: "Missing dashboard UID".to_string(),
                    })?;

                let dashboard = self.get_dashboard(&uid).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    if let Some(dash) = dashboard {
                        map.insert("id".to_string(), dash.id.map(|i| Value::Number(i as f64)).unwrap_or(Value::Null));
                        map.insert("uid".to_string(), dash.uid.map(Value::String).unwrap_or(Value::Null));
                        map.insert("title".to_string(), Value::String(dash.title.to_string()));
                        map.insert("tags".to_string(), Value::Array(dash.tags.into_iter().map(Value::String).collect()));
                    } else {
                        map.insert("dashboard".to_string(), Value::Null);
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete_dashboard" => {
                let uid = params_map.get("uid").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("uid".to_string()),
                        message: "Missing dashboard UID".to_string(),
                    })?;

                self.delete_dashboard(&uid).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("uid".to_string(), Value::String(uid.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "create_alert" => {
                let title = params_map.get("title").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("title".to_string()),
                        message: "Missing alert title".to_string(),
                    })?;

                let condition = params_map.get("condition").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("condition".to_string()),
                        message: "Missing alert condition".to_string(),
                    })?;

                let alert = AlertRule {
                    id: None,
                    uid: uuid::Uuid::new_v4().to_string(),
                    title: title.to_string(),
                    condition: condition.to_string(),
                    data: vec![],
                    no_data_state: "NoData".to_string(),
                    exec_err_state: "Alerting".to_string(),
                    for_duration: "5m".to_string(),
                    annotations: HashMap::new(),
                    labels: HashMap::new(),
                };

                let created = self.create_alert_rule(alert).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("id".to_string(), created.id.map(|i| Value::Number(i as f64)).unwrap_or(Value::Null));
                    map.insert("uid".to_string(), Value::String(created.uid.to_string()));
                    map.insert("title".to_string(), Value::String(created.title.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "health" => {
                let health = self.get_health().await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("health".to_string(), utils::json_value_to_value(&health));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "grafana".to_string(),
                params: format!("Unknown Grafana operation: {}", operator),
            }),
        }
    }
} 