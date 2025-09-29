use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::atp::ast::*;
use crate::atp::value::Value as AstValue;

fn extract_string_value(
    expr: &Option<&Expression>,
) -> Result<String, String> {
    match expr {
        Some(Expression::String(s)) => Ok(s.clone()),
        Some(Expression::Identifier(s)) => Ok(s.clone()),
        _ => Ok(String::new()),
    }
}
fn extract_float_value(expr: &Option<&Expression>) -> Result<f64, String> {
    match expr {
        Some(Expression::Number(n)) => Ok(*n),
        _ => Ok(0.0),
    }
}
fn extract_int_value(expr: &Option<&Expression>) -> Result<i64, String> {
    match expr {
        Some(Expression::Number(n)) => Ok(*n as i64),
        _ => Ok(0),
    }
}
fn extract_bool_value(expr: &Option<&Expression>) -> Result<bool, String> {
    match expr {
        Some(Expression::Bool(b)) => Ok(*b),
        _ => Ok(false),
    }
}
fn extract_duration_value(
    expr: &Option<&Expression>,
) -> Result<Duration, String> {
    match expr {
        Some(Expression::Duration(duration)) => {
            Ok(Duration {
                value: duration.value as u64,
                unit: duration.unit.clone(),
            })
        }
        _ => {
            Ok(Duration {
                value: 0,
                unit: TimeUnit::Seconds,
            })
        }
    }
}
fn extract_array_values(
    expr: &Option<&Expression>,
) -> Result<Vec<String>, String> {
    match expr {
        Some(Expression::Array(items)) => {
            items
                .iter()
                .map(|e| match e {
                    Expression::String(s) => Ok(s.clone()),
                    Expression::Identifier(s) => Ok(s.clone()),
                    _ => Err("Array items must be strings".to_string()),
                })
                .collect()
        }
        _ => Ok(Vec::new()),
    }
}
fn extract_map_values(
    expr: &Option<&Expression>,
) -> Result<std::collections::HashMap<String, String>, String> {
    match expr {
        Some(Expression::Object(map)) => {
            let mut result = std::collections::HashMap::new();
            for (k, v) in map {
                let value = match v {
                    Expression::String(s) => s.clone(),
                    Expression::Identifier(s) => s.clone(),
                    Expression::Number(n) => n.to_string(),
                    Expression::Bool(b) => b.to_string(),
                    _ => String::new(),
                };
                result.insert(k.clone(), value);
            }
            Ok(result)
        }
        _ => Ok(std::collections::HashMap::new()),
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelixConfig {
    pub projects: HashMap<String, ProjectConfig>,
    pub agents: HashMap<String, AgentConfig>,
    pub workflows: HashMap<String, WorkflowConfig>,
    pub memory: Option<MemoryConfig>,
    pub contexts: HashMap<String, ContextConfig>,
    pub crews: HashMap<String, CrewConfig>,
    pub pipelines: HashMap<String, PipelineConfig>,
    pub plugins: Vec<PluginConfig>,
    pub databases: HashMap<String, DatabaseConfig>,
    pub sections: HashMap<String, HashMap<String, Value>>,
}
impl Default for HelixConfig {
    fn default() -> Self {
        Self {
            projects: HashMap::new(),
            agents: HashMap::new(),
            workflows: HashMap::new(),
            memory: None,
            contexts: HashMap::new(),
            crews: HashMap::new(),
            pipelines: HashMap::new(),
            plugins: Vec::new(),
            databases: HashMap::new(),
            sections: HashMap::new(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: Option<String>,
    pub metadata: HashMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub role: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub capabilities: Vec<String>,
    pub backstory: Option<String>,
    pub tools: Vec<String>,
    pub constraints: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub name: String,
    pub trigger: TriggerConfig,
    pub steps: Vec<StepConfig>,
    pub pipeline: Option<PipelineConfig>,
    pub outputs: Vec<String>,
    pub on_error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepConfig {
    pub name: String,
    pub agent: Option<String>,
    pub crew: Option<Vec<String>>,
    pub task: String,
    pub timeout: Option<Duration>,
    pub parallel: bool,
    pub depends_on: Vec<String>,
    pub retry: Option<RetryConfig>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub provider: String,
    pub connection: String,
    pub embeddings: EmbeddingConfig,
    pub cache_size: Option<usize>,
    pub persistence: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub model: String,
    pub dimensions: u32,
    pub batch_size: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub name: String,
    pub environment: String,
    pub debug: bool,
    pub max_tokens: Option<u64>,
    pub secrets: HashMap<String, SecretRef>,
    pub variables: HashMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewConfig {
    pub name: String,
    pub agents: Vec<String>,
    pub process_type: ProcessType,
    pub manager: Option<String>,
    pub max_iterations: Option<u32>,
    pub verbose: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub source: String,
    pub version: String,
    pub config: HashMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub name: String,
    pub path: Option<String>,
    pub shards: Option<i64>,
    pub compression: Option<bool>,
    pub cache_size: Option<i64>,
    pub vector_index: Option<VectorIndexConfig>,
    pub properties: HashMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexConfig {
    pub index_type: String,
    pub dimensions: i64,
    pub m: Option<i64>,
    pub ef_construction: Option<i64>,
    pub distance_metric: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Duration(Duration),
    Reference(String),
    Identifier(String),
}
impl Value {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        self.as_string()
    }
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Duration {
    pub value: u64,
    pub unit: TimeUnit,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerConfig {
    Manual,
    Schedule(String),
    Webhook(String),
    Event(String),
    FileWatch(String),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessType {
    Sequential,
    Hierarchical,
    Parallel,
    Consensus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub delay: Duration,
    pub backoff: BackoffStrategy,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub name: String,
    pub stages: Vec<String>,
    pub flow: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretRef {
    Environment(String),
    Vault(String),
    File(String),
}
pub struct HelixLoader {
    configs: HashMap<String, HelixConfig>,
    current_context: Option<String>,
}
impl HelixLoader {
    pub fn new() -> Self {
        HelixLoader {
            configs: HashMap::new(),
            current_context: None,
        }
    }
    pub fn load_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<HelixConfig, HlxError> {
        let content = fs::read_to_string(path)?;
        self.parse(&content)
    }
    pub fn parse(&mut self, content: &str) -> Result<HelixConfig, HlxError> {
        let tokens = crate::atp::lexer::tokenize(content)?;
        let ast = crate::atp::parser::parse(tokens)?;
        let config = self.ast_to_config(ast)?;
        Ok(config)
    }
    pub fn ast_to_config(
        &self,
        ast: HelixAst,
    ) -> Result<HelixConfig, HlxError> {
        let mut config = HelixConfig {
            projects: HashMap::new(),
            agents: HashMap::new(),
            workflows: HashMap::new(),
            memory: None,
            contexts: HashMap::new(),
            crews: HashMap::new(),
            pipelines: HashMap::new(),
            plugins: Vec::new(),
            databases: HashMap::new(),
            sections: HashMap::new(),
        };
        for decl in ast.declarations {
            match decl {
                Declaration::Project(p) => {
                    let project = self.convert_project(p)?;
                    config.projects.insert(project.name.clone(), project);
                }
                Declaration::Agent(a) => {
                    let agent = self.convert_agent(a)?;
                    config.agents.insert(agent.name.clone(), agent);
                }
                Declaration::Workflow(w) => {
                    let workflow = self.convert_workflow(w)?;
                    config.workflows.insert(workflow.name.clone(), workflow);
                }
                Declaration::Memory(m) => {
                    config.memory = Some(self.convert_memory(m)?);
                }
                Declaration::Context(c) => {
                    let context = self.convert_context(c)?;
                    config.contexts.insert(context.name.clone(), context);
                }
                Declaration::Crew(cr) => {
                    let crew = self.convert_crew(cr)?;
                    config.crews.insert(crew.name.clone(), crew);
                }
                Declaration::Plugin(p) => {
                    config.plugins.push(self.convert_plugin(p)?);
                }
                Declaration::Database(d) => {
                    let database = self.convert_database(d)?;
                    config.databases.insert(database.name.clone(), database);
                }
                Declaration::Task(t) => {
                    let task_data: HashMap<String, Value> = t
                        .properties
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_value()))
                        .collect();
                    config.sections.insert(format!("task.{}", t.name), task_data);
                }
                Declaration::Pipeline(p) => {
                    let pipeline = self.convert_pipeline(p)?;
                    config.pipelines.insert(pipeline.name.clone(), pipeline);
                }
                Declaration::Load(_l) => {}
                Declaration::Section(s) => {
                    let section_data: HashMap<String, Value> = s
                        .properties
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_value()))
                        .collect();
                    config.sections.insert(s.name.clone(), section_data);
                }
            }
        }
        Ok(config)
    }
    fn convert_project(
        &self,
        project: ProjectDecl,
    ) -> Result<ProjectConfig, HlxError> {
        let mut metadata = HashMap::new();
        let mut version = String::new();
        let mut author = String::new();
        let mut description = None;
        for (key, expr) in project.properties {
            let expr_opt = Some(&expr);
            match key.as_str() {
                "version" => {
                    version = extract_string_value(&expr_opt).unwrap_or_default();
                }
                "author" => {
                    author = extract_string_value(&expr_opt).unwrap_or_default();
                }
                "description" => {
                    let desc = extract_string_value(&expr_opt).unwrap_or_default();
                    description = if desc.is_empty() { None } else { Some(desc) };
                }
                _ => {
                    metadata.insert(key, self.expression_to_value(expr));
                }
            }
        }
        Ok(ProjectConfig {
            name: project.name,
            version,
            author,
            description,
            metadata,
        })
    }
    fn convert_agent(
        &self,
        agent: AgentDecl,
    ) -> Result<AgentConfig, HlxError> {
        let mut config = AgentConfig {
            name: agent.name.clone(),
            model: String::new(),
            role: String::new(),
            temperature: None,
            max_tokens: None,
            capabilities: agent.capabilities.unwrap_or_default(),
            backstory: agent.backstory.map(|b| b.lines.join("\n")),
            tools: agent.tools.unwrap_or_default(),
            constraints: Vec::new(),
        };
        for (key, expr) in agent.properties {
            let expr_opt = Some(&expr);
            match key.as_str() {
                "model" => {
                    config.model = extract_string_value(&expr_opt).unwrap_or_default();
                }
                "role" => {
                    config.role = extract_string_value(&expr_opt).unwrap_or_default();
                }
                "temperature" => {
                    config.temperature = extract_float_value(&expr_opt)
                        .ok()
                        .map(|f| f as f32);
                }
                "max_tokens" => {
                    config.max_tokens = extract_int_value(&expr_opt)
                        .ok()
                        .map(|i| i as u32);
                }
                "custom" | "properties" | "config" => {
                    if let Ok(custom_map) = extract_map_values(&expr_opt) {
                        for (key, value) in custom_map {
                            println!("Agent custom property: {} = {}", key, value);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(config)
    }
    fn convert_workflow(
        &self,
        workflow: WorkflowDecl,
    ) -> Result<WorkflowConfig, HlxError> {
        let trigger = if let Some(t) = workflow.trigger {
            self.convert_trigger(t)?
        } else {
            TriggerConfig::Manual
        };
        let mut steps = Vec::new();
        for step in workflow.steps {
            steps.push(self.convert_step(step)?);
        }
        let pipeline = if let Some(p) = workflow.pipeline {
            Some(self.convert_pipeline(p)?)
        } else {
            None
        };
        Ok(WorkflowConfig {
            name: workflow.name,
            trigger,
            steps,
            pipeline,
            outputs: Vec::new(),
            on_error: None,
        })
    }
    fn convert_trigger(
        &self,
        expr: Expression,
    ) -> Result<TriggerConfig, HlxError> {
        match expr {
            Expression::String(s) | Expression::Identifier(s) => {
                match s.as_str() {
                    "manual" => Ok(TriggerConfig::Manual),
                    s if s.starts_with("schedule:") => {
                        Ok(
                            TriggerConfig::Schedule(
                                s.trim_start_matches("schedule:").to_string(),
                            ),
                        )
                    }
                    s if s.starts_with("webhook:") => {
                        Ok(
                            TriggerConfig::Webhook(
                                s.trim_start_matches("webhook:").to_string(),
                            ),
                        )
                    }
                    s if s.starts_with("event:") => {
                        Ok(
                            TriggerConfig::Event(
                                s.trim_start_matches("event:").to_string(),
                            ),
                        )
                    }
                    s if s.starts_with("file:") => {
                        Ok(
                            TriggerConfig::FileWatch(
                                s.trim_start_matches("file:").to_string(),
                            ),
                        )
                    }
                    _ => Ok(TriggerConfig::Manual),
                }
            }
            _ => Ok(TriggerConfig::Manual),
        }
    }
    fn convert_step(&self, step: StepDecl) -> Result<StepConfig, HlxError> {
        let mut config = StepConfig {
            name: step.name,
            agent: step.agent,
            crew: step.crew,
            task: step.task.unwrap_or_default(),
            timeout: None,
            parallel: false,
            depends_on: Vec::new(),
            retry: None,
        };
        for (key, expr) in step.properties {
            let expr_opt = Some(&expr);
            match key.as_str() {
                "timeout" => {
                    config.timeout = extract_duration_value(&expr_opt).ok();
                }
                "parallel" => {
                    config.parallel = extract_bool_value(&expr_opt).unwrap_or(false);
                }
                "depends_on" => {
                    config.depends_on = extract_array_values(&expr_opt)
                        .unwrap_or_default();
                }
                "retry" => {
                    if let Some(obj) = expr.as_object() {
                        config.retry = self.convert_retry_config(obj);
                    }
                }
                _ => {}
            }
        }
        Ok(config)
    }
    fn convert_retry_config(
        &self,
        obj: &HashMap<String, Expression>,
    ) -> Option<RetryConfig> {
        let max_attempts = obj.get("max_attempts")?.as_number()? as u32;
        let delay = obj
            .get("delay")
            .and_then(|e| self.expression_to_duration(e.clone()))?;
        let backoff = obj
            .get("backoff")
            .and_then(|e| e.as_string())
            .and_then(|s| match s.as_str() {
                "fixed" => Some(BackoffStrategy::Fixed),
                "linear" => Some(BackoffStrategy::Linear),
                "exponential" => Some(BackoffStrategy::Exponential),
                _ => None,
            })
            .unwrap_or(BackoffStrategy::Fixed);
        Some(RetryConfig {
            max_attempts,
            delay,
            backoff,
        })
    }
    fn convert_pipeline(
        &self,
        pipeline: PipelineDecl,
    ) -> Result<PipelineConfig, HlxError> {
        let stages = pipeline
            .flow
            .iter()
            .filter_map(|node| {
                if let PipelineNode::Step(name) = node {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        let flow = pipeline
            .flow
            .iter()
            .filter_map(|node| {
                if let PipelineNode::Step(name) = node {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" -> ");
        Ok(PipelineConfig {
            name: "default".to_string(),
            stages,
            flow,
        })
    }
    fn convert_memory(
        &self,
        memory: MemoryDecl,
    ) -> Result<MemoryConfig, HlxError> {
        let embeddings = if let Some(e) = memory.embeddings {
            EmbeddingConfig {
                model: e.model,
                dimensions: e.dimensions,
                batch_size: e
                    .properties
                    .get("batch_size")
                    .and_then(|v| v.as_number())
                    .map(|n| n as u32),
            }
        } else {
            EmbeddingConfig {
                model: String::new(),
                dimensions: 0,
                batch_size: None,
            }
        };
        Ok(MemoryConfig {
            provider: memory.provider,
            connection: memory.connection,
            embeddings,
            cache_size: memory
                .properties
                .get("cache_size")
                .and_then(|v| v.as_number())
                .map(|n| n as usize),
            persistence: memory
                .properties
                .get("persistence")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        })
    }
    fn convert_context(
        &self,
        context: ContextDecl,
    ) -> Result<ContextConfig, HlxError> {
        let mut secrets = HashMap::new();
        if let Some(s) = context.secrets {
            for (key, secret_ref) in s {
                secrets.insert(key, self.convert_secret_ref(secret_ref));
            }
        }
        let mut variables = HashMap::new();
        for (key, expr) in &context.properties {
            if key != "debug" && key != "max_tokens" {
                variables.insert(key.clone(), expr.to_value());
            }
        }
        Ok(ContextConfig {
            name: context.name,
            environment: context.environment,
            debug: context
                .properties
                .get("debug")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            max_tokens: context
                .properties
                .get("max_tokens")
                .and_then(|v| v.as_number())
                .map(|n| n as u64),
            secrets,
            variables,
        })
    }
    fn convert_secret_ref(&self, secret_ref: SecretRef) -> SecretRef {
        match secret_ref {
            SecretRef::Environment(var) => SecretRef::Environment(var),
            SecretRef::Vault(path) => SecretRef::Vault(path),
            SecretRef::File(path) => SecretRef::File(path),
        }
    }
    fn convert_crew(&self, crew: CrewDecl) -> Result<CrewConfig, HlxError> {
        let process_type = crew
            .process_type
            .and_then(|p| match p.as_str() {
                "sequential" => Some(ProcessType::Sequential),
                "hierarchical" => Some(ProcessType::Hierarchical),
                "parallel" => Some(ProcessType::Parallel),
                "consensus" => Some(ProcessType::Consensus),
                _ => None,
            })
            .unwrap_or(ProcessType::Sequential);
        Ok(CrewConfig {
            name: crew.name,
            agents: crew.agents,
            process_type,
            manager: crew.properties.get("manager").and_then(|e| e.as_string()),
            max_iterations: crew
                .properties
                .get("max_iterations")
                .and_then(|e| e.as_number())
                .map(|n| n as u32),
            verbose: crew
                .properties
                .get("verbose")
                .and_then(|e| e.as_bool())
                .unwrap_or(false),
        })
    }
    fn convert_plugin(
        &self,
        plugin: PluginDecl,
    ) -> Result<PluginConfig, HlxError> {
        let mut config = HashMap::new();
        for (key, expr) in plugin.config {
            config.insert(key, self.expression_to_value(expr));
        }
        Ok(PluginConfig {
            name: plugin.name,
            source: plugin.source,
            version: plugin.version.unwrap_or_else(|| "latest".to_string()),
            config,
        })
    }
    fn convert_database(
        &self,
        database: DatabaseDecl,
    ) -> Result<DatabaseConfig, HlxError> {
        let mut properties = HashMap::new();
        for (key, expr) in database.properties {
            properties.insert(key, self.expression_to_value(expr));
        }
        let vector_index = database
            .vector_index
            .map(|vi| VectorIndexConfig {
                index_type: vi.index_type,
                dimensions: vi.dimensions,
                m: vi.m,
                ef_construction: vi.ef_construction,
                distance_metric: vi.distance_metric,
            });
        Ok(DatabaseConfig {
            name: database.name,
            path: database.path,
            shards: database.shards,
            compression: database.compression,
            cache_size: database.cache_size,
            vector_index,
            properties,
        })
    }
    fn expression_to_value(&self, expr: Expression) -> Value {
        expr.to_value()
    }
    fn expression_to_duration(&self, expr: Expression) -> Option<Duration> {
        match expr {
            Expression::Duration(d) => Some(d),
            _ => None,
        }
    }
    fn convert_ast_value_to_types_value(&self, ast_value: Value) -> Value {
        ast_value
    }
    pub fn load_directory<P: AsRef<Path>>(&mut self, dir: P) -> Result<(), HlxError> {
        let dir_path = dir.as_ref();
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("hlx") {
                let config = self.load_file(&path)?;
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("default")
                    .to_string();
                self.configs.insert(name, config);
            }
        }
        Ok(())
    }
    pub fn get_config(&self, name: &str) -> Option<&HelixConfig> {
        self.configs.get(name)
    }
    pub fn set_context(&mut self, context: String) {
        self.current_context = Some(context);
    }
    pub fn merge_configs(&self, configs: Vec<&HelixConfig>) -> HelixConfig {
        let mut merged = HelixConfig::default();
        for config in configs {
            for (name, project) in &config.projects {
                merged.projects.insert(name.clone(), project.clone());
            }
            for (name, agent) in &config.agents {
                merged.agents.insert(name.clone(), agent.clone());
            }
            for (name, workflow) in &config.workflows {
                merged.workflows.insert(name.clone(), workflow.clone());
            }
            for (name, context) in &config.contexts {
                merged.contexts.insert(name.clone(), context.clone());
            }
            for (name, crew) in &config.crews {
                merged.crews.insert(name.clone(), crew.clone());
            }
            if config.memory.is_some() {
                merged.memory = config.memory.clone();
            }
            merged.plugins.extend(config.plugins.clone());
            for (section_name, section_data) in &config.sections {
                merged.sections.insert(section_name.clone(), section_data.clone());
            }
        }
        merged
    }
}
#[derive(Debug, Clone)]
pub enum DataFormat {
    Auto,
    JSON,
    CSV,
    Parquet,
}
#[derive(Debug, Clone)]
pub enum TrainingFormat {
    Preference { prompt_field: String, chosen_field: String, rejected_field: String },
    Completion {
        prompt_field: String,
        completion_field: String,
        label_field: Option<String>,
    },
}
#[derive(Debug)]
pub struct GenericJSONDataset {
    pub data: Vec<serde_json::Value>,
    pub format: DataFormat,
    pub schema: Option<serde_json::Value>,
}
impl GenericJSONDataset {
    pub fn new(
        _paths: &[std::path::PathBuf],
        _schema: Option<serde_json::Value>,
        format: DataFormat,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            data: Vec::new(),
            format,
            schema: _schema,
        })
    }
    pub fn detect_training_format(
        &self,
    ) -> Result<TrainingFormat, Box<dyn std::error::Error>> {
        if let Some(first) = self.data.first() {
            if let Some(obj) = first.as_object() {
                if obj.contains_key("chosen") && obj.contains_key("rejected") {
                    return Ok(TrainingFormat::Preference {
                        prompt_field: "prompt".to_string(),
                        chosen_field: "chosen".to_string(),
                        rejected_field: "rejected".to_string(),
                    });
                } else if obj.contains_key("completion") {
                    return Ok(TrainingFormat::Completion {
                        prompt_field: "prompt".to_string(),
                        completion_field: "completion".to_string(),
                        label_field: Some("label".to_string()),
                    });
                }
            }
        }
        Err("Could not detect training format".into())
    }
    pub fn to_training_dataset(
        &self,
    ) -> Result<TrainingDataset, Box<dyn std::error::Error>> {
        Ok(TrainingDataset {
            samples: self
                .data
                .iter()
                .enumerate()
                .map(|(i, _)| TrainingSample {
                    id: i as u64,
                    prompt: Some("test prompt".to_string()),
                    chosen: Some("test chosen".to_string()),
                    rejected: Some("test rejected".to_string()),
                    completion: Some("test completion".to_string()),
                    label: Some(1.0),
                })
                .collect(),
        })
    }
}
#[derive(Debug)]
pub struct TrainingSample {
    pub id: u64,
    pub prompt: Option<String>,
    pub chosen: Option<String>,
    pub rejected: Option<String>,
    pub completion: Option<String>,
    pub label: Option<f64>,
}
#[derive(Debug)]
pub struct TrainingDataset {
    pub samples: Vec<TrainingSample>,
}
impl TrainingDataset {
    pub fn to_algorithm_format(
        &self,
        _algorithm: &str,
    ) -> Result<AlgorithmFormat, Box<dyn std::error::Error>> {
        Ok(AlgorithmFormat {
            format_type: _algorithm.to_string(),
            data: serde_json::json!({ "samples" : self.samples.len() }),
        })
    }
}
#[derive(Debug)]
pub struct AlgorithmFormat {
    pub format_type: String,
    pub data: serde_json::Value,
}
#[derive(Debug)]
pub enum HlxError {
    IoError(std::io::Error),
    ParseError(String),
    ValidationError(String),
    ReferenceError(String),
}
impl From<std::io::Error> for HlxError {
    fn from(err: std::io::Error) -> Self {
        HlxError::IoError(err)
    }
}
impl From<String> for HlxError {
    fn from(err: String) -> Self {
        HlxError::ParseError(err)
    }
}
impl From<crate::atp::parser::ParseError> for HlxError {
    fn from(err: crate::atp::parser::ParseError) -> Self {
        HlxError::ParseError(err.to_string())
    }
}
impl std::fmt::Display for HlxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HlxError::IoError(e) => write!(f, "IO Error: {}", e),
            HlxError::ParseError(e) => write!(f, "Parse Error: {}", e),
            HlxError::ValidationError(e) => write!(f, "Validation Error: {}", e),
            HlxError::ReferenceError(e) => write!(f, "Reference Error: {}", e),
        }
    }
}
impl std::error::Error for HlxError {}
impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Number(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Object(obj) => {
                write!(f, "{{")?;
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Null => write!(f, "null"),
            Value::Duration(d) => write!(f, "{} {:?}", d.value, d.unit),
            Value::Reference(r) => write!(f, "@{}", r),
            Value::Identifier(i) => write!(f, "{}", i),
        }
    }
}
pub fn load_default_config() -> Result<HelixConfig, HlxError> {
    let mut loader = HelixLoader::new();
    use std::fs;
    let mut paths = Vec::new();
    let search_dirs = vec![".", "./config", "~/.maestro", "~/.helix"];
    for dir in &search_dirs {
        let dir_path = if dir.starts_with("~") {
            if let Some(home) = std::env::var_os("HOME") {
                let mut home_path = std::path::PathBuf::from(home);
                if dir.len() > 1 {
                    home_path.push(&dir[2..]);
                }
                home_path
            } else {
                std::path::PathBuf::from(dir)
            }
        } else {
            std::path::PathBuf::from(dir)
        };
        if let Ok(entries) = fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "hlxb" || ext == "hlx" {
                        if let Some(path_str) = path.to_str() {
                            paths.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    }
    for path in paths {
        if Path::new(&path).exists() {
            return loader.load_file(path);
        }
    }
    Err(
        HlxError::IoError(
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No .hlxbb configuration file found",
            ),
        ),
    )
}