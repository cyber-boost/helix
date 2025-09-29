use crate::dna::atp::lexer::tokenize;
fn main() {
    let input1 = "timeout = 30 m";
    println!("Testing: '{}'", input1);
    let tokens1 = tokenize(input1).unwrap();
    println!("Tokens: {:?}", tokens1);
    let input2 = "timeout = 30m";
    println!("Testing: '{}'", input2);
    let tokens2 = tokenize(input2).unwrap();
    println!("Tokens: {:?}", tokens2);
    let input3 = "section test { }";
    println!("Testing: '{}'", input3);
    let tokens3 = tokenize(input3).unwrap();
    println!("Tokens: {:?}", tokens3);
}