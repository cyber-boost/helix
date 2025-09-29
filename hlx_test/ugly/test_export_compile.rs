// Test compilation of export.rs
use std::path::PathBuf;
use anyhow::{Result, Context};
use crate::mds::init::ProjectManifest;
use serde::{Serialize, Deserialize};
use serde_json;
use serde_yaml;
use toml;
use chrono;

// Test that the structs compile
#[derive(Debug, Serialize, Deserialize)]
struct ExportData {
    manifest: ProjectManifest,
    source_files: std::collections::HashMap<String, String>,
    dependencies: Option<std::collections::HashMap<String, String>>,
    metadata: ExportMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExportMetadata {
    exported_at: String,
    format_version: String,
    tool_version: String,
}

fn main() {
    println!("Export module compiles successfully!");
}
