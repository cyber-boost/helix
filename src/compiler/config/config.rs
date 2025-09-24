use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub fn manage_config(
    action: ConfigAction,
    key: Option<String>,
    value: Option<String>,
    verbose: bool,
) -> Result<()> {
    match action {
        ConfigAction::Get => get_config(key, verbose),
        ConfigAction::Set => set_config(key, value, verbose),
        ConfigAction::List => list_config(verbose),
        ConfigAction::Unset => unset_config(key, verbose),
        ConfigAction::Edit => edit_config(verbose),
    }
}
#[derive(Debug)]
enum ConfigAction {
    Get,
    Set,
    List,
    Unset,
    Edit,
}
impl std::str::FromStr for ConfigAction {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "get" => Ok(ConfigAction::Get),
            "set" => Ok(ConfigAction::Set),
            "list" => Ok(ConfigAction::List),
            "unset" => Ok(ConfigAction::Unset),
            "edit" => Ok(ConfigAction::Edit),
            _ => Err(anyhow::anyhow!("Invalid config action: {}", s)),
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
struct GlobalConfig {
    compiler: CompilerConfig,
    registry: RegistryConfig,
    cache: CacheConfig,
    editor: EditorConfig,
    metadata: ConfigMetadata,
}
#[derive(Debug, Serialize, Deserialize)]
struct CompilerConfig {
    optimization_level: u8,
    compression: bool,
    cache_enabled: bool,
    verbose_output: bool,
    target_platform: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct RegistryConfig {
    default_registry: String,
    auth_token: Option<String>,
    timeout: u64,
    retry_count: u8,
}
#[derive(Debug, Serialize, Deserialize)]
struct CacheConfig {
    enabled: bool,
    max_size: u64,
    ttl: u64,
    location: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct EditorConfig {
    default_editor: String,
    auto_format: bool,
    syntax_highlighting: bool,
}
#[derive(Debug, Serialize, Deserialize)]
struct ConfigMetadata {
    version: String,
    created_at: String,
    last_modified: String,
}
fn get_config(key: Option<String>, verbose: bool) -> Result<()> {
    let config = load_global_config()?;
    if let Some(key) = key {
        let value = get_config_value(&config, &key)?;
        println!("{}", value);
    } else {
        let json = serde_json::to_string_pretty(&config)?;
        println!("{}", json);
    }
    Ok(())
}
fn set_config(key: Option<String>, value: Option<String>, verbose: bool) -> Result<()> {
    let key = key.ok_or_else(|| anyhow::anyhow!("Key is required for set operation"))?;
    let value = value
        .ok_or_else(|| anyhow::anyhow!("Value is required for set operation"))?;
    let mut config = load_global_config()?;
    set_config_value(&mut config, &key, &value)?;
    save_global_config(&config)?;
    if verbose {
        println!("✅ Set {} = {}", key, value);
    }
    Ok(())
}
fn list_config(verbose: bool) -> Result<()> {
    let config = load_global_config()?;
    println!("HELIX Global Configuration");
    println!("========================");
    println!();
    println!("Compiler Settings:");
    println!("  optimization_level: {}", config.compiler.optimization_level);
    println!("  compression: {}", config.compiler.compression);
    println!("  cache_enabled: {}", config.compiler.cache_enabled);
    println!("  verbose_output: {}", config.compiler.verbose_output);
    println!("  target_platform: {}", config.compiler.target_platform);
    println!();
    println!("Registry Settings:");
    println!("  default_registry: {}", config.registry.default_registry);
    println!(
        "  auth_token: {}", config.registry.auth_token.as_deref().unwrap_or("(not set)")
    );
    println!("  timeout: {}s", config.registry.timeout);
    println!("  retry_count: {}", config.registry.retry_count);
    println!();
    println!("Cache Settings:");
    println!("  enabled: {}", config.cache.enabled);
    println!("  max_size: {}MB", config.cache.max_size / (1024 * 1024));
    println!("  ttl: {}s", config.cache.ttl);
    println!("  location: {}", config.cache.location);
    println!();
    println!("Editor Settings:");
    println!("  default_editor: {}", config.editor.default_editor);
    println!("  auto_format: {}", config.editor.auto_format);
    println!("  syntax_highlighting: {}", config.editor.syntax_highlighting);
    println!();
    if verbose {
        println!("Metadata:");
        println!("  version: {}", config.metadata.version);
        println!("  created_at: {}", config.metadata.created_at);
        println!("  last_modified: {}", config.metadata.last_modified);
    }
    Ok(())
}
fn unset_config(key: Option<String>, verbose: bool) -> Result<()> {
    let key = key.ok_or_else(|| anyhow::anyhow!("Key is required for unset operation"))?;
    let mut config = load_global_config()?;
    unset_config_value(&mut config, &key)?;
    save_global_config(&config)?;
    if verbose {
        println!("✅ Unset {}", key);
    }
    Ok(())
}
fn edit_config(verbose: bool) -> Result<()> {
    let config_path = get_global_config_path()?;
    if verbose {
        println!("📝 Opening config file for editing: {}", config_path.display());
    }
    let editor = get_default_editor()?;
    let status = std::process::Command::new(&editor)
        .arg(&config_path)
        .status()
        .context("Failed to open editor")?;
    if !status.success() {
        return Err(
            anyhow::anyhow!(
                "Editor exited with error code: {}", status.code().unwrap_or(- 1)
            ),
        );
    }
    println!("✅ Configuration updated");
    Ok(())
}
fn load_global_config() -> Result<GlobalConfig> {
    let config_path = get_global_config_path()?;
    if !config_path.exists() {
        let default_config = create_default_config()?;
        save_global_config(&default_config)?;
        return Ok(default_config);
    }
    let content = fs::read_to_string(&config_path)
        .context("Failed to read config file")?;
    let config: GlobalConfig = toml::from_str(&content)
        .context("Failed to parse config file")?;
    Ok(config)
}
fn save_global_config(config: &GlobalConfig) -> Result<()> {
    let config_path = get_global_config_path()?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create config directory")?;
    }
    let content = toml::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(&config_path, content).context("Failed to write config file")?;
    Ok(())
}
fn create_default_config() -> Result<GlobalConfig> {
    Ok(GlobalConfig {
        compiler: CompilerConfig {
            optimization_level: 2,
            compression: true,
            cache_enabled: true,
            verbose_output: false,
            target_platform: "native".to_string(),
        },
        registry: RegistryConfig {
            default_registry: "https://registry.helix.cm".to_string(),
            auth_token: None,
            timeout: 30,
            retry_count: 3,
        },
        cache: CacheConfig {
            enabled: true,
            max_size: 100 * 1024 * 1024,
            ttl: 3600,
            location: get_cache_directory()?.to_string_lossy().to_string(),
        },
        editor: EditorConfig {
            default_editor: get_default_editor().unwrap_or_else(|_| "nano".to_string()),
            auto_format: true,
            syntax_highlighting: true,
        },
        metadata: ConfigMetadata {
            version: "1.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_modified: chrono::Utc::now().to_rfc3339(),
        },
    })
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
fn get_default_editor() -> Result<String> {
    if let Ok(editor) = std::env::var("EDITOR") {
        return Ok(editor);
    }
    if let Ok(editor) = std::env::var("VISUAL") {
        return Ok(editor);
    }
    let common_editors = vec!["nano", "vim", "emacs", "code", "subl"];
    for editor in common_editors {
        if which::which(editor).is_ok() {
            return Ok(editor.to_string());
        }
    }
    Err(anyhow::anyhow!("No suitable editor found"))
}
fn get_config_value(config: &GlobalConfig, key: &str) -> Result<String> {
    match key {
        "compiler.optimization_level" => {
            Ok(config.compiler.optimization_level.to_string())
        }
        "compiler.compression" => Ok(config.compiler.compression.to_string()),
        "compiler.cache_enabled" => Ok(config.compiler.cache_enabled.to_string()),
        "compiler.verbose_output" => Ok(config.compiler.verbose_output.to_string()),
        "compiler.target_platform" => Ok(config.compiler.target_platform.clone()),
        "registry.default_registry" => Ok(config.registry.default_registry.clone()),
        "registry.auth_token" => {
            Ok(config.registry.auth_token.clone().unwrap_or_default())
        }
        "registry.timeout" => Ok(config.registry.timeout.to_string()),
        "registry.retry_count" => Ok(config.registry.retry_count.to_string()),
        "cache.enabled" => Ok(config.cache.enabled.to_string()),
        "cache.max_size" => Ok(config.cache.max_size.to_string()),
        "cache.ttl" => Ok(config.cache.ttl.to_string()),
        "cache.location" => Ok(config.cache.location.clone()),
        "editor.default_editor" => Ok(config.editor.default_editor.clone()),
        "editor.auto_format" => Ok(config.editor.auto_format.to_string()),
        "editor.syntax_highlighting" => Ok(config.editor.syntax_highlighting.to_string()),
        _ => Err(anyhow::anyhow!("Unknown config key: {}", key)),
    }
}
fn set_config_value(config: &mut GlobalConfig, key: &str, value: &str) -> Result<()> {
    match key {
        "compiler.optimization_level" => {
            config.compiler.optimization_level = value
                .parse()
                .context("Invalid optimization level")?;
        }
        "compiler.compression" => {
            config.compiler.compression = value
                .parse()
                .context("Invalid compression value")?;
        }
        "compiler.cache_enabled" => {
            config.compiler.cache_enabled = value
                .parse()
                .context("Invalid cache_enabled value")?;
        }
        "compiler.verbose_output" => {
            config.compiler.verbose_output = value
                .parse()
                .context("Invalid verbose_output value")?;
        }
        "compiler.target_platform" => {
            config.compiler.target_platform = value.to_string();
        }
        "registry.default_registry" => {
            config.registry.default_registry = value.to_string();
        }
        "registry.auth_token" => {
            config.registry.auth_token = Some(value.to_string());
        }
        "registry.timeout" => {
            config.registry.timeout = value.parse().context("Invalid timeout value")?;
        }
        "registry.retry_count" => {
            config.registry.retry_count = value
                .parse()
                .context("Invalid retry_count value")?;
        }
        "cache.enabled" => {
            config.cache.enabled = value.parse().context("Invalid cache enabled value")?;
        }
        "cache.max_size" => {
            config.cache.max_size = value.parse().context("Invalid max_size value")?;
        }
        "cache.ttl" => {
            config.cache.ttl = value.parse().context("Invalid ttl value")?;
        }
        "cache.location" => {
            config.cache.location = value.to_string();
        }
        "editor.default_editor" => {
            config.editor.default_editor = value.to_string();
        }
        "editor.auto_format" => {
            config.editor.auto_format = value
                .parse()
                .context("Invalid auto_format value")?;
        }
        "editor.syntax_highlighting" => {
            config.editor.syntax_highlighting = value
                .parse()
                .context("Invalid syntax_highlighting value")?;
        }
        _ => return Err(anyhow::anyhow!("Unknown config key: {}", key)),
    }
    config.metadata.last_modified = chrono::Utc::now().to_rfc3339();
    Ok(())
}
fn unset_config_value(config: &mut GlobalConfig, key: &str) -> Result<()> {
    match key {
        "registry.auth_token" => {
            config.registry.auth_token = None;
        }
        _ => return Err(anyhow::anyhow!("Cannot unset key: {}", key)),
    }
    config.metadata.last_modified = chrono::Utc::now().to_rfc3339();
    Ok(())
}