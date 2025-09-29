use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
pub fn clean_project(all: bool, cache: bool, verbose: bool) -> Result<()> {
    let project_dir = find_project_root()?;
    if verbose {
        println!("🧹 Cleaning project: {}", project_dir.display());
        println!("  All artifacts: {}", all);
        println!("  Cache: {}", cache);
    }
    let mut cleaned_items: Vec<String> = Vec::new();
    let target_dir = project_dir.join("target");
    if target_dir.exists() {
        if all {
            fs::remove_dir_all(target_dir.as_path())
                .context("Failed to remove target directory")?;
            cleaned_items.push(String::from("target/"));
        } else {
            clean_directory(target_dir.as_path(), "*.hlxb", &mut cleaned_items)?;
        }
    }
    if cache {
        let cache_dir = get_cache_directory()?;
        if cache_dir.exists() {
            fs::remove_dir_all(cache_dir.as_path()).context("Failed to remove cache directory")?;
            cleaned_items.push(String::from("cache/"));
        }
    }
    if all {
        clean_temp_files(project_dir.as_path(), &mut cleaned_items)?;
    }
    if cleaned_items.is_empty() {
        println!("✨ Project is already clean!");
    } else {
        println!("✅ Cleaned {} items:", cleaned_items.len());
        for item in cleaned_items {
            println!("  - {}", item);
        }
    }
    Ok(())
}
pub fn reset_project(force: bool, verbose: bool) -> Result<()> {
    let project_dir = find_project_root()?;
    if !force {
        return Err(
            anyhow::anyhow!(
                "Reset will remove all build artifacts and generated files.\n\
            Use --force to confirm this action."
            ),
        );
    }
    if verbose {
        println!("🔄 Resetting project: {}", project_dir.display());
    }
    let mut removed_items: Vec<String> = Vec::new();
    let target_dir = project_dir.join("target");
    if target_dir.exists() {
        fs::remove_dir_all(target_dir.as_path()).context("Failed to remove target directory")?;
        removed_items.push(String::from("target/"));
    }
    let lib_dir = project_dir.join("lib");
    if lib_dir.exists() {
        fs::remove_dir_all(lib_dir.as_path()).context("Failed to remove lib directory")?;
        removed_items.push(String::from("lib/"));
    }
    let cache_dir = get_cache_directory()?;
    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir.as_path()).context("Failed to remove cache directory")?;
        removed_items.push(String::from("cache/"));
    }
    clean_temp_files(project_dir.as_path(), &mut removed_items)?;
    println!("✅ Project reset successfully!");
    println!("  Removed {} items:", removed_items.len());
    for item in removed_items {
        println!("  - {}", item);
    }
    println!("\n📋 Next steps:");
    println!("  1. Run 'helix build' to rebuild the project");
    println!("  2. Run 'helix run' to test the project");
    Ok(())
}
fn clean_directory(
    dir: &Path,
    pattern: &str,
    cleaned_items: &mut Vec<String>,
) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).context("Failed to read directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if pattern == "*.hlxb" && file_name.ends_with(".hlxb") {
                    fs::remove_file(&path).context("Failed to remove file")?;
                    cleaned_items.push(format!("target/{}", file_name));
                }
            }
        }
    }
    Ok(())
}
fn clean_temp_files(
    project_dir: &Path,
    cleaned_items: &mut Vec<String>,
) -> Result<()> {
    let temp_patterns = vec!["*.tmp", "*.temp", "*.log", ".DS_Store", "Thumbs.db",];
    let entries = fs::read_dir(project_dir).context("Failed to read project directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                for pattern in &temp_patterns {
                    if matches_pattern(file_name, pattern) {
                        fs::remove_file(&path).context("Failed to remove temp file")?;
                        cleaned_items.push(file_name.to_string());
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
fn matches_pattern(file_name: &str, pattern: &str) -> bool {
    if pattern.starts_with("*.") {
        let ext = &pattern[2..];
        file_name.ends_with(ext)
    } else {
        file_name == pattern
    }
}
fn get_cache_directory() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
    Ok(home_dir.join(".baton").join("cache"))
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