use clap::Args;
use std::path::PathBuf;
use crate::mds::serve::serve_project;

#[derive(Args)]
pub struct ServeArgs {
    #[arg(short, long)]
    port: Option<u16>,
    #[arg(long)]
    domain: Option<String>,
    #[arg(short, long)]
    directory: Option<PathBuf>,
    #[arg(long)]
    no_convert: bool,
    #[arg(long)]
    cache_timeout: Option<u64>,
    #[arg(long)]
    max_file_size: Option<u64>,
}   

#[derive(Args)]
pub struct ServeProjectArgs {
    #[arg(short, long)]
    port: Option<u16>,
    #[arg(long)]
    host: Option<String>,
    directory: Option<PathBuf>,
}

pub fn run(args: ServeArgs) -> anyhow::Result<()> {
    let port = args.port;
    let host = args.domain;
    let directory = args.directory;
    let verbose = false; // Add default verbose parameter
    let serve_result = serve_project(port, host, directory, verbose);
    serve_result?;
    Ok(())
}