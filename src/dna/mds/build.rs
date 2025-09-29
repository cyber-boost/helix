use std::path::PathBuf;
use crate::dna::compiler::{Compiler, OptimizationLevel};

pub fn run_build(args: crate::dna::cmd::build::BuildArgs) -> anyhow::Result<()> {
    let input = args.input;
    let output = args.output;
    let optimize = args.optimize;
    let compress = args.compress;
    let cache = args.cache;

    build_project(input, output, optimize, compress, cache, true)
        .map_err(|e| anyhow::anyhow!("Build failed: {}", e))
}

fn build_project(
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    optimize: u8,
    compress: bool,
    cache: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = find_project_root()?;
    let input_file = match input {
        Some(path) => path,
        None => {
            let main_file = project_dir.join("src").join("main.hlx");
            if main_file.exists() {
                main_file
            } else {
                return Err(
                    anyhow::anyhow!(
                        "No input file specified and no src/main.hlx found.\n\
                    Specify a file with: helix build <file.hlx>"
                    )
                        .into(),
                );
            }
        }
    };
    let output_file = output
        .unwrap_or_else(|| {
            let target_dir = project_dir.join("target");
            let input_stem = input_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            target_dir.join(format!("{}.hlxb", input_stem))
        });
    if verbose {
        println!("🔨 Building HELIX project:");
        println!("  Input: {}", input_file.display());
        println!("  Output: {}", output_file.display());
        println!("  Optimization: Level {}", optimize);
        println!("  Compression: {}", if compress { "Enabled" } else { "Disabled" });
        println!("  Cache: {}", if cache { "Enabled" } else { "Disabled" });
    }
    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let compiler = Compiler::new(OptimizationLevel::from(optimize));
    let binary = compiler.compile_file(&input_file)?;
    let serializer = crate::mds::serializer::BinarySerializer::new(compress);
    serializer.write_to_file(&binary, &output_file)?;
    println!("✅ Build completed successfully!");
    println!("  Output: {}", output_file.display());
    println!("  Size: {} bytes", binary.size());
    if verbose {
        let stats = binary.symbol_table.stats();
        println!(
            "  Strings: {} (unique: {})", stats.total_strings, stats.unique_strings
        );
        println!("  Agents: {}", stats.agents);
        println!("  Workflows: {}", stats.workflows);
    }
    Ok(())
}

fn find_project_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut current = std::env::current_dir()?;
    loop {
        if current.join("hlx.toml").exists() || current.join("Cargo.toml").exists() {
            return Ok(current);
        }
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            return Err("Could not find project root".into());
        }
    }
}