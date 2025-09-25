//! Temporal Workflow Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalConfig {
    pub server_url: String,
    pub namespace: String,
    pub task_queue: String,
    pub identity: String,
    pub max_concurrent_activities: u32,
    pub max_concurrent_workflows: u32,
    pub activity_timeout: u64,
    pub workflow_timeout: u64,
}

impl Default for TemporalConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:7233".to_string(),
            namespace: "default".to_string(),
            task_queue: "default".to_string(),
            identity: "helix-worker".to_string(),
            max_concurrent_activities: 10,
            max_concurrent_workflows: 10,
            activity_timeout: 300,
            workflow_timeout: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type: String,
    pub status: WorkflowStatus,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub input: JsonValue,
    pub result: Option<JsonValue>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Terminated,
    ContinuedAsNew,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityExecution {
    pub activity_id: String,
    pub activity_type: String,
    pub workflow_id: String,
    pub status: ActivityStatus,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub input: JsonValue,
    pub result: Option<JsonValue>,
    pub error: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityStatus {
    Scheduled,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Default)]
struct TemporalMetrics {
    workflows_started: u64,
    workflows_completed: u64,
    workflows_failed: u64,
    activities_executed: u64,
    activities_failed: u64,
    avg_workflow_duration: f64,
    avg_activity_duration: f64,
}

pub struct TemporalOperator {
    config: TemporalConfig,
    metrics: Arc<Mutex<TemporalMetrics>>,
    workflows: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    activities: Arc<RwLock<HashMap<String, ActivityExecution>>>,
}

impl TemporalOperator {
    pub async fn new(config: TemporalConfig) -> Result<Self, HlxError> {
        info!("Temporal operator initialized with server: {} namespace: {}", 
              config.server_url, config.namespace);

        Ok(Self {
            config,
            metrics: Arc::new(Mutex::new(TemporalMetrics::default())),
            workflows: Arc::new(RwLock::new(HashMap::new())),
            activities: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn start_workflow(&self, workflow_type: &str, workflow_id: Option<&str>, input: &JsonValue) -> Result<WorkflowExecution, HlxError> {
        let wf_id = workflow_id.unwrap_or(&uuid::Uuid::new_v4().to_string()).to_string();
        let run_id = uuid::Uuid::new_v4().to_string();

        debug!("Starting workflow {} of type {}", wf_id, workflow_type);

        let workflow = WorkflowExecution {
            workflow_id: wf_id.clone(),
            run_id,
            workflow_type: workflow_type.to_string(),
            status: WorkflowStatus::Running,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            end_time: None,
            input: input.clone(),
            result: None,
            error: None,
        };

        {
            let mut workflows = self.workflows.write().await;
            workflows.insert(wf_id.clone(), workflow.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.workflows_started += 1;
        }

        info!("Workflow {} started successfully", wf_id);
        Ok(workflow)
    }

    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowExecution>, HlxError> {
        let workflows = self.workflows.read().await;
        Ok(workflows.get(workflow_id).cloned())
    }

    pub async fn complete_workflow(&self, workflow_id: &str, result: &JsonValue) -> Result<(), HlxError> {
        debug!("Completing workflow {}", workflow_id);

        let mut workflows = self.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            workflow.status = WorkflowStatus::Completed;
            workflow.end_time = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            workflow.result = Some(result.clone());

            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.workflows_completed += 1;
                if let Some(end_time) = workflow.end_time {
                    let duration = (end_time - workflow.start_time) as f64;
                    metrics.avg_workflow_duration = (metrics.avg_workflow_duration * (metrics.workflows_completed - 1) as f64 + duration) / metrics.workflows_completed as f64;
                }
            }

            info!("Workflow {} completed successfully", workflow_id);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Workflow".to_string(),
                identifier: workflow_id.to_string(),
            })
        }
    }

    pub async fn fail_workflow(&self, workflow_id: &str, error: &str) -> Result<(), HlxError> {
        debug!("Failing workflow {} with error: {}", workflow_id, error);

        let mut workflows = self.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            workflow.status = WorkflowStatus::Failed;
            workflow.end_time = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
            workflow.error = Some(error.to_string());

            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.workflows_failed += 1;
            }

            info!("Workflow {} failed: {}", workflow_id, error);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Workflow".to_string(),
                identifier: workflow_id.to_string(),
            })
        }
    }

    pub async fn cancel_workflow(&self, workflow_id: &str) -> Result<(), HlxError> {
        debug!("Cancelling workflow {}", workflow_id);

        let mut workflows = self.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            workflow.status = WorkflowStatus::Cancelled;
            workflow.end_time = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());

            info!("Workflow {} cancelled", workflow_id);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Workflow".to_string(),
                identifier: workflow_id.to_string(),
            })
        }
    }

    pub async fn execute_activity(&self, activity_type: &str, workflow_id: &str, input: &JsonValue) -> Result<ActivityExecution, HlxError> {
        let activity_id = uuid::Uuid::new_v4().to_string();

        debug!("Executing activity {} of type {} for workflow {}", activity_id, activity_type, workflow_id);

        let activity = ActivityExecution {
            activity_id: activity_id.clone(),
            activity_type: activity_type.to_string(),
            workflow_id: workflow_id.to_string(),
            status: ActivityStatus::Running,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            end_time: None,
            input: input.clone(),
            result: None,
            error: None,
            retry_count: 0,
        };

        {
            let mut activities = self.activities.write().await;
            activities.insert(activity_id.clone(), activity.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.activities_executed += 1;
        }

        // Simulate activity execution
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Complete the activity with mock result
        {
            let mut activities = self.activities.write().await;
            if let Some(act) = activities.get_mut(&activity_id) {
                act.status = ActivityStatus::Completed;
                act.end_time = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
                act.result = Some(json!({"status": "success", "output": "mock result"}));

                if let Some(end_time) = act.end_time {
                    let duration = (end_time - act.start_time) as f64;
                    let mut metrics = self.metrics.lock().unwrap();
                    metrics.avg_activity_duration = (metrics.avg_activity_duration * (metrics.activities_executed - 1) as f64 + duration) / metrics.activities_executed as f64;
                }
            }
        }

        info!("Activity {} completed successfully", activity_id);
        Ok(activity)
    }

    pub async fn get_activity(&self, activity_id: &str) -> Result<Option<ActivityExecution>, HlxError> {
        let activities = self.activities.read().await;
        Ok(activities.get(activity_id).cloned())
    }

    pub async fn query_workflows(&self, workflow_type: Option<&str>, status: Option<WorkflowStatus>) -> Result<Vec<WorkflowExecution>, HlxError> {
        let workflows = self.workflows.read().await;
        let mut results = Vec::new();

        for workflow in workflows.values() {
            let mut matches = true;

            if let Some(wf_type) = workflow_type {
                if workflow.workflow_type != wf_type {
                    matches = false;
                }
            }

            if let Some(ref st) = status {
                if std::mem::discriminant(&workflow.status) != std::mem::discriminant(st) {
                    matches = false;
                }
            }

            if matches {
                results.push(workflow.clone());
            }
        }

        Ok(results)
    }

    pub async fn signal_workflow(&self, workflow_id: &str, signal_name: &str, input: &JsonValue) -> Result<(), HlxError> {
        debug!("Sending signal {} to workflow {} with input: {:?}", signal_name, workflow_id, input);

        let workflows = self.workflows.read().await;
        if workflows.contains_key(workflow_id) {
            info!("Signal {} sent to workflow {}", signal_name, workflow_id);
            Ok(())
        } else {
            Err(HlxError::NotFoundError {
                resource: "Workflow".to_string(),
                identifier: workflow_id.to_string(),
            })
        }
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("workflows_started".to_string(), Value::Number(metrics.workflows_started as f64));
        result.insert("workflows_completed".to_string(), Value::Number(metrics.workflows_completed as f64));
        result.insert("workflows_failed".to_string(), Value::Number(metrics.workflows_failed as f64));
        result.insert("activities_executed".to_string(), Value::Number(metrics.activities_executed as f64));
        result.insert("activities_failed".to_string(), Value::Number(metrics.activities_failed as f64));
        result.insert("avg_workflow_duration_seconds".to_string(), Value::Number(metrics.avg_workflow_duration));
        result.insert("avg_activity_duration_seconds".to_string(), Value::Number(metrics.avg_activity_duration));

        if metrics.workflows_started > 0 {
            let success_rate = (metrics.workflows_completed as f64 / metrics.workflows_started as f64) * 100.0;
            result.insert("workflow_success_rate_percent".to_string(), Value::Number(success_rate));
        }

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for TemporalOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "start_workflow" => {
                let workflow_type = params_map.get("workflow_type").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("workflow_type".to_string()),
                        message: "Missing workflow type".to_string(),
                    })?;

                let workflow_id = params_map.get("workflow_id").and_then(|v| v.as_string());
                let input = params_map.get("input").map(|v| utils::value_to_json_value(v))
                    .unwrap_or_else(|| json!({}));

                let workflow = self.start_workflow(&workflow_type, workflow_id.as_deref(), &input).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("workflow_id".to_string(), Value::String(workflow.workflow_id.to_string()));
                    map.insert("run_id".to_string(), Value::String(workflow.run_id.to_string()));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_workflow" => {
                let workflow_id = params_map.get("workflow_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("workflow_id".to_string()),
                        message: "Missing workflow ID".to_string(),
                    })?;

                let workflow = self.get_workflow(&workflow_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    if let Some(wf) = workflow {
                        map.insert("workflow_id".to_string(), Value::String(wf.workflow_id.to_string()));
                        map.insert("workflow_type".to_string(), Value::String(wf.workflow_type.to_string()));
                        map.insert("status".to_string(), Value::String(format!("{:?}", wf.status.to_string())));
                        map.insert("start_time".to_string(), Value::Number(wf.start_time as f64));
                    } else {
                        map.insert("workflow".to_string(), Value::Null);
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "complete_workflow" => {
                let workflow_id = params_map.get("workflow_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("workflow_id".to_string()),
                        message: "Missing workflow ID".to_string(),
                    })?;

                let result = params_map.get("result").map(|v| utils::value_to_json_value(v))
                    .unwrap_or_else(|| json!({}));

                self.complete_workflow(&workflow_id, &result).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("workflow_id".to_string(), Value::String(workflow_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "fail_workflow" => {
                let workflow_id = params_map.get("workflow_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("workflow_id".to_string()),
                        message: "Missing workflow ID".to_string(),
                    })?;

                let error = params_map.get("error").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("error".to_string()),
                        message: "Missing error message".to_string(),
                    })?;

                self.fail_workflow(&workflow_id, &error).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("workflow_id".to_string(), Value::String(workflow_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "cancel_workflow" => {
                let workflow_id = params_map.get("workflow_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("workflow_id".to_string()),
                        message: "Missing workflow ID".to_string(),
                    })?;

                self.cancel_workflow(&workflow_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("workflow_id".to_string(), Value::String(workflow_id.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "execute_activity" => {
                let activity_type = params_map.get("activity_type").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("activity_type".to_string()),
                        message: "Missing activity type".to_string(),
                    })?;

                let workflow_id = params_map.get("workflow_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("workflow_id".to_string()),
                        message: "Missing workflow ID".to_string(),
                    })?;

                let input = params_map.get("input").map(|v| utils::value_to_json_value(v))
                    .unwrap_or_else(|| json!({}));

                let activity = self.execute_activity(&activity_type, &workflow_id, &input).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("activity_id".to_string(), Value::String(activity.activity_id.to_string()));
                    map.insert("status".to_string(), Value::String(format!("{:?}", activity.status.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_activity" => {
                let activity_id = params_map.get("activity_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("activity_id".to_string()),
                        message: "Missing activity ID".to_string(),
                    })?;

                let activity = self.get_activity(&activity_id).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    if let Some(act) = activity {
                        map.insert("activity_id".to_string(), Value::String(act.activity_id.to_string()));
                        map.insert("activity_type".to_string(), Value::String(act.activity_type.to_string()));
                        map.insert("workflow_id".to_string(), Value::String(act.workflow_id.to_string()));
                        map.insert("status".to_string(), Value::String(format!("{:?}", act.status.to_string())));
                        map.insert("retry_count".to_string(), Value::Number(act.retry_count as f64));
                    } else {
                        map.insert("activity".to_string(), Value::Null);
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "signal_workflow" => {
                let workflow_id = params_map.get("workflow_id").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("workflow_id".to_string()),
                        message: "Missing workflow ID".to_string(),
                    })?;

                let signal_name = params_map.get("signal_name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("signal_name".to_string()),
                        message: "Missing signal name".to_string(),
                    })?;

                let input = params_map.get("input").map(|v| utils::value_to_json_value(v))
                    .unwrap_or_else(|| json!({}));

                self.signal_workflow(&workflow_id, &signal_name, &input).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("workflow_id".to_string(), Value::String(workflow_id.to_string(.to_string())));
                    map.insert("signal_name".to_string(), Value::String(signal_name.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "query_workflows" => {
                let workflow_type = params_map.get("workflow_type").and_then(|v| v.as_string());
                
                // For simplicity, we'll skip status filtering in this mock
                let workflows = self.query_workflows(workflow_type.as_deref(), None).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("count".to_string(), Value::Number(workflows.len() as f64));
                    map.insert("workflows".to_string(), Value::Array(
                        workflows.into_iter().map(|wf| {
                            let mut workflow_map = HashMap::new();
                            workflow_map.insert("workflow_id".to_string(), Value::String(wf.workflow_id.to_string()));
                            workflow_map.insert("workflow_type".to_string(), Value::String(wf.workflow_type.to_string()));
                            workflow_map.insert("status".to_string(), Value::String(format!("{:?}", wf.status.to_string())));
                            Value::Object(workflow_map)
                        }).collect()
                    ));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "temporal".to_string(),
                params: format!("Unknown Temporal operation: {}", operator),
            }),
        }
    }
} 