use clap::Args;


#[derive(Args)]
pub struct CleanArgs {
    /// All (defaults to false)
    #[arg(long)]
    all: bool,
    #[arg(long)]
    cache: bool,
}




pub async fn run(args: CleanArgs) -> anyhow::Result<()> {
    println!("Clean command: all={}, cache={}", args.all, args.cache);
    Ok(())
}