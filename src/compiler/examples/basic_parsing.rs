use helix_core::{parse, validate, load_file, ast_to_config, HelixConfig, HelixAst};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 HELIX Language - Basic Parsing Example\n");
    println!("1. Parsing from string:");
    let source = r#"
        agent 'assistant' {
            model = 'gpt-4'
            temperature = 0.7
            max_tokens = 2000
        }
    "#;
    let ast = parse(source)?;
    println!("✅ Successfully parsed AST with {} declarations", ast.declarations.len());
    println!("\n2. Validating AST:");
    validate(&ast)?;
    println!("✅ AST validation passed");
    println!("\n3. Converting to configuration:");
    let config = ast_to_config(ast)?;
    println!("✅ Configuration created with {} agents", config.agents.len());
    println!("\n4. Loading from file:");
    let example_file = "examples/minimal.hlx";
    if Path::new(example_file).exists() {
        let file_config = load_file(example_file)?;
        println!("✅ Loaded configuration from {}", example_file);
        println!("   - Agents: {}", file_config.agents.len());
        println!("   - Workflows: {}", file_config.workflows.len());
    } else {
        println!("⚠️  Example file {} not found, skipping file load", example_file);
    }
    println!("\n5. Pretty printing AST:");
    let pretty = helix_core::pretty_print(&config);
    println!("{}", pretty);
    println!("\n🎉 Basic parsing example completed successfully!");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic_parsing() {
        let source = "agent 'test' { model = 'gpt-3.5-turbo' }";
        let ast = parse(source).expect("Should parse successfully");
        validate(&ast).expect("Should validate successfully");
    }
    #[test]
    fn test_config_conversion() {
        let source = r#"
            agent 'test' {
                model = 'gpt-4'
                temperature = 0.5
            }
        "#;
        let ast = parse(source).expect("Should parse successfully");
        let config = ast_to_config(ast).expect("Should convert successfully");
        assert_eq!(config.agents.len(), 1);
    }
}