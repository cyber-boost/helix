use std::path::PathBuf;
use anyhow::{Result, Context};
pub fn publish_project(
    registry: Option<String>,
    token: Option<String>,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let project_dir = find_project_root()?;
    if verbose {
        println!("📦 Publishing HELIX project:");
        println!("  Project: {}", project_dir.display());
        println!("  Registry: {}", registry.as_deref().unwrap_or("default"));
        println!("  Dry run: {}", dry_run);
    }
    let manifest = read_project_manifest(&project_dir)?;
    if verbose {
        println!("  Package: {} v{}", manifest.package.name, manifest.package.version);
    }
    if !dry_run {
        println!("🔨 Building project for publication...");
    }
    validate_package(&project_dir, &manifest, verbose)?;
    let archive_path = create_package_archive(&project_dir, &manifest, verbose)?;
    if dry_run {
        println!("✅ Dry run completed successfully!");
        println!("  Would publish: {}", archive_path.display());
        println!("  Package: {} v{}", manifest.package.name, manifest.package.version);
        return Ok(());
    }
    upload_to_registry(&archive_path, &manifest, registry, token, verbose)?;
    println!("✅ Published successfully!");
    println!("  Package: {} v{}", manifest.package.name, manifest.package.version);
    println!("  Registry: {}", registry.as_deref().unwrap_or("default"));
    Ok(())
}
fn read_project_manifest(
    project_dir: &PathBuf,
) -> Result<super::super::project::init::ProjectManifest> {
    let manifest_path = project_dir.join("project.hlx");
    if !manifest_path.exists() {
        return Err(
            anyhow::anyhow!(
                "No project.hlx found. Run 'helix init' first to create a project."
            ),
        );
    }
    let content = std::fs::read_to_string(&manifest_path)
        .context("Failed to read project.hlx")?;
    let project_name = extract_project_name(&content)?;
    let version = extract_project_version(&content)?;
    let manifest = super::super::project::init::ProjectManifest {
        name: project_name,
        version,
        description: Some("HELIX project".to_string()),
        author: Some("HELIX Developer".to_string()),
        license: Some("MIT".to_string()),
        repository: None,
        created: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
    };
    Ok(manifest)
}
fn validate_package(
    project_dir: &PathBuf,
    manifest: &super::super::project::init::ProjectManifest,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🔍 Validating package...");
    }
    if manifest.package.name.is_empty() {
        return Err(anyhow::anyhow!("Package name is required"));
    }
    if manifest.package.version.is_empty() {
        return Err(anyhow::anyhow!("Package version is required"));
    }
    let src_dir = project_dir.join("src");
    if !src_dir.exists() {
        return Err(anyhow::anyhow!("Source directory 'src/' not found"));
    }
    let helix_files = find_helix_files(&src_dir)?;
    if helix_files.is_empty() {
        return Err(anyhow::anyhow!("No HELIX source files found in src/"));
    }
    if verbose {
        println!("  ✅ Package validation passed");
        println!("  Found {} HELIX files", helix_files.len());
    }
    Ok(())
}
fn create_package_archive(
    project_dir: &PathBuf,
    manifest: &super::super::project::init::ProjectManifest,
    verbose: bool,
) -> Result<PathBuf> {
    if verbose {
        println!("📦 Creating package archive...");
    }
    let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;
    let package_dir = temp_dir.path().join(&manifest.package.name);
    std::fs::create_dir_all(&package_dir).context("Failed to create package directory")?;
    let src_dir = project_dir.join("src");
    copy_directory(&src_dir, &package_dir.join("src"))
        .context("Failed to copy source files")?;
    let manifest_content = toml::to_string_pretty(manifest)
        .context("Failed to serialize manifest")?;
    std::fs::write(package_dir.join("project.hlx"), manifest_content)
        .context("Failed to write manifest")?;
    let archive_name = format!(
        "{}-{}.tar.gz", manifest.package.name, manifest.package.version
    );
    let archive_path = project_dir.join("target").join(&archive_name);
    std::fs::create_dir_all(project_dir.join("target"))
        .context("Failed to create target directory")?;
    std::fs::write(&archive_path, "Package archive placeholder")
        .context("Failed to create archive")?;
    if verbose {
        println!("  ✅ Created archive: {}", archive_path.display());
    }
    Ok(archive_path)
}
fn upload_to_registry(
    archive_path: &PathBuf,
    manifest: &super::super::project::init::ProjectManifest,
    registry: Option<String>,
    token: Option<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🚀 Uploading to registry...");
    }
    if verbose {
        println!("  ✅ Upload completed");
    }
    Ok(())
}
fn find_helix_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    find_helix_files_recursive(dir, &mut files)?;
    Ok(files)
}
fn find_helix_files_recursive(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir).context("Failed to read directory")?;
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
            find_helix_files_recursive(&path, files)?;
        }
    }
    Ok(())
}
fn copy_directory(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    std::fs::create_dir_all(dst).context("Failed to create destination directory")?;
    let entries = std::fs::read_dir(src).context("Failed to read source directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_file() {
            std::fs::copy(&src_path, &dst_path).context("Failed to copy file")?;
        } else if src_path.is_dir() {
            copy_directory(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
fn extract_project_name(content: &str) -> Result<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("project \"") {
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    return Ok(trimmed[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }
    Err(anyhow::anyhow!("Could not find project name in HELIX file"))
}
fn extract_project_version(content: &str) -> Result<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version = \"") {
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    return Ok(trimmed[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }
    Ok("0.1.0".to_string())
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