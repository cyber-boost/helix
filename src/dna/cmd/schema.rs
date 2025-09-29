use clap::Args;
use std::path::PathBuf;
use crate::mds::schema::schema_command;
use crate::mds::schema::Language;

#[derive(Args)]
pub struct SchemaArgs {
    target: PathBuf,
    #[arg(short = 'l', long, default_value = "rust")]
    lang: Language,
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(short, long)]
    verbose: bool,
    #[arg(short, long)]
    quiet: bool,
}


pub fn run(args: SchemaArgs) -> anyhow::Result<()> {
    let input = args.target;
    let lang = args.lang;
    let output = args.output;
    let schema_result = schema_command(input, lang, output, args.verbose)
        .map_err(|e| anyhow::anyhow!("Schema command failed: {}", e))?;
    Ok(())
}