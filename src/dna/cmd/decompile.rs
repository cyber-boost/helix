use clap::Args;
use std::path::PathBuf;
use crate::mds::decompile;

#[derive(Args)]
pub struct DecompileArgs {
    /// Target directory to analyze (defaults to current directory)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output file path (defaults to stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,
}