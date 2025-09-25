//! Parser for the Calculator DSL using Pest
//!
//! This module transforms the Pest parse tree into our AST.

use pest::Parser;
use pest::iterators::{Pair, Pairs};
use anyhow::{anyhow, Result};

use crate::operators::ast::Expr;

/// Pest parser for the calculator DSL
#[derive(pest_derive::Parser)]
#[grammar = "src/operators/calculator.pest"]
pub struct CalcParser;

/// Transform a `Pair` produced by Pest into our `Expr` enum.
fn parse_expr(pair: Pair<Rule>) -> Result<Expr> {
    match pair.as_rule() {
        Rule::number => {
            let n: i64 = pair.as_str().parse()?;
            Ok(Expr::Number(n))
        }
        Rule::identifier => Ok(Expr::Var(pair.as_str().to_string())),
        Rule::reference => {
            let mut inner = pair.into_inner(); // @ identifier # number
            let var = inner.next().unwrap().as_str().to_string(); // identifier
            let modifier = inner.next().unwrap().as_str().parse::<i64>()?;
            Ok(Expr::Ref { var, modifier })
        }
        Rule::factor => parse_expr(pair.into_inner().next().unwrap()),
        Rule::term => {
            // term = factor ( (x|*) factor )*
            let mut inner = pair.into_inner();
            let first = parse_expr(inner.next().unwrap())?;
            let mut acc = first;
            while let Some(op) = inner.next() {
                let right = parse_expr(inner.next().unwrap())?;
                // only multiplication is allowed inside a term
                match op.as_str() {
                    "x" | "*" => acc = Expr::Mul(Box::new(acc), Box::new(right)),
                    _ => unreachable!(),
                }
            }
            Ok(acc)
        }
        Rule::expr => {
            // expr = term ( (+|-) term )*
            let mut inner = pair.into_inner();
            let first = parse_expr(inner.next().unwrap())?;
            let mut acc = first;
            while let Some(op) = inner.next() {
                let right = parse_expr(inner.next().unwrap())?;
                acc = match op.as_str() {
                    "+" => Expr::Add(Box::new(acc), Box::new(right)),
                    "-" => Expr::Sub(Box::new(acc), Box::new(right)),
                    _ => unreachable!(),
                };
            }
            Ok(acc)
        }
        _ => Err(anyhow!("unexpected rule: {:?}", pair.as_rule())),
    }
}

/// Parse the whole program and return a vector of assignments.
pub fn parse_program(source: &str) -> Result<Vec<crate::operators::ast::Assign>> {
    let mut pairs = CalcParser::parse(Rule::program, source)?
        .next()
        .unwrap()
        .into_inner(); // inside `program`

    // Expect the `reproducibility` block
    let block = pairs.next().ok_or_else(|| anyhow!("missing block"))?;
    if block.as_rule() != Rule::reproducibility {
        return Err(anyhow!("expected reproducibility block"));
    }

    let mut assigns = Vec::new();
    for stmt in block.into_inner() {
        // stmt = identifier "=" expr
        let mut inner = stmt.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let expr_pair = inner.next().unwrap(); // the expression
        let expr = parse_expr(expr_pair)?;
        assigns.push(crate::operators::ast::Assign { name, value: expr });
    }
    Ok(assigns)
}
