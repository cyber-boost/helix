use clap::Args;
use std::path::PathBuf;
use crate::mds::preview::preview_command;

#[derive(Args, Debug)]
pub struct PreviewArgs {
    /// File to preview
    #[arg(short, long)]
    file: PathBuf,

    /// Output format (optional)
    #[arg(short, long)]
    format: Option<String>,

    /// Number of rows to preview (optional, default: 10)
    #[arg(short, long)]
    rows: Option<usize>,

    /// Columns to display (comma-separated, optional)
    #[arg(short, long)]
    columns: Option<Vec<String>>,

    /// Verbose output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

pub fn run(args: PreviewArgs) -> anyhow::Result<()> {
    preview_command(
        args.file,
        args.format,
        args.rows,
        args.columns,
        args.verbose,
    ).map_err(|e| anyhow::anyhow!("Preview command failed: {}", e))?;
    Ok(())
}