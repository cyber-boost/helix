use hlx::lexer::tokenize;

fn main() {
    // Test duration with space (should work according to diagnostic)
    let input1 = "timeout = 30 m";
    println!("Testing: '{}'", input1);
    let tokens1 = tokenize(input1).unwrap();
    println!("Tokens: {:?}", tokens1);
    
    // Test duration without space (current working case)
    let input2 = "timeout = 30m";
    println!("Testing: '{}'", input2);
    let tokens2 = tokenize(input2).unwrap();
    println!("Tokens: {:?}", tokens2);
    
    // Test section keyword
    let input3 = "section test { }";
    println!("Testing: '{}'", input3);
    let tokens3 = tokenize(input3).unwrap();
    println!("Tokens: {:?}", tokens3);
}
