//! Calculator DSL Module
//!
//! This module provides a complete calculator that can parse and evaluate
//! a custom DSL with variables, arithmetic operations, and reference-with-modifier syntax.

use crate::dna::hel::error::HlxError;
use crate::ops::eval::{run_program as eval_run_program, Env};
// use crate::ops::parse_program as parse_calculator_program;
// use crate::dna::atp::ops::{Rule, CalcParser};
use crate::dna::atp::value::Value;
use anyhow;
use std::collections::HashMap;
use async_trait::async_trait;

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

    pub async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        self.execute_impl(operator, params).await
    }

    async fn execute_impl(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        match operator {
            "calc" => {
                let parsed_params = crate::ops::utils::parse_params(params)?;
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
                let parsed_params = crate::ops::utils::parse_params(params)?;
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

#[async_trait]
impl crate::ops::OperatorTrait for MathOperators {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        self.execute_impl(operator, params).await
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
    // Simple manual parser for the calculator DSL
    let mut assignments = Vec::new();

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if let Some((name, expr_str)) = line.split_once('=') {
            let name = name.trim().to_string();
            let expr_str = expr_str.trim();

            let expr = parse_expression(expr_str)?;
            assignments.push(Assign {
                name,
                value: expr,
            });
        }
    }

    Ok(assignments)
}

/// Parse a mathematical expression
fn parse_expression(expr_str: &str) -> anyhow::Result<Expr> {
    parse_add_sub(expr_str)
}

/// Parse addition and subtraction (lowest precedence)
fn parse_add_sub(expr_str: &str) -> anyhow::Result<Expr> {
    let mut result = parse_mul_div(expr_str)?;

    let expr_chars: Vec<char> = expr_str.chars().collect();
    let mut i = 0;

    while i < expr_chars.len() {
        match expr_chars[i] {
            '+' => {
                i += 1;
                let right = parse_mul_div(&expr_str[i..])?;
                result = Expr::Add(Box::new(result), Box::new(right));
            }
            '-' => {
                i += 1;
                let right = parse_mul_div(&expr_str[i..])?;
                result = Expr::Sub(Box::new(result), Box::new(right));
            }
            _ => break,
        }
    }

    Ok(result)
}

/// Parse multiplication and division (higher precedence)
fn parse_mul_div(expr_str: &str) -> anyhow::Result<Expr> {
    let mut result = parse_factor(expr_str)?;

    let expr_chars: Vec<char> = expr_str.chars().collect();
    let mut i = 0;

    while i < expr_chars.len() {
        match expr_chars[i] {
            'x' | '*' => {
                i += 1;
                let right = parse_factor(&expr_str[i..])?;
                result = Expr::Mul(Box::new(result), Box::new(right));
            }
            _ => break,
        }
    }

    Ok(result)
}

/// Parse factors (highest precedence)
fn parse_factor(expr_str: &str) -> anyhow::Result<Expr> {
    let expr_str = expr_str.trim();

    // Handle parentheses
    if expr_str.starts_with('(') && expr_str.ends_with(')') {
        let inner = &expr_str[1..expr_str.len()-1];
        return parse_add_sub(inner);
    }

    // Handle references (@variable #modifier)
    if expr_str.starts_with('@') {
        if let Some(hash_pos) = expr_str.find('#') {
            let var = expr_str[1..hash_pos].to_string();
            let modifier_str = &expr_str[hash_pos+1..];
            let modifier = modifier_str.parse::<i64>()?;
            return Ok(Expr::Ref {
                var,
                modifier: Some(modifier),
            });
        } else {
            let var = expr_str[1..].to_string();
            return Ok(Expr::Ref {
                var,
                modifier: None,
            });
        }
    }

    // Handle numbers
    if let Ok(num) = expr_str.parse::<i64>() {
        return Ok(Expr::Number(num));
    }

    // Handle variables
    Ok(Expr::Var(expr_str.to_string()))
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

// CalcParser is now defined in atp/ops.rs

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