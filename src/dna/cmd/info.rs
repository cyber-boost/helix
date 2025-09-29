use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct InfoArgs {
    /// Target directory to analyze (defaults to current directory)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// File to analyze (defaults to current directory)
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Output file path (defaults to stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Format (defaults to text)
    #[arg(short, long, default_value = "text")]
    format: String,

    /// Symbols (defaults to false)
    #[arg(long)]
    symbols: bool,

    /// Sections (defaults to false)
    #[arg(long)]
    sections: bool,
}

pub fn run(args: InfoArgs) -> anyhow::Result<()> {
    let input = args.input.unwrap_or_else(|| PathBuf::from("."));
    let format = args.format;
    let symbols = args.symbols;
    let sections = args.sections;
    let verbose = false; // Add default verbose parameter
    crate::mds::info::info_command(input, format, symbols, sections, verbose)
        .map_err(|e| anyhow::anyhow!("Info command failed: {}", e))
}