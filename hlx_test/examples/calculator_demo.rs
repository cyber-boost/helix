//! Calculator DSL Demo
//!
//! This example demonstrates the calculator DSL that can parse and evaluate
//! custom syntax with variables, arithmetic operations, and reference-with-modifier syntax.

use helix::ops::{ensure_calc, OperatorParser};
use helix::value::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧮 Helix Calculator DSL Demo");
    println!("===========================\n");

    // Ensure calculator directory exists
    let calc_dir = ensure_calc()?;
    println!("📁 Calculator directory: {}", calc_dir.display());

    // Test the ops parser with calculator-like expressions
    let mut ops_parser = tokio::runtime::Runtime::new()?.block_on(OperatorParser::new());

    // Example 1: Basic arithmetic
    println!("📊 Example 1: Basic Arithmetic");
    let src1 = r#"
    reproducibility {
        a = 2
        b = 3
        c = a x b
        d = c + 5
        e = d - 2
    }
    "#;

    let result1 = ops_parser.parse(src1).await.map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Program:\n{}", src1.trim());
    println!("Result: a={}, b={}, c={}, d={}, e={}\n",
             result1["a"], result1["b"], result1["c"],
             result1["d"], result1["e"]);

    // Example 2: Reference with modifier (modulo)
    println!("🔗 Example 2: Reference with Modifier");
    let src2 = r#"
    reproducibility {
        a = 2
        b = 2
        c = a x b
        d = @c #4
    }
    "#;

    let result2 = ops_parser.parse(src2).await.map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Program:\n{}", src2.trim());
    println!("Result: a={}, b={}, c={}, d={} ({} % 4 = {})\n",
             result2["a"], result2["b"], result2["c"],
             result2["d"], result2["c"], result2["d"]);

    // Example 3: Complex expression
    println!("🔢 Example 3: Complex Expression");
    let src3 = r#"
    reproducibility {
        x = 10
        y = 3
        z = x + y
        w = z x 2
        result = @w #7
    }
    "#;

    let result3 = ops_parser.parse(src3).await.map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Program:\n{}", src3.trim());
    println!("Result: x={}, y={}, z={}, w={}, result={} ({} % 7 = {})\n",
             result3["x"], result3["y"], result3["z"],
             result3["w"], result3["result"], result3["w"], result3["result"]);

    // Example 4: Parse-only demonstration
    println!("🔍 Example 4: AST Inspection");
    let assignments = ops_parser.parse(src2).await.map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("AST for the reference example:");
    for (key, value) in &assignments {
        println!("  {} = {:?}", key, value);
    }
    println!();

    println!("✨ Calculator DSL Features:");
    println!("  • Variable declarations: name = expression");
    println!("  • Arithmetic: + (add), - (subtract), x or * (multiply)");
    println!("  • Parentheses for grouping: (a + b) x c");
    println!("  • Reference with modifier: @var #n (currently implements modulo)");
    println!("  • Sequential evaluation in declaration order");
    println!();

    println!("🎯 Ready to use in Helix operators as 'calc' and 'eval'!");

    Ok(())
}
