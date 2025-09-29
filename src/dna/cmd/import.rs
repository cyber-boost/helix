
use clap::Args;
use std::path::PathBuf;


#[derive(Args, Debug)]
pub struct ImportArgs {
    pub input: PathBuf,
    #[arg(short, long)]
    pub registry: Option<String>,
    #[arg(short, long)]
    pub token: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
}


pub async fn run(args: ImportArgs) -> anyhow::Result<()> {
    println!("Import command: {}", args.input.display());
    Ok(())
}