use std::path::PathBuf;
use crate::mds::loader::BinaryLoader;
use crate::hel::binary::{HelixBinary, DataSection};
use anyhow::Result;

pub fn info_command(
    file: PathBuf,
    format: String,
    symbols: bool,
    sections: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let loader = BinaryLoader::new();
    let binary = loader.load_file(&file)?;
    match format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&binary.metadata)?;
            println!("{}", json);
        }
        "yaml" => {
            println!("YAML output not yet implemented");
        }
        "text" | _ => {
            println!("HELIX Binary Information");
            println!("=======================");
            println!("File: {}", file.display());
            println!("Version: {}", binary.version);
            println!("Compiler: {}", binary.metadata.compiler_version);
            println!("Platform: {}", binary.metadata.platform);
            println!("Created: {}", binary.metadata.created_at);
            println!("Optimization: Level {}", binary.metadata.optimization_level);
            println!("Compressed: {}", binary.flags.compressed);
            println!("Size: {} bytes", binary.size());
            println!("Checksum: {:x}", binary.checksum);
            if let Some(source) = &binary.metadata.source_path {
                println!("Source: {}", source);
            }
            if symbols || verbose {
                println!("\nSymbol Table:");
                let stats = binary.symbol_table.stats();
                println!(
                    "  Strings: {} (unique: {})", stats.total_strings, stats
                    .unique_strings
                );
                println!("  Total bytes: {}", stats.total_bytes);
                println!("  Agents: {}", stats.agents);
                println!("  Workflows: {}", stats.workflows);
                println!("  Contexts: {}", stats.contexts);
                println!("  Crews: {}", stats.crews);
            }
            if sections || verbose {
                println!("\nData Sections:");
                for (i, section) in binary.data_sections.iter().enumerate() {
                    println!("  [{}] {:?}", i, section.section_type);
                    println!("      Size: {} bytes", section.size);
                    if let Some(compression) = &section.compression {
                        println!("      Compression: {:?}", compression);
                    }
                }
            }
        }
    }
    Ok(())
}