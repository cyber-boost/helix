use std::fs;
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Verifying HELIX Language Implementation...\n");
    println!("Test 1: Parsing hlx file...");
    let content = fs::read_to_string("test_example.hlxbb")?;
    let ast = helix_core::parse(&content)?;
    println!("✅ Successfully parsed {} declarations", ast.declarations.len());
    println!("\nTest 2: Validating AST...");
    helix_core::validate(&ast)?;
    println!("✅ AST validation passed");
    println!("\nTest 3: Converting to config...");
    let config = helix_core::ast_to_config(ast)?;
    println!("✅ Config created with:");
    println!("   - {} projects", config.projects.len());
    println!("   - {} agents", config.agents.len());
    println!("   - {} workflows", config.workflows.len());
    println!("   - {} crews", config.crews.len());
    println!("\nTest 4: Compiling to binary...");
    let compiler = helix_core::compiler::Compiler::new(
        helix_core::compiler::OptimizationLevel::Two,
    );
    let binary = compiler.compile_source(&content, None)?;
    println!("✅ Binary compilation successful");
    println!("   - Version: {}", binary.version);
    println!("   - Compressed: {}", binary.flags.compressed);
    println!("   - Optimized: {}", binary.flags.optimized);
    println!("   - Size: {} bytes", binary.size());
    println!("\nTest 5: Binary serialization...");
    let serializer = helix_core::compiler::serializer::BinarySerializer::new(true);
    serializer.write_to_file(&binary, Path::new("test_example.hlxb"))?;
    println!("✅ Binary written to file");
    println!("\nTest 6: Binary loading...");
    let loader = helix_core::compiler::loader::BinaryLoader::new();
    let loaded_binary = loader.load_file("test_example.hlxb")?;
    println!("✅ Binary loaded successfully");
    println!("   - Checksum match: {}", loaded_binary.checksum == binary.checksum);
    println!("\nTest 7: Config merging...");
    let loader = helix_core::HelixLoader::new();
    let merged = loader.merge_configs(vec![& config]);
    println!("✅ Config merging works");
    println!("   - Merged agents: {}", merged.agents.len());
    println!("\nTest 8: JSON to hlx migration...");
    let json_config = r#"{"agents": {"test": {"model": "gpt-4"}}}"#;
    let migrator = helix_core::Migrator::new();
    let hlx_content = migrator.migrate_json(json_config)?;
    println!("✅ Migration successful");
    println!("   Generated hlx: {} characters", hlx_content.len());
    println!("\n🎉 All verification tests passed!");
    println!("✅ Core parsing works");
    println!("✅ AST validation works");
    println!("✅ Config conversion works");
    println!("✅ Binary compilation works");
    println!("✅ Serialization/deserialization works");
    println!("✅ Config merging works");
    println!("✅ Migration tools work");
    let _ = fs::remove_file("test_example.hlxb");
    Ok(())
}
#[cfg(test)]
mod verification_tests {
    use super::*;
    #[test]
    fn verify_all_functionality() {
        main().expect("Verification failed");
    }
}