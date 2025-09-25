use helix::{
    parse, validate, ast_to_config, HelixConfig, HelixAst, Declaration, AgentDecl,
    WorkflowDecl, Expression, Value, SemanticAnalyzer, CodeGenerator, HelixIR,
};
use std::collections::HashMap;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 HELIX Language - Advanced Usage Example\n");
    println!("1. Parsing complex configuration:");
    let source = r#"
        agent 'primary_assistant' {
            model = 'gpt-4'
            temperature = 0.7
            max_tokens = 2000
            system_prompt = 'You are a helpful AI assistant'
            memory = {
                type = 'persistent'
                max_entries = 100
            }
        }
        
        agent 'code_reviewer' {
            model = 'gpt-4'
            temperature = 0.3
            max_tokens = 1500
            system_prompt = 'You are an expert code reviewer'
        }
        
        workflow 'development_workflow' {
            agent = 'primary_assistant'
            steps = [
                { action = 'analyze_requirements' }
                { action = 'generate_code' }
                { action = 'review_code', agent = 'code_reviewer' }
                { action = 'refine_code' }
            ]
            retry = {
                max_attempts = 3
                backoff = 'exponential'
            }
        }
        
        crew 'development_team' {
            agents = ['primary_assistant', 'code_reviewer']
            workflow = 'development_workflow'
            collaboration = 'sequential'
        }
    "#;
    let ast = parse(source)?;
    validate(&ast)?;
    println!("✅ Complex configuration parsed and validated");
    println!("   - Declarations: {}", ast.declarations.len());
    println!("\n2. Analyzing AST structure:");
    let mut agent_count = 0;
    let mut workflow_count = 0;
    let mut crew_count = 0;
    for declaration in &ast.declarations {
        match declaration {
            Declaration::Agent(_) => agent_count += 1,
            Declaration::Workflow(_) => workflow_count += 1,
            Declaration::Crew(_) => crew_count += 1,
            _ => {}
        }
    }
    println!("   - Agents: {}", agent_count);
    println!("   - Workflows: {}", workflow_count);
    println!("   - Crews: {}", crew_count);
    println!("\n3. Extracting agent configurations:");
    let mut agents = HashMap::new();
    for declaration in &ast.declarations {
        if let Declaration::Agent(agent_decl) = declaration {
            let agent_name = &agent_decl.name;
            let mut config = HashMap::new();
            for (key, value) in &agent_decl.properties {
                config.insert(key.clone(), format!("{:?}", value));
            }
            agents.insert(agent_name.clone(), config);
            println!(
                "   - Agent '{}': {} properties", agent_name, agent_decl.properties.len()
            );
        }
    }
    println!("\n4. Custom AST processing:");
    let processed_ast = process_ast_for_optimization(ast)?;
    println!("✅ AST processed for optimization");
    println!("\n5. Generating intermediate representation:");
    let codegen = CodeGenerator::new();
    let ir = codegen.generate(&processed_ast);
    println!("✅ Generated IR with {} instructions", ir.instructions.len());
    println!("\n6. Converting to configuration:");
    let config = ast_to_config(processed_ast)?;
    println!("✅ Configuration created");
    println!("   - Agents: {}", config.agents.len());
    println!("   - Workflows: {}", config.workflows.len());
    println!("   - Crews: {}", config.crews.len());
    println!("\n7. Custom validation:");
    validate_agent_temperature_ranges(&config)?;
    validate_workflow_dependencies(&config)?;
    println!("✅ Custom validation passed");
    println!("\n8. Usage statistics:");
    let stats = generate_usage_statistics(&config);
    println!("   - Total agents: {}", stats.total_agents);
    println!("   - Total workflows: {}", stats.total_workflows);
    println!("   - Average steps per workflow: {:.1}", stats.avg_steps_per_workflow);
    println!("   - Most common model: {}", stats.most_common_model);
    println!("\n🎉 Advanced usage example completed successfully!");
    Ok(())
}
fn process_ast_for_optimization(
    mut ast: HelixAst,
) -> Result<HelixAst, Box<dyn std::error::Error>> {
    for declaration in &mut ast.declarations {
        if let Declaration::Agent(agent_decl) = declaration {
            if !agent_decl.properties.contains_key("temperature") {
                agent_decl
                    .properties
                    .insert("temperature".to_string(), Expression::Number(0.7));
            }
            if !agent_decl.properties.contains_key("max_tokens") {
                agent_decl
                    .properties
                    .insert("max_tokens".to_string(), Expression::Number(2000.0));
            }
        }
    }
    Ok(ast)
}
fn validate_agent_temperature_ranges(
    config: &HelixConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, agent) in &config.agents {
        if let Some(temp) = agent.temperature {
            if temp < 0.0 || temp > 2.0 {
                return Err(
                    format!("Agent '{}' has invalid temperature: {}", name, temp).into(),
                );
            }
        }
    }
    Ok(())
}
fn validate_workflow_dependencies(
    config: &HelixConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, workflow) in &config.workflows {
        for step in &workflow.steps {
            if let Some(agent_name) = &step.agent {
                if !config.agents.contains_key(agent_name) {
                    return Err(
                        format!(
                            "Workflow '{}' step '{}' references unknown agent: {}", name, step.name, agent_name
                        )
                            .into(),
                    );
                }
            }
        }
    }
    Ok(())
}
#[derive(Debug)]
struct UsageStats {
    total_agents: usize,
    total_workflows: usize,
    avg_steps_per_workflow: f64,
    most_common_model: String,
}
fn generate_usage_statistics(config: &HelixConfig) -> UsageStats {
    let total_agents = config.agents.len();
    let total_workflows = config.workflows.len();
    let total_steps: usize = config.workflows.values().map(|w| w.steps.len()).sum();
    let avg_steps_per_workflow = if total_workflows > 0 {
        total_steps as f64 / total_workflows as f64
    } else {
        0.0
    };
    let mut model_counts = HashMap::new();
    for agent in config.agents.values() {
        *model_counts.entry(agent.model.clone()).or_insert(0) += 1;
    }
    let most_common_model = model_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(model, _)| model)
        .unwrap_or_else(|| "unknown".to_string());
    UsageStats {
        total_agents,
        total_workflows,
        avg_steps_per_workflow,
        most_common_model,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_advanced_parsing() {
        let source = r#"
            agent 'test' {
                model = 'gpt-4'
                temperature = 0.5
            }
            workflow 'test_workflow' {
                agent = 'test'
                steps = [
                    { action = 'test_action' }
                ]
            }
        "#;
        let ast = parse(source).expect("Should parse successfully");
        validate(&ast).expect("Should validate successfully");
        let processed = process_ast_for_optimization(ast)
            .expect("Should process successfully");
        assert!(! processed.declarations.is_empty());
    }
    #[test]
    fn test_custom_validation() {
        let source = r#"
            agent 'test' {
                model = 'gpt-4'
                temperature = 0.5
            }
        "#;
        let ast = parse(source).expect("Should parse successfully");
        let config = ast_to_config(ast).expect("Should convert successfully");
        validate_agent_temperature_ranges(&config)
            .expect("Should validate successfully");
    }
    #[test]
    fn test_usage_statistics() {
        let source = r#"
            agent 'test1' { model = 'gpt-4' }
            agent 'test2' { model = 'gpt-4' }
            workflow 'test_workflow' {
                agent = 'test1'
                steps = [
                    { action = 'step1' }
                    { action = 'step2' }
                ]
            }
        "#;
        let ast = parse(source).expect("Should parse successfully");
        let config = ast_to_config(ast).expect("Should convert successfully");
        let stats = generate_usage_statistics(&config);
        assert_eq!(stats.total_agents, 2);
        assert_eq!(stats.total_workflows, 1);
        assert_eq!(stats.avg_steps_per_workflow, 2.0);
        assert_eq!(stats.most_common_model, "gpt-4");
    }
}