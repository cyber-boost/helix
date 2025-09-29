use clap::Args;
use std::path::PathBuf;
use crate::mds::compile::compile_command;


#[derive(Args)]
pub struct CompileArgs {
    #[arg(short, long)]
    input: Option<PathBuf>,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long)]
    compress: bool,

    #[arg(short = 'O', long, default_value = "2")]
    optimize: u8,

    #[arg(long)]
    cache: bool,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    quiet: bool,
}

pub fn run(args: CompileArgs) -> anyhow::Result<()> {
    let input = args.input.unwrap_or_else(|| PathBuf::from("."));
    let output = args.output;
    let compress = args.compress;
    let optimize = args.optimize;
    let cache = args.cache;
    compile_command(input, output, compress, optimize, cache, args.verbose, args.quiet)
        .map_err(|e| anyhow::anyhow!("Compilation failed: {}", e))?;
    println!("Compilation completed");

    Ok(())
}