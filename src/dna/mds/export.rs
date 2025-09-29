use std::path::PathBuf;
use anyhow::{Result, Context};
use crate::mds::init::ProjectManifest;
use serde::{Serialize, Deserialize};
use serde_json;
use serde_yaml;
use toml;
use chrono;


pub fn export_project(
    format: String,
    output: Option<PathBuf>,
    include_deps: bool,
    verbose: bool,
) -> Result<()> {
    let project_dir = find_project_root()?;
    if verbose {
        println!("📤 Exporting HELIX project:");
        println!("  Project: {}", project_dir.display());
        println!("  Format: {}", format);
        println!("  Include dependencies: {}", include_deps);
    }
    match format.as_str() {
        "json" => export_to_json(&project_dir, output, include_deps, verbose)?,
        "yaml" => export_to_yaml(&project_dir, output, include_deps, verbose)?,
        "toml" => export_to_toml(&project_dir, output, include_deps, verbose)?,
        "docker" => export_to_docker(&project_dir, output, verbose)?,
        "k8s" => export_to_kubernetes(&project_dir, output, verbose)?,
        _ => {
            return Err(
                anyhow::anyhow!(
                    "Unknown export format '{}'. Available formats: json, yaml, toml, docker, k8s",
                    format
                ),
            );
        }
    }
    println!("✅ Export completed successfully!");
    Ok(())
}
pub fn import_project(
    input: PathBuf,
    format: Option<String>,
    force: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("📥 Importing HELIX project:");
        println!("  Input: {}", input.display());
        println!("  Format: {}", format.as_deref().unwrap_or("auto-detect"));
    }
    if !input.exists() {
        return Err(anyhow::anyhow!("Input file not found: {}", input.display()));
    }
    let detected_format = format
        .unwrap_or_else(|| {
            if let Some(extension) = input.extension().and_then(|ext| ext.to_str()) {
                match extension {
                    "json" => "json".to_string(),
                    "yaml" | "yml" => "yaml".to_string(),
                    "toml" => "toml".to_string(),
                    _ => "json".to_string(),
                }
            } else {
                "json".to_string()
            }
        });
    match detected_format.as_str() {
        "json" => import_from_json(&input, force, verbose)?,
        "yaml" => import_from_yaml(&input, force, verbose)?,
        "toml" => import_from_toml(&input, force, verbose)?,
        _ => {
            return Err(
                anyhow::anyhow!("Unsupported import format: {}", detected_format),
            );
        }
    }
    println!("✅ Import completed successfully!");
    Ok(())
}
pub fn export_to_json(
    project_dir: &PathBuf,
    output: Option<PathBuf>,
    include_deps: bool,
    verbose: bool,
) -> Result<()> {
    let manifest = read_project_manifest(project_dir)?;
    let source_files = collect_source_files(project_dir)?;
    let export_data = ExportData {
        manifest,
        source_files,
        dependencies: if include_deps {
            Some(collect_dependencies(project_dir)?)
        } else {
            None
        },
        metadata: ExportMetadata {
            exported_at: chrono::Utc::now().to_rfc3339(),
            format_version: "1.0".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };
    let json_content = serde_json::to_string_pretty(&export_data)
        .context("Failed to serialize to JSON")?;
    let output_path = output.unwrap_or_else(|| project_dir.join("export.json"));
    std::fs::write(&output_path, json_content).context("Failed to write JSON export")?;
    if verbose {
        println!("  ✅ Exported to: {}", output_path.display());
    }
    Ok(())
}
pub fn export_to_yaml(
    project_dir: &PathBuf,
    output: Option<PathBuf>,
    include_deps: bool,
    verbose: bool,
) -> Result<()> {
    let manifest = read_project_manifest(project_dir)?;
    let source_files = collect_source_files(project_dir)?;
    let export_data = ExportData {
        manifest,
        source_files,
        dependencies: if include_deps {
            Some(collect_dependencies(project_dir)?)
        } else {
            None
        },
        metadata: ExportMetadata {
            exported_at: chrono::Utc::now().to_rfc3339(),
            format_version: "1.0".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };
    let yaml_content = serde_yaml::to_string(&export_data)
        .context("Failed to serialize to YAML")?;
    let output_path = output.unwrap_or_else(|| project_dir.join("export.yaml"));
    std::fs::write(&output_path, yaml_content).context("Failed to write YAML export")?;
    if verbose {
        println!("  ✅ Exported to: {}", output_path.display());
    }
    Ok(())
}
pub fn export_to_toml(
    project_dir: &PathBuf,
    output: Option<PathBuf>,
    include_deps: bool,
    verbose: bool,
) -> Result<()> {
    let manifest = read_project_manifest(project_dir)?;
    let toml_content = toml::to_string_pretty(&manifest)
        .context("Failed to serialize to TOML")?;
    let output_path = output.unwrap_or_else(|| project_dir.join("export.toml"));
    std::fs::write(&output_path, toml_content).context("Failed to write TOML export")?;
    if verbose {
        println!("  ✅ Exported to: {}", output_path.display());
        if include_deps {
            println!("  Note: Dependencies not yet supported in TOML export");
        }
    }
    Ok(())
}
pub fn export_to_docker(
    project_dir: &PathBuf,
    output: Option<PathBuf>,
    verbose: bool,
) -> Result<()> {
    let manifest = read_project_manifest(project_dir)?;
    let dockerfile_content = format!(
        r#"# Dockerfile for HELIX project: {}
FROM ubuntu:22.04

# Install hlx runtime
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install hlx compiler
RUN curl -sSL https://get.helix.cm/install.sh | bash

# Set working directory
WORKDIR /app

# Copy project files
COPY . .

# Build the project
RUN helix build

# Expose port
EXPOSE 8080

# Run the project
CMD ["helix", "run"]
"#,
        manifest.name
    );
    let output_path = output.unwrap_or_else(|| project_dir.join("Dockerfile"));
    std::fs::write(&output_path, dockerfile_content)
        .context("Failed to write Dockerfile")?;
    if verbose {
        println!("  ✅ Exported to: {}", output_path.display());
    }
    Ok(())
}
pub fn export_to_kubernetes(
    project_dir: &PathBuf,
    output: Option<PathBuf>,
    verbose: bool,
) -> Result<()> {
    let manifest = read_project_manifest(project_dir)?;
    let k8s_content = format!(
        r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {}-deployment
  labels:
    app: {}
spec:
  replicas: 3
  selector:
    matchLabels:
      app: {}
  template:
    metadata:
      labels:
        app: {}
    spec:
      containers:
      - name: {}
        image: {}:latest
        ports:
        - containerPort: 8080
        env:
        - name: hlx_ENV
          value: "production"
---
apiVersion: v1
kind: Service
metadata:
  name: {}-service
spec:
  selector:
    app: {}
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
  type: LoadBalancer
"#,
        manifest.name, manifest.name, manifest.name, manifest
        .name, manifest.name, manifest.name, manifest
        .name, manifest.name
    );
    let output_path = output.unwrap_or_else(|| project_dir.join("k8s.yaml"));
    std::fs::write(&output_path, k8s_content)
        .context("Failed to write Kubernetes manifest")?;
    if verbose {
        println!("  ✅ Exported to: {}", output_path.display());
    }
    Ok(())
}
pub fn import_from_json(input: &PathBuf, force: bool, verbose: bool) -> Result<()> {
    let content = std::fs::read_to_string(input).context("Failed to read input file")?;
    let export_data: ExportData = serde_json::from_str(&content)
        .context("Failed to parse JSON")?;
    import_export_data(export_data, force, verbose)
}
pub fn import_from_yaml(input: &PathBuf, force: bool, verbose: bool) -> Result<()> {
    let content = std::fs::read_to_string(input).context("Failed to read input file")?;
    let export_data: ExportData = serde_yaml::from_str(&content)
        .context("Failed to parse YAML")?;
    import_export_data(export_data, force, verbose)
}
pub fn import_from_toml(input: &PathBuf, force: bool, verbose: bool) -> Result<()> {
    let content = std::fs::read_to_string(input).context("Failed to read input file")?;
    let manifest: ProjectManifest = toml::from_str(&content)
        .context("Failed to parse TOML")?;
    let export_data = ExportData {
        manifest,
        source_files: std::collections::HashMap::new(),
        dependencies: None,
        metadata: ExportMetadata {
            exported_at: chrono::Utc::now().to_rfc3339(),
            format_version: "1.0".to_string(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };
    import_export_data(export_data, force, verbose)
}
pub fn import_export_data(
    export_data: ExportData,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let project_dir = std::env::current_dir()
        .context("Failed to get current directory")?;
    let manifest_path = project_dir.join("project.hlx");
    if manifest_path.exists() && !force {
        return Err(anyhow::anyhow!("Project already exists. Use --force to overwrite."));
    }
    std::fs::create_dir_all(project_dir.join("src"))
        .context("Failed to create src directory")?;
    let manifest_content = toml::to_string_pretty(&export_data.manifest)
        .context("Failed to serialize manifest")?;
    std::fs::write(&manifest_path, manifest_content)
        .context("Failed to write manifest")?;
    let source_files_count = export_data.source_files.len();
    for (filename, content) in export_data.source_files {
        let file_path = project_dir.join("src").join(filename);
        std::fs::write(&file_path, content).context("Failed to write source file")?;
    }
    if verbose {
        println!("  ✅ Imported {} source files", source_files_count);
    }
    Ok(())
}
#[derive(Debug, Serialize, Deserialize)]
struct ExportData {
    manifest: ProjectManifest,
    source_files: std::collections::HashMap<String, String>,
    dependencies: Option<std::collections::HashMap<String, String>>,
    metadata: ExportMetadata,
}
#[derive(Debug, Serialize, Deserialize)]
struct ExportMetadata {
    exported_at: String,
    format_version: String,
    tool_version: String,
}
pub fn read_project_manifest(
    project_dir: &PathBuf,
) -> Result<ProjectManifest> {
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
    let manifest = ProjectManifest {
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
pub fn collect_source_files(
    project_dir: &PathBuf,
) -> Result<std::collections::HashMap<String, String>> {
    let mut files = std::collections::HashMap::new();
    let src_dir = project_dir.join("src");
    if src_dir.exists() {
        collect_helix_files(&src_dir, &mut files)?;
    }
    Ok(files)
}
pub fn collect_helix_files(
    dir: &PathBuf,
    files: &mut std::collections::HashMap<String, String>,
) -> Result<()> {
    let entries = std::fs::read_dir(dir).context("Failed to read directory")?;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "hlx" {
                    let filename = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
                    let content = std::fs::read_to_string(&path)
                        .context("Failed to read file")?;
                    files.insert(filename.to_string(), content);
                }
            }
        } else if path.is_dir() {
            collect_helix_files(&path, files)?;
        }
    }
    Ok(())
}
pub fn collect_dependencies(
    _project_dir: &PathBuf,
) -> Result<std::collections::HashMap<String, String>> {
    Ok(std::collections::HashMap::new())
}
pub fn extract_project_name(content: &str) -> Result<String> {
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
pub fn extract_project_version(content: &str) -> Result<String> {
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
pub fn find_project_root() -> Result<PathBuf> {
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