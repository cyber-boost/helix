use clap::Args;

#[derive(Args)]
pub struct ResetArgs {
    /// Force (defaults to false)
    #[arg(short, long)]
    force: bool,
}

#[derive(clap::Subcommand)]
pub enum ResetCommands {
    Reset(ResetArgs),
}

pub async fn run(args: ResetArgs) -> anyhow::Result<()> {
    println!("Reset command: force={}", args.force);
    Ok(())
}