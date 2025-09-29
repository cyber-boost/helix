use clap::Args;

#[derive(Args)]
pub struct TestArgs {
    /// Pattern (defaults to none)
    #[arg(short, long)]
    pattern: Option<String>,
    #[arg(long)]
    integration: bool,
}

#[derive(clap::Subcommand)]
pub enum TestCommands {
    Test(TestArgs),
}

pub async fn run(args: TestArgs) -> anyhow::Result<()> {
    println!("Test command: pattern={:?}, integration={}", args.pattern, args.integration);
    Ok(())
}

