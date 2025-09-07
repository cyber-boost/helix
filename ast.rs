use std::collections::HashMap;
use crate::types::{Value, Duration, TimeUnit};
#[derive(Debug, Clone)]
pub struct HelixAst {
    pub declarations: Vec<Declaration>,
}
#[derive(Debug, Clone)]
pub enum Declaration {
    Project(ProjectDecl),
    Agent(AgentDecl),
    Workflow(WorkflowDecl),
    Memory(MemoryDecl),
    Context(ContextDecl),
    Crew(CrewDecl),
    Pipeline(PipelineDecl),
    Plugin(PluginDecl),
    Database(DatabaseDecl),
    Load(LoadDecl),
}
#[derive(Debug, Clone)]
pub struct ProjectDecl {
    pub name: String,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct AgentDecl {
    pub name: String,
    pub properties: HashMap<String, Expression>,
    pub capabilities: Option<Vec<String>>,
    pub backstory: Option<BackstoryBlock>,
    pub tools: Option<Vec<String>>,
}
#[derive(Debug, Clone)]
pub struct WorkflowDecl {
    pub name: String,
    pub trigger: Option<Expression>,
    pub steps: Vec<StepDecl>,
    pub pipeline: Option<PipelineDecl>,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct StepDecl {
    pub name: String,
    pub agent: Option<String>,
    pub crew: Option<Vec<String>>,
    pub task: Option<String>,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct PipelineDecl {
    pub flow: Vec<PipelineNode>,
}
#[derive(Debug, Clone)]
pub enum PipelineNode {
    Step(String),
    Parallel(Vec<PipelineNode>),
    Conditional {
        condition: Expression,
        then_branch: Box<PipelineNode>,
        else_branch: Option<Box<PipelineNode>>,
    },
}
#[derive(Debug, Clone)]
pub struct MemoryDecl {
    pub provider: String,
    pub connection: String,
    pub embeddings: Option<EmbeddingsDecl>,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct EmbeddingsDecl {
    pub model: String,
    pub dimensions: u32,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct ContextDecl {
    pub name: String,
    pub environment: String,
    pub secrets: Option<HashMap<String, SecretRef>>,
    pub variables: Option<HashMap<String, Expression>>,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct CrewDecl {
    pub name: String,
    pub agents: Vec<String>,
    pub process_type: Option<String>,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct PluginDecl {
    pub name: String,
    pub source: String,
    pub version: Option<String>,
    pub config: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct DatabaseDecl {
    pub name: String,
    pub path: Option<String>,
    pub shards: Option<i64>,
    pub compression: Option<bool>,
    pub cache_size: Option<i64>,
    pub vector_index: Option<VectorIndexConfig>,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct VectorIndexConfig {
    pub index_type: String,
    pub dimensions: i64,
    pub m: Option<i64>,
    pub ef_construction: Option<i64>,
    pub distance_metric: Option<String>,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct LoadDecl {
    pub file_name: String,
    pub properties: HashMap<String, Expression>,
}
#[derive(Debug, Clone)]
pub struct BackstoryBlock {
    pub lines: Vec<String>,
}
#[derive(Debug, Clone)]
pub enum SecretRef {
    Environment(String),
    Vault(String),
    File(String),
}
#[derive(Debug, Clone)]
pub enum Expression {
    String(String),
    Number(f64),
    Bool(bool),
    Duration(Duration),
    Array(Vec<Expression>),
    Object(HashMap<String, Expression>),
    Variable(String),
    Reference(String),
    IndexedReference(String, String),
    Identifier(String),
    Pipeline(Vec<String>),
    Block(Vec<Statement>),
    TextBlock(Vec<String>),
}
#[derive(Debug, Clone)]
pub enum Statement {
    Assignment(String, Expression),
    Declaration(Declaration),
    Expression(Expression),
}
impl HelixAst {
    pub fn new() -> Self {
        HelixAst {
            declarations: Vec::new(),
        }
    }
    pub fn add_declaration(&mut self, decl: Declaration) {
        self.declarations.push(decl);
    }
    pub fn get_projects(&self) -> Vec<&ProjectDecl> {
        self.declarations
            .iter()
            .filter_map(|d| {
                if let Declaration::Project(p) = d { Some(p) } else { None }
            })
            .collect()
    }
    pub fn get_agents(&self) -> Vec<&AgentDecl> {
        self.declarations
            .iter()
            .filter_map(|d| {
                if let Declaration::Agent(a) = d { Some(a) } else { None }
            })
            .collect()
    }
    pub fn get_workflows(&self) -> Vec<&WorkflowDecl> {
        self.declarations
            .iter()
            .filter_map(|d| {
                if let Declaration::Workflow(w) = d { Some(w) } else { None }
            })
            .collect()
    }
    pub fn get_contexts(&self) -> Vec<&ContextDecl> {
        self.declarations
            .iter()
            .filter_map(|d| {
                if let Declaration::Context(c) = d { Some(c) } else { None }
            })
            .collect()
    }
}
#[allow(dead_code)]
pub trait AstVisitor {
    type Result;
    fn visit_ast(&mut self, ast: &HelixAst) -> Self::Result;
    fn visit_declaration(&mut self, decl: &Declaration) -> Self::Result;
    fn visit_project(&mut self, project: &ProjectDecl) -> Self::Result;
    fn visit_agent(&mut self, agent: &AgentDecl) -> Self::Result;
    fn visit_workflow(&mut self, workflow: &WorkflowDecl) -> Self::Result;
    fn visit_memory(&mut self, memory: &MemoryDecl) -> Self::Result;
    fn visit_context(&mut self, context: &ContextDecl) -> Self::Result;
    fn visit_crew(&mut self, crew: &CrewDecl) -> Self::Result;
    fn visit_expression(&mut self, expr: &Expression) -> Self::Result;
}
pub struct AstPrettyPrinter {
    indent: usize,
    indent_str: String,
}
impl AstPrettyPrinter {
    pub fn new() -> Self {
        AstPrettyPrinter {
            indent: 0,
            indent_str: "  ".to_string(),
        }
    }
    fn write_indent(&self) -> String {
        self.indent_str.repeat(self.indent)
    }
    pub fn print(&mut self, ast: &HelixAst) -> String {
        let mut result = String::new();
        result.push_str("# HELIX Language AST\n\n");
        for decl in &ast.declarations {
            result.push_str(&self.print_declaration(decl));
            result.push_str("\n");
        }
        result
    }
    fn print_declaration(&mut self, decl: &Declaration) -> String {
        match decl {
            Declaration::Project(p) => self.print_project(p),
            Declaration::Agent(a) => self.print_agent(a),
            Declaration::Workflow(w) => self.print_workflow(w),
            Declaration::Memory(m) => self.print_memory(m),
            Declaration::Context(c) => self.print_context(c),
            Declaration::Crew(cr) => self.print_crew(cr),
            Declaration::Pipeline(p) => self.print_pipeline(p),
            Declaration::Plugin(p) => self.print_plugin(p),
            Declaration::Database(d) => self.print_database(d),
            Declaration::Load(l) => self.print_load(l),
        }
    }
    fn print_project(&mut self, project: &ProjectDecl) -> String {
        let mut result = format!(
            "{}project \"{}\" {{\n", self.write_indent(), project.name
        );
        self.indent += 1;
        for (key, value) in &project.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_agent(&mut self, agent: &AgentDecl) -> String {
        let mut result = format!("{}agent \"{}\" {{\n", self.write_indent(), agent.name);
        self.indent += 1;
        for (key, value) in &agent.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        if let Some(capabilities) = &agent.capabilities {
            result.push_str(&format!("{}capabilities [\n", self.write_indent()));
            self.indent += 1;
            for cap in capabilities {
                result.push_str(&format!("{}\"{}\"\n", self.write_indent(), cap));
            }
            self.indent -= 1;
            result.push_str(&format!("{}]\n", self.write_indent()));
        }
        if let Some(backstory) = &agent.backstory {
            result.push_str(&format!("{}backstory {{\n", self.write_indent()));
            self.indent += 1;
            for line in &backstory.lines {
                result.push_str(&format!("{}{}\n", self.write_indent(), line));
            }
            self.indent -= 1;
            result.push_str(&format!("{}}}\n", self.write_indent()));
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_workflow(&mut self, workflow: &WorkflowDecl) -> String {
        let mut result = format!(
            "{}workflow \"{}\" {{\n", self.write_indent(), workflow.name
        );
        self.indent += 1;
        if let Some(trigger) = &workflow.trigger {
            result
                .push_str(
                    &format!(
                        "{}trigger = {}\n", self.write_indent(), self
                        .print_expression(trigger)
                    ),
                );
        }
        for step in &workflow.steps {
            result.push_str(&self.print_step(step));
        }
        if let Some(pipeline) = &workflow.pipeline {
            result.push_str(&self.print_pipeline(pipeline));
        }
        for (key, value) in &workflow.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_step(&mut self, step: &StepDecl) -> String {
        let mut result = format!("{}step \"{}\" {{\n", self.write_indent(), step.name);
        self.indent += 1;
        if let Some(agent) = &step.agent {
            result.push_str(&format!("{}agent = \"{}\"\n", self.write_indent(), agent));
        }
        if let Some(crew) = &step.crew {
            result.push_str(&format!("{}crew = [", self.write_indent()));
            result
                .push_str(
                    &crew
                        .iter()
                        .map(|c| format!("\"{}\"", c))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            result.push_str("]\n");
        }
        if let Some(task) = &step.task {
            result.push_str(&format!("{}task = \"{}\"\n", self.write_indent(), task));
        }
        for (key, value) in &step.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_memory(&mut self, memory: &MemoryDecl) -> String {
        let mut result = format!("{}memory {{\n", self.write_indent());
        self.indent += 1;
        result
            .push_str(
                &format!("{}provider = \"{}\"\n", self.write_indent(), memory.provider),
            );
        result
            .push_str(
                &format!(
                    "{}connection = \"{}\"\n", self.write_indent(), memory.connection
                ),
            );
        if let Some(embeddings) = &memory.embeddings {
            result.push_str(&self.print_embeddings(embeddings));
        }
        for (key, value) in &memory.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_embeddings(&mut self, embeddings: &EmbeddingsDecl) -> String {
        let mut result = format!("{}embeddings {{\n", self.write_indent());
        self.indent += 1;
        result
            .push_str(
                &format!("{}model = \"{}\"\n", self.write_indent(), embeddings.model),
            );
        result
            .push_str(
                &format!(
                    "{}dimensions = {}\n", self.write_indent(), embeddings.dimensions
                ),
            );
        for (key, value) in &embeddings.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_context(&mut self, context: &ContextDecl) -> String {
        let mut result = format!(
            "{}context \"{}\" {{\n", self.write_indent(), context.name
        );
        self.indent += 1;
        result
            .push_str(
                &format!(
                    "{}environment = \"{}\"\n", self.write_indent(), context.environment
                ),
            );
        if let Some(secrets) = &context.secrets {
            result.push_str(&format!("{}secrets {{\n", self.write_indent()));
            self.indent += 1;
            for (key, secret_ref) in secrets {
                result
                    .push_str(
                        &format!(
                            "{}{} = {}\n", self.write_indent(), key, self
                            .print_secret_ref(secret_ref)
                        ),
                    );
            }
            self.indent -= 1;
            result.push_str(&format!("{}}}\n", self.write_indent()));
        }
        if let Some(variables) = &context.variables {
            result.push_str(&format!("{}variables {{\n", self.write_indent()));
            self.indent += 1;
            for (key, value) in variables {
                result
                    .push_str(
                        &format!(
                            "{}{} = {}\n", self.write_indent(), key, self
                            .print_expression(value)
                        ),
                    );
            }
            self.indent -= 1;
            result.push_str(&format!("{}}}\n", self.write_indent()));
        }
        for (key, value) in &context.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_crew(&mut self, crew: &CrewDecl) -> String {
        let mut result = format!("{}crew \"{}\" {{\n", self.write_indent(), crew.name);
        self.indent += 1;
        result.push_str(&format!("{}agents [\n", self.write_indent()));
        self.indent += 1;
        for agent in &crew.agents {
            result.push_str(&format!("{}\"{}\"\n", self.write_indent(), agent));
        }
        self.indent -= 1;
        result.push_str(&format!("{}]\n", self.write_indent()));
        if let Some(process_type) = &crew.process_type {
            result
                .push_str(
                    &format!("{}process = \"{}\"\n", self.write_indent(), process_type),
                );
        }
        for (key, value) in &crew.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_plugin(&mut self, plugin: &PluginDecl) -> String {
        let mut result = format!(
            "{}plugin \"{}\" {{\n", self.write_indent(), plugin.name
        );
        self.indent += 1;
        result
            .push_str(
                &format!("{}source = \"{}\"\n", self.write_indent(), plugin.source),
            );
        if let Some(version) = &plugin.version {
            result
                .push_str(
                    &format!("{}version = \"{}\"\n", self.write_indent(), version),
                );
        }
        for (key, value) in &plugin.config {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_database(&mut self, database: &DatabaseDecl) -> String {
        let mut result = format!(
            "{}database \"{}\" {{\n", self.write_indent(), database.name
        );
        self.indent += 1;
        if let Some(path) = &database.path {
            result.push_str(&format!("{}path = \"{}\"\n", self.write_indent(), path));
        }
        if let Some(shards) = database.shards {
            result.push_str(&format!("{}shards = {}\n", self.write_indent(), shards));
        }
        if let Some(compression) = database.compression {
            result
                .push_str(
                    &format!("{}compression = {}\n", self.write_indent(), compression),
                );
        }
        if let Some(cache_size) = database.cache_size {
            result
                .push_str(
                    &format!("{}cache_size = {}\n", self.write_indent(), cache_size),
                );
        }
        if let Some(vector_index) = &database.vector_index {
            result.push_str(&format!("{}vector_index {{\n", self.write_indent()));
            self.indent += 1;
            result
                .push_str(
                    &format!(
                        "{}index_type = \"{}\"\n", self.write_indent(), vector_index
                        .index_type
                    ),
                );
            result
                .push_str(
                    &format!(
                        "{}dimensions = {}\n", self.write_indent(), vector_index
                        .dimensions
                    ),
                );
            if let Some(m) = vector_index.m {
                result.push_str(&format!("{}m = {}\n", self.write_indent(), m));
            }
            if let Some(ef_construction) = vector_index.ef_construction {
                result
                    .push_str(
                        &format!(
                            "{}ef_construction = {}\n", self.write_indent(),
                            ef_construction
                        ),
                    );
            }
            if let Some(distance_metric) = &vector_index.distance_metric {
                result
                    .push_str(
                        &format!(
                            "{}distance_metric = \"{}\"\n", self.write_indent(),
                            distance_metric
                        ),
                    );
            }
            self.indent -= 1;
            result.push_str(&format!("{}}}\n", self.write_indent()));
        }
        for (key, value) in &database.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_load(&mut self, load: &LoadDecl) -> String {
        let mut result = format!(
            "{}load \"{}\" {{\n", self.write_indent(), load.file_name
        );
        self.indent += 1;
        for (key, value) in &load.properties {
            result
                .push_str(
                    &format!(
                        "{}{} = {}\n", self.write_indent(), key, self
                        .print_expression(value)
                    ),
                );
        }
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_pipeline(&mut self, pipeline: &PipelineDecl) -> String {
        let mut result = format!("{}pipeline {{\n", self.write_indent());
        self.indent += 1;
        let flow_str = pipeline
            .flow
            .iter()
            .map(|node| match node {
                PipelineNode::Step(s) => s.clone(),
                _ => "...".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" -> ");
        result.push_str(&format!("{}{}\n", self.write_indent(), flow_str));
        self.indent -= 1;
        result.push_str(&format!("{}}}\n", self.write_indent()));
        result
    }
    fn print_secret_ref(&mut self, secret_ref: &SecretRef) -> String {
        match secret_ref {
            SecretRef::Environment(var) => format!("${}", var),
            SecretRef::Vault(path) => format!("vault:\"{}\"", path),
            SecretRef::File(path) => format!("file:\"{}\"", path),
        }
    }
    fn print_expression(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::String(s) => format!("\"{}\"", s),
            Expression::Number(n) => format!("{}", n),
            Expression::Bool(b) => format!("{}", b),
            Expression::Duration(d) => {
                format!(
                    "{}{}", d.value, match d.unit { TimeUnit::Seconds => "s",
                    TimeUnit::Minutes => "m", TimeUnit::Hours => "h", TimeUnit::Days =>
                    "d", }
                )
            }
            Expression::Variable(v) => format!("${}", v),
            Expression::Reference(r) => format!("@{}", r),
            Expression::IndexedReference(file, key) => format!("@{}[{}]", file, key),
            Expression::Identifier(i) => i.clone(),
            Expression::Pipeline(stages) => stages.join(" -> "),
            Expression::Array(items) => {
                format!(
                    "[{}]", items.iter().map(| i | self.print_expression(i)).collect::<
                    Vec < _ >> ().join(", ")
                )
            }
            Expression::Object(map) => {
                let items = map
                    .iter()
                    .map(|(k, v)| format!("{} = {}", k, self.print_expression(v)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", items)
            }
            _ => "...".to_string(),
        }
    }
}
impl Expression {
    pub fn as_string(&self) -> Option<String> {
        match self {
            Expression::String(s) => Some(s.clone()),
            Expression::Identifier(s) => Some(s.clone()),
            _ => None,
        }
    }
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Expression::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Expression::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<Vec<Expression>> {
        match self {
            Expression::Array(arr) => Some(arr.clone()),
            _ => None,
        }
    }
    pub fn as_object(&self) -> Option<&HashMap<String, Expression>> {
        match self {
            Expression::Object(map) => Some(map),
            _ => None,
        }
    }
    pub fn to_value(&self) -> Value {
        match self {
            Expression::String(s) => Value::String(s.clone()),
            Expression::Number(n) => Value::Number(*n),
            Expression::Bool(b) => Value::Bool(*b),
            Expression::Duration(d) => Value::Duration(d.clone()),
            Expression::Array(arr) => {
                Value::Array(arr.iter().map(|e| e.to_value()).collect())
            }
            Expression::Object(map) => {
                Value::Object(
                    map.iter().map(|(k, v)| (k.clone(), v.to_value())).collect(),
                )
            }
            Expression::Variable(v) => Value::Reference(format!("${}", v)),
            Expression::Reference(r) => Value::Reference(format!("@{}", r)),
            Expression::IndexedReference(file, key) => {
                Value::Reference(format!("@{}[{}]", file, key))
            }
            Expression::Identifier(i) => Value::String(i.clone()),
            _ => Value::String("".to_string()),
        }
    }
}
#[allow(dead_code)]
pub struct AstBuilder {
    ast: HelixAst,
}
#[allow(dead_code)]
impl AstBuilder {
    pub fn new() -> Self {
        AstBuilder { ast: HelixAst::new() }
    }
    pub fn add_project(
        mut self,
        name: String,
        properties: HashMap<String, Expression>,
    ) -> Self {
        self.ast.add_declaration(Declaration::Project(ProjectDecl { name, properties }));
        self
    }
    pub fn add_agent(mut self, agent: AgentDecl) -> Self {
        self.ast.add_declaration(Declaration::Agent(agent));
        self
    }
    pub fn add_workflow(mut self, workflow: WorkflowDecl) -> Self {
        self.ast.add_declaration(Declaration::Workflow(workflow));
        self
    }
    pub fn add_context(mut self, context: ContextDecl) -> Self {
        self.ast.add_declaration(Declaration::Context(context));
        self
    }
    pub fn build(self) -> HelixAst {
        self.ast
    }
}