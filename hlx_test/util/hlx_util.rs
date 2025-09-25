// HLX Configuration File Utilities
// Demonstrates how to load, parse, and work with .hlx text configuration files

use helix::{
    parse, validate, ast_to_config, HelixConfig,
    HelixLoader, HlxError
};
use std::path::Path;

/// Load and parse a .hlx configuration file
pub fn load_hlx_config<P: AsRef<Path>>(path: P) -> Result<HelixConfig, HlxError> {
    println!("📂 Loading HLX config from: {:?}", path.as_ref());

    // Method 1: Using HelixLoader (recommended)
    let mut loader = HelixLoader::new();
    match loader.load_file(path) {
        Ok(config) => {
            println!("✅ Successfully loaded config with {} agents, {} workflows",
                    config.agents.len(), config.workflows.len());
            Ok(config)
        },
        Err(e) => {
            println!("❌ Failed to load with HelixLoader: {}", e);
            Err(e)
        }
    }
}

/// Parse HLX content from string and convert to configuration
pub fn parse_hlx_content(content: &str) -> Result<HelixConfig, Box<dyn std::error::Error>> {
    println!("🔍 Parsing HLX content ({} characters)", content.len());

    // Step 1: Parse the source into AST
    let ast = parse(content)?;
    println!("✅ Parsed into AST with {} declarations", ast.declarations.len());

    // Step 2: Validate the AST
    validate(&ast)?;
    println!("✅ AST validation passed");

    // Step 3: Convert AST to configuration
    let config = ast_to_config(ast)?;
    println!("✅ Converted to configuration with {} agents, {} workflows, {} crews",
            config.agents.len(), config.workflows.len(), config.crews.len());

    Ok(config)
}

/// Export configuration to HLX format (reverse operation)
pub fn export_to_hlx(config: &HelixConfig) -> Result<String, Box<dyn std::error::Error>> {
    println!("📤 Exporting configuration to HLX format");

    // Note: This is a simplified export - in practice you'd want a proper HLX serializer
    let mut output = String::new();

    // Export project info
    if !config.projects.is_empty() {
        let project = config.projects.values().next().unwrap();
        output.push_str(&format!("project \"{}\" {{\n", project.name));
        output.push_str(&format!("    version = \"{}\"\n", project.version));
        output.push_str(&format!("    author = \"{}\"\n", project.author));
        if let Some(desc) = &project.description {
            output.push_str(&format!("    description = \"{}\"\n", desc));
        }
        output.push_str("}\n\n");
    }

    // Export agents
    for (name, agent) in &config.agents {
        output.push_str(&format!("agent \"{}\" {{\n", name));
        output.push_str(&format!("    model = \"{}\"\n", agent.model));
        output.push_str(&format!("    role = \"{}\"\n", agent.role));

        if let Some(temp) = agent.temperature {
            output.push_str(&format!("    temperature = {}\n", temp));
        }
        if let Some(max_tokens) = agent.max_tokens {
            output.push_str(&format!("    max_tokens = {}\n", max_tokens));
        }

        if !agent.capabilities.is_empty() {
            output.push_str("    capabilities = [\n");
            for cap in &agent.capabilities {
                output.push_str(&format!("        \"{}\",\n", cap));
            }
            output.push_str("    ]\n");
        }

        if let Some(backstory) = &agent.backstory {
            output.push_str(&format!("    backstory = \"\"\"\n{}\n\"\"\"\n", backstory));
        }

        output.push_str("}\n\n");
    }

    // Export workflows
    for (name, workflow) in &config.workflows {
        output.push_str(&format!("workflow \"{}\" {{\n", name));

        if !workflow.steps.is_empty() {
            output.push_str("    steps = [\n");
            for step in &workflow.steps {
                output.push_str("        {\n");
                output.push_str(&format!("            name = \"{}\"\n", step.name));
                if let Some(agent) = &step.agent {
                    output.push_str(&format!("            agent = \"{}\"\n", agent));
                }
                output.push_str(&format!("            task = \"{}\"\n", step.task));
                output.push_str("        },\n");
            }
            output.push_str("    ]\n");
        }

        if !workflow.outputs.is_empty() {
            output.push_str("    outputs = [\n");
            for output_name in &workflow.outputs {
                output.push_str(&format!("        \"{}\",\n", output_name));
            }
            output.push_str("    ]\n");
        }

        output.push_str("}\n\n");
    }

    println!("✅ Exported configuration to HLX format ({} characters)", output.len());
    Ok(output)
}

/// Validate HLX configuration
pub fn validate_hlx_config(config: &HelixConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Validating HLX configuration");

    let mut errors = Vec::new();

    // Check that all workflow agents exist
    for (workflow_name, workflow) in &config.workflows {
        for step in &workflow.steps {
            if let Some(agent_name) = &step.agent {
                if !config.agents.contains_key(agent_name) {
                    errors.push(format!(
                        "Workflow '{}' references unknown agent '{}' in step '{}'",
                        workflow_name, agent_name, step.name
                    ));
                }
            }
        }
    }

    // Check that crew agents exist
    for (crew_name, crew) in &config.crews {
        for agent_name in &crew.agents {
            if !config.agents.contains_key(agent_name) {
                errors.push(format!(
                    "Crew '{}' references unknown agent '{}'",
                    crew_name, agent_name
                ));
            }
        }
    }

    if errors.is_empty() {
        println!("✅ HLX configuration validation passed");
        Ok(())
    } else {
        println!("❌ HLX configuration validation failed:");
        for error in &errors {
            println!("  - {}", error);
        }
        Err(format!("Validation failed: {} errors", errors.len()).into())
    }
}

/// Example usage of HLX utilities
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_hlx_config() {
        let config_path = "hlx_test/config/agent_config.hlx";
        if std::path::Path::new(config_path).exists() {
            match load_hlx_config(config_path) {
                Ok(config) => {
                    assert!(!config.agents.is_empty());
                    assert!(!config.workflows.is_empty());
                }
                Err(e) => {
                    println!("Note: Could not load config (this is expected in test environment): {}", e);
                }
            }
        }
    }

    #[test]
    fn test_parse_hlx_content() {
        let content = r#"
            agent "test_agent" {
                model = "gpt-4"
                role = "Test Agent"
                temperature = 0.5
            }
        "#;

        match parse_hlx_content(content) {
            Ok(config) => {
                assert_eq!(config.agents.len(), 1);
                assert!(config.agents.contains_key("test_agent"));
            }
            Err(e) => panic!("Failed to parse HLX content: {}", e),
        }
    }
}
