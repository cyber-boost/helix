use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct BundleArgs {
    directory: PathBuf,
    /// Target directory to analyze (defaults to current directory)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output file path (defaults to stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Include files (defaults to all files)
    #[arg(short, long)]
    include: Vec<String>,

    /// Exclude files (defaults to none)
    #[arg(short = 'x', long)]
    exclude: Vec<String>,

    /// Tree shake (defaults to false)
    #[arg(long)]
    tree_shake: bool,

    /// Optimize (defaults to 2)
    #[arg(short = 'O', long, default_value = "2")]
    optimize: u8,

    

}

pub fn run(args: BundleArgs) -> anyhow::Result<()> {
    let input = args.input.unwrap_or_else(|| PathBuf::from("."));
    let output = args.output.unwrap_or_else(|| PathBuf::from("bundle.hlxb"));
    let include = args.include;
    let exclude = args.exclude;
    let tree_shake = args.tree_shake;
    let optimize = args.optimize;
    crate::dna::mds::bundle::bundle_command(input, output, include, exclude, tree_shake, optimize, false)
        .map_err(|e| anyhow::anyhow!("Bundle command failed: {}", e))?;
    Ok(())
}
