use clap::Args;
use std::path::PathBuf;
use crate::mds::requirements;

#[derive(Args)]
pub struct RequirementsArgs {
    /// Target directory to analyze (defaults to current directory)
    #[arg(short, long)]
    target: Option<PathBuf>,

    /// Output file path (defaults to stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

pub fn run(args: RequirementsArgs) -> anyhow::Result<()> {
    let target_dir = args.target.unwrap_or_else(|| PathBuf::from("."));
    let requirements = requirements::analyze_python_files(&target_dir)?;

    match args.output {
        Some(output_path) => {
            std::fs::write(&output_path, requirements)?;
            println!("Requirements written to {}", output_path.display());
        }
        None => {
            println!("{}", requirements);
        }
    }

    Ok(())
}
