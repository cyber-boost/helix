use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use colored::*;
use helix::*;
fn main() -> Result<()> {
    println!("{}", "🚀 Helix Enhancement Test Runner".bright_green().bold());
    println!("This test processes all .hlx files in the config/ directory");
    println!("and demonstrates all the new parser enhancements.");
    println!();

    // Set up runtime context
    let mut runtime_context = HashMap::new();
    runtime_context.insert("TEST_VAR".to_string(), "resolved_value".to_string());
    runtime_context.insert("PROJECT_NAME".to_string(), "MyTestApp".to_string());
    runtime_context.insert("DEBUG_MODE".to_string(), "true".to_string());

    // Find all .hlx files
    let config_dir = Path::new("config");
    let hlx_files: Vec<_> = WalkDir::new(config_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|path| path.extension().map_or(false, |ext| ext == "hlx"))
        .collect();

    println!("📁 Processing {} files:", hlx_files.len());

    for file in &hlx_files {
        let hlx = Hlx::load(file).await? {
            let name = hlx.get('tasks', 'name');

            
        }
    }

    println!();
    println!("✅ Test runner completed successfully!");
    println!("Run 'cargo test' to execute the full test suite.");

    Ok(())
}
