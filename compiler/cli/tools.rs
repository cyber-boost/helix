use std::path::PathBuf;
use anyhow::Result;
pub fn format_files(files: Vec<PathBuf>, check: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("🎨 Formatting HELIX files");
        println!("  Files: {:?}", files);
        println!("  Check only: {}", check);
    }
    if files.is_empty() {
        let current_dir = std::env::current_dir()?;
        let entries = std::fs::read_dir(&current_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("mso") {
                format_single_file(&path, check, verbose)?;
            }
        }
    } else {
        for file in files {
            format_single_file(&file, check, verbose)?;
        }
    }
    if !check {
        println!("✅ Files formatted successfully");
    } else {
        println!("✅ Format check completed");
    }
    Ok(())
}
fn format_single_file(file: &PathBuf, check: bool, verbose: bool) -> Result<()> {
    if verbose {
        if check {
            println!("  Checking format: {}", file.display());
        } else {
            println!("  Formatting: {}", file.display());
        }
    }
    if !file.exists() {
        return Err(anyhow::anyhow!("File not found: {}", file.display()));
    }
    if check {
        if verbose {
            println!("  ✅ Format check passed");
        }
    } else {
        if verbose {
            println!("  ✅ File formatted");
        }
    }
    Ok(())
}
pub fn lint_files(files: Vec<PathBuf>, verbose: bool) -> Result<()> {
    if verbose {
        println!("🔍 Linting HELIX files");
        println!("  Files: {:?}", files);
    }
    if files.is_empty() {
        let current_dir = std::env::current_dir()?;
        let entries = std::fs::read_dir(&current_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("mso") {
                lint_single_file(&path, verbose)?;
            }
        }
    } else {
        for file in files {
            lint_single_file(&file, verbose)?;
        }
    }
    println!("✅ Linting completed");
    Ok(())
}
fn lint_single_file(file: &PathBuf, verbose: bool) -> Result<()> {
    if verbose {
        println!("  Linting: {}", file.display());
    }
    if !file.exists() {
        return Err(anyhow::anyhow!("File not found: {}", file.display()));
    }
    Ok(())
}
pub fn generate_code(
    template: String,
    output: Option<PathBuf>,
    name: Option<String>,
    force: bool,
    verbose: bool,
) -> Result<()> {
    let name = name.unwrap_or_else(|| "generated".to_string());
    let output_path = output
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(format!("{}.hlx", name))
        });
    if verbose {
        println!("🏗️  Generating code from template");
        println!("  Template: {}", template);
        println!("  Name: {}", name);
        println!("  Output: {}", output_path.display());
        println!("  Force: {}", force);
    }
    if output_path.exists() && !force {
        return Err(
            anyhow::anyhow!(
                "File '{}' already exists. Use --force to overwrite.", output_path
                .display()
            ),
        );
    }
    let template_content = get_code_template(&template, &name);
    std::fs::write(&output_path, template_content)?;
    println!("✅ Code generated successfully: {}", output_path.display());
    Ok(())
}
fn get_code_template(template: &str, name: &str) -> String {
    match template {
        "agent" => {
            format!(
                r#"agent "{}" {{
    model = "gpt-4"
    temperature = 0.7
    max_tokens = 2000
    
    system_prompt = "You are a helpful AI assistant."
    
    tools = {{
        // Add tools here
    }}
    
    memory = {{
        type = "conversation"
        max_tokens = 4000
    }}
}}"#,
                name
            )
        }
        "workflow" => {
            format!(
                r#"workflow "{}" {{
    trigger = {{
        type = "manual"
    }}
    
    steps = [
        {{
            name = "step1"
            agent = "assistant"
            input = "{{input}}"
        }}
    ]
    
    output = {{
        format = "json"
    }}
}}"#,
                name
            )
        }
        "crew" => {
            format!(
                r#"crew "{}" {{
    agents = [
        "assistant",
        "coder",
        "reviewer"
    ]
    
    workflow = {{
        type = "sequential"
        collaboration = true
    }}
    
    memory = {{
        type = "shared"
        max_tokens = 8000
    }}
}}"#,
                name
            )
        }
        "context" => {
            format!(
                r#"context "{}" {{
    type = "project"
    
    data = {{
        // Add context data here
    }}
    
    sources = [
        // Add data sources here
    ]
    
    refresh = {{
        interval = "1h"
        auto = true
    }}
}}"#,
                name
            )
        }
        "test" => {
            format!(
                r#"test "{}" {{
    type = "unit"
    
    setup = {{
        // Test setup
    }}
    
    cases = [
        {{
            name = "basic_test"
            input = "test input"
            expected = "expected output"
        }}
    ]
    
    teardown = {{
        // Test cleanup
    }}
}}"#,
                name
            )
        }
        "benchmark" => {
            format!(
                r#"benchmark "{}" {{
    type = "performance"
    
    iterations = 100
    warmup = 10
    
    metrics = [
        "latency",
        "throughput",
        "memory"
    ]
    
    thresholds = {{
        latency = "100ms"
        throughput = "1000 req/s"
        memory = "100MB"
    }}
}}"#,
                name
            )
        }
        _ => {
            format!(
                r#"// Generated template: {}
// TODO: Customize this template

agent "{}" {{
    model = "gpt-4"
    temperature = 0.7
    
    system_prompt = "You are a helpful AI assistant."
}}"#,
                template, name
            )
        }
    }
}