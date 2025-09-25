use helix::{parse, validate, load_file, ast_to_config};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 HELIX Language - Basic Parsing Example\n");
    println!("1. Parsing from string:");
    let source = r#"
        agent "assistant" {
            model = "gpt-4"
            temperature = 0.7
            max_tokens = 2000
        }
    "#;
    let ast = parse(source)?;
    println!("✅ Successfully parsed AST with {} declarations", ast.declarations.len());

    println!("\n2. Testing scientific notation parsing:");
    let scientific_source = r#"
        optimizer {
            type = "AdamW"
            learning_rate = 5e-5
            weight_decay = 0.01
            betas = [0.9, 0.999]
            epsilon = 1e-8
            adam_w_mode = true
        }
    "#;
    let scientific_ast = parse(scientific_source)?;
    println!("✅ Successfully parsed scientific notation with {} declarations", scientific_ast.declarations.len());

    println!("\n3. Testing positive numbers:");
    let positive_source = r#"
        agent "test" {
            temperature = +0.5
            max_tokens = +1000
            rate = +1.5e10
        }
    "#;
    let positive_ast = parse(positive_source)?;
    println!("✅ Successfully parsed positive numbers with {} declarations", positive_ast.declarations.len());

    println!("\n4. Validating AST:");
    validate(&ast)?;
    println!("✅ AST validation passed");

    println!("\n5. Converting to configuration:");
    let config = ast_to_config(ast.clone())?;
    println!("✅ Configuration created with {} agents", config.agents.len());

    println!("\n6. Loading from file:");
    let example_file = "examples/minimal.hlx";
    if Path::new(example_file).exists() {
        let file_config = load_file(example_file)?;
        println!("✅ Loaded configuration from {}", example_file);
        println!("   - Agents: {}", file_config.agents.len());
        println!("   - Workflows: {}", file_config.workflows.len());
    } else {
        println!("⚠️  Example file {} not found, skipping file load", example_file);
    }

    println!("\n7. Pretty printing AST:");
    let pretty = helix::pretty_print(&ast);
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