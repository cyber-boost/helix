use clap::Args;
use std::path::PathBuf;
use crate::mds::sign::sign_binary;


#[derive(Args)]
pub struct SignArgs {
    input: PathBuf,
    key: Option<String>,
    output: Option<PathBuf>,
    verify: bool,
    verbose: bool,
    quiet: bool,
    content: Option<String>,
}

pub fn run(args: SignArgs) -> anyhow::Result<()> {
    let input = args.input;
    let key = args.key;
    let output = args.output;
    let verify = args.verify;
    let verbose = args.verbose;
    let sign = sign_binary(input, key, output, verify, verbose);
    sign?;
    Ok(())
}

