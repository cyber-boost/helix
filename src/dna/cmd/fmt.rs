use clap::Args;
use std::path::PathBuf;
use crate::mds::fmt;

#[derive(Args)]
pub struct FmtArgs {
    /// Files to format (if empty, formats the whole project)
    #[arg(short, long)]
    files: Vec<PathBuf>,

    /// Check if files are formatted (does not write changes)
    #[arg(long)]
    check: bool,

    /// Show verbose output
    #[arg(long)]
    verbose: bool,
}

pub fn run(args: FmtArgs) -> anyhow::Result<()> {
    fmt::format_files(args.files, args.check, args.verbose)
}
