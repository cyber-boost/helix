//! HLX Test Library
//!
//! This library provides testing utilities for the Helix configuration language enhancements.
//! It includes functionality to process .hlx files and test all the new features.

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use helix::{hlx, parse, validate, load_file, ast_to_config};
use helix::ops::{ensure_calc, OperatorParser};
use helix::value::Value;
use helix::hlx::{HlxDatasetProcessor, start_default_server, start_server};
use helix::server::ServerConfig;

/// Process all .hlx files in a directory
pub async fn process_hlx_files(
    config_dir: &Path,
    _runtime_context: &HashMap<String, String>
) -> Result<(usize, Vec<String>)> {
    let hlx_files: Vec<_> = WalkDir::new(config_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|path| path.extension().map_or(false, |ext| ext == "hlx"))
        .collect();

    let mut processed_files = Vec::new();
    let mut total_lines = 0;

    for file in &hlx_files {
        if let Ok(content) = fs::read_to_string(file) {
            total_lines += content.lines().count();
            processed_files.push(file.display().to_string());
        }
    }

    Ok((total_lines, processed_files))
}

/// Get file statistics
pub fn get_file_stats(config_dir: &Path) -> Result<(usize, Vec<String>)> {
    let hlx_files: Vec<_> = WalkDir::new(config_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_path_buf())
        .filter(|path| path.extension().map_or(false, |ext| ext == "hlx"))
        .collect();

    let mut file_names = Vec::new();
    let mut total_lines = 0;

    for file in &hlx_files {
        file_names.push(file.display().to_string());
        if let Ok(content) = fs::read_to_string(file) {
            total_lines += content.lines().count();
        }
    }

    Ok((total_lines, file_names))
}
