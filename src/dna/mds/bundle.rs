use crate::dna::compiler::Compiler;
use crate::dna::compiler::CompileError;
use crate::dna::hel::binary::{HelixBinary, DataSection};
use super::optimizer::OptimizationLevel;
use super::serializer::BinarySerializer;
use super::codegen::Instruction;
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::{HashMap, HashSet, VecDeque};
pub struct Bundler {
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    follow_imports: bool,
    tree_shake: bool,
    verbose: bool,
}
impl Bundler {
    pub fn new() -> Self {
        Self {
            include_patterns: vec!["*.hlx".to_string()],
            exclude_patterns: Vec::new(),
            follow_imports: true,
            tree_shake: false,
            verbose: false,
        }
    }
    pub fn include(mut self, pattern: &str) -> Self {
        self.include_patterns.push(pattern.to_string());
        self
    }
    pub fn exclude(mut self, pattern: &str) -> Self {
        self.exclude_patterns.push(pattern.to_string());
        self
    }
    pub fn with_imports(mut self, follow: bool) -> Self {
        self.follow_imports = follow;
        self
    }
    pub fn with_tree_shaking(mut self, enable: bool) -> Self {
        self.tree_shake = enable;
        self
    }
    pub fn verbose(mut self, enable: bool) -> Self {
        self.verbose = enable;
        self
    }
    pub fn bundle_directory<P: AsRef<Path>>(
        &self,
        directory: P,
        optimization_level: OptimizationLevel,
    ) -> Result<HelixBinary, CompileError> {
        let directory = directory.as_ref();
        if self.verbose {
            println!("Bundling directory: {}", directory.display());
        }
        let files = self.collect_files(directory)?;
        if files.is_empty() {
            return Err(CompileError::IoError("No HELIX files found".to_string()));
        }
        if self.verbose {
            println!("Found {} files to bundle", files.len());
        }
        self.bundle_files(&files, optimization_level)
    }
    pub fn bundle_files(
        &self,
        files: &[PathBuf],
        optimization_level: OptimizationLevel,
    ) -> Result<HelixBinary, CompileError> {
        let mut bundle = BundleBuilder::new();
        let compiler = Compiler::new(optimization_level);
        for file in files {
            if self.verbose {
                println!("  Processing: {}", file.display());
            }
            let source = fs::read_to_string(file)
                .map_err(|e| CompileError::IoError(
                    format!("Failed to read {}: {}", file.display(), e),
                ))?;
            let binary = compiler.compile_source(&source, Some(file))?;
            bundle.add_file(file.clone(), binary);
        }
        if self.follow_imports {
            self.resolve_dependencies(&mut bundle)?;
        }
        if self.tree_shake {
            self.apply_tree_shaking(&mut bundle)?;
        }
        let merged = bundle.build()?;
        if self.verbose {
            println!("Bundle created successfully");
            println!("  Total size: {} bytes", merged.size());
        }
        Ok(merged)
    }
    fn collect_files(&self, directory: &Path) -> Result<Vec<PathBuf>, CompileError> {
        let mut files = Vec::new();
        for entry in fs::read_dir(directory)
            .map_err(|e| CompileError::IoError(
                format!("Failed to read directory: {}", e),
            ))?
        {
            let entry = entry
                .map_err(|e| CompileError::IoError(
                    format!("Failed to read entry: {}", e),
                ))?;
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if self.should_include(file_name) {
                    files.push(path);
                }
            } else if path.is_dir() && self.follow_imports {
                let mut sub_files = self.collect_files(&path)?;
                files.append(&mut sub_files);
            }
        }
        Ok(files)
    }
    fn should_include(&self, file_name: &str) -> bool {
        for pattern in &self.exclude_patterns {
            if self.matches_pattern(file_name, pattern) {
                return false;
            }
        }
        for pattern in &self.include_patterns {
            if self.matches_pattern(file_name, pattern) {
                return true;
            }
        }
        false
    }
    fn matches_pattern(&self, file_name: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                return file_name.starts_with(prefix) && file_name.ends_with(suffix);
            }
        }
        file_name == pattern
    }
    fn resolve_dependencies(
        &self,
        bundle: &mut BundleBuilder,
    ) -> Result<(), CompileError> {
        for (path, binary) in &bundle.files {
            let mut deps = HashSet::new();
            let serializer = BinarySerializer::new(false);
            if let Ok(ir) = serializer.deserialize_to_ir(&binary) {
                for instruction in &ir.instructions {
                    if let Instruction::ResolveReference { ref_type: _, index } = instruction {
                        if *index > 1000 {
                            deps.insert(PathBuf::from("external.hlx"));
                        }
                    }
                }
            }
            bundle.dependencies.insert(path.clone(), deps);
        }
        if let Some(cycle) = self.detect_circular_dependencies(&bundle.dependencies) {
            return Err(
                CompileError::ParseError(
                    format!("Circular dependency detected: {:?}", cycle),
                ),
            );
        }
        Ok(())
    }
    fn detect_circular_dependencies(
        &self,
        deps: &HashMap<PathBuf, HashSet<PathBuf>>,
    ) -> Option<Vec<PathBuf>> {
        for (start, _) in deps {
            let mut visited = HashSet::new();
            let mut path = VecDeque::new();
            path.push_back(start.clone());
            if self.has_cycle_from(start, deps, &mut visited, &mut path) {
                return Some(path.into_iter().collect());
            }
        }
        None
    }
    fn has_cycle_from(
        &self,
        node: &PathBuf,
        deps: &HashMap<PathBuf, HashSet<PathBuf>>,
        visited: &mut HashSet<PathBuf>,
        path: &mut VecDeque<PathBuf>,
    ) -> bool {
        if visited.contains(node) {
            return true;
        }
        visited.insert(node.clone());
        if let Some(node_deps) = deps.get(node) {
            for dep in node_deps {
                path.push_back(dep.clone());
                if self.has_cycle_from(dep, deps, visited, path) {
                    return true;
                }
                path.pop_back();
            }
        }
        visited.remove(node);
        false
    }
    fn apply_tree_shaking(
        &self,
        bundle: &mut BundleBuilder,
    ) -> Result<(), CompileError> {
        use std::collections::HashSet;
        for (path, binary) in &mut bundle.files {
            let serializer = BinarySerializer::new(false);
            let mut ir = serializer
                .deserialize_to_ir(&binary)
                .map_err(|e| CompileError::SerializationError(e.to_string()))?;
            let mut used_agents = HashSet::new();
            let mut used_workflows = HashSet::new();
            let mut used_contexts = HashSet::new();
            for (_id, crew) in &ir.symbol_table.crews {
                for agent_id in &crew.agent_ids {
                    used_agents.insert(*agent_id);
                }
            }
            for workflow in ir.symbol_table.workflows.values() {
                if let Some(pipeline) = &workflow.pipeline {
                    for workflow_id in pipeline {
                        used_workflows.insert(*workflow_id);
                    }
                }
            }
            for instruction in &ir.instructions {
                if let Instruction::DeclareContext(id) = instruction {
                    used_contexts.insert(id);
                }
            }
            let unused_agents: Vec<u32> = ir
                .symbol_table
                .agents
                .keys()
                .filter(|id| !used_agents.contains(id))
                .cloned()
                .collect();
            for id in unused_agents {
                ir.symbol_table.agents.remove(&id);
            }
            let unused_workflows: Vec<u32> = ir
                .symbol_table
                .workflows
                .keys()
                .filter(|id| !used_workflows.contains(id))
                .cloned()
                .collect();
            for id in unused_workflows {
                ir.symbol_table.workflows.remove(&id);
            }
            let unused_contexts: Vec<u32> = ir
                .symbol_table
                .contexts
                .keys()
                .filter(|id| !used_contexts.contains(id))
                .cloned()
                .collect();
            for id in unused_contexts {
                ir.symbol_table.contexts.remove(&id);
            }
            *binary = serializer
                .serialize(ir, Some(path))
                .map_err(|e| CompileError::SerializationError(e.to_string()))?;
        }
        Ok(())
    }
}
impl Default for Bundler {
    fn default() -> Self {
        Self::new()
    }
}
struct BundleBuilder {
    files: HashMap<PathBuf, HelixBinary>,
    dependencies: HashMap<PathBuf, HashSet<PathBuf>>,
}
impl BundleBuilder {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }
    fn add_file(&mut self, path: PathBuf, binary: HelixBinary) {
        self.files.insert(path, binary);
    }
    #[allow(dead_code)]
    fn add_dependency(&mut self, from: PathBuf, to: PathBuf) {
        self.dependencies.entry(from).or_insert_with(HashSet::new).insert(to);
    }
    fn build(self) -> Result<HelixBinary, CompileError> {
        if self.files.is_empty() {
            return Err(CompileError::IoError("No files in bundle".to_string()));
        }
        let mut merged = self.files.values().next().unwrap().clone();
        for binary in self.files.values().skip(1) {
            Self::merge_binary(&mut merged, binary)?;
        }
        merged.metadata.extra.insert("bundle".to_string(), "true".to_string());
        merged
            .metadata
            .extra
            .insert("bundle_files".to_string(), self.files.len().to_string());
        merged.checksum = merged.calculate_checksum();
        Ok(merged)
    }
    fn merge_binary(
        target: &mut HelixBinary,
        source: &HelixBinary,
    ) -> Result<(), CompileError> {
        Self::merge_symbol_tables(&mut target.symbol_table, &source.symbol_table);
        for section in &source.data_sections {
            let existing = target
                .data_sections
                .iter_mut()
                .find(|s| {
                    std::mem::discriminant(&s.section_type)
                        == std::mem::discriminant(&section.section_type)
                });
            if let Some(existing_section) = existing {
                Self::merge_sections(existing_section, section)?;
            } else {
                target.data_sections.push(section.clone());
            }
        }
        Ok(())
    }
    fn merge_symbol_tables(
        target: &mut crate::dna::hel::binary::SymbolTable,
        source: &crate::dna::hel::binary::SymbolTable,
    ) {
        for string in &source.strings {
            if !target.strings.contains(string) {
                let id = target.strings.len() as u32;
                target.strings.push(string.clone());
                target.string_map.insert(string.clone(), id);
            }
        }
        for (name, id) in &source.agents {
            if !target.agents.contains_key(name) {
                target.agents.insert(name.clone(), *id);
            }
        }
        for (name, id) in &source.workflows {
            if !target.workflows.contains_key(name) {
                target.workflows.insert(name.clone(), *id);
            }
        }
        for (name, id) in &source.contexts {
            if !target.contexts.contains_key(name) {
                target.contexts.insert(name.clone(), *id);
            }
        }
        for (name, id) in &source.crews {
            if !target.crews.contains_key(name) {
                target.crews.insert(name.clone(), *id);
            }
        }
    }
    fn merge_sections(
        target: &mut DataSection,
        source: &DataSection,
    ) -> Result<(), CompileError> {
        target.data.extend_from_slice(&source.data);
        target.size += source.size;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bundler_creation() {
        let bundler = Bundler::new();
        assert_eq!(bundler.include_patterns, vec!["*.hlx"]);
        assert!(bundler.exclude_patterns.is_empty());
        assert!(bundler.follow_imports);
        assert!(! bundler.tree_shake);
    }
    #[test]
    fn test_pattern_matching() {
        let bundler = Bundler::new();
        assert!(bundler.matches_pattern("config.hlx", "*.hlx"));
        assert!(bundler.matches_pattern("test.hlx", "*.hlx"));
        assert!(! bundler.matches_pattern("config.txt", "*.hlx"));
        assert!(bundler.matches_pattern("exact.hlx", "exact.hlx"));
    }
    #[test]
    fn test_bundler_builder() {
        let bundler = Bundler::new()
            .include("*.hlx")
            .exclude("test_*.hlx")
            .with_tree_shaking(true)
            .verbose(true);
        assert_eq!(bundler.include_patterns.len(), 2);
        assert_eq!(bundler.exclude_patterns.len(), 1);
        assert!(bundler.tree_shake);
        assert!(bundler.verbose);
    }
}

pub fn bundle_command(
    directory: PathBuf,
    output: PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
    tree_shake: bool,
    optimize: u8,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("📦 Bundling directory: {}", directory.display());
        if !include.is_empty() {
            println!("  Include patterns: {:?}", include);
        }
        if !exclude.is_empty() {
            println!("  Exclude patterns: {:?}", exclude);
        }
        println!("  Tree shaking: {}", if tree_shake { "Enabled" } else { "Disabled" });
    }
    let mut bundler = Bundler::new().with_tree_shaking(tree_shake).verbose(verbose);
    for pattern in include {
        bundler = bundler.include(&pattern);
    }
    for pattern in exclude {
        bundler = bundler.exclude(&pattern);
    }
    let binary = bundler
        .bundle_directory(&directory, OptimizationLevel::from(optimize))?;
    let serializer = crate::mds::serializer::BinarySerializer::new(true);
    serializer.write_to_file(&binary, &output)?;
    println!("✅ Bundle created: {}", output.display());
    println!("  Size: {} bytes", binary.size());
    if let Some(file_count) = binary.metadata.extra.get("bundle_files") {
        println!("  Files bundled: {}", file_count);
    }
    Ok(())
}