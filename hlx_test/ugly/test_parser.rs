use hlx::lexer::Lexer;
use hlx::parser::Parser;
use std::collections::HashMap;

fn main() {
    // Set up some test environment variables
    std::env::set_var("ENV_MODE", "production");
    std::env::set_var("DEPLOYMENT_ENV", "staging");
    std::env::set_var("REDIS_CONFIG", "redis://localhost:6379");
    std::env::set_var("base_dir", "/opt/app");

    // Read the test file
    let content = std::fs::read_to_string("test_variable_markers.hlx")
        .expect("Failed to read test file");

    // Tokenize
    let mut lexer = Lexer::new(&content);
    let tokens = lexer.tokenize().expect("Failed to tokenize");

    // Parse
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Failed to parse");

    println!("Parsed AST successfully!");
    println!("Number of declarations: {}", ast.declarations.len());

    // Print each declaration
    for (i, decl) in ast.declarations.iter().enumerate() {
        println!("Declaration {}: {:?}", i, decl);
    }
}
