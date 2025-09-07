use std::path::PathBuf;
use std::process::Command;
use anyhow::{Result, Context};
use crate::compiler::Compiler;
use crate::compiler::optimizer::OptimizationLevel;
pub fn run_project(
    input: Option<PathBuf>,
    args: Vec<String>,
    optimize: u8,
    verbose: bool,
) -> Result<()> {
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
                    Specify a file with: helix run <file.mso>"
                    ),
                );
            }
        }
    };
    if verbose {
        println!("🚀 Running HELIX project:");
        println!("  Input: {}", input_file.display());
        println!("  Optimization: Level {}", optimize);
        if !args.is_empty() {
            println!("  Arguments: {:?}", args);
        }
    }
    let output_file = compile_for_run(&input_file, optimize, verbose)?;
    execute_binary(&output_file, args, verbose)?;
    Ok(())
}
fn compile_for_run(input: &PathBuf, optimize: u8, verbose: bool) -> Result<PathBuf> {
    let project_dir = find_project_root()?;
    let target_dir = project_dir.join("target");
    std::fs::create_dir_all(&target_dir).context("Failed to create target directory")?;
    let input_stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid input filename"))?;
    let output_file = target_dir.join(format!("{}.hlxb", input_stem));
    if verbose {
        println!("📦 Compiling for execution...");
    }
    let compiler = Compiler::builder()
        .optimization_level(OptimizationLevel::from(optimize))
        .compression(true)
        .cache(true)
        .verbose(verbose)
        .build();
    let binary = compiler.compile_file(input).context("Failed to compile HELIX file")?;
    let serializer = crate::compiler::serializer::BinarySerializer::new(true);
    serializer
        .write_to_file(&binary, &output_file)
        .context("Failed to write compiled binary")?;
    if verbose {
        println!("✅ Compiled successfully: {}", output_file.display());
        println!("  Size: {} bytes", binary.size());
    }
    Ok(output_file)
}
fn execute_binary(
    binary_path: &PathBuf,
    args: Vec<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("▶️  Executing binary: {}", binary_path.display());
    }
    let mut cmd = Command::new("echo");
    cmd.arg("HELIX Runtime not yet implemented");
    cmd.arg("Binary compiled successfully:");
    cmd.arg(binary_path.to_string_lossy().as_ref());
    if !args.is_empty() {
        cmd.arg("Arguments:");
        for arg in &args {
            cmd.arg(arg);
        }
    }
    let output = cmd.output().context("Failed to execute binary")?;
    if !output.status.success() {
        return Err(
            anyhow::anyhow!(
                "Binary execution failed with exit code: {}", output.status.code()
                .unwrap_or(- 1)
            ),
        );
    }
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(& output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(& output.stderr));
    }
    Ok(())
}
fn find_project_root() -> Result<PathBuf> {
    let mut current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    loop {
        let manifest_path = current_dir.join("project.hlx");
        if manifest_path.exists() {
            return Ok(current_dir);
        }
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    Err(anyhow::anyhow!("No HELIX project found. Run 'helix init' first."))
}