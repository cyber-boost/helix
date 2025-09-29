use clap::Args;
use std::path::PathBuf;


#[derive(Args)]
pub struct DiffArgs {
    /// Target directory to analyze (defaults to current directory)
    #[arg(short, long)]
    file1: Option<PathBuf>,

    #[arg(short, long)]
    file2: Option<PathBuf>,

    #[arg(short, long)]
    detailed: bool,
}



pub fn run(args: DiffArgs) -> anyhow::Result<()> {
    let file1 = args.file1.unwrap_or_else(|| PathBuf::from("."));
    let file2 = args.file2.unwrap_or_else(|| PathBuf::from("."));
    let detailed = args.detailed;
    crate::mds::diff::diff_command(file1, file2, detailed)
        .map_err(|e| anyhow::anyhow!("Diff command failed: {}", e))?;
    Ok(())
}