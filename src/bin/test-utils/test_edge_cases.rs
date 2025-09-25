use std::fs;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HELIX Language Edge Cases and Error Handling...\n");
    println!("Test 1: Invalid syntax error handling...");
    let invalid_content = fs::read_to_string("test_invalid.hlxbb")?;
    match helix::parse(&invalid_content) {
        Ok(_) => println!("❌ Should have failed to parse invalid syntax"),
        Err(e) => println!("✅ Correctly caught parse error: {}", e.message),
    }
    println!("\nTest 2: Empty file handling...");
    match helix::parse("") {
        Ok(ast) => {
            println!(
                "✅ Empty file parsed with {} declarations", ast.declarations.len()
            )
        }
        Err(e) => println!("❌ Empty file should parse: {}", e.message),
    }
    println!("\nTest 3: Comments-only file handling...");
    let comments_only = "# This is just a comment\n# Another comment";
    match helix::parse(comments_only) {
        Ok(ast) => {
            println!(
                "✅ Comments-only file parsed with {} declarations", ast.declarations
                .len()
            )
        }
        Err(e) => println!("❌ Comments should be ignored: {}", e.message),
    }
    println!("\nTest 4: Duration parsing edge cases...");
    let duration_tests = vec![
        "agent \"test\" { timeout = 1s }", "agent \"test\" { timeout = 60m }",
        "agent \"test\" { timeout = 24h }", "agent \"test\" { timeout = 7d }",
        "agent \"test\" { timeout = 0s }",
    ];
    for (i, test) in duration_tests.iter().enumerate() {
        match helix::parse(test) {
            Ok(_) => println!("✅ Duration test {} passed", i + 1),
            Err(e) => println!("❌ Duration test {} failed: {}", i + 1, e.message),
        }
    }
    println!("\nTest 5: Large numbers...");
    let large_num_test = "agent \"test\" { max_tokens = 999999999 }";
    match helix::parse(large_num_test) {
        Ok(_) => println!("✅ Large numbers handled correctly"),
        Err(e) => println!("❌ Large number parsing failed: {}", e.message),
    }
    println!("\nTest 6: Unicode string handling...");
    let unicode_test = r#"agent "test" { model = "🤖 GPT-4 émojis åçcénts" }"#;
    match helix::parse(unicode_test) {
        Ok(_) => println!("✅ Unicode strings handled correctly"),
        Err(e) => println!("❌ Unicode parsing failed: {}", e.message),
    }
    println!("\nTest 7: Complex nested structures...");
    let complex_test = r#"
    project "complex" {
        version = "1.0.0"
        nested = {
            level1 = {
                level2 = {
                    deep_value = "test"
                }
            }
        }
    }
    "#;
    match helix::parse(complex_test) {
        Ok(_) => println!("✅ Complex nested structures handled"),
        Err(e) => println!("❌ Complex nesting failed: {}", e.message),
    }
    println!("\nTest 8: Config conversion with minimal data...");
    let minimal_test = "agent \"minimal\" { model = \"gpt-3.5\" }";
    match helix::parse_and_validate(minimal_test) {
        Ok(config) => {
            println!("✅ Minimal config created");
            println!("   - Agents: {}", config.agents.len());
            println!("   - Workflows: {}", config.workflows.len());
            println!("   - Crews: {}", config.crews.len());
        }
        Err(e) => println!("❌ Minimal config failed: {}", e),
    }
    println!("\nTest 9: VM with minimal binary...");
    let source = "agent \"vm_test\" { model = \"test\" }";
    let compiler = helix::compiler::Compiler::new(
        helix::compiler::OptimizationLevel::Zero,
    );
    match compiler.compile_source(source, None) {
        Ok(binary) => {
            let mut vm = helix::HelixVM::new();
            match vm.execute_binary(&binary) {
                Ok(_) => println!("✅ VM execution completed"),
                Err(e) => println!("❌ VM execution failed: {}", e.message),
            }
        }
        Err(e) => println!("❌ Binary compilation failed: {}", e),
    }
    println!("\nTest 10: All optimization levels...");
    let opt_levels = vec![
        ("Zero", helix::OptimizationLevel::Zero), ("One",
        helix::OptimizationLevel::One), ("Two", helix::OptimizationLevel::Two),
        ("Three", helix::OptimizationLevel::Three),
    ];
    let test_source = r#"
        agent "opt_test" { model = "gpt-4" }
        workflow "opt_wf" { 
            trigger = "manual"
            step "test" { agent = "opt_test" task = "test" }
        }
    "#;
    for (name, level) in opt_levels {
        let compiler = helix::compiler::Compiler::new(level);
        match compiler.compile_source(test_source, None) {
            Ok(binary) => println!("✅ {} optimization: {} bytes", name, binary.size()),
            Err(e) => println!("❌ {} optimization failed: {}", name, e),
        }
    }
    println!("\n🎉 Edge case testing completed!");
    Ok(())
}