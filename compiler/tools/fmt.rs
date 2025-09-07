use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};
pub fn format_files(files: Vec<PathBuf>, check: bool, verbose: bool) -> Result<()> {
    if files.is_empty() {
        format_project(check, verbose)
    } else {
        format_specific_files(files, check, verbose)
    }
}
fn format_project(check: bool, verbose: bool) -> Result<()> {
    let project_dir = find_project_root()?;
    if verbose {
        println!("🎨 Formatting HELIX project:");
        println!("  Project: {}", project_dir.display());
        println!("  Check mode: {}", check);
    }
    let mut helix_files = Vec::new();
    find_helix_files(&project_dir, &mut helix_files)?;
    if helix_files.is_empty() {
        println!("ℹ️  No HELIX files found to format.");
        return Ok(());
    }
    println!("📋 Found {} HELIX files to format", helix_files.len());
    let mut formatted_count = 0;
    let mut unchanged_count = 0;
    for file in helix_files {
        match format_single_file(&file, check, verbose) {
            Ok(FormatResult::Formatted) => {
                formatted_count += 1;
                if !check {
                    println!("✅ Formatted: {}", file.display());
                }
            }
            Ok(FormatResult::Unchanged) => {
                unchanged_count += 1;
                if verbose {
                    println!("ℹ️  Unchanged: {}", file.display());
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to format {}: {}", file.display(), e);
            }
        }
    }
    if check {
        if formatted_count > 0 {
            println!("❌ {} files need formatting", formatted_count);
            std::process::exit(1);
        } else {
            println!("✅ All files are properly formatted");
        }
    } else {
        println!("\n📊 Formatting Results:");
        println!("  Formatted: {}", formatted_count);
        println!("  Unchanged: {}", unchanged_count);
    }
    Ok(())
}
fn format_specific_files(files: Vec<PathBuf>, check: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("🎨 Formatting specific files:");
        println!("  Files: {}", files.len());
        println!("  Check mode: {}", check);
    }
    let mut formatted_count = 0;
    let mut unchanged_count = 0;
    for file in files {
        if !file.exists() {
            eprintln!("❌ File not found: {}", file.display());
            continue;
        }
        if !file.extension().map_or(false, |ext| ext == "hlx") {
            eprintln!("⚠️  Skipping non-HELIX file: {}", file.display());
            continue;
        }
        match format_single_file(&file, check, verbose) {
            Ok(FormatResult::Formatted) => {
                formatted_count += 1;
                if !check {
                    println!("✅ Formatted: {}", file.display());
                }
            }
            Ok(FormatResult::Unchanged) => {
                unchanged_count += 1;
                if verbose {
                    println!("ℹ️  Unchanged: {}", file.display());
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to format {}: {}", file.display(), e);
            }
        }
    }
    if check {
        if formatted_count > 0 {
            println!("❌ {} files need formatting", formatted_count);
            std::process::exit(1);
        } else {
            println!("✅ All files are properly formatted");
        }
    } else {
        println!("\n📊 Formatting Results:");
        println!("  Formatted: {}", formatted_count);
        println!("  Unchanged: {}", unchanged_count);
    }
    Ok(())
}
#[derive(Debug)]
enum FormatResult {
    Formatted,
    Unchanged,
}
fn format_single_file(
    file: &PathBuf,
    check: bool,
    _verbose: bool,
) -> Result<FormatResult> {
    let content = fs::read_to_string(file).context("Failed to read file")?;
    let formatted_content = format_helix_content(&content)?;
    if content == formatted_content {
        return Ok(FormatResult::Unchanged);
    }
    if !check {
        fs::write(file, formatted_content).context("Failed to write formatted content")?;
    }
    Ok(FormatResult::Formatted)
}
fn format_helix_content(content: &str) -> Result<String> {
    let mut formatted = String::new();
    let mut indent_level: i32 = 0;
    let indent_str = "  ";
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            formatted.push('\n');
            continue;
        }
        if trimmed.starts_with('}') {
            indent_level = indent_level.saturating_sub(1);
        }
        for _ in 0..indent_level {
            formatted.push_str(indent_str);
        }
        formatted.push_str(trimmed);
        formatted.push('\n');
        if trimmed.ends_with('{') {
            indent_level += 1;
        }
    }
    while formatted.ends_with('\n') {
        formatted.pop();
    }
    Ok(formatted)
}
fn find_helix_files(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).context("Failed to read directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "hlx" {
                    files.push(path);
                }
            }
        } else if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if dir_name == "target" || dir_name == "lib" {
                    continue;
                }
            }
            find_helix_files(&path, files)?;
        }
    }
    Ok(())
}
fn find_project_root() -> Result<PathBuf> {
    let mut current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    loop {
        let manifest_path = current_dir.join("project.hlx");
        if manifest_path.exists() {
            return Ok(current_dir);
        }
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    Err(anyhow::anyhow!("No HELIX project found. Run 'helix init' first."))
}