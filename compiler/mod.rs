pub mod binary;
pub mod optimizer;
pub mod serializer;
pub mod loader;
pub mod bundle;
pub mod modules;
pub mod runtime;
pub mod workflow;
#[cfg(feature = "cli")]
pub mod cli;
pub mod tools;
pub use binary::{HelixBinary, BinaryFlags, BinaryMetadata, DataSection, SectionType};
pub use optimizer::{Optimizer, OptimizationLevel};
pub use serializer::BinarySerializer;
pub use loader::BinaryLoader;
pub use bundle::Bundler;
pub use modules::{ModuleSystem, DependencyBundler, ModuleResolver};
pub use runtime::{HelixVM, VMExecutor, VMConfig};
use crate::{parse, validate, ast::HelixAst, codegen::{CodeGenerator, HelixIR}};
use std::path::{Path, PathBuf};
use std::fs;
use std::time::Instant;
#[derive(Clone)]
pub struct Compiler {
    optimization_level: OptimizationLevel,
    enable_compression: bool,
    enable_cache: bool,
    verbose: bool,
    cache_dir: Option<PathBuf>,
}
impl Compiler {
    pub fn new(optimization_level: OptimizationLevel) -> Self {
        Self {
            optimization_level,
            enable_compression: true,
            enable_cache: true,
            verbose: false,
            cache_dir: None,
        }
    }
    pub fn builder() -> CompilerBuilder {
        CompilerBuilder::default()
    }
    pub fn compile_file<P: AsRef<Path>>(
        &self,
        input: P,
    ) -> Result<HelixBinary, CompileError> {
        let start = Instant::now();
        let path = input.as_ref();
        if self.verbose {
            println!("Compiling: {}", path.display());
        }
        if self.enable_cache {
            if let Some(cached) = self.check_cache(path)? {
                if self.verbose {
                    println!("  Using cached version");
                }
                return Ok(cached);
            }
        }
        let source = fs::read_to_string(path)
            .map_err(|e| CompileError::IoError(format!("Failed to read file: {}", e)))?;
        let binary = self.compile_source(&source, Some(path))?;
        if self.enable_cache {
            self.cache_binary(path, &binary)?;
        }
        if self.verbose {
            let elapsed = start.elapsed();
            println!("  Compiled in {:?}", elapsed);
        }
        Ok(binary)
    }
    pub fn compile_source(
        &self,
        source: &str,
        source_path: Option<&Path>,
    ) -> Result<HelixBinary, CompileError> {
        let ast = parse(source).map_err(|e| CompileError::ParseError(e.to_string()))?;
        validate(&ast).map_err(|e| CompileError::ValidationError(e))?;
        let mut generator = CodeGenerator::new();
        let ir = generator.generate(&ast);
        let optimized_ir = self.optimize_ir(ir);
        let binary = self.ir_to_binary(optimized_ir, source_path)?;
        Ok(binary)
    }
    pub fn compile_bundle<P: AsRef<Path>>(
        &self,
        directory: P,
    ) -> Result<HelixBinary, CompileError> {
        let bundler = Bundler::new();
        bundler.bundle_directory(directory, self.optimization_level)
    }
    pub fn decompile(&self, binary: &HelixBinary) -> Result<String, CompileError> {
        let ast = self.binary_to_ast(binary)?;
        Ok(crate::pretty_print(&ast))
    }
    fn optimize_ir(&self, mut ir: HelixIR) -> HelixIR {
        let mut optimizer = Optimizer::new(self.optimization_level);
        optimizer.optimize(&mut ir);
        ir
    }
    fn ir_to_binary(
        &self,
        ir: HelixIR,
        source_path: Option<&Path>,
    ) -> Result<HelixBinary, CompileError> {
        let serializer = BinarySerializer::new(self.enable_compression)
            .with_compression_method(
                if self.enable_compression {
                    binary::CompressionMethod::Lz4
                } else {
                    binary::CompressionMethod::None
                },
            );
        let binary = serializer
            .serialize(ir, source_path)
            .map_err(|e| CompileError::SerializationError(e.to_string()))?;
        Ok(binary)
    }
    fn binary_to_ast(&self, binary: &HelixBinary) -> Result<HelixAst, CompileError> {
        let deserializer = BinarySerializer::new(false);
        let ir = deserializer
            .deserialize_to_ir(binary)
            .map_err(|e| CompileError::DeserializationError(e.to_string()))?;
        ir_to_ast(ir)
    }
    fn check_cache(
        &self,
        source_path: &Path,
    ) -> Result<Option<HelixBinary>, CompileError> {
        if let Some(cache_dir) = &self.cache_dir {
            let cache_path = cache_path_for(cache_dir, source_path);
            if cache_path.exists() {
                let source_modified = fs::metadata(source_path)
                    .and_then(|m| m.modified())
                    .map_err(|e| CompileError::IoError(e.to_string()))?;
                let cache_modified = fs::metadata(&cache_path)
                    .and_then(|m| m.modified())
                    .map_err(|e| CompileError::IoError(e.to_string()))?;
                if cache_modified > source_modified {
                    let serializer = BinarySerializer::new(false);
                    return serializer
                        .read_from_file(&cache_path)
                        .map(Some)
                        .map_err(|e| CompileError::CacheError(e.to_string()));
                }
            }
        }
        Ok(None)
    }
    fn cache_binary(
        &self,
        source_path: &Path,
        binary: &HelixBinary,
    ) -> Result<(), CompileError> {
        if let Some(cache_dir) = &self.cache_dir {
            let cache_path = cache_path_for(cache_dir, source_path);
            if let Some(parent) = cache_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| CompileError::IoError(e.to_string()))?;
            }
            let serializer = BinarySerializer::new(self.enable_compression);
            serializer
                .write_to_file(binary, &cache_path)
                .map_err(|e| CompileError::CacheError(e.to_string()))?;
        }
        Ok(())
    }
}
#[derive(Default)]
pub struct CompilerBuilder {
    optimization_level: OptimizationLevel,
    enable_compression: bool,
    enable_cache: bool,
    verbose: bool,
    cache_dir: Option<PathBuf>,
}
impl CompilerBuilder {
    pub fn optimization_level(mut self, level: OptimizationLevel) -> Self {
        self.optimization_level = level;
        self
    }
    pub fn compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }
    pub fn cache(mut self, enable: bool) -> Self {
        self.enable_cache = enable;
        self
    }
    pub fn cache_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.cache_dir = Some(dir.into());
        self
    }
    pub fn verbose(mut self, enable: bool) -> Self {
        self.verbose = enable;
        self
    }
    pub fn build(self) -> Compiler {
        Compiler {
            optimization_level: self.optimization_level,
            enable_compression: self.enable_compression,
            enable_cache: self.enable_cache,
            verbose: self.verbose,
            cache_dir: self.cache_dir,
        }
    }
}
#[derive(Debug)]
pub enum CompileError {
    IoError(String),
    ParseError(String),
    ValidationError(String),
    OptimizationError(String),
    SerializationError(String),
    DeserializationError(String),
    CacheError(String),
}
impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "I/O error: {}", e),
            Self::ParseError(e) => write!(f, "Parse error: {}", e),
            Self::ValidationError(e) => write!(f, "Validation error: {}", e),
            Self::OptimizationError(e) => write!(f, "Optimization error: {}", e),
            Self::SerializationError(e) => write!(f, "Serialization error: {}", e),
            Self::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
            Self::CacheError(e) => write!(f, "Cache error: {}", e),
        }
    }
}
impl std::error::Error for CompileError {}
fn ir_to_ast(ir: HelixIR) -> Result<HelixAst, CompileError> {
    let mut declarations = Vec::new();
    for (_id, agent) in ir.symbol_table.agents {
        let name = ir
            .string_pool
            .get(agent.name_idx)
            .unwrap_or(&format!("agent_{}", agent.id))
            .clone();
        declarations
            .push(
                crate::ast::Declaration::Agent(crate::ast::AgentDecl {
                    name,
                    properties: std::collections::HashMap::new(),
                    capabilities: None,
                    backstory: None,
                    tools: None,
                }),
            );
    }
    Ok(HelixAst { declarations })
}
fn cache_path_for(cache_dir: &Path, source_path: &Path) -> PathBuf {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::{Hash, Hasher};
    source_path.hash(&mut hasher);
    let hash = hasher.finish();
    cache_dir.join(format!("{:x}.hlxb", hash))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_compiler_builder() {
        let compiler = Compiler::builder()
            .optimization_level(OptimizationLevel::Two)
            .compression(true)
            .cache(false)
            .verbose(true)
            .build();
        assert_eq!(compiler.optimization_level, OptimizationLevel::Two);
        assert!(compiler.enable_compression);
        assert!(! compiler.enable_cache);
        assert!(compiler.verbose);
    }
    #[test]
    fn test_compile_simple() {
        let source = r#"
            agent "test" {
                model = "gpt-4"
                temperature = 0.7
            }
        "#;
        let compiler = Compiler::new(OptimizationLevel::Zero);
        let result = compiler.compile_source(source, None);
        assert!(result.is_ok());
    }
}