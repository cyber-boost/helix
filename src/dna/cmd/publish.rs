use clap::Args;
use std::path::PathBuf;
//use crate::mds::publish::publish_command;

#[derive(Args, Debug)]
pub struct PublishArgs {
    /// Action to perform (e.g., publish, sign, export)
    #[clap(value_parser)]
    pub action: String,

    /// Optional registry to publish to
    #[clap(long)]
    pub registry: Option<String>,

    /// API token for authentication
    #[clap(long)]
    pub token: Option<String>,

    /// Perform a dry run without actual publishing
    #[clap(long)]
    pub dry_run: bool,

    /// Enable verbose output
    #[clap(long)]
    pub verbose: bool,

    /// Suppress output
    #[clap(long)]
    pub quiet: bool,

    /// Path to input binary (for sign action)
    #[clap(long)]
    pub input: Option<PathBuf>,

    /// Signing key (for sign action)
    #[clap(long)]
    pub key: Option<String>,

    /// Output path (for sign/export actions)
    #[clap(long)]
    pub output: Option<PathBuf>,

    /// Verify signature (for sign action)
    #[clap(long)]
    pub verify: bool,

    /// Export format (for export action)
    #[clap(long)]
    pub format: Option<String>,

    /// Include dependencies in export (for export action)
    #[clap(long)]
    pub include_deps: bool,
}

pub fn run(args: PublishArgs) -> anyhow::Result<()> {
    match args.action.as_str() {
        "publish" => {
            crate::mds::publish::publish_project(
                args.registry,
                args.token,
                args.dry_run,
                args.verbose,
            )?;
        }
        "sign" => {
            let input = args.input.clone().ok_or_else(|| anyhow::anyhow!("--input is required for sign action"))?;
            crate::mds::publish::sign_binary(
                input,
                args.key.clone(),
                args.output.clone(),
                args.verify,
                args.verbose,
            )?;
        }
        "export" => {
            let format = args.format.clone().ok_or_else(|| anyhow::anyhow!("--format is required for export action"))?;
            crate::mds::publish::export_project(
                format,
                args.output.clone(),
                args.include_deps,
                args.verbose,
            )?;
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown publish action: {}", args.action));
        }
    }
    Ok(())
}