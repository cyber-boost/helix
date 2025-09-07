#[cfg(feature = "compiler")]
use helix_core::compiler::{BinaryLoader, Compiler, OptimizationLevel};
use helix_core::{parse, validate, ast_to_config, HelixConfig};
use std::path::Path;
#[cfg(feature = "compiler")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 HELIX Language - Real Binary Loading Example\n");
    println!("1. Loading real compiled binary files:");
    let loader = BinaryLoader::new();
    let binary_files = vec![
        "binaries/minimal_opt_zero.hlxb", "binaries/minimal_opt_one.hlxb",
        "binaries/minimal_opt_two.hlxb", "binaries/minimal_opt_three.hlxb",
    ];
    for binary_file in &binary_files {
        if Path::new(binary_file).exists() {
            match loader.load_file(binary_file) {
                Ok(binary) => {
                    println!("✅ Loaded {} ({} bytes)", binary_file, binary.size());
                    println!("   - Binary size: {} bytes", binary.size());
                    println!("   - File exists and is loadable");
                }
                Err(e) => {
                    println!("❌ Failed to load {}: {}", binary_file, e);
                }
            }
        } else {
            println!("⚠️  File {} does not exist", binary_file);
        }
    }
    println!("\n2. Loading and decompiling real binary:");
    let compiler = Compiler::new(OptimizationLevel::Zero);
    let test_binary = "test_fresh.hlxb";
    if Path::new(test_binary).exists() {
        match loader.load_file(test_binary) {
            Ok(binary) => {
                println!("✅ Loaded binary: {} ({} bytes)", test_binary, binary.size());
                match compiler.decompile(&binary) {
                    Ok(source) => {
                        println!("✅ Successfully decompiled to source");
                        println!("   - Source length: {} characters", source.len());
                        match parse(&source) {
                            Ok(ast) => {
                                println!("✅ Parsed decompiled source to AST");
                                println!("   - Declarations: {}", ast.declarations.len());
                                match ast_to_config(ast) {
                                    Ok(config) => {
                                        println!("✅ Converted to configuration:");
                                        println!("   - Agents: {}", config.agents.len());
                                        println!("   - Workflows: {}", config.workflows.len());
                                        println!("   - Projects: {}", config.projects.len());
                                        for (name, agent) in &config.agents {
                                            println!(
                                                "   - Agent '{}': model={:?}, temperature={:?}", name, agent
                                                .model, agent.temperature
                                            );
                                        }
                                    }
                                    Err(e) => println!("❌ Failed to convert to config: {}", e),
                                }
                            }
                            Err(e) => {
                                println!("❌ Failed to parse decompiled source: {}", e)
                            }
                        }
                    }
                    Err(e) => println!("❌ Failed to decompile: {}", e),
                }
            }
            Err(e) => println!("❌ Failed to load binary: {}", e),
        }
    } else {
        println!("⚠️  Test binary {} does not exist", test_binary);
    }
    println!("\n3. Comparing optimization levels:");
    let optimization_files = vec![
        ("Zero", "binaries/minimal_opt_zero.hlxb"), ("One",
        "binaries/minimal_opt_one.hlxb"), ("Two", "binaries/minimal_opt_two.hlxb"),
        ("Three", "binaries/minimal_opt_three.hlxb"),
    ];
    for (level_name, file_path) in &optimization_files {
        if Path::new(file_path).exists() {
            match loader.load_file(file_path) {
                Ok(binary) => {
                    println!("✅ {}: {} bytes", level_name, binary.size());
                }
                Err(e) => {
                    println!("❌ {}: Failed to load - {}", level_name, e);
                }
            }
        } else {
            println!("⚠️  {}: File does not exist", level_name);
        }
    }
    println!("\n4. Real usage - Loading and using configuration:");
    let source_file = "examples/minimal.hlxb";
    if Path::new(source_file).exists() {
        println!("Loading source file: {}", source_file);
        let source_content = std::fs::read_to_string(source_file)?;
        println!("✅ Read source file ({} characters)", source_content.len());
        let ast = parse(&source_content)?;
        println!("✅ Parsed AST with {} declarations", ast.declarations.len());
        validate(&ast)?;
        println!("✅ AST validation passed");
        let config = ast_to_config(ast)?;
        println!("✅ Configuration created:");
        println!("   - Projects: {}", config.projects.len());
        println!("   - Agents: {}", config.agents.len());
        println!("   - Workflows: {}", config.workflows.len());
        for (name, agent) in &config.agents {
            println!("   - Agent '{}':", name);
            println!("     * Model: {:?}", agent.model);
            println!("     * Role: {:?}", agent.role);
            println!("     * Temperature: {:?}", agent.temperature);
            println!("     * Max tokens: {:?}", agent.max_tokens);
        }
        for (name, workflow) in &config.workflows {
            println!("   - Workflow '{}':", name);
            println!("     * Trigger: {:?}", workflow.trigger);
            println!("     * Steps: {}", workflow.steps.len());
            for (i, step) in workflow.steps.iter().enumerate() {
                println!(
                    "       {}. Agent: {:?}, Task: {:?}", i + 1, step.agent, step.task
                );
            }
        }
        println!("\n5. Real application usage example:");
        simulate_ai_workflow(&config)?;
    } else {
        println!("⚠️  Source file {} does not exist", source_file);
    }
    println!("\n🎉 Real binary loading example completed successfully!");
    Ok(())
}
#[cfg(not(feature = "compiler"))]
fn main() {
    println!(
        "⚠️  Compiler features not enabled. Run with: cargo run --example load --features compiler"
    );
    println!("   This example requires the 'compiler' feature to be enabled.");
}
fn simulate_ai_workflow(config: &HelixConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Simulating AI workflow with loaded configuration:");
    if let Some((workflow_name, workflow)) = config.workflows.iter().next() {
        println!("   Executing workflow: {}", workflow_name);
        for (i, step) in workflow.steps.iter().enumerate() {
            println!("   Step {}: {}", i + 1, & step.task);
            if let Some(agent_name) = &step.agent {
                if let Some(agent) = config.agents.get(agent_name) {
                    println!(
                        "     Using agent: {} (model: {:?})", agent_name, agent.model
                    );
                    let model = &agent.model;
                    match model.as_str() {
                        "gpt-3.5-turbo" => {
                            println!("     → GPT-3.5 Turbo processing...")
                        }
                        "gpt-4" => println!("     → GPT-4 processing..."),
                        "claude-3-opus" => {
                            println!("     → Claude-3 Opus processing...")
                        }
                        _ => println!("     → {} processing...", model),
                    }
                    if let Some(temp) = agent.temperature {
                        if temp > 0.8 {
                            println!("     → High creativity mode (temp: {})", temp);
                        } else if temp < 0.3 {
                            println!("     → Precise mode (temp: {})", temp);
                        } else {
                            println!("     → Balanced mode (temp: {})", temp);
                        }
                    }
                } else {
                    println!(
                        "     ❌ Agent '{}' not found in configuration", agent_name
                    );
                }
            }
        }
        println!("   ✅ Workflow '{}' completed successfully", workflow_name);
    } else {
        println!("   ⚠️  No workflows found in configuration");
    }
    Ok(())
}
#[cfg(feature = "compiler")]
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load_real_binary() {
        let loader = BinaryLoader::new();
        let binary_file = "test_fresh.hlxb";
        if Path::new(binary_file).exists() {
            let result = loader.load_file(binary_file);
            assert!(result.is_ok(), "Should be able to load real binary file");
            let binary = result.unwrap();
            assert!(binary.size() > 0, "Binary should have content");
        }
    }
    #[test]
    fn test_decompile_real_binary() {
        let loader = BinaryLoader::new();
        let compiler = Compiler::new(OptimizationLevel::Zero);
        let binary_file = "test_fresh.hlxb";
        if Path::new(binary_file).exists() {
            let binary = loader.load_file(binary_file).expect("Should load binary");
            let source = compiler.decompile(&binary).expect("Should decompile");
            let ast = parse(&source).expect("Should parse decompiled source");
            let config = ast_to_config(ast).expect("Should convert to config");
            assert!(
                config.agents.len() > 0 || config.workflows.len() > 0,
                "Should have some configuration data"
            );
        }
    }
    #[test]
    fn test_load_source_file() {
        let source_file = "examples/minimal.hlxb";
        if Path::new(source_file).exists() {
            let content = std::fs::read_to_string(source_file)
                .expect("Should read file");
            let ast = parse(&content).expect("Should parse");
            let config = ast_to_config(ast).expect("Should convert");
            assert!(config.projects.len() > 0, "Should have project configuration");
        }
    }
}