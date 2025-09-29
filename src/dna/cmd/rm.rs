use clap::Args;
use std::path::PathBuf;


#[derive(Args)]
pub struct RemoveArgs {
    /// Files to remove
    #[arg(short, long)]
    files: Vec<PathBuf>,
}

#[derive(clap::Subcommand)]
pub enum RemoveCommands {
    Remove(RemoveArgs),
}

pub async fn run(args: RemoveArgs) -> anyhow::Result<()> {
    println!("Remove command with {} files", args.files.len());
    Ok(())
}