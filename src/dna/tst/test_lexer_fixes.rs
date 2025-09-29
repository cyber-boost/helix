use crate::dna::atp::lexer::tokenize;
fn main() {
    println!("Testing lexer fixes...");
    let input1 = "section test { }";
    println!("\nTest 1 - Section keyword: '{}'", input1);
    match tokenize(input1) {
        Ok(tokens) => {
            println!("✓ Token count: {}", tokens.len());
            for (i, token) in tokens.iter().enumerate() {
                println!("  [{}] {:?}", i, token);
            }
        }
        Err(e) => println!("✗ Error: {}", e),
    }
    let input2 = "timeout = 30 m";
    println!("\nTest 2 - Duration with space: '{}'", input2);
    match tokenize(input2) {
        Ok(tokens) => {
            println!("✓ Token count: {}", tokens.len());
            for (i, token) in tokens.iter().enumerate() {
                println!("  [{}] {:?}", i, token);
            }
        }
        Err(e) => println!("✗ Error: {}", e),
    }
    let input3 = "timeout = 30m";
    println!("\nTest 3 - Duration without space: '{}'", input3);
    match tokenize(input3) {
        Ok(tokens) => {
            println!("✓ Token count: {}", tokens.len());
            for (i, token) in tokens.iter().enumerate() {
                println!("  [{}] {:?}", i, token);
            }
        }
        Err(e) => println!("✗ Error: {}", e),
    }
    println!("\nAll tests completed!");
}