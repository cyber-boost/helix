// HLXB Binary Configuration File Utilities
// Demonstrates how to create, read, and work with .hlxb binary configuration files

use helix::output::hlxb_config_format::*;
use helix::types::{HelixConfig, AgentConfig, WorkflowConfig};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::collections::HashMap;

/// Create a sample Helix configuration for HLXB testing
pub fn create_sample_config() -> HelixConfig {
    println!("🏗️  Creating sample Helix configuration for HLXB testing");

    let mut config = HelixConfig::default();

    // Add sample agent
    let agent = AgentConfig {
        name: "test_agent".to_string(),
        model: "gpt-4".to_string(),
        role: "Test Agent".to_string(),
        temperature: Some(0.7),
        max_tokens: Some(2000),
        capabilities: vec!["code_generation".to_string(), "data_analysis".to_string()],
        backstory: Some("A helpful test agent for demonstration purposes.".to_string()),
        tools: vec!["calculator".to_string(), "search".to_string()],
        constraints: vec!["safe_content".to_string()],
    };
    config.agents.insert("test_agent".to_string(), agent);

    // Add sample workflow
    let workflow = WorkflowConfig {
        name: "test_workflow".to_string(),
        trigger: Default::default(),
        steps: vec![],
        pipeline: None,
        outputs: vec!["result.txt".to_string()],
        on_error: Some("cleanup".to_string()),
    };
    config.workflows.insert("test_workflow".to_string(), workflow);

    println!("✅ Created sample config with {} agents, {} workflows",
            config.agents.len(), config.workflows.len());

    config
}

/// Create an HLXB file from Helix configuration
pub fn create_hlxb_file<P: AsRef<Path>>(
    path: P,
    config: &HelixConfig
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Creating HLXB file: {:?}", path.as_ref());

    let file = File::create(&path)?;
    let mut writer = HlxbWriter::new(file);

    // Write header
    writer.write_header()?;

    // Write agents
    for (name, agent) in &config.agents {
        writer.write_agent(name, agent)?;
    }

    // Write workflows
    for (name, workflow) in &config.workflows {
        writer.write_workflow(name, workflow)?;
    }

    // Write other sections
    if !config.crews.is_empty() {
        writer.write_crews(&config.crews)?;
    }

    if !config.contexts.is_empty() {
        writer.write_contexts(&config.contexts)?;
    }

    // Finalize
    writer.finalize()?;

    println!("✅ HLXB file created successfully");
    Ok(())
}

/// Read and parse an HLXB file
pub fn read_hlxb_file<P: AsRef<Path>>(path: P) -> Result<HelixConfig, Box<dyn std::error::Error>> {
    println!("📖 Reading HLXB file: {:?}", path.as_ref());

    let file = File::open(&path)?;
    let mut reader = HlxbReader::new(file);

    // Read header
    let header = reader.read_header()?;
    println!("📋 HLXB Header:");
    println!("  - Version: {}", header.version);
    println!("  - Sections: {}", header.section_count);
    println!("  - Created: {}", header.created_at);

    // Read configuration
    let config = reader.read_config()?;

    println!("✅ Successfully read configuration:");
    println!("  - Agents: {}", config.agents.len());
    println!("  - Workflows: {}", config.workflows.len());
    println!("  - Crews: {}", config.crews.len());
    println!("  - Contexts: {}", config.contexts.len());

    Ok(config)
}

/// Compare file sizes: HLX vs HLXB
pub fn compare_binary_vs_text<P: AsRef<Path>>(
    hlx_path: P,
    hlxb_path: P
) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚖️  Comparing HLXB vs HLX file sizes");

    let hlx_size = std::fs::metadata(&hlx_path)?.len();
    let hlxb_size = std::fs::metadata(&hlxb_path)?.len();

    let ratio = hlx_size as f64 / hlxb_size as f64;
    let savings = ((hlx_size - hlxb_size) as f64 / hlx_size as f64) * 100.0;

    println!("📏 File Size Comparison:");
    println!("  - HLX (text): {} bytes ({:.2} KB)", hlx_size, hlx_size as f64 / 1024.0);
    println!("  - HLXB (binary): {} bytes ({:.2} KB)", hlxb_size, hlxb_size as f64 / 1024.0);
    println!("  - Size ratio: {:.2}x", ratio);
    println!("  - Space savings: {:.1}%", savings);

    if ratio > 1.2 {
        println!("✅ Good binary efficiency! HLXB is {:.1}x smaller than HLX", ratio);
    } else {
        println!("🤔 Binary format has minimal size benefit for this configuration");
    }

    Ok(())
}

/// Benchmark load times: HLX vs HLXB
pub fn benchmark_load_times<P: AsRef<Path>>(
    hlx_path: P,
    hlxb_path: P,
    iterations: usize
) -> Result<(), Box<dyn std::error::Error>> {
    println!("⏱️  Benchmarking load times ({} iterations)", iterations);

    // Benchmark HLX loading
    let hlx_start = std::time::Instant::now();
    for _ in 0..iterations {
        let _content = std::fs::read_to_string(&hlx_path)?;
        // In real benchmark, you'd parse the content too
    }
    let hlx_time = hlx_start.elapsed();

    // Benchmark HLXB loading
    let hlxb_start = std::time::Instant::now();
    for _ in 0..iterations {
        let _config = read_hlxb_file(&hlxb_path)?;
    }
    let hlxb_time = hlxb_start.elapsed();

    println!("⏱️  Load Time Results:");
    println!("  - HLX (text): {:?}", hlx_time);
    println!("  - HLXB (binary): {:?}", hlxb_time);
    println!("  - Speed improvement: {:.1}x faster",
            hlx_time.as_millis() as f64 / hlxb_time.as_millis() as f64);

    Ok(())
}

/// Validate HLXB file integrity
pub fn validate_hlxb_file<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Validating HLXB file integrity: {:?}", path.as_ref());

    let file = File::open(&path)?;
    let mut reader = HlxbReader::new(file);

    // Check magic number
    let header = reader.read_header()?;
    if header.magic != HLXB_MAGIC {
        return Err("Invalid HLXB magic number".into());
    }

    // Validate version
    if header.version > HLXB_VERSION {
        return Err(format!("Unsupported HLXB version: {}", header.version).into());
    }

    // Try to read the full configuration
    let config = reader.read_config()?;

    // Basic validation
    if config.agents.is_empty() && config.workflows.is_empty() {
        println!("⚠️  Warning: HLXB file contains no agents or workflows");
    }

    // Cross-reference validation
    for (workflow_name, workflow) in &config.workflows {
        for step in &workflow.steps {
            if let Some(agent_name) = &step.agent {
                if !config.agents.contains_key(agent_name) {
                    return Err(format!(
                        "Workflow '{}' references unknown agent '{}' in step '{}'",
                        workflow_name, agent_name, step.name
                    ).into());
                }
            }
        }
    }

    println!("✅ HLXB file validation passed");
    Ok(())
}

/// Example usage of HLXB utilities
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hlxb_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let hlxb_path = temp_dir.path().join("test.hlxb");

        // Create sample config
        let original_config = create_sample_config();

        // Write to HLXB
        create_hlxb_file(&hlxb_path, &original_config).unwrap();

        // Read back
        let loaded_config = read_hlxb_file(&hlxb_path).unwrap();

        // Basic validation
        assert_eq!(original_config.agents.len(), loaded_config.agents.len());
        assert_eq!(original_config.workflows.len(), loaded_config.workflows.len());
    }

    #[test]
    fn test_hlxb_validation() {
        let temp_dir = TempDir::new().unwrap();
        let hlxb_path = temp_dir.path().join("valid.hlxb");

        // Create and write valid config
        let config = create_sample_config();
        create_hlxb_file(&hlxb_path, &config).unwrap();

        // Validate
        validate_hlxb_file(&hlxb_path).unwrap();
    }
}
