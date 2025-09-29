use crate::dna::hel::error::HlxError;
use crate::ops::conditional::ConditionalOperators;
use crate::ops::string_processing::StringOperators;
use crate::ops::fundamental::OperatorRegistry;
use crate::ops::validation::ValidationOperators;
use crate::ops::math::MathOperators;
use crate::dna::atp::value::Value;

pub struct OperatorEngine {
    conditional_operators: ConditionalOperators,
    string_operators: StringOperators,
    operator_registry: OperatorRegistry,
    validation_operators: ValidationOperators,
    math_operators: MathOperators,
}
impl OperatorEngine {
    pub async fn new() -> Result<Self, HlxError> {
        Ok(Self {
            conditional_operators: ConditionalOperators::new().await?,
            string_operators: StringOperators::new().await?,
            operator_registry: OperatorRegistry::new().await?,
            validation_operators: ValidationOperators::new().await?,
            math_operators: MathOperators::new().await?,
        })
    }
    pub async fn execute_operator(
        &self,
        operator: &str,
        params: &str,
    ) -> Result<Value, HlxError> {
        if operator.starts_with('@') {
            return self.operator_registry.execute(operator, params).await;
        }
        match operator {
            "var" => self.operator_registry.execute("variable", params).await,
            "date" => self.operator_registry.execute("date", params).await,
            "file" => self.operator_registry.execute("file", params).await,
            "json" => self.operator_registry.execute("json", params).await,
            "query" => self.operator_registry.execute("query", params).await,
            "base64" => self.operator_registry.execute("base64", params).await,
            "uuid" => self.operator_registry.execute("uuid", params).await,
            "if" => self.conditional_operators.execute("if", params).await,
            "switch" => self.conditional_operators.execute("switch", params).await,
            "loop" => self.conditional_operators.execute("loop", params).await,
            "filter" => self.conditional_operators.execute("filter", params).await,
            "map" => self.conditional_operators.execute("map", params).await,
            "reduce" => self.conditional_operators.execute("reduce", params).await,
            "concat" => self.string_operators.execute("concat", params).await,
            "split" => self.string_operators.execute("split", params).await,
            "replace" => self.string_operators.execute("replace", params).await,
            "trim" => self.string_operators.execute("trim", params).await,
            "upper" => self.string_operators.execute("upper", params).await,
            "lower" => self.string_operators.execute("lower", params).await,
            "hash" => self.string_operators.execute("hash", params).await,
            "format" => self.string_operators.execute("format", params).await,
            "calc" => self.math_operators.execute("calc", params).await,
            "eval" => self.math_operators.execute("eval", params).await,
            _ => Err(HlxError::unknown_operator(operator)),
        }
    }
    pub fn operator_registry(&self) -> &OperatorRegistry {
        &self.operator_registry
    }
    pub fn get_variable(&self, name: &str) -> Result<Value, HlxError> {
        self.operator_registry.get_variable(name)
    }
}