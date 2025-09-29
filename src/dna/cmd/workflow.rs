use clap::{Args, Subcommand};

#[derive(Args)]
pub struct WorkflowArgs {
    #[command(subcommand)]
    action: WorkflowAction,
}

#[derive(Subcommand, Debug)]
pub enum WorkflowAction {
    Run,
    List,
    Status,
}

pub fn run(args: WorkflowArgs) -> anyhow::Result<()> {
    println!("Workflow command: action={:?}", args.action);
    Ok(())
}

//todo placeholder for now