use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct FilterArgs {
    /// Files to filter
    #[arg(short, long)]
    files: Vec<PathBuf>,
}

#[derive(clap::Subcommand)]
pub enum FilterCommands {
    Filter(FilterArgs),
}

pub async fn run(args: FilterArgs) -> anyhow::Result<()> {
    println!("Filter command with {} files", args.files.len());
    Ok(())
}