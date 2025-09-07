use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub fn add_dependency(
    dependency: String,
    version: Option<String>,
    dev: bool,
    verbose: bool,
) -> Result<()> {
    let project_dir = find_project_root()?;
    let manifest_path = project_dir.join("project.hlx");
    if !manifest_path.exists() {
        return Err(
            anyhow::anyhow!(
                "No project.hlx found. Run 'helix init' first to create a project."
            ),
        );
    }
    if verbose {
        println!("📦 Adding dependency: {}", dependency);
        println!("  Version: {}", version.as_deref().unwrap_or("latest"));
        println!(
            "  Type: {}", if dev { "dev dependency" } else { "runtime dependency" }
        );
    }
    let manifest_content = fs::read_to_string(&manifest_path)
        .context("Failed to read project.hlx")?;
    let dep_version = version.unwrap_or_else(|| "latest".to_string());
    let dependency_section = if dev {
        format!("\n# Dev Dependency: {} v{}\n", dependency, dep_version)
    } else {
        format!("\n# Dependency: {} v{}\n", dependency, dep_version)
    };
    let updated_content = manifest_content + &dependency_section;
    fs::write(&manifest_path, updated_content)
        .context("Failed to write updated project.hlx")?;
    println!(
        "✅ Added dependency: {} {}", dependency, version.as_deref().unwrap_or("latest")
    );
    if verbose {
        println!("  Updated project.hlx");
        println!("  Run 'helix build' to install dependencies");
    }
    Ok(())
}
pub fn add_dependencies(
    dependencies: Vec<String>,
    version: Option<String>,
    dev: bool,
    verbose: bool,
) -> Result<()> {
    for dep in dependencies {
        add_dependency(dep, version.clone(), dev, verbose)?;
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