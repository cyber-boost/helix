use clap::Args;
use clap_complete::Shell;
use crate::mds::completions::completions_command;

#[derive(Args)]
pub struct CompletionsArgs {
    #[arg(value_enum)]
    shell: Shell,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    quiet: bool,
}

pub fn run(args: CompletionsArgs) -> anyhow::Result<()> {
    let shell = args.shell;
    let completions = completions_command(shell, args.verbose, args.quiet);
    println!("{}", completions);
    Ok(())
}