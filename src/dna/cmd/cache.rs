use clap::Args;

#[derive(Args)]
pub struct CacheArgs {
    action: String,
}

pub fn run(args: CacheArgs) -> anyhow::Result<()> {
    println!("Cache command: {}", args.action);
    Ok(())
}