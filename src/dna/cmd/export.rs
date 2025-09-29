use clap::Args;
use std::path::PathBuf;
use crate::mds::export;

#[derive(Args)]
pub struct ExportArgs {
    /// Export format: json, yaml, toml, docker, k8s
    #[arg(short, long, default_value = "json")]
    pub format: String,

    /// Output file path (defaults to stdout if not specified)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Include dependencies in export
    #[arg(long, default_value_t = false)]
    pub include_deps: bool,

    /// Enable verbose output
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}

pub fn run(args: ExportArgs) -> anyhow::Result<()> {
    export::export_project(
        args.format,
        args.output,
        args.include_deps,
        args.verbose,
    )
}
