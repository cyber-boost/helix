use std::path::PathBuf;
use crate::mds::templates::get_embedded_templates;

pub fn init_command(
    template: String,
    dir: Option<PathBuf>,
    name: Option<String>,
    force: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let templates = get_embedded_templates();
    let template_content = templates
        .iter()
        .find(|(t, _)| t == &template)
        .map(|(_, content)| *content)
        .ok_or_else(|| {
            let available: Vec<&str> = templates
                .iter()
                .map(|(name, _)| *name)
                .collect();
            format!(
                "Unknown template '{}'. Available templates: {}", template, available
                .join(", ")
            )
        })?;
    let output_dir = dir
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });
    let filename = name
        .unwrap_or_else(|| {
            match template.as_str() {
                "ai-dev" => "ai_development_team.hlx".to_string(),
                "data-pipeline" => "data_pipeline.hlx".to_string(),
                _ => format!("{}.hlx", template),
            }
        });
    let output_path = output_dir.join(&filename);
    if output_path.exists() && !force {
        return Err(
            anyhow::anyhow!(
                "File '{}' already exists. Use --force to overwrite.", output_path
                .display()
            )
                .into(),
        );
    }
    if verbose {
        println!("🚀 Initializing HELIX project:");
        println!("  Template: {}", template);
        println!("  Output: {}", output_path.display());
        println!("  Force: {}", force);
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, template_content)?;
    println!("✅ HELIX project initialized successfully!");
    println!("  Created: {}", output_path.display());
    println!("  Template: {}", template);
    if verbose {
        let content_size = template_content.len();
        println!("  Size: {} bytes", content_size);
        let description = match template.as_str() {
            "minimal" => "Simple hlx configuration with basic agent and workflow",
            "ai-dev" => {
                "Complete AI development team with specialized agents for full-stack development"
            }
            "support" => {
                "Multi-tier customer support system with escalation and knowledge management"
            }
            "data-pipeline" => {
                "High-throughput data processing pipeline with ML integration"
            }
            "research" => {
                "AI-powered research assistant for literature review and paper writing"
            }
            _ => "HELIX configuration template",
        };
        println!("  Description: {}", description);
    }
    println!("\n📋 Next steps:");
    println!("  1. Review and customize the configuration");
    println!("  2. Set up your API keys and environment variables");
    println!("  3. Compile with: helix compile {}", filename);
    println!("  4. Run with your hlx runtime");
    Ok(())
}
fn install_command(
    local_only: bool,
    force: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        println!("🔧 Installing Helix compiler globally...");
    }
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    if verbose {
        println!("  Source: {}", current_exe.display());
    }
    let home_dir = std::env::var("HOME")
        .map_err(|e| format!("Failed to get HOME directory: {}", e))?;
    let baton_dir = PathBuf::from(&home_dir).join(".baton");
    let baton_bin_dir = baton_dir.join("bin");
    let target_binary = baton_bin_dir.join("hlx");
    if verbose {
        println!("  Target: {}", target_binary.display());
    }
    std::fs::create_dir_all(&baton_bin_dir)
        .map_err(|e| {
            format!("Failed to create directory {}: {}", baton_bin_dir.display(), e)
        })?;
    if verbose {
        println!("  ✅ Created directory: {}", baton_bin_dir.display());
    }
    if target_binary.exists() && !force {
        return Err(
            format!(
                "HELIX compiler already installed at {}. Use --force to overwrite.",
                target_binary.display()
            )
                .into(),
        );
    }
    std::fs::copy(&current_exe, &target_binary)
        .map_err(|e| {
            format!("Failed to copy binary to {}: {}", target_binary.display(), e)
        })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target_binary)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target_binary, perms)?;
    }
    if verbose {
        println!("  ✅ Copied binary to: {}", target_binary.display());
    }
    println!("✅ Helix compiler installed successfully!");
    println!("  Location: {}", target_binary.display());
    if local_only {
        println!("\n📋 Local installation complete!");
        println!("  Add {} to your PATH to use 'hlx' command", baton_bin_dir.display());
        println!("  Or run: export PATH=\"{}:$PATH\"", baton_bin_dir.display());
        return Ok(());
    }
    let global_bin_paths = vec![
        PathBuf::from("/usr/local/bin"), PathBuf::from("/usr/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin"),
    ];
    let mut symlink_created = false;
    for global_bin in global_bin_paths {
        if global_bin.exists() && global_bin.is_dir() {
            let symlink_path = global_bin.join("hlx");
            if symlink_path.exists() && !force {
                if verbose {
                    println!(
                        "  ⚠️  Symlink already exists: {}", symlink_path.display()
                    );
                }
                continue;
            }
            if symlink_path.exists() {
                std::fs::remove_file(&symlink_path)
                    .map_err(|e| {
                        format!(
                            "Failed to remove existing symlink {}: {}", symlink_path
                            .display(), e
                        )
                    })?;
            }
            #[cfg(unix)]
            let symlink_result = std::os::unix::fs::symlink(
                &target_binary,
                &symlink_path,
            );
            #[cfg(windows)]
            let symlink_result = {
                std::fs::copy(&target_binary, &symlink_path)
                    .map(|_| ())
                    .or_else(|_| std::os::windows::fs::symlink_file(
                        &target_binary,
                        &symlink_path,
                    ))
            };
            #[cfg(not(any(unix, windows)))]
            let symlink_result = std::fs::copy(&target_binary, &symlink_path)
                .map(|_| ());
            match symlink_result {
                Ok(_) => {
                    println!("  ✅ Created global link: {}", symlink_path.display());
                    symlink_created = true;
                    break;
                }
                Err(e) => {
                    if verbose {
                        println!(
                            "  ⚠️  Failed to create link at {}: {}", symlink_path
                            .display(), e
                        );
                    }
                    continue;
                }
            }
        }
    }
    if symlink_created {
        println!("\n🎉 Global installation complete!");
        println!("  You can now use 'hlx' command from anywhere");
        println!("  Try: hlx --help");
    } else {
        println!("\n📋 Installation complete, but global symlink creation failed");
        println!("  This might be due to insufficient permissions");
        println!(
            "  You can still use hlx by adding {} to your PATH", baton_bin_dir.display()
        );
        println!("  Or run: export PATH=\"{}:$PATH\"", baton_bin_dir.display());
        if verbose {
            println!("\n💡 To create global symlink manually:");
            println!("  sudo ln -sf {} /usr/local/bin/hlx", target_binary.display());
        }
    }
    Ok(())
}

use std::fs;
use anyhow::{Result, Context};
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub created: Option<String>,
}
pub fn init_project(
    name: Option<String>,
    dir: Option<PathBuf>,
    template: Option<String>,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let project_dir = dir
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });
    let project_name = name
        .unwrap_or_else(|| {
            project_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("hlx-project")
                .to_string()
        });
    if verbose {
        println!("🚀 Initializing HELIX project:");
        println!("  Name: {}", project_name);
        println!("  Directory: {}", project_dir.display());
        println!("  Template: {}", template.as_deref().unwrap_or("minimal"));
    }
    create_project_structure(&project_dir, &project_name, force)?;
    create_manifest(&project_dir, &project_name, template.as_deref())?;
    create_example_files(&project_dir, template.as_deref())?;
    register_project_globally(&project_name, &project_dir)?;
    println!("✅ HELIX project '{}' initialized successfully!", project_name);
    println!("  Location: {}", project_dir.display());
    if verbose {
        println!("\n📁 Project structure created:");
        println!("  project.hlx - Project manifest");
        println!("  src/ - Source files directory");
        println!("  target/ - Build artifacts");
        println!("  lib/ - Dependencies");
    }
    println!("\n📋 Next steps:");
    println!("  1. cd {}", project_dir.display());
    println!("  2. Edit src/main.hlx to customize your configuration");
    println!("  3. Add dependencies with: helix add <dependency>");
    println!("  4. Build with: helix build");
    println!("  5. Run with: helix run");
    Ok(())
}
fn create_project_structure(
    project_dir: &PathBuf,
    _project_name: &str,
    force: bool,
) -> Result<()> {
    if project_dir.exists() && !force {
        let entries: Vec<_> = fs::read_dir(project_dir)
            .context("Failed to read project directory")?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to read directory entries")?;
        if !entries.is_empty() {
            return Err(
                anyhow::anyhow!(
                    "Directory '{}' is not empty. Use --force to initialize anyway.",
                    project_dir.display()
                ),
            );
        }
    }
    let src_dir = project_dir.join("src");
    let target_dir = project_dir.join("target");
    let lib_dir = project_dir.join("lib");
    fs::create_dir_all(&src_dir).context("Failed to create src directory")?;
    fs::create_dir_all(&target_dir).context("Failed to create target directory")?;
    fs::create_dir_all(&lib_dir).context("Failed to create lib directory")?;
    Ok(())
}
fn create_manifest(
    project_dir: &PathBuf,
    project_name: &str,
    template: Option<&str>,
) -> Result<()> {
    let manifest_path = project_dir.join("project.hlx");
    let current_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let helix_content = format!(
        r#"# {} Project Configuration
# Generated by HELIX Compiler

project "{}" {{
    version = "0.1.0"
    author = "HELIX Developer"
    description = "HELIX project: {}"
    created = "{}"
    license = "MIT"
}}

# Basic agent for the project
agent "main-agent" {{
    model = "gpt-4"
    role = "Main Agent"
    temperature = 0.7
    max_tokens = 50000
    
    capabilities [
        "general-purpose"
        "task-execution"
        "problem-solving"
    ]
}}

# Basic workflow
workflow "main-workflow" {{
    trigger = "manual"
    
    step "execute" {{
        agent = "main-agent"
        task = "Execute main task"
        timeout = 30m
    }}
}}

# Development context
context "development" {{
    environment = "dev"
    debug = true
    max_tokens = 50000
    
    variables {{
        log_level = "debug"
        timeout = 60s
        retry_count = 3
    }}
}}
"#,
        project_name, project_name, project_name, current_date
    );
    fs::write(&manifest_path, helix_content).context("Failed to write project.hlx")?;
    Ok(())
}
fn create_example_files(project_dir: &PathBuf, template: Option<&str>) -> Result<()> {
    let src_dir = project_dir.join("src");
    let main_content = match template {
        Some("ai-dev") => r#"# AI Development Team Configuration
project "ai-dev-team" {
    version = "1.0.0"
    description = "AI-powered development team"
}

agent "architect" {
    model = "gpt-4"
    role = "System Architect"
    temperature = 0.7
}

agent "developer" {
    model = "gpt-4"
    role = "Code Developer"
    temperature = 0.3
}
"#,
        Some("support") => r#"# Customer Support Configuration
project "support-team" {
    version = "1.0.0"
    description = "Customer support system"
}

agent "support-agent" {
    model = "gpt-4"
    role = "Customer Support Specialist"
    temperature = 0.8
}
"#,
        Some("data-pipeline") => r#"# Data Pipeline Configuration
project "data-pipeline" {
    version = "1.0.0"
    description = "Data processing pipeline"
}

agent "data-processor" {
    model = "gpt-4"
    role = "Data Processing Agent"
    temperature = 0.5
}
"#,
        Some("research") => r#"# Research Assistant Configuration
project "research-assistant" {
    version = "1.0.0"
    description = "AI research assistant"
}

agent "researcher" {
    model = "gpt-4"
    role = "Research Assistant"
    temperature = 0.6
}
"#,
        _ => r#"# Minimal HELIX Configuration
project "minimal-project" {
    version = "0.1.0"
    description = "A minimal HELIX project"
}

agent "basic-agent" {
    model = "gpt-4"
    role = "Basic Assistant"
    temperature = 0.7
    max_tokens = 50000
}

workflow "basic-workflow" {
    trigger = "manual"

    step "process" {
        agent = "basic-agent"
        task = "Process the request"
        timeout = 30m
    }
}
"#,
    };
    let main_path = src_dir.join("main.hlx");
    fs::write(&main_path, main_content).context("Failed to write main.hlx")?;
    let gitignore_content = r#"# hlx Build artifacts
target/
*.hlxb

# Dependencies
lib/

# IDE files
.vscode/
.idea/
*.swp
*.swo

# OS files
.DS_Store
Thumbs.db

# Logs
*.log
"#;
    let gitignore_path = project_dir.join(".gitignore");
    fs::write(&gitignore_path, gitignore_content).context("Failed to write .gitignore")?;
    Ok(())
}
fn register_project_globally(project_name: &str, project_dir: &PathBuf) -> Result<()> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
    let baton_dir = home_dir.join(".baton");
    let projects_dir = baton_dir.join("projects");
    fs::create_dir_all(&projects_dir)
        .context("Failed to create .baton/projects directory")?;
    let project_registry = projects_dir.join(format!("{}.hlx", project_name));
    let current_time = chrono::Utc::now().to_rfc3339();
    let registry_content = format!(
        r#"# Project Registry Entry: {}
# Generated by HELIX Compiler

project "{}" {{
    name = "{}"
    path = "{}"
    created_at = "{}"
    last_accessed = "{}"
    status = "active"
}}
"#,
        project_name, project_name, project_name, project_dir.to_string_lossy(),
        current_time, current_time
    );
    fs::write(&project_registry, registry_content)
        .context("Failed to write project registry entry")?;
    Ok(())
}