use std::collections::{HashMap, HashSet};
use super::ast::*;
use crate::ast::AstVisitor;
#[derive(Debug, Clone)]
pub enum SemanticError {
    UndefinedAgent { name: String, location: String },
    UndefinedWorkflow { name: String, location: String },
    UndefinedStep { name: String, workflow: String },
    UndefinedReference { reference: String, location: String },
    DuplicateDefinition { name: String, kind: String },
    TypeMismatch { expected: String, found: String, location: String },
    CircularDependency { items: Vec<String> },
    InvalidDuration { value: String, location: String },
    MissingRequiredField { field: String, declaration: String },
    InvalidTriggerType { trigger: String },
    InvalidProcessType { process: String },
    InvalidBackoffStrategy { strategy: String },
}
impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemanticError::UndefinedAgent { name, location } => {
                write!(f, "Undefined agent '{}' referenced in {}", name, location)
            }
            SemanticError::UndefinedWorkflow { name, location } => {
                write!(f, "Undefined workflow '{}' referenced in {}", name, location)
            }
            SemanticError::UndefinedStep { name, workflow } => {
                write!(f, "Undefined step '{}' in workflow '{}'", name, workflow)
            }
            SemanticError::UndefinedReference { reference, location } => {
                write!(f, "Undefined reference '{}' in {}", reference, location)
            }
            SemanticError::DuplicateDefinition { name, kind } => {
                write!(f, "Duplicate {} definition: '{}'", kind, name)
            }
            SemanticError::TypeMismatch { expected, found, location } => {
                write!(
                    f, "Type mismatch in {}: expected {}, found {}", location, expected,
                    found
                )
            }
            SemanticError::CircularDependency { items } => {
                write!(f, "Circular dependency detected: {}", items.join(" -> "))
            }
            SemanticError::InvalidDuration { value, location } => {
                write!(f, "Invalid duration '{}' in {}", value, location)
            }
            SemanticError::MissingRequiredField { field, declaration } => {
                write!(f, "Missing required field '{}' in {}", field, declaration)
            }
            SemanticError::InvalidTriggerType { trigger } => {
                write!(f, "Invalid trigger type: '{}'", trigger)
            }
            SemanticError::InvalidProcessType { process } => {
                write!(
                    f,
                    "Invalid process type: '{}'. Must be one of: sequential, hierarchical, parallel, consensus",
                    process
                )
            }
            SemanticError::InvalidBackoffStrategy { strategy } => {
                write!(
                    f,
                    "Invalid backoff strategy: '{}'. Must be one of: fixed, linear, exponential",
                    strategy
                )
            }
        }
    }
}
pub struct SemanticAnalyzer {
    pub agents: HashMap<String, AgentDecl>,
    pub workflows: HashMap<String, WorkflowDecl>,
    pub contexts: HashMap<String, ContextDecl>,
    pub crews: HashMap<String, CrewDecl>,
    pub expected_env_vars: HashSet<String>,
    pub _expected_memory_refs: HashSet<String>,
    pub errors: Vec<SemanticError>,
}
impl AstVisitor for SemanticAnalyzer {
    type Result = ();
    fn visit_ast(&mut self, ast: &HelixAst) -> Self::Result {
        for decl in &ast.declarations {
            self.visit_declaration(decl);
        }
    }
    fn visit_declaration(&mut self, decl: &Declaration) -> Self::Result {
        match decl {
            Declaration::Project(p) => self.visit_project(p),
            Declaration::Agent(a) => self.visit_agent(a),
            Declaration::Workflow(w) => self.visit_workflow(w),
            Declaration::Memory(m) => self.visit_memory(m),
            Declaration::Context(c) => self.visit_context(c),
            Declaration::Crew(crew) => self.visit_crew(crew),
            _ => {}
        }
    }
    fn visit_project(&mut self, _project: &ProjectDecl) -> Self::Result {}
    fn visit_agent(&mut self, _agent: &AgentDecl) -> Self::Result {}
    fn visit_workflow(&mut self, _workflow: &WorkflowDecl) -> Self::Result {}
    fn visit_memory(&mut self, memory: &MemoryDecl) -> Self::Result {
        for (_key, expr) in &memory.properties {
            self.visit_expression(expr);
        }
    }
    fn visit_context(&mut self, _context: &ContextDecl) -> Self::Result {}
    fn visit_crew(&mut self, _crew: &CrewDecl) -> Self::Result {}
    fn visit_section(&mut self, section: &SectionDecl) -> Self::Result {
        for (_key, expr) in &section.properties {
            self.visit_expression(expr);
        }
    }
    fn visit_expression(&mut self, expr: &Expression) -> Self::Result {
        match expr {
            Expression::Variable(var) => {
                self.expected_env_vars.insert(var.clone());
            }
            Expression::Reference(ref_name) => {
                self._expected_memory_refs.insert(ref_name.clone());
            }
            _ => {}
        }
    }
}
impl SemanticAnalyzer {
    pub fn new() -> Self {
        SemanticAnalyzer {
            agents: HashMap::new(),
            workflows: HashMap::new(),
            contexts: HashMap::new(),
            crews: HashMap::new(),
            expected_env_vars: HashSet::new(),
            _expected_memory_refs: HashSet::new(),
            errors: Vec::new(),
        }
    }
    pub fn analyze(&mut self, ast: &HelixAst) -> Result<(), Vec<SemanticError>> {
        for decl in &ast.declarations {
            match decl {
                Declaration::Agent(agent) => self.visit_agent(agent),
                Declaration::Workflow(workflow) => self.visit_workflow(workflow),
                Declaration::Context(context) => self.visit_context(context),
                Declaration::Crew(crew) => self.visit_crew(crew),
                _ => {}
            }
        }
        self.collect_definitions(ast)?;
        self.validate_references(ast);
        let type_checker = TypeChecker::new();
        self.type_check_with_checker(ast, &type_checker);
        self.analyze_dependencies(ast);
        if !self.errors.is_empty() { Err(self.errors.clone()) } else { Ok(()) }
    }
    fn collect_definitions(&mut self, ast: &HelixAst) -> Result<(), Vec<SemanticError>> {
        for decl in &ast.declarations {
            match decl {
                Declaration::Agent(agent) => {
                    if self.agents.contains_key(&agent.name) {
                        self.errors
                            .push(SemanticError::DuplicateDefinition {
                                name: agent.name.clone(),
                                kind: "agent".to_string(),
                            });
                    } else {
                        self.agents.insert(agent.name.clone(), agent.clone());
                    }
                }
                Declaration::Workflow(workflow) => {
                    if self.workflows.contains_key(&workflow.name) {
                        self.errors
                            .push(SemanticError::DuplicateDefinition {
                                name: workflow.name.clone(),
                                kind: "workflow".to_string(),
                            });
                    } else {
                        self.workflows.insert(workflow.name.clone(), workflow.clone());
                    }
                }
                Declaration::Context(context) => {
                    if self.contexts.contains_key(&context.name) {
                        self.errors
                            .push(SemanticError::DuplicateDefinition {
                                name: context.name.clone(),
                                kind: "context".to_string(),
                            });
                    } else {
                        if let Some(secrets) = &context.secrets {
                            for (_key, secret_ref) in secrets {
                                if let SecretRef::Environment(var) = secret_ref {
                                    self.expected_env_vars.insert(var.clone());
                                }
                            }
                        }
                        self.contexts.insert(context.name.clone(), context.clone());
                    }
                }
                Declaration::Crew(crew) => {
                    if self.crews.contains_key(&crew.name) {
                        self.errors
                            .push(SemanticError::DuplicateDefinition {
                                name: crew.name.clone(),
                                kind: "crew".to_string(),
                            });
                    } else {
                        self.crews.insert(crew.name.clone(), crew.clone());
                    }
                }
                _ => {}
            }
        }
        if !self.errors.is_empty() { Err(self.errors.clone()) } else { Ok(()) }
    }
    fn validate_references(&mut self, ast: &HelixAst) {
        for decl in &ast.declarations {
            match decl {
                Declaration::Workflow(workflow) => {
                    self.validate_workflow_references(workflow);
                }
                Declaration::Crew(crew) => {
                    self.validate_crew_references(crew);
                }
                _ => {}
            }
        }
    }
    fn validate_workflow_references(&mut self, workflow: &WorkflowDecl) {
        for step in &workflow.steps {
            if let Some(agent_name) = &step.agent {
                if !self.agents.contains_key(agent_name) {
                    self.errors
                        .push(SemanticError::UndefinedAgent {
                            name: agent_name.clone(),
                            location: format!(
                                "workflow '{}', step '{}'", workflow.name, step.name
                            ),
                        });
                }
            }
            if let Some(crew_agents) = &step.crew {
                for agent_name in crew_agents {
                    if !self.agents.contains_key(agent_name) {
                        self.errors
                            .push(SemanticError::UndefinedAgent {
                                name: agent_name.clone(),
                                location: format!(
                                    "workflow '{}', step '{}'", workflow.name, step.name
                                ),
                            });
                    }
                }
            }
            if let Some(depends_on) = step.properties.get("depends_on") {
                if let Some(deps) = depends_on.as_array() {
                    for dep in deps {
                        if let Some(dep_name) = dep.as_string() {
                            let step_exists = workflow
                                .steps
                                .iter()
                                .any(|s| s.name == dep_name);
                            if !step_exists {
                                self.errors
                                    .push(SemanticError::UndefinedStep {
                                        name: dep_name,
                                        workflow: workflow.name.clone(),
                                    });
                            }
                        }
                    }
                }
            }
        }
        if let Some(pipeline) = &workflow.pipeline {
            for node in &pipeline.flow {
                if let PipelineNode::Step(step_name) = node {
                    let step_exists = workflow
                        .steps
                        .iter()
                        .any(|s| s.name == *step_name);
                    if !step_exists {
                        self.errors
                            .push(SemanticError::UndefinedStep {
                                name: step_name.clone(),
                                workflow: workflow.name.clone(),
                            });
                    }
                }
            }
        }
        if let Some(trigger) = &workflow.trigger {
            self.validate_trigger(trigger, &workflow.name);
        }
    }
    fn validate_trigger(&mut self, trigger: &Expression, workflow_name: &str) {
        match trigger {
            Expression::String(s) | Expression::Identifier(s) => {
                let valid_triggers = ["manual", "webhook", "event", "file_watch"];
                if !valid_triggers.contains(&s.as_str()) && !s.starts_with("schedule:") {
                    self.errors
                        .push(SemanticError::InvalidTriggerType {
                            trigger: s.clone(),
                        });
                }
            }
            Expression::Object(map) => {
                if let Some(trigger_type) = map.get("type") {
                    self.validate_trigger(trigger_type, workflow_name);
                }
            }
            _ => {}
        }
    }
    fn validate_crew_references(&mut self, crew: &CrewDecl) {
        for agent_name in &crew.agents {
            if !self.agents.contains_key(agent_name) {
                self.errors
                    .push(SemanticError::UndefinedAgent {
                        name: agent_name.clone(),
                        location: format!("crew '{}'", crew.name),
                    });
            }
        }
        if let Some(process_type) = &crew.process_type {
            let valid_types = ["sequential", "hierarchical", "parallel", "consensus"];
            if !valid_types.contains(&process_type.as_str()) {
                self.errors
                    .push(SemanticError::InvalidProcessType {
                        process: process_type.clone(),
                    });
            }
        }
        if let Some(process) = &crew.process_type {
            if process == "hierarchical" {
                if let Some(manager) = crew.properties.get("manager") {
                    if let Some(manager_name) = manager.as_string() {
                        if !self.agents.contains_key(&manager_name) {
                            self.errors
                                .push(SemanticError::UndefinedAgent {
                                    name: manager_name,
                                    location: format!("crew '{}' manager", crew.name),
                                });
                        }
                    }
                } else {
                    self.errors
                        .push(SemanticError::MissingRequiredField {
                            field: "manager".to_string(),
                            declaration: format!("hierarchical crew '{}'", crew.name),
                        });
                }
            }
        }
    }
    #[allow(dead_code)]
    fn type_check(&mut self, ast: &HelixAst) {
        self.type_check_with_checker(ast, &TypeChecker::new());
    }
    fn type_check_with_checker(&mut self, ast: &HelixAst, checker: &TypeChecker) {
        for decl in &ast.declarations {
            match decl {
                Declaration::Agent(agent) => {
                    for (key, expr) in &agent.properties {
                        if let Err(_msg) = checker.check_type(key, expr) {
                            self.errors
                                .push(SemanticError::TypeMismatch {
                                    expected: "valid type".to_string(),
                                    found: checker.infer_type(expr).to_string(),
                                    location: format!("agent '{}'", agent.name),
                                });
                        }
                    }
                    self.type_check_agent(agent);
                }
                Declaration::Workflow(workflow) => {
                    self.type_check_workflow(workflow);
                }
                _ => {}
            }
        }
    }
    fn type_check_agent(&mut self, agent: &AgentDecl) {
        if let Some(temp) = agent.properties.get("temperature") {
            if let Some(temp_val) = temp.as_number() {
                if temp_val < 0.0 || temp_val > 2.0 {
                    self.errors
                        .push(SemanticError::TypeMismatch {
                            expected: "number between 0 and 2".to_string(),
                            found: format!("{}", temp_val),
                            location: format!("agent '{}' temperature", agent.name),
                        });
                }
            }
        }
        if let Some(tokens) = agent.properties.get("max_tokens") {
            if let Some(tokens_val) = tokens.as_number() {
                if tokens_val <= 0.0 {
                    self.errors
                        .push(SemanticError::TypeMismatch {
                            expected: "positive number".to_string(),
                            found: format!("{}", tokens_val),
                            location: format!("agent '{}' max_tokens", agent.name),
                        });
                }
            }
        }
    }
    fn type_check_workflow(&mut self, workflow: &WorkflowDecl) {
        for step in &workflow.steps {
            if let Some(retry) = step.properties.get("retry") {
                if let Some(retry_obj) = retry.as_object() {
                    if let Some(max_attempts) = retry_obj.get("max_attempts") {
                        if let Some(attempts) = max_attempts.as_number() {
                            if attempts <= 0.0 {
                                self.errors
                                    .push(SemanticError::TypeMismatch {
                                        expected: "positive number".to_string(),
                                        found: format!("{}", attempts),
                                        location: format!(
                                            "workflow '{}', step '{}' retry.max_attempts", workflow
                                            .name, step.name
                                        ),
                                    });
                            }
                        }
                    }
                    if let Some(backoff) = retry_obj.get("backoff") {
                        if let Some(strategy) = backoff.as_string() {
                            let valid_strategies = ["fixed", "linear", "exponential"];
                            if !valid_strategies.contains(&strategy.as_str()) {
                                self.errors
                                    .push(SemanticError::InvalidBackoffStrategy {
                                        strategy,
                                    });
                            }
                        }
                    }
                }
            }
        }
    }
    fn analyze_dependencies(&mut self, ast: &HelixAst) {
        for decl in &ast.declarations {
            if let Declaration::Workflow(workflow) = decl {
                self.check_circular_dependencies(workflow);
            }
        }
    }
    fn check_circular_dependencies(&mut self, workflow: &WorkflowDecl) {
        let mut dependency_graph: HashMap<String, Vec<String>> = HashMap::new();
        for step in &workflow.steps {
            let deps = if let Some(depends_on) = step.properties.get("depends_on") {
                if let Some(deps_array) = depends_on.as_array() {
                    deps_array.iter().filter_map(|d| d.as_string()).collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            dependency_graph.insert(step.name.clone(), deps);
        }
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        for step in &workflow.steps {
            if !visited.contains(&step.name) {
                if let Some(cycle) = self
                    .has_cycle(
                        &step.name,
                        &dependency_graph,
                        &mut visited,
                        &mut rec_stack,
                        &mut Vec::new(),
                    )
                {
                    self.errors
                        .push(SemanticError::CircularDependency {
                            items: cycle,
                        });
                    break;
                }
            }
        }
    }
    fn has_cycle(
        &self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = self
                        .has_cycle(neighbor, graph, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(neighbor.clone());
                    return Some(cycle);
                }
            }
        }
        rec_stack.remove(node);
        path.pop();
        None
    }
}
pub struct TypeChecker {
    expected_types: HashMap<String, ExpressionType>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionType {
    String,
    Number,
    Bool,
    Duration,
    Array(Box<ExpressionType>),
    Object,
    Any,
}
impl std::fmt::Display for ExpressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionType::String => write!(f, "string"),
            ExpressionType::Number => write!(f, "number"),
            ExpressionType::Bool => write!(f, "boolean"),
            ExpressionType::Duration => write!(f, "duration"),
            ExpressionType::Array(inner) => write!(f, "array<{}>", inner),
            ExpressionType::Object => write!(f, "object"),
            ExpressionType::Any => write!(f, "any"),
        }
    }
}
impl TypeChecker {
    pub fn new() -> Self {
        let mut expected_types = HashMap::new();
        expected_types.insert("temperature".to_string(), ExpressionType::Number);
        expected_types.insert("max_tokens".to_string(), ExpressionType::Number);
        expected_types.insert("timeout".to_string(), ExpressionType::Duration);
        expected_types.insert("debug".to_string(), ExpressionType::Bool);
        expected_types.insert("parallel".to_string(), ExpressionType::Bool);
        expected_types.insert("verbose".to_string(), ExpressionType::Bool);
        expected_types.insert("persistence".to_string(), ExpressionType::Bool);
        expected_types.insert("dimensions".to_string(), ExpressionType::Number);
        expected_types.insert("batch_size".to_string(), ExpressionType::Number);
        expected_types.insert("max_iterations".to_string(), ExpressionType::Number);
        expected_types.insert("cache_size".to_string(), ExpressionType::Number);
        TypeChecker { expected_types }
    }
    pub fn infer_type(&self, expr: &Expression) -> ExpressionType {
        match expr {
            Expression::String(_) | Expression::Identifier(_) => ExpressionType::String,
            Expression::Number(_) => ExpressionType::Number,
            Expression::Bool(_) => ExpressionType::Bool,
            Expression::Duration(_) => ExpressionType::Duration,
            Expression::Array(items) => {
                if items.is_empty() {
                    ExpressionType::Array(Box::new(ExpressionType::Any))
                } else {
                    let first_type = self.infer_type(&items[0]);
                    ExpressionType::Array(Box::new(first_type))
                }
            }
            Expression::Object(_) => ExpressionType::Object,
            Expression::Variable(_) | Expression::Reference(_) => ExpressionType::Any,
            _ => ExpressionType::Any,
        }
    }
    pub fn check_type(&self, field: &str, expr: &Expression) -> Result<(), String> {
        if let Some(expected) = self.expected_types.get(field) {
            let actual = self.infer_type(expr);
            if actual != *expected && actual != ExpressionType::Any {
                return Err(
                    format!(
                        "Type mismatch for field '{}': expected {:?}, found {:?}", field,
                        expected, actual
                    ),
                );
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_duplicate_detection() {
        let mut analyzer = SemanticAnalyzer::new();
        let mut ast = HelixAst::new();
        ast.add_declaration(
            Declaration::Agent(AgentDecl {
                name: "test_agent".to_string(),
                properties: HashMap::new(),
                capabilities: None,
                backstory: None,
                tools: None,
            }),
        );
        ast.add_declaration(
            Declaration::Agent(AgentDecl {
                name: "test_agent".to_string(),
                properties: HashMap::new(),
                capabilities: None,
                backstory: None,
                tools: None,
            }),
        );
        let result = analyzer.analyze(&ast);
        assert!(result.is_err());
        if let Err(errors) = result {
            assert!(
                errors.iter().any(| e | matches!(e, SemanticError::DuplicateDefinition {
                name, kind } if name == "test_agent" && kind == "agent"))
            );
        }
    }
    #[test]
    fn test_undefined_agent_reference() {
        let mut analyzer = SemanticAnalyzer::new();
        let mut ast = HelixAst::new();
        let mut step = StepDecl {
            name: "test_step".to_string(),
            agent: Some("undefined_agent".to_string()),
            crew: None,
            task: None,
            properties: HashMap::new(),
        };
        ast.add_declaration(
            Declaration::Workflow(WorkflowDecl {
                name: "test_workflow".to_string(),
                trigger: None,
                steps: vec![step],
                pipeline: None,
                properties: HashMap::new(),
            }),
        );
        let result = analyzer.analyze(&ast);
        assert!(result.is_err());
        if let Err(errors) = result {
            assert!(
                errors.iter().any(| e | matches!(e, SemanticError::UndefinedAgent { name,
                .. } if name == "undefined_agent"))
            );
        }
    }
}