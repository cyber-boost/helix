
use std::path::PathBuf;
use clap::Args;

/// Arguments for the `generate` command.
#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// The template to use for code generation.
    #[arg(value_name = "TEMPLATE")]
    pub template: String,

    /// Output path for the generated code.
    #[arg(short, long, value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Optional name for the generated artifact.
    #[arg(short, long, value_name = "NAME")]
    pub name: Option<String>,

    /// Overwrite existing files if present.
    #[arg(short, long)]
    pub force: bool,

    /// Enable verbose output.
    #[arg(short, long)]
    pub verbose: bool,
}

/// Run the generate command with the provided arguments.
pub fn run(args: GenerateArgs) -> anyhow::Result<()> {
    crate::dna::mds::generate::generate_code(
        args.template,
        args.output,
        args.name,
        args.force,
        args.verbose,
    )
}