use crate::*;
use tempfile::TempDir;
use std::fs;
#[test]
fn test_full_compilation_pipeline() {
    let source = r#"
# Test configuration
project "test" {
    version = "1.0.0"
    author = "test"
    description = "Test project"
}

agent "assistant" {
    model = "gpt-4"
    temperature = 0.7
    max_tokens = 100000
    timeout = 30m
    
    capabilities [
        "reasoning"
        "generation"
    ]
}

workflow "test-flow" {
    trigger = "manual"
    
    step "analyze" {
        agent = "assistant"
        task = "Analyze the input"
        timeout = 5m
    }
}

memory {
    provider = "hlx_db"
    connection = "memory.db"
    embedding_model = "text-embedding-3"
    embedding_dimensions = 1536
    cache_size = 1000
    cache_ttl = 24h
}

context "production" {
    environment = "prod"
    debug = false
    max_tokens = 100000
    
    variables {
        api_endpoint = "https:
    }
    
    secrets {
        api_key = $API_KEY
    }
}
"#;
    let ast = parse(source).expect("Failed to parse");
    assert_eq!(ast.declarations.len(), 5);
    validate(&ast).expect("Failed to validate");
    let mut generator = codegen::CodeGenerator::new();
    let ir = generator.generate(&ast);
    assert!(ir.instructions.len() > 0);
    let compiler = crate::compiler::Compiler::new(crate::compiler::optimizer::OptimizationLevel::Two);
    let binary = compiler.compile_source(source, None).expect("Failed to compile");
    assert_eq!(binary.magic, compiler::binary::MAGIC_BYTES);
    let serializer = compiler::serializer::BinarySerializer::new(true);
    let temp_dir = TempDir::new().unwrap();
    let binary_path = temp_dir.path().join("test.hlxb");
    serializer.write_to_file(&binary, &binary_path).expect("Failed to write binary");
    assert!(binary_path.exists());
    let loader = compiler::loader::BinaryLoader::new();
    let loaded = loader.load_file(&binary_path).expect("Failed to load binary");
    assert_eq!(loaded.version, binary.version);
}
#[test]
fn test_migration_from_json() {
    let json = r#"{
        "agent": {
            "name": "coder",
            "model": "gpt-4",
            "temperature": 0.7,
            "timeout": "30 minutes"
        },
        "workflow": {
            "name": "build",
            "trigger": "push",
            "steps": [
                {
                    "name": "compile",
                    "agent": "coder",
                    "task": "Compile the code"
                }
            ]
        }
    }"#;
    let migrator = crate::compiler::tools::migrate::Migrator::new();
    let hlx = migrator.migrate_json(json).expect("Failed to migrate JSON");
    assert!(hlx.contains("agent \"coder\""));
    assert!(hlx.contains("workflow \"build\""));
    assert!(hlx.contains("temperature = 0.7"));
}
#[test]
fn test_migration_from_toml() {
    let toml = r#"
[project]
name = "test"
version = "1.0.0"

[agent.assistant]
model = "gpt-4"
temperature = 0.7

[workflow.test]
trigger = "manual"
"#;
    let migrator = crate::compiler::tools::migrate::Migrator::new();
    let hlx = migrator.migrate_toml(toml).expect("Failed to migrate TOML");
    assert!(hlx.contains("project"));
    assert!(hlx.contains("agent \"assistant\""));
    assert!(hlx.contains("workflow"));
}
#[test]
fn test_migration_from_env() {
    let env = r#"
DATABASE_URL=postgres://localhost/db
API_KEY=secret123
DEBUG=true
MAX_TOKENS=100000
"#;
    let migrator = crate::compiler::tools::migrate::Migrator::new();
    let hlx = migrator.migrate_env(env).expect("Failed to migrate env");
    assert!(hlx.contains("context \"environment\""));
    assert!(hlx.contains("database_url"));
    assert!(hlx.contains("api_key = $API_KEY"));
    assert!(hlx.contains("max_tokens"));
}
#[test]
fn test_bundle_creation() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("agents.hlxbb");
    fs::write(&file1, r#"
agent "coder" {
    model = "gpt-4"
    temperature = 0.7
}
"#)
        .unwrap();
    let file2 = temp_dir.path().join("workflows.hlxbb");
    fs::write(
            &file2,
            r#"
workflow "build" {
    trigger = "push"
    
    step "compile" {
        agent = "coder"
        task = "Compile"
        timeout = 5m
    }
}
"#,
        )
        .unwrap();
    let bundler = compiler::bundle::Bundler::new();
    let binary = bundler
        .bundle_directory(temp_dir.path(), compiler::optimizer::OptimizationLevel::Two)
        .expect("Failed to create bundle");
    assert!(binary.metadata.extra.contains_key("bundle"));
    assert_eq!(binary.metadata.extra.get("bundle_files"), Some(& "2".to_string()));
}
#[test]
fn test_optimization_levels() {
    let source = r#"
agent "test" {
    model = "gpt-4"
    temperature = 0.7
    temperature = 0.7  # Duplicate for testing
}
"#;
    for level in 0..=3 {
        let compiler = crate::compiler::Compiler::new(
            compiler::optimizer::OptimizationLevel::from(level),
        );
        let binary = compiler
            .compile_source(source, None)
            .expect(&format!("Failed to compile with level {}", level));
        assert_eq!(binary.metadata.optimization_level, level);
    }
}
#[test]
fn test_ast_to_config_conversion() {
    let source = r#"
agent "assistant" {
    model = "gpt-4"
    role = "Assistant"
    goal = "Help users"
    backstory = "I am a helpful assistant"
    temperature = 0.7
    max_tokens = 100000
    
    capabilities ["reasoning", "generation"]
    tools ["search", "calculate"]
}

workflow "process" {
    description = "Process workflow"
    trigger = "manual"
    max_iterations = 10
    verbose = true
    
    step "analyze" {
        agent = "assistant"
        task = "Analyze"
        timeout = 5m
        parallel = false
        depends_on = []
    }
}
"#;
    let config = parse_and_validate(source).expect("Failed to parse and validate");
    let agent = config.agents.get("assistant").expect("Agent not found");
    assert_eq!(agent.model, "gpt-4");
    assert_eq!(agent.temperature, Some(0.7));
    assert_eq!(agent.max_tokens, Some(100000));
    assert_eq!(agent.capabilities.len(), 2);
    let workflow = config.workflows.get("process").expect("Workflow not found");
    assert_eq!(workflow.name, "process");
    assert_eq!(workflow.steps.len(), 1);
}
#[test]
fn test_file_loading() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("config.hlxbb");
    fs::write(
            &file_path,
            r#"
agent "test" {
    model = "gpt-4"
    temperature = 0.7
}
"#,
        )
        .unwrap();
    let config = load_file(&file_path).expect("Failed to load file");
    assert!(config.agents.contains_key("test"));
}
#[test]
fn test_directory_loading() {
    let temp_dir = TempDir::new().unwrap();
    for i in 1..=3 {
        let file_path = temp_dir.path().join(format!("config{}.hlxbb", i));
        fs::write(
                &file_path,
                format!(
                    r#"
agent "test{}" {{
    model = "gpt-4"
    temperature = 0.7
}}
"#,
                    i
                ),
            )
            .unwrap();
    }
    let configs = load_directory(temp_dir.path()).expect("Failed to load directory");
    assert_eq!(configs.len(), 3);
}
#[test]
fn test_error_recovery() {
    let source = r#"
agent "test" {
    model = "gpt-4"
    temperature = 0.7
    # Missing closing brace on purpose
    
agent "test2" {
    model = "gpt-4o"
}
"#;
    let result = parse(source);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Expected"));
}
#[test]
fn test_circular_dependency_detection() {
    let graph = crate::compiler::DependencyGraph::new();
    let result = graph.check_circular();
    assert!(result.is_ok());
}
#[test]
fn test_compression() {
    let source = r#"
agent "test" {
    model = "gpt-4"
    description = "This is a very long description that should compress well because it has lots of repeated words repeated words repeated words"
}
"#;
    let compiler = crate::compiler::Compiler::builder()
        .optimization_level(compiler::optimizer::OptimizationLevel::Three)
        .compression(true)
        .build();
    let compressed = compiler.compile_source(source, None).expect("Failed to compile");
    let compiler_no_compress = crate::compiler::Compiler::builder()
        .optimization_level(compiler::optimizer::OptimizationLevel::Three)
        .compression(false)
        .build();
    let uncompressed = compiler_no_compress
        .compile_source(source, None)
        .expect("Failed to compile");
    assert!(compressed.size() <= uncompressed.size());
}
#[test]
fn test_pretty_printing() {
    let source = r#"
agent "test" {
    model = "gpt-4"
    temperature = 0.7
}
"#;
    let ast = parse(source).expect("Failed to parse");
    let pretty = pretty_print(&ast);
    assert!(pretty.contains("agent"));
    assert!(pretty.contains("test"));
}