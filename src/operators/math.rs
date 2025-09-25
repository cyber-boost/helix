//! Calculator DSL Module
//!
//! This module provides a complete calculator that can parse and evaluate
//! a custom DSL with variables, arithmetic operations, and reference-with-modifier syntax.

use crate::error::HlxError;
use crate::operators::utils;
use crate::operators::eval::{run_program as eval_run_program, Env};
use crate::operators::OperatorTrait;
use crate::ops;
use crate::value::Value;
use async_trait::async_trait;
use pest::Parser;
use std::collections::HashMap;
use std::path::Path;

// AST definitions for the calculator DSL
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Integer literal (e.g., 42)
    Number(i64),
    /// Variable reference (e.g., a, b, c)
    Var(String),
    /// Multiplication (e.g., a x b)
    Mul(Box<Expr>, Box<Expr>),
    /// Addition (e.g., a + b)
    Add(Box<Expr>, Box<Expr>),
    /// Subtraction (e.g., a - b)
    Sub(Box<Expr>, Box<Expr>),
    /// Reference with optional modifier (e.g., @c or @c #4)
    Ref {
        /// Variable name to reference
        var: String,
        /// Optional modifier value (number after #)
        modifier: Option<i64>,
    },
}

/// A single assignment statement (e.g., `a = 2`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assign {
    /// Variable name being assigned
    pub name: String,
    /// Expression being assigned
    pub value: Expr,
}

// Note: Env type is imported from the eval module

/// Math operators implementation for the calculator DSL
pub struct MathOperators {
    calculator: Calculator,
}

impl MathOperators {
    pub async fn new() -> Result<Self, HlxError> {
        Ok(Self {
            calculator: Calculator::new(),
        })
    }
}

#[async_trait]
impl OperatorTrait for MathOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        match operator {
            "calc" => {
                let parsed_params = utils::parse_params(params)?;
                let source = parsed_params.get("source")
                    .ok_or_else(|| HlxError::invalid_input("Missing 'source' parameter", "Check the source parameter"))?
                    .to_string();
                
                let result = self.calculator.evaluate(&source)
                    .map_err(|e| HlxError::execution_error(format!("Calculator error: {}", e), "Check calculator syntax"))?;
                
                // Convert the environment to a Value
                let mut result_obj = HashMap::new();
                for (key, value) in result.env {
                    result_obj.insert(key, Value::Number(value as f64));
                }
                Ok(Value::Object(result_obj))
            }
            "eval" => {
                let parsed_params = utils::parse_params(params)?;
                let expression = parsed_params.get("expression")
                    .ok_or_else(|| HlxError::invalid_input("Missing 'expression' parameter", "Check the expression parameter"))?
                    .to_string();
                
                // Simple expression evaluation
                let result = self.calculator.evaluate(&format!("reproducibility {{ result = {} }}", expression))
                    .map_err(|e| HlxError::execution_error(format!("Evaluation error: {}", e), "Check expression syntax"))?;
                
                if let Some(value) = result.env.get("result") {
                    Ok(Value::Number(*value as f64))
                } else {
                    Ok(Value::Number(0.0))
                }
            }
            _ => Err(HlxError::invalid_input(format!("Unknown math operator: {}", operator), "Check the operator name"))
        }
    }
}

/// Calculator engine that parses and evaluates DSL programs
pub struct Calculator;

/// Result of evaluating a calculator program
pub struct CalcResult {
    /// Final variable environment
    pub env: Env,
}

impl Calculator {
    /// Create a new calculator instance
    pub fn new() -> Self {
        Self
    }

    /// Parse and evaluate a calculator DSL program
    ///
    /// # Example
    /// ```
    /// use helix::operators::math::Calculator;
    ///
    /// let calc = Calculator::new();
    /// let src = r#"
    /// reproducibility {
    ///     a = 2
    ///     b = 2
    ///     c = a x b
    ///     d = @c #4
    /// }
    /// "#;
    ///
    /// let result = calc.evaluate(src).unwrap();
    /// assert_eq!(result.env["a"], 2);
    /// assert_eq!(result.env["b"], 2);
    /// assert_eq!(result.env["c"], 4); // 2 * 2
    /// assert_eq!(result.env["d"], 0); // 4 % 4
    /// ```
    pub fn evaluate(&self, source: &str) -> anyhow::Result<CalcResult> {
        // Parse the program
        let assignments = parse_program(source)?;

        // Evaluate the assignments
        let env = eval_run_program(&assignments)?;

        Ok(CalcResult { env })
    }

    /// Parse a program and return the AST without evaluating
    pub fn parse_only(&self, source: &str) -> anyhow::Result<Vec<Assign>> {
        parse_program(source)
    }
}

/// Parse a calculator DSL program into a list of assignments
pub fn parse_program(source: &str) -> anyhow::Result<Vec<Assign>> {
    // Ensure the calc directory and ulator.pest file exist
    let _ = ops::ensure_calc().map_err(|e| anyhow::anyhow!("Failed to ensure calc directory: {}", e))?;
    
    let mut pairs = CalcParser::parse(Rule::program, source)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    let program = pairs.next().ok_or_else(|| anyhow::anyhow!("Empty program"))?;
    let block = program.into_inner().next().ok_or_else(|| anyhow::anyhow!("No reproducibility block"))?;

    if block.as_rule() != Rule::reproducibility {
        return Err(anyhow::anyhow!("Expected reproducibility block"));
    }

    let mut assignments = Vec::new();
    for statement in block.into_inner() {
        if statement.as_rule() == Rule::statement {
            let mut parts = statement.into_inner();
            let identifier = parts.next().unwrap().as_str().to_string();
            let expr = parts.next().unwrap();
            let expression = parse_expr(expr)?;
            assignments.push(Assign {
                name: identifier,
                value: expression,
            });
        }
    }

    Ok(assignments)
}

fn parse_expr(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Expr> {
    match pair.as_rule() {
        Rule::signed_number => {
            let num_str = pair.as_str().replace("_", ""); // Remove underscores
            let num = num_str.parse::<i64>()?;
            Ok(Expr::Number(num))
        }
        Rule::identifier => Ok(Expr::Var(pair.as_str().to_string())),
        Rule::reference => {
            let mut parts = pair.into_inner();
            let var = parts.next().unwrap().as_str().to_string();
            let modifier = parts.next().map(|m| m.as_str().parse::<i64>()).transpose()?;
            Ok(Expr::Ref { var, modifier })
        }
        Rule::factor => parse_expr(pair.into_inner().next().unwrap()),
        Rule::term => {
            let mut parts: Vec<_> = pair.into_inner().collect();

            if parts.is_empty() {
                return Err(anyhow::anyhow!("Empty term"));
            }

            let mut result = parse_expr(parts.remove(0))?;

            while parts.len() >= 2 {
                let op = parts.remove(0);
                let right = parse_expr(parts.remove(0))?;
                match op.as_str() {
                    "x" | "*" => result = Expr::Mul(Box::new(result), Box::new(right)),
                    _ => return Err(anyhow::anyhow!("Unknown term operator: {}", op.as_str())),
                }
            }
            Ok(result)
        }
        Rule::expr => {
            let mut parts: Vec<_> = pair.into_inner().collect();

            if parts.is_empty() {
                return Err(anyhow::anyhow!("Empty expression"));
            }

            let mut result = parse_expr(parts.remove(0))?;

            while parts.len() >= 2 {
                let op = parts.remove(0);
                let right = parse_expr(parts.remove(0))?;
                match op.as_str() {
                    "+" => result = Expr::Add(Box::new(result), Box::new(right)),
                    "-" => result = Expr::Sub(Box::new(result), Box::new(right)),
                    _ => return Err(anyhow::anyhow!("Unknown expr operator: {}", op.as_str())),
                }
            }
            Ok(result)
        }
        _ => Err(anyhow::anyhow!("Unexpected rule: {:?}", pair.as_rule())),
    }
}

/// Evaluate an expression in the given environment
pub fn eval_expr(expr: &Expr, env: &Env) -> i64 {
    match expr {
        Expr::Number(n) => *n,
        Expr::Var(name) => env.get(name).copied().unwrap_or(0),
        Expr::Add(left, right) => eval_expr(left, env) + eval_expr(right, env),
        Expr::Sub(left, right) => eval_expr(left, env) - eval_expr(right, env),
        Expr::Mul(left, right) => eval_expr(left, env) * eval_expr(right, env),
        Expr::Ref { var, modifier } => {
            let value = env.get(var).copied().unwrap_or(0);
            match modifier {
                Some(mod_val) => value % mod_val,
                None => value,
            }
        }
    }
}

/// Run a program (list of assignments) and return the final environment
pub fn run_program(assignments: &[Assign]) -> anyhow::Result<Env> {
    let mut env = Env::new();
    
    for assignment in assignments {
        let value = eval_expr(&assignment.value, &env);
        env.insert(assignment.name.clone(), value);
    }
    
    Ok(env)
}

// Note: eval_expr and run_program are imported from the eval module

// Pest parser for the calculator DSL
// Note: The grammar file is dynamically created in ~/.dna/calc/ulator.pest
// by the ensure_calc() function in ops.rs
#[derive(pest_derive::Parser)]
#[grammar = "operators/ulator.pest"]
struct CalcParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        let calc = Calculator::new();
        let src = r#"
            reproducibility {
                a = 2
                b = 3
                c = a x b
            }
        "#;
        
        let result = calc.evaluate(src).unwrap();
        assert_eq!(result.env["a"], 2);
        assert_eq!(result.env["b"], 3);
        assert_eq!(result.env["c"], 6);
    }

    #[test]
    fn test_reference_with_modifier() {
        let calc = Calculator::new();
        let src = r#"
            reproducibility {
                a = 10
                b = 3
                c = a x b
                d = @c #4
            }
        "#;
        
        let result = calc.evaluate(src).unwrap();
        assert_eq!(result.env["a"], 10);
        assert_eq!(result.env["b"], 3);
        assert_eq!(result.env["c"], 30);
        assert_eq!(result.env["d"], 2); // 30 % 4 = 2
    }

    #[test]
    fn test_complex_expression() {
        let calc = Calculator::new();
        let src = r#"
            reproducibility {
                x = 5
                y = 3
                z = (x + y) x (x - y)
            }
        "#;
        
        let result = calc.evaluate(src).unwrap();
        assert_eq!(result.env["x"], 5);
        assert_eq!(result.env["y"], 3);
        assert_eq!(result.env["z"], 16); // (5+3) * (5-3) = 8 * 2 = 16
    }
}