use clap::Args;
use std::path::PathBuf;
use crate::mds::config;

#[derive(Args)]
pub struct ConfigArgs {
    action: String,
    key: Option<String>,
    value: Option<String>,
}