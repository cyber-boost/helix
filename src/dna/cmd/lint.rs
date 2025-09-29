use clap::Args;
use std::path::PathBuf;
use crate::mds::lint;

/// Arguments for the `lint` command.
#[derive(Args, Debug)]
pub struct LintArgs {
    /// Files to lint (if empty, lints the whole project)
    #[arg(value_name = "FILES")]
    pub files: Vec<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

pub async fn run(args: LintArgs) -> anyhow::Result<()> {
    // Directly call the lint_files function from mds::lint
    lint::lint_files(args.files, args.verbose)
}