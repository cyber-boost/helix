use helix::{parse, pretty_print};

fn main() {
    // Test the new parser features
    let input = r#"
    test_section {
        tags = ["test", "example"]
        name = "test"
        value = 42
        enabled = true
        disabled = null
        condition = "a == b" && c > 10
        calculation = x + y * 2
        comparison = p != null && q <= 5
    }

    agent "test_agent" {
        capabilities ["reasoning", "planning"]
        backstory {
            "This is a test agent"
            "It can handle complex expressions"
        }
        timeout = 30s
    }
    "#;

    match parse(input) {
        Ok(ast) => {
            println!("✅ Parser successfully parsed input!");
            println!("AST:\n{}", pretty_print(&ast));
        }
        Err(e) => {
            println!("❌ Parser failed: {:?}", e);
        }
    }
}
