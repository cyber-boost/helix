use clap::Args;
use std::path::PathBuf;
use crate::mds::tools;
use anyhow::Result;

#[derive(Args, Debug)]
pub struct RequirementsArgs {
    /// Target directory to analyze (defaults to current directory)
    #[arg(short, long)]
    pub target: Option<PathBuf>,

    /// Output file path (defaults to stdout if not specified)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Only list top-level requirements (no submodules)
    #[arg(long, default_value_t = false)]
    pub top_level: bool,

    /// Include version specifiers if available
    #[arg(long, default_value_t = false)]
    pub with_versions: bool,

    /// Verbose output
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}

pub fn run(args: RequirementsArgs) -> Result<()> {
    let target_dir = args.target.clone().unwrap_or_else(|| PathBuf::from("."));
    if args.verbose {
        println!("🔍 Analyzing Python files in: {}", target_dir.display());
        if args.top_level {
            println!("  Only top-level requirements will be listed.");
        }
        if args.with_versions {
            println!("  Including version specifiers if available.");
        }
    }
    let requirements = tools::analyze_python_files_with_options(
        &target_dir,
        args.top_level,
        args.with_versions,
        args.verbose,
    )?;

    match args.output {
        Some(ref output_path) => {
            std::fs::write(output_path, &requirements)?;
            if args.verbose {
                println!("✅ Requirements written to {}", output_path.display());
            }
        }
        None => {
            println!("{}", requirements);
        }
    }

    Ok(())
}
