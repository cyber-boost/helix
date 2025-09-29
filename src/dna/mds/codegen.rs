use std::collections::HashMap;
use std::io::{Write, Read};
use serde::{Serialize, Deserialize};
use crate::dna::atp::ast::*;
use crate::dna::atp::types::TimeUnit;
use crate::dna::atp::types::Duration;
pub use crate::dna::atp::types::SecretRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelixIR {
    pub version: u32,
    pub metadata: Metadata,
    pub symbol_table: SymbolTable,
    pub instructions: Vec<Instruction>,
    pub string_pool: StringPool,
    pub constants: ConstantPool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub source_file: Option<String>,
    pub compile_time: u64,
    pub compiler_version: String,
    pub checksum: Option<u64>,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            source_file: None,
            compile_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            compiler_version: "1.0.0".to_string(),
            checksum: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTable {
    pub agents: HashMap<u32, AgentSymbol>,
    pub workflows: HashMap<u32, WorkflowSymbol>,
    pub contexts: HashMap<u32, ContextSymbol>,
    pub crews: HashMap<u32, CrewSymbol>,
    pub next_id: u32,
}
impl Default for SymbolTable {
    fn default() -> Self {
        Self {
            agents: HashMap::new(),
            workflows: HashMap::new(),
            contexts: HashMap::new(),
            crews: HashMap::new(),
            next_id: 1,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSymbol {
    pub id: u32,
    pub name_idx: u32,
    pub model_idx: u32,
    pub role_idx: u32,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub capabilities: Vec<u32>,
    pub backstory_idx: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSymbol {
    pub id: u32,
    pub name_idx: u32,
    pub trigger_type: TriggerType,
    pub steps: Vec<u32>,
    pub pipeline: Option<Vec<u32>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSymbol {
    pub id: u32,
    pub name_idx: u32,
    pub environment_idx: u32,
    pub debug: bool,
    pub max_tokens: Option<u64>,
    pub secrets: HashMap<u32, SecretType>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewSymbol {
    pub id: u32,
    pub name_idx: u32,
    pub agent_ids: Vec<u32>,
    pub process_type: ProcessTypeIR,
    pub manager_id: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    Manual,
    Schedule(u32),
    Webhook(u32),
    Event(u32),
    FileWatch(u32),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessTypeIR {
    Sequential,
    Hierarchical,
    Parallel,
    Consensus,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretType {
    Environment(u32),
    Vault(u32),
    File(u32),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    DeclareAgent(u32),
    DeclareWorkflow(u32),
    DeclareContext(u32),
    DeclareCrew(u32),
    SetProperty { target: u32, key: u32, value: ConstantValue },
    SetCapability { agent: u32, capability: u32 },
    SetSecret { context: u32, key: u32, secret: SecretType },
    DefineStep { workflow: u32, step: StepDefinition },
    DefinePipeline { workflow: u32, nodes: Vec<PipelineNodeIR> },
    ResolveReference { ref_type: ReferenceType, index: u32 },
    SetMetadata { key: u32, value: u32 },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDefinition {
    pub id: u32,
    pub name_idx: u32,
    pub agent_id: Option<u32>,
    pub crew_ids: Option<Vec<u32>>,
    pub task_idx: Option<u32>,
    pub timeout: Option<DurationIR>,
    pub parallel: bool,
    pub depends_on: Vec<u32>,
    pub retry: Option<RetryPolicy>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub delay: DurationIR,
    pub backoff: BackoffStrategyIR,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategyIR {
    Fixed,
    Linear,
    Exponential,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DurationIR {
    pub value: u64,
    pub unit: TimeUnitIR,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimeUnitIR {
    Seconds,
    Minutes,
    Hours,
    Days,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineNodeIR {
    Step(u32),
    Parallel(Vec<PipelineNodeIR>),
    Conditional {
        condition: u32,
        then_branch: Box<PipelineNodeIR>,
        else_branch: Option<Box<PipelineNodeIR>>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Environment,
    Memory,
    Variable,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConstantValue {
    String(u32),
    Number(f64),
    Bool(bool),
    Duration(DurationIR),
    Null,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringPool {
    pub strings: Vec<String>,
    pub index: HashMap<String, u32>,
}
impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}
impl StringPool {
    pub fn new() -> Self {
        StringPool {
            strings: Vec::new(),
            index: HashMap::new(),
        }
    }
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.index.get(s) {
            idx
        } else {
            let idx = self.strings.len() as u32;
            self.strings.push(s.to_string());
            self.index.insert(s.to_string(), idx);
            idx
        }
    }
    pub fn get(&self, idx: u32) -> Option<&String> {
        self.strings.get(idx as usize)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantPool {
    constants: Vec<ConstantValue>,
}
impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
    }
}
impl ConstantPool {
    pub fn new() -> Self {
        ConstantPool {
            constants: Vec::new(),
        }
    }
    pub fn add(&mut self, value: ConstantValue) -> u32 {
        let idx = self.constants.len() as u32;
        self.constants.push(value);
        idx
    }
    pub fn get(&self, idx: u32) -> Option<&ConstantValue> {
        self.constants.get(idx as usize)
    }
}
pub struct CodeGenerator {
    ir: HelixIR,
    current_id: u32,
}
impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            ir: HelixIR {
                version: 1,
                metadata: Metadata {
                    source_file: None,
                    compile_time: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    compiler_version: "1.0.0".to_string(),
                    checksum: None,
                },
                symbol_table: SymbolTable {
                    agents: HashMap::new(),
                    workflows: HashMap::new(),
                    contexts: HashMap::new(),
                    crews: HashMap::new(),
                    next_id: 1,
                },
                instructions: Vec::new(),
                string_pool: StringPool::new(),
                constants: ConstantPool::new(),
            },
            current_id: 1,
        }
    }
    fn next_id(&mut self) -> u32 {
        let id = self.current_id;
        self.current_id += 1;
        id
    }
    pub fn generate(&mut self, ast: &HelixAst) -> HelixIR {
        for decl in &ast.declarations {
            self.generate_declaration(decl);
        }
        self.optimize();
        self.ir.metadata.checksum = Some(self.calculate_checksum());
        self.ir.clone()
    }
    fn generate_declaration(&mut self, decl: &Declaration) {
        match decl {
            Declaration::Agent(agent) => self.generate_agent(agent),
            Declaration::Workflow(workflow) => self.generate_workflow(workflow),
            Declaration::Context(context) => self.generate_context(context),
            Declaration::Crew(crew) => self.generate_crew(crew),
            _ => {}
        }
    }
    fn generate_agent(&mut self, agent: &AgentDecl) {
        let id = self.next_id();
        let name_idx = self.ir.string_pool.intern(&agent.name);
        let model_idx = agent
            .properties
            .get("model")
            .and_then(|e| e.as_string())
            .map(|s| self.ir.string_pool.intern(&s))
            .unwrap_or_else(|| self.ir.string_pool.intern("gpt-4"));
        let role_idx = agent
            .properties
            .get("role")
            .and_then(|e| e.as_string())
            .map(|s| self.ir.string_pool.intern(&s))
            .unwrap_or_else(|| self.ir.string_pool.intern("Assistant"));
        let temperature = agent
            .properties
            .get("temperature")
            .and_then(|e| e.as_number())
            .map(|n| n as f32);
        let max_tokens = agent
            .properties
            .get("max_tokens")
            .and_then(|e| e.as_number())
            .map(|n| n as u32);
        let capabilities = agent
            .capabilities
            .as_ref()
            .map(|caps| caps.iter().map(|c| self.ir.string_pool.intern(c)).collect())
            .unwrap_or_default();
        let backstory_idx = agent
            .backstory
            .as_ref()
            .map(|b| {
                let backstory_text = b.lines.join("\n");
                self.ir.string_pool.intern(&backstory_text)
            });
        let symbol = AgentSymbol {
            id,
            name_idx,
            model_idx,
            role_idx,
            temperature,
            max_tokens,
            capabilities,
            backstory_idx,
        };
        self.ir.symbol_table.agents.insert(id, symbol);
        self.ir.instructions.push(Instruction::DeclareAgent(id));
    }
    fn generate_workflow(&mut self, workflow: &WorkflowDecl) {
        let id = self.next_id();
        let name_idx = self.ir.string_pool.intern(&workflow.name);
        let trigger_type = if let Some(trigger) = &workflow.trigger {
            self.parse_trigger(trigger)
        } else {
            TriggerType::Manual
        };
        let mut step_ids = Vec::new();
        for step in &workflow.steps {
            let step_def = self.generate_step(step, id);
            step_ids.push(step_def.id);
            self.ir
                .instructions
                .push(Instruction::DefineStep {
                    workflow: id,
                    step: step_def,
                });
        }
        let pipeline = workflow
            .pipeline
            .as_ref()
            .map(|p| {
                p.flow
                    .iter()
                    .filter_map(|node| {
                        if let PipelineNode::Step(name) = node {
                            workflow
                                .steps
                                .iter()
                                .find(|s| s.name == *name)
                                .and_then(|_s| step_ids.iter().find(|&&_sid| { true }))
                                .copied()
                        } else {
                            None
                        }
                    })
                    .collect()
            });
        let symbol = WorkflowSymbol {
            id,
            name_idx,
            trigger_type,
            steps: step_ids,
            pipeline,
        };
        self.ir.symbol_table.workflows.insert(id, symbol);
        self.ir.instructions.push(Instruction::DeclareWorkflow(id));
    }
    fn generate_step(&mut self, step: &StepDecl, _workflow_id: u32) -> StepDefinition {
        let id = self.next_id();
        let name_idx = self.ir.string_pool.intern(&step.name);
        let agent_id = step
            .agent
            .as_ref()
            .and_then(|name| {
                self.ir
                    .symbol_table
                    .agents
                    .values()
                    .find(|a| self.ir.string_pool.get(a.name_idx) == Some(name))
                    .map(|a| a.id)
            });
        let task_idx = step.task.as_ref().map(|t| self.ir.string_pool.intern(t));
        let timeout = step
            .properties
            .get("timeout")
            .and_then(|e| self.expression_to_duration(e));
        let parallel = step
            .properties
            .get("parallel")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);
        let depends_on = step
            .properties
            .get("depends_on")
            .and_then(|e| e.as_array())
            .map(|deps| {
                deps.iter()
                    .filter_map(|d| { d.as_string().and_then(|_name| { Some(0) }) })
                    .collect()
            })
            .unwrap_or_default();
        let retry = step
            .properties
            .get("retry")
            .and_then(|e| e.as_object())
            .and_then(|obj| self.parse_retry_config(obj));
        StepDefinition {
            id,
            name_idx,
            agent_id,
            crew_ids: step.crew.as_ref().map(|_| Vec::new()),
            task_idx,
            timeout,
            parallel,
            depends_on,
            retry,
        }
    }
    fn generate_context(&mut self, context: &ContextDecl) {
        let id = self.next_id();
        let name_idx = self.ir.string_pool.intern(&context.name);
        let environment_idx = self.ir.string_pool.intern(&context.environment);
        let debug = context
            .properties
            .get("debug")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);
        let max_tokens = context
            .properties
            .get("max_tokens")
            .and_then(|e| e.as_number())
            .map(|n| n as u64);
        let secrets = context
            .secrets
            .as_ref()
            .map(|s| {
                s.iter()
                    .map(|(key, secret_ref)| {
                        let key_idx = self.ir.string_pool.intern(key);
                        let secret_type = match secret_ref {
                            SecretRef::Environment(var) => {
                                SecretType::Environment(self.ir.string_pool.intern(var))
                            }
                            SecretRef::Vault(path) => {
                                SecretType::Vault(self.ir.string_pool.intern(path))
                            }
                            SecretRef::File(path) => {
                                SecretType::File(self.ir.string_pool.intern(path))
                            }
                        };
                        (key_idx, secret_type)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let symbol = ContextSymbol {
            id,
            name_idx,
            environment_idx,
            debug,
            max_tokens,
            secrets,
        };
        self.ir.symbol_table.contexts.insert(id, symbol);
        self.ir.instructions.push(Instruction::DeclareContext(id));
    }
    fn generate_crew(&mut self, crew: &CrewDecl) {
        let id = self.next_id();
        let name_idx = self.ir.string_pool.intern(&crew.name);
        let agent_ids = crew
            .agents
            .iter()
            .filter_map(|name| {
                self.ir
                    .symbol_table
                    .agents
                    .values()
                    .find(|a| self.ir.string_pool.get(a.name_idx) == Some(name))
                    .map(|a| a.id)
            })
            .collect();
        let process_type = crew
            .process_type
            .as_ref()
            .and_then(|p| match p.as_str() {
                "sequential" => Some(ProcessTypeIR::Sequential),
                "hierarchical" => Some(ProcessTypeIR::Hierarchical),
                "parallel" => Some(ProcessTypeIR::Parallel),
                "consensus" => Some(ProcessTypeIR::Consensus),
                _ => None,
            })
            .unwrap_or(ProcessTypeIR::Sequential);
        let manager_id = crew
            .properties
            .get("manager")
            .and_then(|e| e.as_string())
            .and_then(|name| {
                self.ir
                    .symbol_table
                    .agents
                    .values()
                    .find(|a| self.ir.string_pool.get(a.name_idx) == Some(&name))
                    .map(|a| a.id)
            });
        let symbol = CrewSymbol {
            id,
            name_idx,
            agent_ids,
            process_type,
            manager_id,
        };
        self.ir.symbol_table.crews.insert(id, symbol);
        self.ir.instructions.push(Instruction::DeclareCrew(id));
    }
    fn parse_trigger(&mut self, trigger: &Expression) -> TriggerType {
        match trigger {
            Expression::String(s) | Expression::Identifier(s) => {
                if s == "manual" {
                    TriggerType::Manual
                } else if s.starts_with("schedule:") {
                    let cron = s.trim_start_matches("schedule:");
                    TriggerType::Schedule(self.ir.string_pool.intern(cron))
                } else if s.starts_with("webhook:") {
                    let url = s.trim_start_matches("webhook:");
                    TriggerType::Webhook(self.ir.string_pool.intern(url))
                } else if s.starts_with("event:") {
                    let event = s.trim_start_matches("event:");
                    TriggerType::Event(self.ir.string_pool.intern(event))
                } else if s.starts_with("file:") {
                    let pattern = s.trim_start_matches("file:");
                    TriggerType::FileWatch(self.ir.string_pool.intern(pattern))
                } else {
                    TriggerType::Manual
                }
            }
            Expression::Object(map) => {
                if let Some(type_expr) = map.get("type") {
                    self.parse_trigger(type_expr)
                } else {
                    TriggerType::Manual
                }
            }
            _ => TriggerType::Manual,
        }
    }
    fn expression_to_duration(&mut self, expr: &Expression) -> Option<DurationIR> {
        match expr {
            Expression::Duration(d) => {
                Some(DurationIR {
                    value: d.value,
                    unit: match d.unit {
                        TimeUnit::Seconds => TimeUnitIR::Seconds,
                        TimeUnit::Minutes => TimeUnitIR::Minutes,
                        TimeUnit::Hours => TimeUnitIR::Hours,
                        TimeUnit::Days => TimeUnitIR::Days,
                    },
                })
            }
            _ => None,
        }
    }
    fn parse_retry_config(
        &mut self,
        obj: &HashMap<String, Expression>,
    ) -> Option<RetryPolicy> {
        let max_attempts = obj
            .get("max_attempts")
            .and_then(|e| e.as_number())
            .map(|n| n as u32)?;
        let delay = obj.get("delay").and_then(|e| self.expression_to_duration(e))?;
        let backoff = obj
            .get("backoff")
            .and_then(|e| e.as_string())
            .and_then(|s| match s.as_str() {
                "fixed" => Some(BackoffStrategyIR::Fixed),
                "linear" => Some(BackoffStrategyIR::Linear),
                "exponential" => Some(BackoffStrategyIR::Exponential),
                _ => None,
            })
            .unwrap_or(BackoffStrategyIR::Fixed);
        Some(RetryPolicy {
            max_attempts,
            delay,
            backoff,
        })
    }
    fn optimize(&mut self) {
        self.constant_folding();
        self.dead_code_elimination();
        self.string_deduplication();
    }
    fn constant_folding(&mut self) {}
    fn dead_code_elimination(&mut self) {}
    fn string_deduplication(&mut self) {}
    fn calculate_checksum(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.ir.version.hash(&mut hasher);
        self.ir.instructions.len().hash(&mut hasher);
        hasher.finish()
    }
}
#[allow(dead_code)]
pub struct BinarySerializer;
#[allow(dead_code)]
impl BinarySerializer {
    pub fn serialize(ir: &HelixIR) -> Result<Vec<u8>, std::io::Error> {
        #[cfg(feature = "compiler")]
        {
            bincode::serialize(ir)
                .map_err(|e| { std::io::Error::new(std::io::ErrorKind::Other, e) })
        }
        #[cfg(not(feature = "compiler"))]
        {
            Err(
                std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Binary serialization requires the 'compiler' feature",
                ),
            )
        }
    }
    pub fn deserialize(data: &[u8]) -> Result<HelixIR, std::io::Error> {
        #[cfg(feature = "compiler")]
        {
            bincode::deserialize(data)
                .map_err(|e| { std::io::Error::new(std::io::ErrorKind::Other, e) })
        }
        #[cfg(not(feature = "compiler"))]
        {
            Err(
                std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Binary deserialization requires the 'compiler' feature",
                ),
            )
        }
    }
    pub fn write_to_file(ir: &HelixIR, path: &str) -> Result<(), std::io::Error> {
        let data = Self::serialize(ir)?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(&data)?;
        Ok(())
    }
    pub fn read_from_file(path: &str) -> Result<HelixIR, std::io::Error> {
        let mut file = std::fs::File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Self::deserialize(&data)
    }
}
#[allow(dead_code)]
pub struct VersionChecker;
#[allow(dead_code)]
impl VersionChecker {
    const CURRENT_VERSION: u32 = 1;
    const MIN_COMPATIBLE_VERSION: u32 = 1;
    pub fn is_compatible(ir: &HelixIR) -> bool {
        ir.version >= Self::MIN_COMPATIBLE_VERSION && ir.version <= Self::CURRENT_VERSION
    }
    pub fn migrate(ir: &mut HelixIR) -> Result<(), String> {
        if ir.version < Self::MIN_COMPATIBLE_VERSION {
            return Err(
                format!(
                    "IR version {} is too old. Minimum supported version is {}", ir
                    .version, Self::MIN_COMPATIBLE_VERSION
                ),
            );
        }
        if ir.version > Self::CURRENT_VERSION {
            return Err(
                format!(
                    "IR version {} is newer than current version {}", ir.version,
                    Self::CURRENT_VERSION
                ),
            );
        }
        while ir.version < Self::CURRENT_VERSION {
            match ir.version {
                _ => ir.version += 1,
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_string_pool() {
        let mut pool = StringPool::new();
        let idx1 = pool.intern("hello");
        let idx2 = pool.intern("world");
        let idx3 = pool.intern("hello");
        assert_eq!(idx1, idx3);
        assert_ne!(idx1, idx2);
        assert_eq!(pool.get(idx1), Some(& "hello".to_string()));
        assert_eq!(pool.get(idx2), Some(& "world".to_string()));
    }
    #[test]
    fn test_constant_pool() {
        let mut pool = ConstantPool::new();
        let idx1 = pool.add(ConstantValue::Number(42.0));
        let idx2 = pool.add(ConstantValue::Bool(true));
        assert_eq!(pool.get(idx1), Some(& ConstantValue::Number(42.0)));
        assert_eq!(pool.get(idx2), Some(& ConstantValue::Bool(true)));
    }
    #[test]
    fn test_version_compatibility() {
        let mut ir = HelixIR {
            version: 1,
            metadata: Metadata {
                source_file: None,
                compile_time: 0,
                compiler_version: "1.0.0".to_string(),
                checksum: None,
            },
            symbol_table: SymbolTable {
                agents: HashMap::new(),
                workflows: HashMap::new(),
                contexts: HashMap::new(),
                crews: HashMap::new(),
                next_id: 1,
            },
            instructions: Vec::new(),
            string_pool: StringPool::new(),
            constants: ConstantPool::new(),
        };
        assert!(VersionChecker::is_compatible(& ir));
        assert!(VersionChecker::migrate(& mut ir).is_ok());
    }
}