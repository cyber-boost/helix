use std::fs;
use helix::{parse, ast::*, semantic::SemanticAnalyzer};
fn main() {
    println!("🔍 HELIX Semantic Analyzer Debug");
    println!("================================");
    let content = fs::read_to_string("/tmp/simple_agent.hlxbb")
        .expect("Failed to read test file");
    println!("📄 File content:");
    println!("{}", content);
    println!();
    let ast = match parse(&content) {
        Ok(ast) => {
            println!("✅ Parsing successful");
            ast
        }
        Err(e) => {
            println!("❌ Parsing failed: {:?}", e);
            return;
        }
    };
    println!("📊 AST Analysis:");
    println!("  Declarations: {}", ast.declarations.len());
    for (i, decl) in ast.declarations.iter().enumerate() {
        match decl {
            Declaration::Agent(agent) => {
                println!("  [{}] Agent: {}", i, agent.name);
            }
            Declaration::Workflow(workflow) => {
                println!("  [{}] Workflow: {}", i, workflow.name);
            }
            Declaration::Context(context) => {
                println!("  [{}] Context: {}", i, context.name);
            }
            Declaration::Crew(crew) => {
                println!("  [{}] Crew: {}", i, crew.name);
            }
            Declaration::Memory(mem) => {
                println!("  [{}] Memory: {}", i, mem.provider);
            }
            _ => {
                println!("  [{}] Other declaration", i);
            }
        }
    }
    println!();
    let mut analyzer = SemanticAnalyzer::new();
    println!("🔍 Checking semantic analyzer state before analysis:");
    println!("  Agents in map: {}", analyzer.agents.len());
    println!("  Workflows in map: {}", analyzer.workflows.len());
    println!("  Contexts in map: {}", analyzer.contexts.len());
    println!("  Crews in map: {}", analyzer.crews.len());
    println!();
    println!("🚀 Running semantic analysis...");
    match analyzer.analyze(&ast) {
        Ok(_) => {
            println!("✅ Semantic analysis passed");
        }
        Err(errors) => {
            println!("❌ Semantic analysis failed with {} errors:", errors.len());
            for error in errors {
                println!("  - {}", error);
            }
        }
    }
    println!();
    println!("🔍 Final semantic analyzer state:");
    println!("  Agents in map: {}", analyzer.agents.len());
    println!("  Workflows in map: {}", analyzer.workflows.len());
    println!("  Contexts in map: {}", analyzer.contexts.len());
    println!("  Crews in map: {}", analyzer.crews.len());
    if !analyzer.agents.is_empty() {
        println!("  Agent names:");
        for name in analyzer.agents.keys() {
            println!("    - {}", name);
        }
    }
}