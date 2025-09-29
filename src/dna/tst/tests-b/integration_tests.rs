#[cfg(test)]
mod integration_tests {
    use crate::{parse, validate, ast_to_config};
    use crate::compiler::Compiler;
    use crate::compiler::optimizer::OptimizationLevel;
    use crate::compiler::{serializer::BinarySerializer, loader::BinaryLoader};
    use tempfile::TempDir;
    #[test]
    fn test_parse_validate_convert_pipeline() {
        let source = r#"
            agent "test-agent" {
                model = "gpt-4"
                temperature = 0.7
                capabilities ["coding", "testing"]
            }
            
            workflow "test-workflow" {
                trigger = "manual"
                step "analyze" {
                    agent = "test-agent"
                    task = "Analyze the code"
                }
            }
        "#;
        let ast = parse(source).expect("Failed to parse");
        validate(&ast).expect("Failed to validate");
        let config = ast_to_config(ast).expect("Failed to convert to config");
        assert!(config.agents.contains_key("test-agent"));
        assert!(config.workflows.contains_key("test-workflow"));
        let agent = &config.agents["test-agent"];
        assert_eq!(agent.model, "gpt-4".to_string());
        assert_eq!(agent.temperature, Some(0.7));
    }
    #[test]
    fn test_compile_and_load_binary() {
        let source = r#"
            project "test" {
                version = "1.0.0"
                author = "tester"
            }
            
            agent "assistant" {
                model = "claude-3"
                role = "Helper"
            }
        "#;
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("test.hlxbb");
        let binary_path = temp_dir.path().join("test.hlxb");
        std::fs::write(&source_path, source).unwrap();
        let compiler = Compiler::new(OptimizationLevel::Two);
        let binary = compiler.compile_file(&source_path).expect("Failed to compile");
        let serializer = BinarySerializer::new(true);
        serializer.write_to_file(&binary, &binary_path).expect("Failed to write binary");
        let loader = BinaryLoader::new();
        let loaded = loader.load_file(&binary_path).expect("Failed to load binary");
        assert_eq!(loaded.version, binary.version);
        assert_eq!(loaded.flags.compressed, true);
        assert_eq!(loaded.flags.optimized, true);
    }
    #[test]
    fn test_config_merging() {
        use crate::types::{HelixConfig, HelixLoader, AgentConfig};
        let mut config1 = HelixConfig::default();
        config1
            .agents
            .insert(
                "agent1".to_string(),
                AgentConfig {
                    name: "agent1".to_string(),
                    model: "gpt-4".to_string(),
                    role: "Assistant".to_string(),
                    temperature: None,
                    max_tokens: None,
                    capabilities: vec![],
                    backstory: None,
                    tools: vec![],
                    constraints: vec![],
                },
            );
        let mut config2 = HelixConfig::default();
        config2
            .agents
            .insert(
                "agent2".to_string(),
                AgentConfig {
                    name: "agent2".to_string(),
                    model: "claude-3".to_string(),
                    role: "Assistant".to_string(),
                    temperature: None,
                    max_tokens: None,
                    capabilities: vec![],
                    backstory: None,
                    tools: vec![],
                    constraints: vec![],
                },
            );
        config2
            .agents
            .insert(
                "agent1".to_string(),
                AgentConfig {
                    name: "agent1".to_string(),
                    model: "gpt-3.5".to_string(),
                    role: "Assistant".to_string(),
                    temperature: None,
                    max_tokens: None,
                    capabilities: vec![],
                    backstory: None,
                    tools: vec![],
                    constraints: vec![],
                },
            );
        let loader = HelixLoader::new();
        let merged = loader.merge_configs(vec![& config1, & config2]);
        assert_eq!(merged.agents.len(), 2);
        assert_eq!(merged.agents["agent1"].model, "gpt-3.5".to_string());
        assert_eq!(merged.agents["agent2"].model, "claude-3".to_string());
    }
    #[test]
    fn test_optimization_string_deduplication() {
        use crate::codegen::{HelixIR, StringPool, Metadata, SymbolTable, ConstantPool};
        use crate::compiler::optimizer::{Optimizer, OptimizationLevel};
        let mut string_pool = StringPool::new();
        string_pool.intern("duplicate");
        string_pool.intern("unique");
        string_pool.intern("duplicate");
        let mut ir = HelixIR {
            version: 1,
            metadata: Metadata::default(),
            symbol_table: SymbolTable::default(),
            instructions: vec![],
            string_pool,
            constants: ConstantPool::new(),
        };
        let mut optimizer = Optimizer::new(OptimizationLevel::One);
        optimizer.optimize(&mut ir);
        assert_eq!(ir.string_pool.strings.len(), 2);
    }
    #[test]
    fn test_optimization_remove_duplicate_properties() {
        use crate::codegen::{
            HelixIR, Instruction, ConstantValue, StringPool, Metadata, SymbolTable,
            ConstantPool,
        };
        use crate::compiler::optimizer::{Optimizer, OptimizationLevel};
        let mut string_pool = StringPool::new();
        let prop_name_idx = string_pool.intern("model");
        let value1_idx = string_pool.intern("gpt-4");
        let value2_idx = string_pool.intern("gpt-3.5");
        let ir_instructions = vec![
            Instruction::DeclareAgent(1), Instruction::SetProperty { target : 1, key :
            prop_name_idx, value : ConstantValue::String(value1_idx) },
            Instruction::SetProperty { target : 1, key : prop_name_idx, value :
            ConstantValue::String(value2_idx) }, Instruction::SetProperty { target : 1,
            key : 1, value : ConstantValue::Number(0.7) },
        ];
        let mut ir = HelixIR {
            version: 1,
            metadata: Metadata::default(),
            symbol_table: SymbolTable::default(),
            instructions: ir_instructions,
            string_pool,
            constants: ConstantPool::new(),
        };
        let initial_count = ir.instructions.len();
        let mut optimizer = Optimizer::new(OptimizationLevel::Two);
        optimizer.optimize(&mut ir);
        assert!(ir.instructions.len() < initial_count);
        let prop_count = ir
            .instructions
            .iter()
            .filter(|i| matches!(i, Instruction::SetProperty { key : 0, .. }))
            .count();
        assert_eq!(prop_count, 1);
    }
    #[test]
    fn test_tree_shaking_removes_unused() {
        use crate::codegen::{
            HelixIR, Instruction, AgentSymbol, CrewSymbol, StringPool, Metadata,
            SymbolTable, ConstantPool,
        };
        use crate::compiler::optimizer::{Optimizer, OptimizationLevel};
        let mut symbol_table = SymbolTable::default();
        symbol_table
            .agents
            .insert(
                1,
                AgentSymbol {
                    id: 1,
                    name_idx: 0,
                    model_idx: 1,
                    role_idx: 2,
                    temperature: None,
                    max_tokens: None,
                    capabilities: vec![],
                    backstory_idx: None,
                },
            );
        symbol_table
            .agents
            .insert(
                2,
                AgentSymbol {
                    id: 2,
                    name_idx: 3,
                    model_idx: 1,
                    role_idx: 2,
                    temperature: None,
                    max_tokens: None,
                    capabilities: vec![],
                    backstory_idx: None,
                },
            );
        symbol_table
            .crews
            .insert(
                1,
                CrewSymbol {
                    id: 1,
                    name_idx: 4,
                    agent_ids: vec![1],
                    process_type: crate::codegen::ProcessTypeIR::Sequential,
                    manager_id: None,
                },
            );
        let mut ir = HelixIR {
            version: 1,
            metadata: Metadata::default(),
            symbol_table,
            instructions: vec![
                Instruction::DeclareAgent(1), Instruction::DeclareAgent(2),
                Instruction::DeclareCrew(1),
            ],
            string_pool: StringPool::new(),
            constants: ConstantPool::new(),
        };
        let mut optimizer = Optimizer::new(OptimizationLevel::Three);
        optimizer.optimize(&mut ir);
        assert_eq!(ir.symbol_table.agents.len(), 1);
        assert!(ir.symbol_table.agents.contains_key(& 1));
        assert!(! ir.symbol_table.agents.contains_key(& 2));
    }
    #[test]
    fn test_binary_roundtrip_preserves_data() {
        use crate::codegen::{
            HelixIR, Instruction, AgentSymbol, StringPool, Metadata, SymbolTable,
            ConstantPool,
        };
        use crate::compiler::serializer::BinarySerializer;
        let mut string_pool = StringPool::new();
        let name_idx = string_pool.intern("test-agent");
        let model_idx = string_pool.intern("gpt-4");
        let role_idx = string_pool.intern("Assistant");
        let mut symbol_table = SymbolTable::default();
        symbol_table
            .agents
            .insert(
                1,
                AgentSymbol {
                    id: 1,
                    name_idx,
                    model_idx,
                    role_idx,
                    temperature: Some(0.7),
                    max_tokens: Some(2000),
                    capabilities: vec![],
                    backstory_idx: None,
                },
            );
        let original_ir = HelixIR {
            version: 42,
            metadata: Metadata::default(),
            symbol_table,
            instructions: vec![Instruction::DeclareAgent(1)],
            string_pool,
            constants: ConstantPool::new(),
        };
        let serializer = BinarySerializer::new(false);
        let binary = serializer.serialize(original_ir.clone(), None).unwrap();
        let restored_ir = serializer.deserialize_to_ir(&binary).unwrap();
        assert_eq!(restored_ir.version, original_ir.version);
        assert_eq!(restored_ir.instructions.len(), original_ir.instructions.len());
        assert!(restored_ir.symbol_table.agents.contains_key(& 1));
        assert_eq!(
            restored_ir.string_pool.strings.len(), original_ir.string_pool.strings.len()
        );
    }
    #[test]
    fn test_cache_invalidation() {
        use std::thread::sleep;
        use std::time::Duration;
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("test.hlxbb");
        let cache_dir = temp_dir.path().join("cache");
        std::fs::write(&source_path, "agent \"test\" { model = \"gpt-4\" }").unwrap();
        let compiler = Compiler::builder()
            .optimization_level(OptimizationLevel::One)
            .cache(true)
            .cache_dir(cache_dir.clone())
            .build();
        let binary1 = compiler.compile_file(&source_path).unwrap();
        sleep(Duration::from_millis(10));
        std::fs::write(&source_path, "agent \"test\" { model = \"claude-3\" }").unwrap();
        let binary2 = compiler.compile_file(&source_path).unwrap();
        assert_ne!(binary1.checksum, binary2.checksum);
    }
    #[test]
    fn test_complex_workflow_with_dependencies() {
        let source = r#"
            agent "researcher" {
                model = "claude-3"
                role = "Research Specialist"
                temperature = 0.3
                max_tokens = 4000
            }
            
            agent "writer" {
                model = "gpt-4"
                role = "Technical Writer"
                temperature = 0.7
                max_tokens = 2000
            }
            
            workflow "research-and-write" {
                trigger = "api"
                timeout = 30m
                
                step "research" {
                    agent = "researcher"
                    task = "Research the topic thoroughly"
                    timeout = 15m
                }
                
                step "write" {
                    agent = "writer"
                    task = "Write comprehensive documentation"
                    depends_on = ["research"]
                    timeout = 10m
                }
                
                step "review" {
                    agent = "researcher"
                    task = "Review the written content"
                    depends_on = ["write"]
                    timeout = 5m
                }
            }
        "#;
        let ast = parse(source).expect("Failed to parse complex workflow");
        validate(&ast).expect("Failed to validate complex workflow");
        let config = ast_to_config(ast).expect("Failed to convert complex workflow");
        assert_eq!(config.agents.len(), 2);
        assert!(config.agents.contains_key("researcher"));
        assert!(config.agents.contains_key("writer"));
        assert!(config.workflows.contains_key("research-and-write"));
        let workflow = &config.workflows["research-and-write"];
        assert_eq!(workflow.steps.len(), 3);
        let write_step = workflow.steps.iter().find(|s| s.name == "write").unwrap();
        assert_eq!(write_step.depends_on.len(), 1);
        assert_eq!(write_step.depends_on[0], "research");
    }
    #[test]
    fn test_crew_with_hierarchical_structure() {
        let source = r#"
            agent "manager" {
                model = "gpt-4"
                role = "Project Manager"
                temperature = 0.2
                capabilities ["planning", "coordination"]
            }
            
            agent "developer" {
                model = "claude-3"
                role = "Senior Developer"
                capabilities ["rust", "python", "javascript"]
            }
            
            agent "tester" {
                model = "gpt-3.5-turbo"
                role = "QA Engineer"
                capabilities ["testing", "automation"]
            }
            
            crew "development-team" {
                agents ["manager", "developer", "tester"]
                manager = "manager"
                process = "hierarchical"
                max_execution_time = 2h
                verbose = true
            }
        "#;
        let ast = parse(source).expect("Failed to parse crew");
        validate(&ast).expect("Failed to validate crew");
        let config = ast_to_config(ast).expect("Failed to convert crew");
        assert!(config.crews.contains_key("development-team"));
        let crew = &config.crews["development-team"];
        assert_eq!(crew.agents.len(), 3);
        assert!(crew.agents.contains(& "manager".to_string()));
        assert!(crew.agents.contains(& "developer".to_string()));
        assert!(crew.agents.contains(& "tester".to_string()));
        assert_eq!(crew.manager, Some("manager".to_string()));
        assert_eq!(crew.process_type, crate ::types::ProcessType::Hierarchical);
        assert_eq!(crew.verbose, true);
    }
    #[test]
    fn test_error_handling_with_missing_fields() {
        let source = r#"
            agent "incomplete-agent" {
                # Missing model field
                temperature = 0.7
            }
        "#;
        let ast = parse(source).expect("Should parse syntactically");
        let config = ast_to_config(ast).expect("Should convert with defaults");
        assert!(config.agents.contains_key("incomplete-agent"));
        let agent = &config.agents["incomplete-agent"];
        assert_eq!(agent.name, "incomplete-agent");
        assert_eq!(agent.temperature, Some(0.7));
    }
    #[test]
    fn test_complex_agent_configuration() {
        let source = r#"
            agent "comprehensive-agent" {
                model = "gpt-4-turbo"
                role = "Senior Engineer"
                temperature = 0.3
                max_tokens = 4000
                capabilities ["rust", "systems", "performance"]
                backstory = "Experienced systems programmer"
            }
        "#;
        let ast = parse(source).expect("Failed to parse comprehensive agent");
        validate(&ast).expect("Failed to validate comprehensive agent");
        let config = ast_to_config(ast).expect("Failed to convert comprehensive agent");
        let agent = &config.agents["comprehensive-agent"];
        assert_eq!(agent.model, "gpt-4-turbo");
        assert_eq!(agent.role, "Senior Engineer");
        assert_eq!(agent.temperature, Some(0.3));
        assert_eq!(agent.max_tokens, Some(4000));
        assert_eq!(agent.capabilities, vec!["rust", "systems", "performance"]);
        assert_eq!(agent.backstory, Some("Experienced systems programmer".to_string()));
    }
    #[test]
    fn test_multiple_workflow_steps() {
        let source = r#"
            agent "worker" {
                model = "claude-3"
                role = "Worker"
            }
            
            workflow "multi-step" {
                trigger = "manual"
                
                step "step1" {
                    agent = "worker"
                    task = "Do first task"
                }
                
                step "step2" {
                    agent = "worker"
                    task = "Do second task"
                }
                
                step "step3" {
                    agent = "worker"  
                    task = "Do final task"
                }
            }
        "#;
        let ast = parse(source).expect("Failed to parse multi-step workflow");
        validate(&ast).expect("Failed to validate multi-step workflow");
        let config = ast_to_config(ast).expect("Failed to convert multi-step workflow");
        let workflow = &config.workflows["multi-step"];
        assert_eq!(workflow.steps.len(), 3);
        let step_names: Vec<&str> = workflow
            .steps
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(step_names.contains(& "step1"));
        assert!(step_names.contains(& "step2"));
        assert!(step_names.contains(& "step3"));
    }
    #[test]
    fn test_error_handling_with_invalid_references() {
        let source = r#"
            workflow "broken-workflow" {
                trigger = "manual"
                step "broken-step" {
                    agent = "non-existent-agent"
                    task = "This will fail validation"
                }
            }
        "#;
        let ast = parse(source).expect("Should parse syntactically valid code");
        let validation_result = validate(&ast);
        assert!(
            validation_result.is_err(),
            "Should fail validation due to missing agent reference"
        );
        let error_message = format!("{:?}", validation_result.unwrap_err());
        assert!(
            error_message.contains("non-existent-agent") || error_message
            .contains("reference") || error_message.contains("not found")
        );
    }
    #[test]
    fn test_large_configuration_performance() {
        let mut source = String::from(
            "project \"large-test\" { version = \"1.0.0\" }\n\n",
        );
        for i in 0..50 {
            source
                .push_str(
                    &format!(
                        "agent \"agent-{}\" {{\n  model = \"gpt-3.5-turbo\"\n  temperature = 0.{}\n}}\n\n",
                        i, i % 10
                    ),
                );
        }
        for i in 0..20 {
            source
                .push_str(
                    &format!(
                        "workflow \"workflow-{}\" {{\n  trigger = \"manual\"\n  step \"step-1\" {{\n    agent = \"agent-{}\"\n    task = \"Task {}\"\n  }}\n}}\n\n",
                        i, i % 50, i
                    ),
                );
        }
        for i in 0..10 {
            let agents: Vec<String> = (i * 5..i * 5 + 5)
                .map(|j| format!("\"agent-{}\"", j % 50))
                .collect();
            source
                .push_str(
                    &format!(
                        "crew \"crew-{}\" {{\n  agents [{}]\n  process = \"sequential\"\n}}\n\n",
                        i, agents.join(", ")
                    ),
                );
        }
        let start = std::time::Instant::now();
        let ast = parse(&source).expect("Failed to parse large config");
        let parse_time = start.elapsed();
        let validate_start = std::time::Instant::now();
        validate(&ast).expect("Failed to validate large config");
        let validate_time = validate_start.elapsed();
        let convert_start = std::time::Instant::now();
        let config = ast_to_config(ast).expect("Failed to convert large config");
        let convert_time = convert_start.elapsed();
        assert_eq!(config.agents.len(), 50);
        assert_eq!(config.workflows.len(), 20);
        assert_eq!(config.crews.len(), 10);
        assert!(parse_time.as_millis() < 100, "Parse time too slow: {:?}", parse_time);
        assert!(
            validate_time.as_millis() < 50, "Validate time too slow: {:?}", validate_time
        );
        assert!(
            convert_time.as_millis() < 50, "Convert time too slow: {:?}", convert_time
        );
    }
    #[test]
    fn test_binary_compression_effectiveness() {
        let source = r#"
            project "compression-test" {
                version = "1.0.0"
                author = "test-suite"
                description = "This is a test configuration with repetitive content to test compression effectiveness. The compression algorithm should be able to significantly reduce the size of this configuration due to the repetitive nature of the content."
            }
            
            agent "agent-alpha" {
                model = "gpt-4-turbo-preview"
                role = "Senior Software Engineer with extensive experience in Rust programming"
                temperature = 0.7
                max_tokens = 4000
                capabilities ["rust", "systems-programming", "performance-optimization", "memory-management"]
                backstory = "An experienced systems programmer with deep knowledge of Rust and performance optimization techniques"
            }
            
            agent "agent-beta" {
                model = "gpt-4-turbo-preview"
                role = "Senior Software Engineer with extensive experience in Rust programming"
                temperature = 0.7
                max_tokens = 4000
                capabilities ["rust", "systems-programming", "performance-optimization", "memory-management"]
                backstory = "An experienced systems programmer with deep knowledge of Rust and performance optimization techniques"
            }
            
            agent "agent-gamma" {
                model = "gpt-4-turbo-preview"
                role = "Senior Software Engineer with extensive experience in Rust programming"
                temperature = 0.7
                max_tokens = 4000
                capabilities ["rust", "systems-programming", "performance-optimization", "memory-management"]
                backstory = "An experienced systems programmer with deep knowledge of Rust and performance optimization techniques"
            }
        "#;
        let temp_dir = TempDir::new().unwrap();
        let source_path = temp_dir.path().join("compression_test.hlxbb");
        let uncompressed_path = temp_dir.path().join("uncompressed.hlxb");
        let compressed_path = temp_dir.path().join("compressed.hlxb");
        std::fs::write(&source_path, source).unwrap();
        let compiler = Compiler::new(OptimizationLevel::Two);
        let binary = compiler.compile_file(&source_path).unwrap();
        let uncompressed_serializer = BinarySerializer::new(false);
        uncompressed_serializer.write_to_file(&binary, &uncompressed_path).unwrap();
        let compressed_serializer = BinarySerializer::new(true);
        compressed_serializer.write_to_file(&binary, &compressed_path).unwrap();
        let uncompressed_size = std::fs::metadata(&uncompressed_path).unwrap().len();
        let compressed_size = std::fs::metadata(&compressed_path).unwrap().len();
        let compression_ratio = compressed_size as f64 / uncompressed_size as f64;
        assert!(
            compression_ratio < 0.8, "Compression ineffective: {:.2}% reduction", (1.0 -
            compression_ratio) * 100.0
        );
        let loader = crate::compiler::loader::BinaryLoader::new();
        let uncompressed_loaded = loader.load_file(&uncompressed_path).unwrap();
        let compressed_loaded = loader.load_file(&compressed_path).unwrap();
        assert_eq!(uncompressed_loaded.checksum, compressed_loaded.checksum);
    }
    #[test]
    fn test_concurrent_compilation_safety() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        let source = r#"
            agent "concurrent-agent" {
                model = "gpt-4"
                temperature = 0.7
            }
            
            workflow "concurrent-workflow" {
                trigger = "manual"
                step "process" {
                    agent = "concurrent-agent"
                    task = "Concurrent processing test"
                }
            }
        "#;
        let temp_dir = TempDir::new().unwrap();
        let results = Arc::new(Mutex::new(Vec::new()));
        let mut handles = vec![];
        for i in 0..5 {
            let source = source.to_string();
            let temp_dir = temp_dir.path().to_path_buf();
            let results = Arc::clone(&results);
            let handle = thread::spawn(move || {
                let source_path = temp_dir.join(format!("concurrent_{}.hlxbb", i));
                let binary_path = temp_dir.join(format!("concurrent_{}.hlxb", i));
                std::fs::write(&source_path, &source).unwrap();
                let compiler = Compiler::new(OptimizationLevel::One);
                let binary = compiler.compile_file(&source_path).unwrap();
                let serializer = BinarySerializer::new(false);
                serializer.write_to_file(&binary, &binary_path).unwrap();
                let loader = crate::compiler::loader::BinaryLoader::new();
                let loaded = loader.load_file(&binary_path).unwrap();
                results.lock().unwrap().push((i, loaded.checksum));
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let results = results.lock().unwrap();
        assert_eq!(results.len(), 5);
        let first_checksum = results[0].1;
        for (_, checksum) in results.iter() {
            assert_eq!(
                * checksum, first_checksum,
                "Concurrent compilations produced different results"
            );
        }
    }
}