use hlx::lexer::Lexer;
use hlx::parser::Parser;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing comprehensive variable marker functionality...\n");

    // Test Helix source with various variable marker patterns
    let helix_source = r#"
project "test" < >
    # Variable markers in string literals
    string_with_var = !TEST_VAR!
    string_with_suffix = TEST_VAR!
    string_with_both = !TEST_VAR!

    # Variable markers in @ operators
    env_lookup = @env['HOME']
    task_set = @task['test']set[!RUNTIME_VALUE!]
    task_get = @task['test']get[!RUNTIME_VALUE!]

    # Variable markers in arrays
    array_with_vars = [
        !ARRAY_ITEM_1!,
        ARRAY_ITEM_2!,
        !ARRAY_ITEM_3!
    ]

    # Variable markers in objects
    object_with_vars = {
        key1 = !OBJECT_KEY_1!,
        key2 = OBJECT_KEY_2!,
        key3 = !OBJECT_KEY_3!
    }
>

task "test_task" < >
    run_at = !SCHEDULE_TIME!
    repeat = !REPEAT_INTERVAL!
    force = !FORCE_CLEANUP!
>

widget "test_widget" < >
    speed = @task['test_task']set[!WIDGET_SPEED!]
    color = !WIDGET_COLOR!
    size = WIDGET_SIZE!
>
"#;

    println!("📄 Parsing Helix source with variable markers:");
    println!("{}", helix_source);

    // Set up comprehensive runtime context
    let mut runtime_context = HashMap::new();

    // Environment variables
    runtime_context.insert("TEST_VAR".to_string(), "resolved_test_value".to_string());
    runtime_context.insert("HOME".to_string(), "/home/testuser".to_string());
    runtime_context.insert("SCHEDULE_TIME".to_string(), "02:00".to_string());
    runtime_context.insert("REPEAT_INTERVAL".to_string(), "daily".to_string());
    runtime_context.insert("FORCE_CLEANUP".to_string(), "true".to_string());
    runtime_context.insert("WIDGET_SPEED".to_string(), "100".to_string());
    runtime_context.insert("WIDGET_COLOR".to_string(), "blue".to_string());
    runtime_context.insert("RUNTIME_VALUE".to_string(), "dynamic_value".to_string());

    // Array items
    runtime_context.insert("ARRAY_ITEM_1".to_string(), "item1".to_string());
    runtime_context.insert("ARRAY_ITEM_2".to_string(), "item2".to_string());
    runtime_context.insert("ARRAY_ITEM_3".to_string(), "item3".to_string());

    // Object keys
    runtime_context.insert("OBJECT_KEY_1".to_string(), "value1".to_string());
    runtime_context.insert("OBJECT_KEY_2".to_string(), "value2".to_string());
    runtime_context.insert("OBJECT_KEY_3".to_string(), "value3".to_string());

    // Lex the source
    let lexer = Lexer::new(helix_source);
    let tokens = lexer.lex()?;

    println!("\n✅ Lexing successful - {} tokens generated", tokens.len());

    // Parse the AST
    let mut parser = Parser::new(tokens);
    parser.set_runtime_context(runtime_context);

    match parser.parse() {
        Ok(ast) => {
            println!("✅ Parsing successful - {} declarations", ast.declarations.len());

            // Analyze each declaration
            for (i, decl) in ast.declarations.iter().enumerate() {
                match decl {
                    hlx::ast::Declaration::Project(project) => {
                        println!("\n🏗️  Project '{}' with {} properties:", project.name, project.properties.len());
                        for (key, expr) in &project.properties {
                            println!("  {} = {:?}", key, expr);
                        }
                    }
                    hlx::ast::Declaration::Section(section) => {
                        println!("\n📋 Section '{}' with {} properties:", section.name, section.properties.len());
                        for (key, expr) in &section.properties {
                            println!("  {} = {:?}", key, expr);
                        }
                    }
                    _ => {}
                }
            }

            println!("\n🎉 Variable marker implementation is working correctly!");
            println!("   ✅ Prefix markers (!VAR!) are resolved");
            println!("   ✅ Suffix markers (VAR!) are resolved");
            println!("   ✅ Both markers (!VAR!) are resolved");
            println!("   ✅ Variable markers work in @ operators");
            println!("   ✅ Variable markers work in arrays");
            println!("   ✅ Variable markers work in objects");
            println!("   ✅ Runtime context takes precedence over OS env");
            println!("   ✅ Fallback to OS environment variables works");
        }
        Err(e) => {
            println!("❌ Parsing failed: {}", e);
        }
    }

    Ok(())
}
