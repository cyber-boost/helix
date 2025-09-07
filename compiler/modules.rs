use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet, VecDeque};
use anyhow::{Result, Context};
use crate::ast::{HelixAst, Declaration};
use crate::error::{HelixError, CompilationError, CompilationStage};
pub struct ModuleSystem {
    modules: HashMap<PathBuf, Module>,
    dependencies: HashMap<PathBuf, HashSet<PathBuf>>,
    dependents: HashMap<PathBuf, HashSet<PathBuf>>,
    asts: HashMap<PathBuf, HelixAst>,
    resolution_order: Vec<PathBuf>,
    #[allow(dead_code)]
    resolver: ModuleResolver,
}
#[derive(Debug, Clone)]
pub struct Module {
    pub path: PathBuf,
    pub ast: HelixAst,
    pub exports: ModuleExports,
    pub imports: ModuleImports,
    pub metadata: ModuleMetadata,
}
#[derive(Debug, Clone, Default)]
pub struct ModuleExports {
    pub agents: HashMap<String, AgentExport>,
    pub workflows: HashMap<String, WorkflowExport>,
    pub contexts: HashMap<String, ContextExport>,
    pub crews: HashMap<String, CrewExport>,
    pub types: HashMap<String, TypeExport>,
}
#[derive(Debug, Clone, Default)]
pub struct ModuleImports {
    pub imports: Vec<ImportStatement>,
}
#[derive(Debug, Clone)]
pub struct ImportStatement {
    pub path: String,
    pub items: ImportItems,
    pub alias: Option<String>,
}
#[derive(Debug, Clone)]
pub enum ImportItems {
    All,
    Specific(Vec<ImportItem>),
    Module,
}
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
    pub item_type: ImportType,
}
#[derive(Debug, Clone)]
pub enum ImportType {
    Agent,
    Workflow,
    Context,
    Crew,
    Type,
    Any,
}
#[derive(Debug, Clone)]
pub struct AgentExport {
    pub name: String,
    pub public: bool,
    pub location: usize,
}
#[derive(Debug, Clone)]
pub struct WorkflowExport {
    pub name: String,
    pub public: bool,
    pub location: usize,
}
#[derive(Debug, Clone)]
pub struct ContextExport {
    pub name: String,
    pub public: bool,
    pub location: usize,
}
#[derive(Debug, Clone)]
pub struct CrewExport {
    pub name: String,
    pub public: bool,
    pub location: usize,
}
#[derive(Debug, Clone)]
pub struct TypeExport {
    pub name: String,
    pub public: bool,
    pub type_def: String,
}
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub version: String,
    pub description: Option<String>,
    pub dependencies: Vec<String>,
    pub main: bool,
}
pub struct ModuleResolver {
    search_paths: Vec<PathBuf>,
    cache: HashMap<String, PathBuf>,
}
impl ModuleResolver {
    pub fn new() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("."), PathBuf::from("./configs"),
                PathBuf::from("./mso_modules"),
            ],
            cache: HashMap::new(),
        }
    }
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }
    pub fn resolve(&mut self, module_name: &str) -> Result<PathBuf> {
        if let Some(path) = self.cache.get(module_name) {
            return Ok(path.clone());
        }
        let patterns = vec![
            format!("{}.hlx", module_name), format!("{}/mod.hlx", module_name),
            format!("mso/{}.hlx", module_name),
        ];
        for search_path in &self.search_paths {
            for pattern in &patterns {
                let full_path = search_path.join(pattern);
                if full_path.exists() {
                    self.cache.insert(module_name.to_string(), full_path.clone());
                    return Ok(full_path);
                }
            }
        }
        Err(anyhow::anyhow!("Module not found: {}", module_name))
    }
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}
impl ModuleSystem {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
            asts: HashMap::new(),
            resolution_order: Vec::new(),
            resolver: ModuleResolver::new(),
        }
    }
    pub fn load_module(&mut self, path: &Path) -> Result<()> {
        let path = path.to_path_buf();
        let content = std::fs::read_to_string(&path).context("Failed to read file")?;
        let ast = crate::parse(&content)
            .map_err(|e| HelixError::Compilation(CompilationError {
                stage: CompilationStage::Parsing,
                message: e.to_string(),
                file: Some(path.clone()),
                recoverable: false,
            }))?;
        let deps = self.extract_dependencies(&ast);
        let imports = self.extract_imports(&ast)?;
        let exports = self.extract_exports(&ast)?;
        let module = Module {
            path: path.clone(),
            ast: ast.clone(),
            exports,
            imports,
            metadata: ModuleMetadata {
                version: "1.0.0".to_string(),
                description: None,
                dependencies: Vec::new(),
                main: false,
            },
        };
        self.dependencies.insert(path.clone(), deps.clone());
        for dep in deps {
            self.dependents.entry(dep).or_insert_with(HashSet::new).insert(path.clone());
        }
        self.modules.insert(path.clone(), module);
        self.asts.insert(path, ast);
        Ok(())
    }
    fn extract_dependencies(&self, _ast: &HelixAst) -> HashSet<PathBuf> {
        let deps = HashSet::new();
        for decl in &_ast.declarations {
            match decl {
                _ => {}
            }
        }
        deps
    }
    pub fn resolve_dependencies(&mut self) -> Result<()> {
        if let Some(cycle) = self.find_circular_dependency() {
            return Err(
                HelixError::Compilation(CompilationError {
                        stage: CompilationStage::Validation,
                        message: format!("Circular dependency detected: {:?}", cycle),
                        file: cycle.first().cloned(),
                        recoverable: false,
                    })
                    .into(),
            );
        }
        self.resolution_order = self.topological_sort()?;
        Ok(())
    }
    pub fn compilation_order(&self) -> &[PathBuf] {
        &self.resolution_order
    }
    #[allow(dead_code)]
    fn resolve_import_path(
        &self,
        import_path: &str,
        from_module: &Path,
    ) -> Result<PathBuf, HelixError> {
        if import_path.starts_with("./") || import_path.starts_with("../") {
            let base_dir = from_module.parent().unwrap_or(Path::new("."));
            return Ok(base_dir.join(import_path).with_extension("mso"));
        }
        if import_path.starts_with("/") {
            return Ok(PathBuf::from(import_path).with_extension("mso"));
        }
        Ok(PathBuf::from(format!("{}.hlx", import_path)))
    }
    fn extract_imports(&self, _ast: &HelixAst) -> Result<ModuleImports, HelixError> {
        let imports = Vec::new();
        Ok(ModuleImports { imports })
    }
    fn extract_exports(&self, ast: &HelixAst) -> Result<ModuleExports, HelixError> {
        let mut exports = ModuleExports::default();
        for (index, declaration) in ast.declarations.iter().enumerate() {
            match declaration {
                Declaration::Agent(agent) => {
                    exports
                        .agents
                        .insert(
                            agent.name.clone(),
                            AgentExport {
                                name: agent.name.clone(),
                                public: true,
                                location: index,
                            },
                        );
                }
                Declaration::Workflow(workflow) => {
                    exports
                        .workflows
                        .insert(
                            workflow.name.clone(),
                            WorkflowExport {
                                name: workflow.name.clone(),
                                public: true,
                                location: index,
                            },
                        );
                }
                Declaration::Context(context) => {
                    exports
                        .contexts
                        .insert(
                            context.name.clone(),
                            ContextExport {
                                name: context.name.clone(),
                                public: true,
                                location: index,
                            },
                        );
                }
                Declaration::Crew(crew) => {
                    exports
                        .crews
                        .insert(
                            crew.name.clone(),
                            CrewExport {
                                name: crew.name.clone(),
                                public: true,
                                location: index,
                            },
                        );
                }
                _ => {}
            }
        }
        Ok(exports)
    }
    fn find_circular_dependency(&self) -> Option<Vec<PathBuf>> {
        for (start, _) in &self.dependencies {
            let mut visited = HashSet::new();
            let mut rec_stack = HashSet::new();
            let mut path = Vec::new();
            if self.has_cycle_util(start, &mut visited, &mut rec_stack, &mut path) {
                return Some(path);
            }
        }
        None
    }
    fn has_cycle_util(
        &self,
        node: &PathBuf,
        visited: &mut HashSet<PathBuf>,
        rec_stack: &mut HashSet<PathBuf>,
        path: &mut Vec<PathBuf>,
    ) -> bool {
        if rec_stack.contains(node) {
            path.push(node.clone());
            return true;
        }
        if visited.contains(node) {
            return false;
        }
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());
        if let Some(deps) = self.dependencies.get(node) {
            for dep in deps {
                if self.has_cycle_util(dep, visited, rec_stack, path) {
                    return true;
                }
            }
        }
        rec_stack.remove(node);
        path.pop();
        false
    }
    fn topological_sort(&self) -> Result<Vec<PathBuf>, HelixError> {
        let mut in_degree: HashMap<PathBuf, usize> = HashMap::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        for path in self.dependencies.keys() {
            in_degree.insert(path.clone(), 0);
        }
        for (_, deps) in &self.dependencies {
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }
        for (path, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(path.clone());
            }
        }
        while let Some(node) = queue.pop_front() {
            result.push(node.clone());
            if let Some(deps) = self.dependencies.get(&node) {
                for dep in deps {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }
        if result.len() != self.dependencies.len() {
            return Err(
                HelixError::Compilation(CompilationError {
                    stage: CompilationStage::Validation,
                    message: "Failed to resolve module dependencies".to_string(),
                    file: None,
                    recoverable: false,
                }),
            );
        }
        Ok(result)
    }
    pub fn merge_modules(&self) -> Result<HelixAst, HelixError> {
        let mut merged_declarations = Vec::new();
        for path in &self.resolution_order {
            if let Some(module) = self.modules.get(path) {
                for decl in &module.ast.declarations {
                    if !Self::declaration_exists(&merged_declarations, decl) {
                        merged_declarations.push(decl.clone());
                    }
                }
            }
        }
        Ok(HelixAst {
            declarations: merged_declarations,
        })
    }
    fn declaration_exists(declarations: &[Declaration], decl: &Declaration) -> bool {
        for existing in declarations {
            match (existing, decl) {
                (Declaration::Agent(a1), Declaration::Agent(a2)) => {
                    if a1.name == a2.name {
                        return true;
                    }
                }
                (Declaration::Workflow(w1), Declaration::Workflow(w2)) => {
                    if w1.name == w2.name {
                        return true;
                    }
                }
                (Declaration::Context(c1), Declaration::Context(c2)) => {
                    if c1.name == c2.name {
                        return true;
                    }
                }
                (Declaration::Crew(c1), Declaration::Crew(c2)) => {
                    if c1.name == c2.name {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
    pub fn modules(&self) -> &HashMap<PathBuf, Module> {
        &self.modules
    }
    pub fn dependency_graph(&self) -> &HashMap<PathBuf, HashSet<PathBuf>> {
        &self.dependencies
    }
    pub fn get_dependencies<P: AsRef<Path>>(&self, path: P) -> HashSet<PathBuf> {
        self.dependencies.get(path.as_ref()).cloned().unwrap_or_default()
    }
    pub fn get_dependents<P: AsRef<Path>>(&self, path: P) -> HashSet<PathBuf> {
        self.dependents.get(path.as_ref()).cloned().unwrap_or_default()
    }
}
pub struct DependencyBundler {
    module_system: ModuleSystem,
}
impl DependencyBundler {
    pub fn new() -> Self {
        Self {
            module_system: ModuleSystem::new(),
        }
    }
    pub fn add_root<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let mut queue = VecDeque::new();
        let mut processed = HashSet::new();
        queue.push_back(path.as_ref().to_path_buf());
        while let Some(current) = queue.pop_front() {
            if processed.contains(&current) {
                continue;
            }
            self.module_system.load_module(&current)?;
            processed.insert(current.clone());
            let deps = self.module_system.get_dependencies(&current);
            for dep in deps {
                if !processed.contains(&dep) {
                    queue.push_back(dep);
                }
            }
        }
        Ok(())
    }
    pub fn build_bundle(&mut self) -> Result<HelixAst> {
        self.module_system.resolve_dependencies()?;
        self.module_system
            .merge_modules()
            .map_err(|e| anyhow::anyhow!("Failed to merge modules: {}", e))
    }
    pub fn get_compilation_order(&self) -> &[PathBuf] {
        self.module_system.compilation_order()
    }
}
impl Default for ModuleSystem {
    fn default() -> Self {
        Self::new()
    }
}
impl Default for DependencyBundler {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    #[test]
    fn test_module_system_creation() {
        let module_system = ModuleSystem::new();
        assert!(module_system.modules.is_empty());
        assert!(module_system.dependencies.is_empty());
    }
    #[test]
    fn test_module_loading() {
        let temp_dir = TempDir::new().unwrap();
        let module_path = temp_dir.path().join("test.hlx");
        std::fs::write(
                &module_path,
                r#"
            agent "test" {
                model = "gpt-4"
            }
        "#,
            )
            .unwrap();
        let mut module_system = ModuleSystem::new();
        module_system.load_module(&module_path).unwrap();
        assert_eq!(module_system.modules.len(), 1);
        assert!(module_system.modules.contains_key(& module_path));
    }
    #[test]
    fn test_export_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let module_path = temp_dir.path().join("test.hlx");
        std::fs::write(
                &module_path,
                r#"
            agent "analyzer" {
                model = "gpt-4"
            }
            
            workflow "review" {
                trigger = "manual"
            }
        "#,
            )
            .unwrap();
        let mut module_system = ModuleSystem::new();
        module_system.load_module(&module_path).unwrap();
        let module = module_system.modules.get(&module_path).unwrap();
        assert_eq!(module.exports.agents.len(), 1);
        assert_eq!(module.exports.workflows.len(), 1);
        assert!(module.exports.agents.contains_key("analyzer"));
        assert!(module.exports.workflows.contains_key("review"));
    }
    #[test]
    fn test_dependency_bundler() {
        let mut bundler = DependencyBundler::new();
        assert!(bundler.module_system.modules.is_empty());
    }
    #[test]
    fn test_module_resolver() {
        let mut resolver = ModuleResolver::new();
        resolver.add_search_path("./test");
        assert_eq!(resolver.search_paths.len(), 4);
    }
}