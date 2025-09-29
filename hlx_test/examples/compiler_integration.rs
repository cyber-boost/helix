#[cfg(feature = "compiler")]
use helix::compiler::{Compiler, OptimizationLevel, BinarySerializer, BinaryLoader};
use helix::{parse, validate, ast_to_config, HelixConfig};
use std::path::Path;
#[cfg(feature = "compiler")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 HELIX Language - Compiler Integration Example\n");
    println!("1. Creating compiler with optimization:");
    let compiler = Compiler::new(OptimizationLevel::Two);
    println!("✅ Compiler created with optimization level 2");
    println!("\n2. Parsing and compiling source:");
    let source = r#"
        agent 'ai_assistant' {
            model = 'gpt-4'
            temperature = 0.7
            max_tokens = 2000
            system_prompt = 'You are a helpful AI assistant'
        }
        
        workflow 'chat_workflow' {
            agent = 'ai_assistant'
            steps = [
                { action = 'receive_message' }
                { action = 'process_with_ai' }
                { action = 'send_response' }
            ]
        }
    "#;
    let ast = parse(source)?;
    validate(&ast)?;
    println!("✅ Source parsed and validated");
    println!("\n3. Compiling to binary:");
    let binary = compiler.compile_source(&source, None)?;
    println!("✅ Compiled to binary ({} bytes)", binary.size());
    println!("\n4. Serializing to file:");
    let serializer = BinarySerializer::new(true);
    let output_path = "examples/compiled_example.hlxb";
    serializer.write_to_file(&binary, Path::new(output_path))?;
    println!("✅ Binary written to {}", output_path);
    println!("\n5. Loading and verifying binary:");
    let loader = BinaryLoader::new();
    let loaded_binary = loader.load_file(output_path)?;
    println!("✅ Binary loaded successfully ({} bytes)", loaded_binary.size());
    println!("\n6. Decompiling binary:");
    let decompiled_source = compiler.decompile(&loaded_binary)?;
    println!(
        "✅ Binary decompiled to source ({} characters)", decompiled_source.len()
    );
    println!("\n7. Verifying round-trip compilation:");
    let original_config = ast_to_config(ast)?;
    let decompiled_ast = parse(&decompiled_source)?;
    let decompiled_config = ast_to_config(decompiled_ast)?;
    println!("   - Original agents: {}", original_config.agents.len());
    println!("   - Decompiled agents: {}", decompiled_config.agents.len());
    println!("   - Original workflows: {}", original_config.workflows.len());
    println!("   - Decompiled workflows: {}", decompiled_config.workflows.len());
    let _ = std::fs::remove_file(output_path);
    println!("\n🎉 Compiler integration example completed successfully!");
    Ok(())
}
#[cfg(not(feature = "compiler"))]
fn main() {
    println!(
        "⚠️  Compiler features not enabled. Run with: cargo run --example compiler_integration --features compiler"
    );
    println!("   This example requires the 'compiler' feature to be enabled.");
}
#[cfg(feature = "compiler")]
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_compiler_round_trip() {
        let source = "agent 'test' { model = 'gpt-4' }";
        let ast = parse(source).expect("Should parse successfully");
        let compiler = Compiler::new(OptimizationLevel::One);
        let binary = compiler.compile_ast(&ast).expect("Should compile successfully");
        let decompiled = compiler
            .decompile_binary(&binary)
            .expect("Should decompile successfully");
        assert_eq!(ast.declarations.len(), decompiled.declarations.len());
    }
    #[test]
    fn test_serialization() {
        let source = "agent 'test' { model = 'gpt-3.5-turbo' }";
        let ast = parse(source).expect("Should parse successfully");
        let compiler = Compiler::new(OptimizationLevel::Zero);
        let binary = compiler.compile_ast(&ast).expect("Should compile successfully");
        let serializer = BinarySerializer::new();
        let temp_path = std::env::temp_dir().join("test_serialization.hlxb");
        serializer
            .write_to_file(&binary, &temp_path)
            .expect("Should serialize successfully");
        let loader = BinaryLoader::new();
        let loaded = loader.load_file(&temp_path).expect("Should load successfully");
        assert_eq!(binary.len(), loaded.len());
        let _ = std::fs::remove_file(&temp_path);
    }
}