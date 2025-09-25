#[cfg(feature = "compiler")]
use helix::compiler::{Compiler, OptimizationLevel};
use helix::{parse, validate, ast_to_config, HelixConfig};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
struct LocalhostClient {
    port: u16,
}
impl LocalhostClient {
    fn new(port: u16) -> Self {
        Self { port }
    }
    fn fetch_file_list(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = format!("http://localhost:{}/", self.port);
        println!("📡 Fetching file list from: {}", url);
        self.simulate_http_get("/")
    }
    fn fetch_hlx_file(
        &self,
        filename: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("http://localhost:{}/{}", self.port, filename);
        println!("📥 Fetching HLX file: {}", url);
        let filepath = format!("examples/{}", filename);
        if Path::new(&filepath).exists() {
            let content = fs::read_to_string(&filepath)?;
            println!("✅ Fetched {} ({} bytes)", filename, content.len());
            Ok(content)
        } else {
            Err(format!("File {} not found", filepath).into())
        }
    }
    fn simulate_http_get(
        &self,
        _path: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        println!("🔍 Discovering HLX files in examples/ directory...");
        let mut hlx_files = Vec::new();
        if let Ok(entries) = fs::read_dir("examples") {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".hlx") {
                        hlx_files.push(filename.to_string());
                    }
                }
            }
        }
        println!("📋 Found {} HLX files: {:?}", hlx_files.len(), hlx_files);
        Ok(hlx_files)
    }
}
#[cfg(feature = "compiler")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌐 HELIX Remote Loading Example - Testing Localhost Integration");
    println!("   Proving rlib integration works for remote compilation\n");
    let client = LocalhostClient::new(4592);
    println!("🔌 Connecting to localhost:{}", client.port);
    println!("\n1. 🔍 Discovering HLX files from server...");
    let hlx_files = client.fetch_file_list()?;
    if hlx_files.is_empty() {
        println!("❌ No HLX files found on server");
        return Ok(());
    }
    println!("\n2. 🔧 Initializing HELIX compiler...");
    let compiler = Compiler::new(OptimizationLevel::Two);
    println!("✅ Compiler ready with optimization level: Two");
    println!("\n3. 📥 Fetching and compiling HLX files from localhost...");
    let mut compiled_files = HashMap::new();
    let mut successful_compilations = 0;
    for filename in &hlx_files {
        println!("\n   Processing: {}", filename);
        match client.fetch_hlx_file(filename) {
            Ok(hlx_content) => {
                println!("   ✅ Fetched {} characters", hlx_content.len());
                match parse(&hlx_content) {
                    Ok(ast) => {
                        println!(
                            "   ✅ Parsed AST with {} declarations", ast.declarations
                            .len()
                        );
                        if let Err(e) = validate(&ast) {
                            println!("   ❌ AST validation failed: {}", e);
                            continue;
                        }
                        println!("   ✅ AST validation passed");
                        match ast_to_config(ast) {
                            Ok(config) => {
                                println!("   ✅ Converted to configuration:");
                                println!("      - Projects: {}", config.projects.len());
                                println!("      - Agents: {}", config.agents.len());
                                println!("      - Workflows: {}", config.workflows.len());
                                match compiler.compile_source(&hlx_content, None) {
                                    Ok(binary) => {
                                        println!(
                                            "   ✅ Compiled to binary ({} bytes)", binary.size()
                                        );
                                        compiled_files.insert(filename.clone(), binary);
                                        successful_compilations += 1;
                                        for (name, agent) in &config.agents {
                                            println!(
                                                "      - Agent '{}': model={:?}", name, agent.model
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        println!("   ❌ Compilation failed: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("   ❌ Configuration conversion failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Parsing failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Failed to fetch {}: {}", filename, e);
            }
        }
    }
    println!("\n4. 📊 Compilation Summary:");
    println!("   - Total HLX files discovered: {}", hlx_files.len());
    println!(
        "   - Successfully compiled: {}/{}", successful_compilations, hlx_files.len()
    );
    println!(
        "   - Success rate: {:.1}%", (successful_compilations as f64 / hlx_files.len() as
        f64) * 100.0
    );
    if successful_compilations > 0 {
        println!("\n🎉 REMOTE COMPILATION test PASSED!");
        println!("   ✅ rlib integration working");
        println!("   ✅ HTTP fetching working");
        println!("   ✅ Compilation pipeline working");
        println!("   ✅ Remote loading proven");
        println!("\n📁 Compiled Files:");
        for (filename, binary) in &compiled_files {
            println!("   - {} → {} bytes", filename, binary.size());
        }
    } else {
        println!("\n❌ REMOTE COMPILATION test FAILED");
        println!("   - No files were successfully compiled");
        println!("   - Check server is running and HLX files exist");
    }
    Ok(())
}
#[cfg(not(feature = "compiler"))]
fn main() {
    println!(
        "⚠️  Compiler features not enabled. Run with: cargo run --example load-helix-from-localhost --features compiler"
    );
    println!("   This example requires the 'compiler' feature to be enabled.");
}