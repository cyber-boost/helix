//! Evaluator for the Calculator DSL
//!
//! This module executes the AST against a mutable environment.

use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::ops::math::{Expr, Assign};

/// Environment mapping variable names to integer values.
pub type Env = HashMap<String, i64>;

/// Recursively evaluate an expression inside `env`.
pub fn eval_expr(expr: &Expr, env: &mut Env) -> Result<i64> {
    match expr {
        Expr::Number(n) => Ok(*n),

        Expr::Var(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("variable `{}` not defined", name)),

        Expr::Mul(l, r) => Ok(eval_expr(l, env)? * eval_expr(r, env)?),

        Expr::Add(l, r) => Ok(eval_expr(l, env)? + eval_expr(r, env)?),

        Expr::Sub(l, r) => Ok(eval_expr(l, env)? - eval_expr(r, env)?),

        Expr::Ref { var, modifier } => {
            let val = env
                .get(var)
                .cloned()
                .ok_or_else(|| anyhow!("variable `{}` not defined", var))?;
            // For this demo we interpret `#n` as **modulo**.
            // Change the implementation to whatever you need (e.g. scaling).
            match modifier {
                Some(mod_val) => Ok(val % mod_val),
                None => Ok(val),
            }
        }
    }
}

/// Execute a list of assignments. The environment is mutated in‑place and
/// also returned for convenience.
pub fn run_program(assignments: &[Assign]) -> Result<Env> {
    let mut env = Env::new();

    for assign in assignments {
        let value = eval_expr(&assign.value, &mut env)?;
        env.insert(assign.name.clone(), value);
    }

    Ok(env)
}
