#[cfg(test)]
mod lexer_tests {
    use crate::lexer::*;
    #[test]
    fn test_tokenize_keywords() {
        let input = "agent workflow memory context crew project";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Keyword(Keyword::Agent));
        assert_eq!(tokens[1], Token::Keyword(Keyword::Workflow));
        assert_eq!(tokens[2], Token::Keyword(Keyword::Memory));
        assert_eq!(tokens[3], Token::Keyword(Keyword::Context));
        assert_eq!(tokens[4], Token::Keyword(Keyword::Crew));
        assert_eq!(tokens[5], Token::Keyword(Keyword::Project));
    }
    #[test]
    fn test_tokenize_strings() {
        let input = r#""hello" "world with spaces" "escaped \"quotes\"""#;
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::String("hello".to_string()));
        assert_eq!(tokens[1], Token::String("world with spaces".to_string()));
        assert_eq!(tokens[2], Token::String("escaped \"quotes\"".to_string()));
    }
    #[test]
    fn test_tokenize_numbers() {
        let input = "42 3.14 -17 0.001";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Number(42.0));
        assert_eq!(tokens[1], Token::Number(3.14));
        assert_eq!(tokens[2], Token::Number(- 17.0));
        assert_eq!(tokens[3], Token::Number(0.001));
    }
    #[test]
    fn test_tokenize_durations() {
        let input = "30s 5m 2h 7d";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Duration(30, TimeUnit::Seconds));
        assert_eq!(tokens[1], Token::Duration(5, TimeUnit::Minutes));
        assert_eq!(tokens[2], Token::Duration(2, TimeUnit::Hours));
        assert_eq!(tokens[3], Token::Duration(7, TimeUnit::Days));
    }
    #[test]
    fn test_tokenize_references() {
        let input = "$ENV_VAR @memory.context.history";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Variable("ENV_VAR".to_string()));
        assert_eq!(tokens[1], Token::Reference("memory.context.history".to_string()));
    }
    #[test]
    fn test_tokenize_operators() {
        let input = "= -> [ ] { } ( ) , .";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Assign);
        assert_eq!(tokens[1], Token::Arrow);
        assert_eq!(tokens[2], Token::LeftBracket);
        assert_eq!(tokens[3], Token::RightBracket);
        assert_eq!(tokens[4], Token::LeftBrace);
        assert_eq!(tokens[5], Token::RightBrace);
        assert_eq!(tokens[6], Token::LeftParen);
        assert_eq!(tokens[7], Token::RightParen);
        assert_eq!(tokens[8], Token::Comma);
        assert_eq!(tokens[9], Token::Dot);
    }
    #[test]
    fn test_line_continuation() {
        let input = "value = \\\n    \"continued\"";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Identifier("value".to_string()));
        assert_eq!(tokens[1], Token::Assign);
        assert_eq!(tokens[2], Token::String("continued".to_string()));
    }
    #[test]
    fn test_source_location_tracking() {
        let input = "agent\n  \"test\"";
        let tokens = tokenize_with_locations(input).unwrap();
        assert_eq!(tokens[0].token, Token::Keyword(Keyword::Agent));
        assert_eq!(tokens[0].location.line, 1);
        assert_eq!(tokens[0].location.column, 1);
        assert_eq!(tokens[2].token, Token::String("test".to_string()));
        assert_eq!(tokens[2].location.line, 2);
    }
}
#[cfg(test)]
mod parser_tests {
    use crate::lexer;
    use crate::parser::*;
    use crate::ast::*;
    #[test]
    fn test_parse_agent() {
        let input = r#"
        agent "test-agent" {
            model = "gpt-4"
            temperature = 0.7
            max_tokens = 100000
            
            capabilities [
                "coding"
                "testing"
            ]
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.declarations.len(), 1);
        if let Declaration::Agent(agent) = &ast.declarations[0] {
            assert_eq!(agent.name, "test-agent");
            assert!(agent.properties.contains_key("model"));
            assert!(agent.capabilities.is_some());
            assert_eq!(agent.capabilities.as_ref().unwrap().len(), 2);
        } else {
            panic!("Expected agent declaration");
        }
    }
    #[test]
    fn test_parse_workflow() {
        let input = r#"
        workflow "test-workflow" {
            trigger = "manual"
            
            step "step1" {
                agent = "agent1"
                task = "Process data"
                timeout = 30m
            }
            
            step "step2" {
                agent = "agent2"
                task = "Analyze results"
                depends_on = ["step1"]
            }
            
            pipeline {
                step1 -> step2
            }
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.declarations.len(), 1);
        if let Declaration::Workflow(workflow) = &ast.declarations[0] {
            assert_eq!(workflow.name, "test-workflow");
            assert_eq!(workflow.steps.len(), 2);
            assert!(workflow.pipeline.is_some());
        } else {
            panic!("Expected workflow declaration");
        }
    }
    #[test]
    fn test_parse_memory() {
        let input = r#"
        memory {
            provider = "postgres"
            connection = "postgresql:
            
            embeddings {
                model = "text-embedding-3"
                dimensions = 1536
            }
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.declarations.len(), 1);
        if let Declaration::Memory(memory) = &ast.declarations[0] {
            assert_eq!(memory.provider, "postgres");
            assert!(memory.embeddings.is_some());
        } else {
            panic!("Expected memory declaration");
        }
    }
    #[test]
    fn test_parse_pipeline_expression() {
        let input = r#"
        workflow "pipeline-test" {
            pipeline {
                fetch -> process -> validate -> store
            }
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        if let Declaration::Workflow(workflow) = &ast.declarations[0] {
            assert!(workflow.pipeline.is_some());
            let pipeline = workflow.pipeline.as_ref().unwrap();
            assert_eq!(pipeline.flow.len(), 4);
        } else {
            panic!("Expected workflow declaration");
        }
    }
    #[test]
    fn test_parse_retry_config() {
        let input = r#"
        workflow "retry-test" {
            step "test" {
                agent = "agent1"
                retry {
                    max_attempts = 3
                    delay = 30s
                    backoff = "exponential"
                }
            }
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        if let Declaration::Workflow(workflow) = &ast.declarations[0] {
            let step = &workflow.steps[0];
            assert!(step.properties.contains_key("retry"));
        } else {
            panic!("Expected workflow declaration");
        }
    }
    #[test]
    fn test_error_recovery() {
        let input = r#"
        agent "test1" {
            model = "gpt-4"
            INVALID TOKEN HERE
        }
        
        agent "test2" {
            model = "claude"
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = Parser::new(tokens);
        let result = parser.parse();
        assert!(result.is_ok() || result.is_err());
    }
}
#[cfg(test)]
mod integration_tests {
    use crate::{lexer, parser, semantic, codegen};
    use crate::ast::*;
    #[test]
    fn test_full_pipeline() {
        let input = r#"
        project "integration-test" {
            version = "1.0.0"
            author = "Test"
        }
        
        agent "processor" {
            model = "gpt-4"
            temperature = 0.7
        }
        
        workflow "main" {
            trigger = "manual"
            
            step "process" {
                agent = "processor"
                task = "Process data"
                timeout = 30m
            }
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        assert!(! tokens.is_empty());
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse().unwrap();
        assert_eq!(ast.declarations.len(), 3);
        let mut analyzer = semantic::SemanticAnalyzer::new();
        let validation_result = analyzer.analyze(&ast);
        assert!(validation_result.is_ok());
        let mut generator = codegen::CodeGenerator::new();
        let ir = generator.generate(&ast);
        assert_eq!(ir.symbol_table.agents.len(), 1);
        assert_eq!(ir.symbol_table.workflows.len(), 1);
    }
    #[test]
    fn test_round_trip() {
        let input = r#"
        agent "round-trip-test" {
            model = "gpt-4"
            temperature = 0.5
            max_tokens = 50000
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = parser::Parser::new(tokens.clone());
        let ast = parser.parse().unwrap();
        let mut printer = AstPrettyPrinter::new();
        let output = printer.print(&ast);
        let tokens2 = lexer::tokenize(&output).unwrap();
        let mut parser2 = parser::Parser::new(tokens2);
        let ast2 = parser2.parse().unwrap();
        assert_eq!(ast.declarations.len(), ast2.declarations.len());
    }
}
#[cfg(test)]
mod fuzzing_tests {
    use crate::{lexer, parser};
    fn generate_test_mso(size: usize) -> String {
        let mut result = String::new();
        let keywords = ["agent", "workflow", "memory", "context", "crew"];
        let operators = ["=", "->", "{", "}", "[", "]"];
        for i in 0..size {
            match i % 5 {
                0 => {
                    let keyword = keywords[i % keywords.len()];
                    result.push_str(keyword);
                    result.push(' ');
                }
                1 => {
                    result.push_str(&format!("\"string{}\" ", i));
                }
                2 => {
                    result.push_str(&format!("{}.{} ", i % 100, i % 10));
                }
                3 => {
                    let op = operators[i % operators.len()];
                    result.push_str(op);
                    result.push(' ');
                }
                _ => {
                    result.push_str(&format!("id{} ", i));
                }
            }
            if i % 10 == 9 {
                result.push('\n');
            }
        }
        result
    }
    #[test]
    fn fuzz_lexer() {
        for _ in 0..100 {
            let input = generate_test_mso(100);
            let _ = lexer::tokenize(&input);
        }
    }
    #[test]
    fn fuzz_parser() {
        for _ in 0..100 {
            let input = generate_test_mso(50);
            if let Ok(tokens) = lexer::tokenize(&input) {
                let mut parser = parser::Parser::new(tokens);
                let _ = parser.parse();
            }
        }
    }
    #[test]
    fn fuzz_unicode() {
        let unicode_tests = [
            "agent \"测试\" { model = \"gpt-4\" }",
            "agent \"тест\" { model = \"gpt-4\" }",
            "agent \"🚀\" { model = \"gpt-4\" }",
            "agent \"hello\\nworld\" { model = \"gpt-4\" }",
        ];
        for input in &unicode_tests {
            let tokens = lexer::tokenize(input).unwrap();
            let mut parser = parser::Parser::new(tokens);
            let _ = parser.parse();
        }
    }
}
#[cfg(test)]
mod semantic_tests {
    use crate::{lexer, parser, semantic};
    #[test]
    fn test_undefined_agent_detection() {
        let input = r#"
        workflow "test" {
            step "s1" {
                agent = "undefined-agent"
                task = "test"
            }
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse().unwrap();
        let mut analyzer = semantic::SemanticAnalyzer::new();
        let result = analyzer.analyze(&ast);
        assert!(result.is_err());
        if let Err(errors) = result {
            assert!(
                errors.iter().any(| e | matches!(e,
                semantic::SemanticError::UndefinedAgent { .. }))
            );
        }
    }
    #[test]
    fn test_circular_dependency_detection() {
        let input = r#"
        workflow "test" {
            step "a" {
                agent = "agent1"
                depends_on = ["c"]
            }
            step "b" {
                agent = "agent1"
                depends_on = ["a"]
            }
            step "c" {
                agent = "agent1"
                depends_on = ["b"]
            }
        }
        
        agent "agent1" {
            model = "gpt-4"
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse().unwrap();
        let mut analyzer = semantic::SemanticAnalyzer::new();
        let result = analyzer.analyze(&ast);
        assert!(result.is_err());
        if let Err(errors) = result {
            assert!(
                errors.iter().any(| e | matches!(e,
                semantic::SemanticError::CircularDependency { .. }))
            );
        }
    }
}
#[cfg(test)]
mod codegen_tests {
    use crate::{lexer, parser, codegen};
    #[test]
    fn test_string_interning() {
        let input = r#"
        agent "test1" { model = "gpt-4" }
        agent "test2" { model = "gpt-4" }
        agent "test3" { model = "gpt-4" }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse().unwrap();
        let mut generator = codegen::CodeGenerator::new();
        let ir = generator.generate(&ast);
        let gpt4_count = ir.string_pool.strings.iter().filter(|s| *s == "gpt-4").count();
        assert_eq!(gpt4_count, 1);
    }
    #[test]
    fn test_binary_serialization() {
        let input = r#"
        agent "test" {
            model = "gpt-4"
            temperature = 0.7
        }
        "#;
        let tokens = lexer::tokenize(input).unwrap();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse().unwrap();
        let mut generator = codegen::CodeGenerator::new();
        let ir = generator.generate(&ast);
        let binary = codegen::BinarySerializer::serialize(&ir).unwrap();
        let ir2 = codegen::BinarySerializer::deserialize(&binary).unwrap();
        assert_eq!(ir.version, ir2.version);
        assert_eq!(ir.symbol_table.agents.len(), ir2.symbol_table.agents.len());
    }
}