use clap::Args;

#[derive(Args)]
pub struct DoctorArgs {
    action: String,
}

pub fn run(args: DoctorArgs) -> anyhow::Result<()> {
    println!("Doctor command: {}", args.action);
    Ok(())
}