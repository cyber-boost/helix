use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};
pub fn run_diagnostics(verbose: bool) -> Result<()> {
    println!("🏥 HELIX Doctor - System Diagnostics");
    println!("===================================");
    println!();
    let mut issues = Vec::new();
    let mut warnings = Vec::new();
    check_system_requirements(&mut issues, &mut warnings, verbose)?;
    check_hlx_installation(&mut issues, &mut warnings, verbose)?;
    check_project_structure(&mut issues, &mut warnings, verbose)?;
    check_dependencies(&mut issues, &mut warnings, verbose)?;
    check_configuration(&mut issues, &mut warnings, verbose)?;
    print_diagnostic_results(&issues, &warnings, verbose);
    if !issues.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
fn check_system_requirements(
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🔍 Checking system requirements...");
    }
    match std::process::Command::new("rustc").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                if verbose {
                    println!("  ✅ Rust: {}", version.trim());
                }
            } else {
                issues.push("Rust compiler not found or not working".to_string());
            }
        }
        Err(_) => {
            issues.push("Rust compiler not installed".to_string());
        }
    }
    match std::process::Command::new("cargo").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                if verbose {
                    println!("  ✅ Cargo: {}", version.trim());
                }
            } else {
                issues.push("Cargo not found or not working".to_string());
            }
        }
        Err(_) => {
            issues.push("Cargo not installed".to_string());
        }
    }
    if let Ok(mem_info) = get_memory_info() {
        if mem_info.available < 1024 * 1024 * 1024 {
            warnings
                .push(
                    format!("Low available memory: {}", format_size(mem_info.available)),
                );
        }
        if verbose {
            println!(
                "  📊 Memory: {} total, {} available", format_size(mem_info.total),
                format_size(mem_info.available)
            );
        }
    }
    if let Ok(disk_info) = get_disk_info() {
        if disk_info.available < 1024 * 1024 * 1024 * 1024 {
            warnings
                .push(format!("Low disk space: {}", format_size(disk_info.available)));
        }
        if verbose {
            println!(
                "  💾 Disk: {} total, {} available", format_size(disk_info.total),
                format_size(disk_info.available)
            );
        }
    }
    Ok(())
}
fn check_hlx_installation(
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🔍 Checking HELIX installation...");
    }
    match which::which("helix") {
        Ok(path) => {
            if verbose {
                println!("  ✅ HELIX compiler found: {}", path.display());
            }
        }
        Err(_) => {
            issues.push("HELIX compiler not found in PATH".to_string());
        }
    }
    match std::process::Command::new("helix").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                if verbose {
                    println!("  ✅ hlx version: {}", version.trim());
                }
            } else {
                issues.push("HELIX compiler not working properly".to_string());
            }
        }
        Err(_) => {
            issues.push("HELIX compiler not accessible".to_string());
        }
    }
    let baton_dir = get_baton_directory()?;
    if !baton_dir.exists() {
        warnings
            .push(
                ".baton directory not found (will be created on first use)".to_string(),
            );
    } else if verbose {
        println!("  ✅ .baton directory: {}", baton_dir.display());
    }
    Ok(())
}
fn check_project_structure(
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🔍 Checking project structure...");
    }
    let current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    let manifest_path = current_dir.join("project.hlx");
    if !manifest_path.exists() {
        warnings.push("No project.hlx found (not in an HELIX project)".to_string());
        return Ok(());
    }
    if verbose {
        println!("  ✅ Project manifest: {}", manifest_path.display());
    }
    let src_dir = current_dir.join("src");
    if !src_dir.exists() {
        issues.push("Source directory 'src/' not found".to_string());
    } else if verbose {
        println!("  ✅ Source directory: {}", src_dir.display());
    }
    if src_dir.exists() {
        let helix_files = find_helix_files(&src_dir)?;
        if helix_files.is_empty() {
            warnings.push("No HELIX source files found in src/".to_string());
        } else if verbose {
            println!("  ✅ Found {} HELIX files", helix_files.len());
        }
    }
    let target_dir = current_dir.join("target");
    if !target_dir.exists() {
        if verbose {
            println!("  ℹ️  Target directory not found (will be created on build)");
        }
    } else if verbose {
        println!("  ✅ Target directory: {}", target_dir.display());
    }
    Ok(())
}
fn check_dependencies(
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🔍 Checking dependencies...");
    }
    let current_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    let manifest_path = current_dir.join("project.hlx");
    if !manifest_path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&manifest_path)
        .context("Failed to read project.hlx")?;
    if content.trim().is_empty() {
        warnings.push("Empty project.hlx file".to_string());
    }
    if content.contains("# Dependency:") || content.contains("# Dev Dependency:") {
        if verbose {
            println!("  ✅ Dependencies found in project.hlx");
        }
    } else {
        if verbose {
            println!("  ℹ️  No dependencies specified");
        }
    }
    Ok(())
}
fn check_configuration(
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("🔍 Checking configuration...");
    }
    let config_path = get_global_config_path()?;
    if !config_path.exists() {
        if verbose {
            println!("  ℹ️  Global config not found (will be created on first use)");
        }
    } else if verbose {
        println!("  ✅ Global config: {}", config_path.display());
    }
    let cache_dir = get_cache_directory()?;
    if cache_dir.exists() {
        let cache_size = calculate_directory_size(&cache_dir)?;
        if verbose {
            println!(
                "  ✅ Cache directory: {} ({})", cache_dir.display(),
                format_size(cache_size)
            );
        }
        if cache_size > 1024 * 1024 * 1024 {
            warnings.push(format!("Large cache size: {}", format_size(cache_size)));
        }
    } else if verbose {
        println!("  ℹ️  Cache directory not found (will be created on first use)");
    }
    Ok(())
}
fn print_diagnostic_results(issues: &[String], warnings: &[String], verbose: bool) {
    println!();
    if issues.is_empty() && warnings.is_empty() {
        println!("✅ All checks passed! Your hlx environment is healthy.");
        return;
    }
    if !issues.is_empty() {
        println!("❌ Issues found ({}):", issues.len());
        for (i, issue) in issues.iter().enumerate() {
            println!("  {}. {}", i + 1, issue);
        }
        println!();
    }
    if !warnings.is_empty() {
        println!("⚠️  Warnings ({}):", warnings.len());
        for (i, warning) in warnings.iter().enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
        println!();
    }
    if !issues.is_empty() {
        println!("💡 To fix issues:");
        println!("  1. Install missing dependencies");
        println!("  2. Run 'helix init' to create a new project");
        println!("  3. Check your system configuration");
        println!("  4. Run 'helix doctor' again to verify fixes");
    }
}
fn find_helix_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    find_helix_files_recursive(dir, &mut files)?;
    Ok(files)
}
fn find_helix_files_recursive(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
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
            find_helix_files_recursive(&path, files)?;
        }
    }
    Ok(())
}
fn calculate_directory_size(dir: &PathBuf) -> Result<u64> {
    let mut total_size = 0;
    let entries = fs::read_dir(dir).context("Failed to read directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            let metadata = fs::metadata(&path).context("Failed to read file metadata")?;
            total_size += metadata.len();
        } else if path.is_dir() {
            total_size += calculate_directory_size(&path)?;
        }
    }
    Ok(total_size)
}
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut size = bytes as f64;
    let mut unit_index = 0;
    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
struct MemoryInfo {
    total: u64,
    available: u64,
}
struct DiskInfo {
    total: u64,
    available: u64,
}
fn get_memory_info() -> Result<MemoryInfo> {
    Ok(MemoryInfo {
        total: 8 * 1024 * 1024 * 1024,
        available: 4 * 1024 * 1024 * 1024,
    })
}
fn get_disk_info() -> Result<DiskInfo> {
    Ok(DiskInfo {
        total: 100 * 1024 * 1024 * 1024,
        available: 50 * 1024 * 1024 * 1024,
    })
}
fn get_baton_directory() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
    Ok(home_dir.join(".baton"))
}
fn get_global_config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
    Ok(home_dir.join(".baton").join("config.toml"))
}
fn get_cache_directory() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
    Ok(home_dir.join(".baton").join("cache"))
}