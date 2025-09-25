use helix::parse;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string("test_simple.hlxbb")?;
    let ast = parse(&content)?;
    println!("AST has {} declarations:", ast.declarations.len());
    for (i, decl) in ast.declarations.iter().enumerate() {
        match decl {
            helix::ast::Declaration::Agent(agent) => {
                println!("  {}: Agent '{}'", i, agent.name);
            }
            helix::ast::Declaration::Workflow(workflow) => {
                println!("  {}: Workflow '{}'", i, workflow.name);
            }
            helix::ast::Declaration::Project(project) => {
                println!("  {}: Project '{}'", i, project.name);
            }
            _ => {
                println!("  {}: Other declaration", i);
            }
        }
    }
    Ok(())
}