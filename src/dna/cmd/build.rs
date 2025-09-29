use clap::Args;
use std::path::PathBuf;
use crate::mds::build;



#[derive(Args)]
pub struct BuildArgs {
    /// Input file path (defaults to current directory)
    #[arg(short, long)]
    pub input: Option<PathBuf>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(short = 'O', long, default_value = "2")]
    pub optimize: u8,
    #[arg(short, long)]
    pub compress: bool,
    #[arg(long)]
    pub cache: bool,
}

#[derive(clap::Subcommand)]
pub enum BuildCommands {
    Build(BuildArgs),
}

pub fn run(args: BuildArgs) -> anyhow::Result<()> {
    build::run_build(args)
}