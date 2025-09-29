use crate::dna::hel::error::HlxError;
pub use crate::dna::atp::ops::OperatorParser;
use crate::dna::atp::value::Value;
use async_trait::async_trait;

pub mod conditional;
pub mod string_processing;
pub mod fundamental;
pub mod validation;
pub mod math;
pub mod eval;
pub mod utils;
pub mod engine;

pub use eval::{run_program, Env};

#[async_trait]
pub trait OperatorTrait: Send + Sync {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError>;
}


