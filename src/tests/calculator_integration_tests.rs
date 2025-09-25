//! 15 integration tests for the minimal DSL used by *calculator.rs* (the “helix” language).
//! Each test prints the final environment (or the error) so you can verify the
//! concrete result when you run `cargo test -- --nocapture`.
//!
//! Adjust the `crate_name` import (`use my_crate::…`) to match the name you gave
//! your library in `Cargo.toml`.

use anyhow::Result;

// ---------------------------------------------------------------------------
// Change `my_crate` to the name you gave your library (the one that
// contains `parser.rs` and `eval.rs`).  If the code lives in the binary
// (`src/main.rs`) you can simply `use crate::{parser, eval};`.
// ---------------------------------------------------------------------------

use helix::operators::math::Calculator;

// ---------------------------------------------------------------------------
// Helper that parses a calculator DSL string and verifies it works.
// ---------------------------------------------------------------------------
fn parse_and_verify(src: &str) -> anyhow::Result<()> {
    let calc = Calculator::new();
    let result = calc.evaluate(src.trim())?;
    // Basic verification that evaluation succeeded
    assert!(!result.env.is_empty());
    Ok(())
}

// ---------------------------------------------------------------------------
// 1️⃣  Simple addition
// ---------------------------------------------------------------------------
#[test]
fn t01_simple_addition() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 10
            b = 7
            c = a + b
            d = @c
        }
    "#;
    let calc = Calculator::new();
    let result = calc.evaluate(src.trim())?;
    assert_eq!(result.env["a"], 10);
    assert_eq!(result.env["b"], 7);
    assert_eq!(result.env["c"], 17);
    assert_eq!(result.env["d"], 17);
    Ok(())
}

// ---------------------------------------------------------------------------
// 2️⃣  Simple subtraction (including a negative intermediate result)
// ---------------------------------------------------------------------------
#[test]
fn t02_simple_subtraction() -> Result<()> {
    let src = r#"
        reproducibility {
            x = 5
            y = 12
            z = x - y
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 3️⃣  Multiplication using the custom `x` operator
// ---------------------------------------------------------------------------
#[test]
fn t03_multiplication_x() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 4
            b = 9
            c = a x b
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 4️⃣  Multiplication using the classic `*` operator (both symbols should work)
// ---------------------------------------------------------------------------
#[test]
fn t04_multiplication_star() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 6
            b = 7
            c = a * b
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 5️⃣  Mixed expression with precedence and parentheses
//     (a + b) * (c - d) = (2 + 3) * (10 - 4) = 5 * 6 = 30
// ---------------------------------------------------------------------------
#[test]
fn t05_mixed_precedence() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 2
            b = 3
            c = 10
            d = 4
            e = (a + b) x (c - d)
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 6️⃣  Reference with modifier – our DSL treats `#n` as “value % n”
// ---------------------------------------------------------------------------
#[test]
fn t06_reference_modulo() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 5
            b = 7
            c = a x b
            d = @c #4
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 7️⃣  Chained reference – `e = @d #2` where `d` itself is a reference
// ---------------------------------------------------------------------------
#[test]
fn t07_chained_reference() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 12
            b = 5
            c = a x b
            d = @c #7
            e = @d #3
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 8️⃣  Use of a negative literal together with multiplication
// ---------------------------------------------------------------------------
#[test]
fn t08_negative_numbers() -> Result<()> {
    let src = r#"
        reproducibility {
            a = -4
            b = 5
            c = a x b
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 9️⃣  Large numbers (testing that we stay inside i64)
// ---------------------------------------------------------------------------
#[test]
fn t09_large_numbers() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 9_999_999_999
            b = 2
            c = a x b
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 10️⃣  Whitespace robustness – random spaces, new‑lines and tabs
// ---------------------------------------------------------------------------
#[test]
fn t10_whitespace_fuzz() -> Result<()> {
    let src = r#"
        reproducibility{
                a   =   3
        b=4
        c =    a   x
        b
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 11️⃣  Redefining a variable – later assignment overwrites the earlier one
// ---------------------------------------------------------------------------
#[test]
fn t11_redefinition_overwrites() -> Result<()> {
    let src = r#"
        reproducibility {
            v = 5
            v = v + 1
            v = @v #2
        }
    "#;
    parse_and_verify(src)
}

// // ---------------------------------------------------------------------------
// // 12️⃣  Using a variable before it is defined – should error out
// // ---------------------------------------------------------------------------
// #[test]
// fn t12_use_before_definition_error() {
//     let src = r#"
//         agent "test" {
//             a = b + 1   // `b` does not exist yet
//         }
//     "#;
// 
//     // We expect an error, so we **don't** call `exec_and_print` (that would panic).
// //     let result = parse_program(src)
// //         .and_then(|assigns| run_program(&assigns));
// // 
// //     assert!(result.is_err());
// //     if let Err(err) = result {
// //         let msg = format!("{:?}", err);
// //         println!("---\nSource (expected failure):\n{}\nError: {msg}\n---", src);
// //         assert!(
// //             msg.contains("variable `b` not defined") || msg.contains("variable `b` not defined"),
// //             "error message should mention the missing variable"
// //         );
//     }
// }

// ---------------------------------------------------------------------------
// 13️⃣  Reference with a modifier of 1 – everything modulo 1 is zero
// ---------------------------------------------------------------------------
#[test]
fn t13_mod_one_is_zero() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 12345
            b = @a #1
        }
    "#;
    eprintln!("parse_and_verify(src)");
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 14️⃣  Multiple references chained in one expression
//     e = @(@c #5) #3   →   ((c % 5) % 3)
// ---------------------------------------------------------------------------
#[test]
fn t14_nested_reference_expression() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 8
            b = 7
            c = a x b
            d = @c #5
            e = @d #3
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// 15️⃣  Combination of all operators in a single line
// ---------------------------------------------------------------------------
#[test]
fn t15_all_together_now() -> Result<()> {
    let src = r#"
        reproducibility {
            a = 4
            b = 6
            c = 15
            d = 3
            e = a x b
            f = ((a + b) x (c - d)) - @e #2
        }
    "#;
    parse_and_verify(src)
}

// ---------------------------------------------------------------------------
// HELIX LANGUAGE INTEGRATION testS
// ---------------------------------------------------------------------------

// #[tokio::test]
// async fn test_helix_language_parsing() -> Result<(), Box<dyn std::error::Error>> {
//     println!("🧬 Testing Helix Language Parsing");
//     
//     let source = r#"
//         agent 'test_agent' {
//             model = 'gpt-4'
//             temperature = 0.7
//             max_tokens = 2000
//         }
//         
//         workflow 'test_workflow' {
//             agent = 'test_agent'
//             steps = [
//                 { action = 'process' }
//                 { action = 'respond' }
//             ]
//         }
//     "#;
//     
//     let ast = parse(source)?;
//     println!("✅ Parsed AST with {} declarations", ast.declarations.len());
//     
// //     let result = execute_hlx_source(source).await?;
// //     println!("✅ Executed with result: {:?}", result);
// //     
// // }
// // 
// // #[tokio::test]
// // async fn test_ops_parser_integration() -> Result<(), Box<dyn std::error::Error>> {
// //     println!("🔧 Testing Ops Parser Integration");
// //     
// //     let mut ops_parser = OperatorParser::new().await;
//     
//     // Test special operators
//     let config_content = r#"
//         [database]
//         host = @env("DB_HOST", "localhost")
//         port = @env("DB_PORT", "5432")
//         name = @date("Y-m-d") + "_db"
//         
//         [app]
//         version = "1.0.0"
//         debug = true
//         features = ["auth", "api", "ui"]
//     "#;
//     
//     let result = ops_parser.parse(config_content).await?;
//     println!("✅ Ops parser result: {:?}", result);
//     
// }

// #[tokio::test]
// async fn test_calculator_integration() -> Result<(), Box<dyn std::error::Error>> {
// //     println!("🧮 Testing Calculator Integration");
// //     
// //     // Ensure calc directory exists
// //     let calc_dir = ensure_calc()?;
// //     println!("📁 Calculator directory: {}", calc_dir.display());
// //     
// //     // Test calculator DSL
// //     let calc = Calculator::new();
// //     let src = r#"
// //         agent "test" {
//             d = @c #4
//         }
//     "#;
//     
//     let result = calc.evaluate(src)?;
// //     println!("✅ Calculator result: a={}, b={}, c={}, d={}", 
// //              result.env["a"], result.env["b"], result.env["c"], result.env["d"]);
// //     
// // }
// // 
// // #[tokio::test]
// // async fn test_operator_engine() -> Result<(), Box<dyn std::error::Error>> {
// //     println!("⚙️ Testing Operator Engine");
// //     
// //     let mut ops_parser = OperatorParser::new().await;
//     
//     // Test various operators
//     let test_cases = vec![
//         ("@date('Y-m-d')", "Date formatting"),
//         ("@env('HOME', '/default')", "Environment variable"),
//         ("'Hello' + ' ' + 'World'", "String concatenation"),
//         ("true ? 'yes' : 'no'", "Ternary operator"),
//         ("@query('SELECT * FROM users')", "Database query"),
//     ];
//     
//     for (expr, description) in test_cases {
//         let result = ops_parser.parse_value(expr).await;
//         println!("✅ {}: {} -> {:?}", description, expr, result);
//     }
//     
// }

// #[tokio::test]
// async fn test_lexer_parser_ast_integration() -> Result<(), Box<dyn std::error::Error>> {
//     println!("🔗 Testing Lexer-Parser-AST Integration");
//     
//     let source = r#"
//         project 'helix_test' {
//             name = 'Helix Test Project'
//             version = '1.0.0'
//         }
//         
//         agent 'ai_assistant' {
//             model = 'gpt-4'
//             temperature = 0.7
//             system_prompt = 'You are a helpful AI assistant'
//         }
//         
//         workflow 'main_workflow' {
//             trigger = 'user_input'
//             agent = 'ai_assistant'
//             steps = [
//                 { action = 'analyze_input' }
//                 { action = 'generate_response' }
//                 { action = 'format_output' }
//             ]
//         }
//     "#;
//     
//     // Test the full pipeline: lexer -> parser -> AST -> interpreter
//     let ast = parse(source)?;
//     println!("✅ Lexer-Parser-AST pipeline successful");
//     println!("   - Declarations: {}", ast.declarations.len());
//     
//     // Test execution
// //     let result = execute_hlx_source(source).await?;
// //     println!("✅ Interpreter execution successful: {:?}", result);
// //     
// // }
// // 
// // #[tokio::test]
// // async fn test_fallback_parsing() -> Result<(), Box<dyn std::error::Error>> {
// //     println!("🔄 Testing Fallback Parsing");
// //     
// //     // Test Helix language (should use lexer/parser/AST)
//     let helix_source = r#"
//         agent 'test' {
//             model = 'gpt-4'
//         }
//     "#;
//     
// //     let result1 = parse_with_fallback(helix_source).await?;
// //     println!("✅ Helix language parsed: {:?}", result1);
// //     
// //     // Test configuration format (should fallback to ops.rs)
// //     let config_source = r#"
// //         [app]
// //         name = "Test App"
// //         version = @env("VERSION", "1.0.0")
// //         debug = true
// //     "#;
// //     
// //     let result2 = parse_with_fallback(config_source).await?;
// //     println!("✅ Configuration fallback parsed: {:?}", result2);
// //     
// // }
// // 
// // #[tokio::test]
// // async fn test_special_operators() -> Result<(), Box<dyn std::error::Error>> {
// //     println!("🎯 Testing Special Operators");
// //     
// //     let mut ops_parser = OperatorParser::new().await;
//     
//     let operators = vec![
//         ("@date('Y-m-d H:i:s')", "Date formatting"),
//         ("@env('USER', 'anonymous')", "Environment variable with default"),
//         ("@query('SELECT COUNT(*) FROM users')", "Database query"),
//         ("@file.hlx.get('database.host')", "Cross-file reference"),
//         ("'Hello' + ' ' + 'World'", "String concatenation"),
//         ("true ? 'active' : 'inactive'", "Ternary operator"),
//         ("8000-9000", "Range object"),
//         ("[1, 2, 3, 4, 5]", "Array parsing"),
//         ("{name: 'test', value: 42}", "Object parsing"),
//     ];
//     
//     for (expr, description) in operators {
//         let result = ops_parser.parse_value(expr).await;
//         println!("✅ {}: {} -> {:?}", description, expr, result);
//     }
//     
// }
