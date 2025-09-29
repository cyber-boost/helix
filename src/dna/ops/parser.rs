// This file is deprecated - the real CalcParser is now in src/dna/atp/ops.rs
// Keeping this file for reference but it's no longer used

/*
use pest::Parser;
use pest::iterators::{Pair, Pairs};
use anyhow::{anyhow, Result};
use crate::ops::math::Expr;
#[derive(pest_derive::Parser)]
#[grammar = "ulator.pest"]
pub struct CalcParser;

// Re-export the Rule enum for use by other modules
pub use self::Rule;
pub fn parse_expr(pair: Pair<Rule>) -> Result<Expr> {
    match pair.as_rule() {
        Rule::number => {
            let n: i64 = pair.as_str().parse()?;
            Ok(Expr::Number(n))
        }
        Rule::identifier => Ok(Expr::Var(pair.as_str().to_string())),
        Rule::reference => {
            let mut inner = pair.into_inner();
            let var = inner.next().unwrap().as_str().to_string();
            let modifier = inner.next().unwrap().as_str().parse::<i64>()?;
            Ok(Expr::Ref { var, modifier })
        }
        Rule::factor => parse_expr(pair.into_inner().next().unwrap()),
        Rule::term => {
            let mut inner = pair.into_inner();
            let first = parse_expr(inner.next().unwrap())?;
            let mut acc = first;
            while let Some(op) = inner.next() {
                let right = parse_expr(inner.next().unwrap())?;
                match op.as_str() {
                    "x" | "*" => acc = Expr::Mul(Box::new(acc), Box::new(right)),
                    _ => unreachable!(),
                }
            }
            Ok(acc)
        }
        Rule::expr => {
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
pub fn parse_program(source: &str) -> Result<Vec<crate::ops::math::Assign>> {
    let mut pairs = CalcParser::parse(Rule::program, source)?
        .next()
        .unwrap()
        .into_inner();
    let block = pairs.next().ok_or_else(|| anyhow!("missing block"))?;
    if block.as_rule() != Rule::reproducibility {
        return Err(anyhow!("expected reproducibility block"));
    }
    let mut assigns = Vec::new();
    for stmt in block.into_inner() {
        let mut inner = stmt.into_inner();
        let name = inner.next().unwrap().as_str().to_string();
        let expr_pair = inner.next().unwrap();
        let expr = parse_expr(expr_pair)?;
        assigns
            .push(crate::ops::math::Assign {
                name,
                value: expr,
            });
    }
    Ok(assigns)
}
*/