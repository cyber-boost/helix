mod lexer;
mod parser;
mod ast;
mod semantic;
mod codegen;
mod types;
pub mod json;
pub mod error;
pub mod hlx;
pub mod server;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod benches;
#[cfg(test)]
mod integration;
#[cfg(test)]
#[path = "tests/integration_tests.rs"]
mod integration_tests;
#[cfg(feature = "compiler")]
pub mod compiler;
pub use types::{
    HelixConfig, ProjectConfig, AgentConfig, WorkflowConfig, MemoryConfig, ContextConfig,
    CrewConfig, PipelineConfig, RetryConfig, TriggerConfig, StepConfig, Value,
    load_default_config,
};
pub use ast::{
    HelixAst, Declaration, Expression, Statement, AgentDecl, WorkflowDecl, MemoryDecl,
    ContextDecl, CrewDecl, PipelineDecl,
};
pub use lexer::{Token, SourceLocation};
pub use parser::{Parser, ParseError};
pub use semantic::{SemanticAnalyzer, SemanticError};
pub use codegen::{CodeGenerator, HelixIR};
#[cfg(feature = "compiler")]
pub use compiler::optimizer::OptimizationLevel;
pub use types::HelixLoader;
pub use server::{HelixServer, ServerConfig};
use std::path::Path;
pub fn parse(source: &str) -> Result<HelixAst, ParseError> {
    parse_with_locations(source).or_else(|_| parse_legacy(source))
}
pub fn parse_with_locations(source: &str) -> Result<HelixAst, ParseError> {
    use lexer::{tokenize_with_locations, SourceMap};
    let tokens_with_loc = tokenize_with_locations(source)
        .map_err(|e| ParseError {
            message: format!("Lexer error: {}", e),
            location: None,
            token_index: 0,
            expected: None,
            found: String::new(),
            context: String::new(),
        })?;
    let source_map = SourceMap {
        tokens: tokens_with_loc.clone(),
        source: source.to_string(),
    };
    let mut parser = Parser::new_with_source_map(source_map);
    parser
        .parse()
        .map_err(|msg| ParseError {
            message: msg,
            location: None,
            token_index: 0,
            expected: None,
            found: String::new(),
            context: String::new(),
        })
}
fn parse_legacy(source: &str) -> Result<HelixAst, ParseError> {
    let tokens = lexer::tokenize(source)
        .map_err(|e| ParseError {
            message: format!("Lexer error: {}", e),
            location: None,
            token_index: 0,
            expected: None,
            found: String::new(),
            context: String::new(),
        })?;
    let mut parser = Parser::new(tokens);
    parser
        .parse()
        .map_err(|msg| ParseError {
            message: msg,
            location: None,
            token_index: 0,
            expected: None,
            found: String::new(),
            context: String::new(),
        })
}
pub fn parse_and_validate(source: &str) -> Result<HelixConfig, String> {
    let ast = parse(source).map_err(|e| e.to_string())?;
    validate(&ast)?;
    ast_to_config(ast)
}
pub fn validate(ast: &HelixAst) -> Result<(), String> {
    let mut analyzer = SemanticAnalyzer::new();
    analyzer
        .analyze(ast)
        .map_err(|errors| {
            errors.iter().map(|e| format!("{:?}", e)).collect::<Vec<_>>().join("\n")
        })
}
pub fn ast_to_config(ast: HelixAst) -> Result<HelixConfig, String> {
    let loader = types::HelixLoader::new();
    loader.ast_to_config(ast).map_err(|e| e.to_string())
}
pub fn load_file<P: AsRef<Path>>(path: P) -> Result<HelixConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    parse_and_validate(&content)
}
pub fn load_directory<P: AsRef<Path>>(path: P) -> Result<Vec<HelixConfig>, String> {
    let mut configs = Vec::new();
    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("HELIX") {
            let config = load_file(&path)?;
            configs.push(config);
        }
    }
    Ok(configs)
}
pub fn pretty_print(ast: &HelixAst) -> String {
    let mut printer = ast::AstPrettyPrinter::new();
    printer.print(ast)
}
#[cfg(feature = "compiler")]
pub use compiler::tools::migrate::Migrator;
#[cfg(feature = "compiler")]
pub use compiler::{
    ModuleSystem, DependencyBundler, ModuleResolver, HelixVM, VMExecutor, VMConfig,
};
#[cfg(feature = "cli")]
pub use compiler::workflow::watch::{HelixWatcher, CompileWatcher, HotReloadManager};
pub use hlx::{HlxDatasetProcessor, HlxBridge, DatasetConfig, ValidationResult, CacheStats};