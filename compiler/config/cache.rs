use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};
pub fn manage_cache(action: CacheAction, verbose: bool) -> Result<()> {
    match action {
        CacheAction::Show => show_cache_info(verbose),
        CacheAction::Clear => clear_cache(verbose),
        CacheAction::Clean => clean_cache(verbose),
        CacheAction::Size => show_cache_size(verbose),
    }
}
#[derive(Debug)]
enum CacheAction {
    Show,
    Clear,
    Clean,
    Size,
}
impl std::str::FromStr for CacheAction {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "show" => Ok(CacheAction::Show),
            "clear" => Ok(CacheAction::Clear),
            "clean" => Ok(CacheAction::Clean),
            "size" => Ok(CacheAction::Size),
            _ => Err(anyhow::anyhow!("Invalid cache action: {}", s)),
        }
    }
}
fn show_cache_info(verbose: bool) -> Result<()> {
    let cache_dir = get_cache_directory()?;
    if !cache_dir.exists() {
        println!("📁 Cache directory: {}", cache_dir.display());
        println!("   Status: Not created yet");
        return Ok(());
    }
    let cache_size = calculate_directory_size(&cache_dir)?;
    let file_count = count_files_in_directory(&cache_dir)?;
    println!("📁 Cache Information");
    println!("===================");
    println!("Location: {}", cache_dir.display());
    println!("Size: {}", format_size(cache_size));
    println!("Files: {}", file_count);
    if verbose {
        println!("\n📋 Cache Contents:");
        list_cache_contents(&cache_dir, 0)?;
    }
    Ok(())
}
fn clear_cache(verbose: bool) -> Result<()> {
    let cache_dir = get_cache_directory()?;
    if !cache_dir.exists() {
        println!("✅ Cache is already empty");
        return Ok(());
    }
    if verbose {
        println!("🗑️  Clearing cache directory: {}", cache_dir.display());
    }
    let cache_size = calculate_directory_size(&cache_dir)?;
    let file_count = count_files_in_directory(&cache_dir)?;
    fs::remove_dir_all(&cache_dir).context("Failed to remove cache directory")?;
    println!("✅ Cache cleared successfully!");
    println!("  Removed: {} files", file_count);
    println!("  Freed: {}", format_size(cache_size));
    Ok(())
}
fn clean_cache(verbose: bool) -> Result<()> {
    let cache_dir = get_cache_directory()?;
    if !cache_dir.exists() {
        println!("✅ Cache is already clean");
        return Ok(());
    }
    if verbose {
        println!("🧹 Cleaning cache directory: {}", cache_dir.display());
    }
    let mut removed_files = 0;
    let mut freed_space = 0;
    clean_expired_entries(&cache_dir, &mut removed_files, &mut freed_space, verbose)?;
    clean_orphaned_files(&cache_dir, &mut removed_files, &mut freed_space, verbose)?;
    if removed_files > 0 {
        println!("✅ Cache cleaned successfully!");
        println!("  Removed: {} files", removed_files);
        println!("  Freed: {}", format_size(freed_space));
    } else {
        println!("✅ Cache is already clean");
    }
    Ok(())
}
fn show_cache_size(verbose: bool) -> Result<()> {
    let cache_dir = get_cache_directory()?;
    if !cache_dir.exists() {
        println!("0 bytes");
        return Ok(());
    }
    let cache_size = calculate_directory_size(&cache_dir)?;
    let file_count = count_files_in_directory(&cache_dir)?;
    if verbose {
        println!("Cache Size: {}", format_size(cache_size));
        println!("Files: {}", file_count);
    } else {
        println!("{}", format_size(cache_size));
    }
    Ok(())
}
fn clean_expired_entries(
    cache_dir: &PathBuf,
    removed_files: &mut usize,
    freed_space: &mut u64,
    verbose: bool,
) -> Result<()> {
    let entries = fs::read_dir(cache_dir).context("Failed to read cache directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            let metadata = fs::metadata(&path).context("Failed to read file metadata")?;
            let modified = metadata
                .modified()
                .context("Failed to get file modification time")?;
            let age = std::time::SystemTime::now()
                .duration_since(modified)
                .unwrap_or_default();
            if age.as_secs() > 24 * 3600 {
                let file_size = metadata.len();
                fs::remove_file(&path).context("Failed to remove expired file")?;
                *removed_files += 1;
                *freed_space += file_size;
                if verbose {
                    println!(
                        "  Removed expired: {}", path.file_name().unwrap()
                        .to_string_lossy()
                    );
                }
            }
        } else if path.is_dir() {
            clean_expired_entries(&path, removed_files, freed_space, verbose)?;
        }
    }
    Ok(())
}
fn clean_orphaned_files(
    cache_dir: &PathBuf,
    removed_files: &mut usize,
    freed_space: &mut u64,
    verbose: bool,
) -> Result<()> {
    let entries = fs::read_dir(cache_dir).context("Failed to read cache directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".hlxb") {
                    let source_name = file_name.replace(".hlxb", ".hlx");
                    let source_path = cache_dir.join(&source_name);
                    if !source_path.exists() {
                        let file_size = fs::metadata(&path)?.len();
                        fs::remove_file(&path)
                            .context("Failed to remove orphaned file")?;
                        *removed_files += 1;
                        *freed_space += file_size;
                        if verbose {
                            println!("  Removed orphaned: {}", file_name);
                        }
                    }
                }
            }
        } else if path.is_dir() {
            clean_orphaned_files(&path, removed_files, freed_space, verbose)?;
        }
    }
    Ok(())
}
fn list_cache_contents(dir: &PathBuf, depth: usize) -> Result<()> {
    let entries = fs::read_dir(dir).context("Failed to read directory")?;
    let mut entries: Vec<_> = entries
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let indent = "  ".repeat(depth);
        if path.is_file() {
            let metadata = fs::metadata(&path)?;
            let size = format_size(metadata.len());
            let modified = metadata.modified()?;
            let age = std::time::SystemTime::now()
                .duration_since(modified)
                .unwrap_or_default();
            println!(
                "{}{} ({} - {} ago)", indent, path.file_name().unwrap()
                .to_string_lossy(), size, format_duration(age)
            );
        } else if path.is_dir() {
            println!("{}{}/", indent, path.file_name().unwrap().to_string_lossy());
            list_cache_contents(&path, depth + 1)?;
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
fn count_files_in_directory(dir: &PathBuf) -> Result<usize> {
    let mut count = 0;
    let entries = fs::read_dir(dir).context("Failed to read directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            count += 1;
        } else if path.is_dir() {
            count += count_files_in_directory(&path)?;
        }
    }
    Ok(count)
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
fn format_duration(duration: std::time::Duration) -> String {
    let seconds = duration.as_secs();
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86400 {
        format!("{}h", seconds / 3600)
    } else {
        format!("{}d", seconds / 86400)
    }
}
fn get_cache_directory() -> Result<PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
    Ok(home_dir.join(".baton").join("cache"))
}