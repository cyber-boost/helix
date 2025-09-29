use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct DatasetArgs {
    /// Files to process
    #[arg(short, long)]
    files: Vec<PathBuf>,

    /// Output file path (defaults to stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Format (defaults to json)
    #[arg(long)]
    format: Option<String>,
}

pub async fn run(args: DatasetArgs) -> anyhow::Result<()> {
    use crate::mds::dataset::{dataset_command, DatasetAction};

    let action = DatasetAction::Process {
        files: args.files,
        output: args.output,
        format: args.format,
        algorithm: None,
        validate: false,
    };

    dataset_command(action, false).await.map_err(|e| anyhow::anyhow!("Dataset command failed: {}", e))?;
    Ok(())
}