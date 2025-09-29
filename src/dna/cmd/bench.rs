use clap::Args;
use crate::mds::bench;


#[derive(Args)]
pub struct BenchArgs {
    #[arg(short, long)]
    pattern: Option<String>,
    #[arg(short, long)]
    iterations: Option<usize>,
}

#[derive(clap::Subcommand)]
pub enum BenchCommands {
    Bench(BenchArgs),
}

pub fn run(args: BenchArgs) -> anyhow::Result<()> {
    bench::run_benchmarks(args.pattern, args.iterations, true)
}