use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};
use std::collections::HashMap;
pub fn remove_dependency(dependency: String, dev: bool, verbose: bool) -> Result<()> {
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
        println!("🗑️  Removing dependency: {}", dependency);
        println!(
            "  Type: {}", if dev { "dev dependency" } else { "runtime dependency" }
        );
    }
    let manifest_content = fs::read_to_string(&manifest_path)
        .context("Failed to read project.hlx")?;
    let dependency_pattern = if dev {
        format!("# Dev Dependency: {} v", dependency)
    } else {
        format!("# Dependency: {} v", dependency)
    };
    let updated_content = manifest_content
        .lines()
        .filter(|line| !line.trim().starts_with(&dependency_pattern))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&manifest_path, updated_content)
        .context("Failed to write updated project.hlx")?;
    println!("✅ Removed dependency: {}", dependency);
    if verbose {
        println!("  Updated project.hlx");
        println!("  Run 'helix build' to update dependencies");
    }
    Ok(())
}
pub fn remove_dependencies(
    dependencies: Vec<String>,
    dev: bool,
    verbose: bool,
) -> Result<()> {
    for dep in dependencies {
        remove_dependency(dep, dev, verbose)?;
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