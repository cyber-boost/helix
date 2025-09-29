use crate::dna::atp::parser::parse;
use crate::dna::atp::lexer::tokenize;
use crate::dna::atp::ast::Declaration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string("test_simple.hlxbb")?;
    let tokens = tokenize(&content)?;
    let ast = parse(tokens).map_err(|e| format!("Parse error: {:?}", e))?;
    println!("AST has {} declarations:", ast.declarations.len());
    for (i, decl) in ast.declarations.iter().enumerate() {
        match decl {
            Declaration::Agent(agent) => {
                println!("  {}: Agent '{}'", i, agent.name);
            }
            Declaration::Workflow(workflow) => {
                println!("  {}: Workflow '{}'", i, workflow.name);
            }
            Declaration::Project(project) => {
                println!("  {}: Project '{}'", i, project.name);
            }
            _ => {
                println!("  {}: Other declaration", i);
            }
        }
    }
    Ok(())
}