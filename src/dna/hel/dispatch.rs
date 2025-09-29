use crate::dna::atp::lexer::{tokenize_with_locations, SourceMap};
use crate::dna::atp::parser::{Parser, ParseError};
use crate::dna::atp::ast::HelixAst;
use crate::atp::types::Value;
use crate::dna::atp::interpreter::HelixInterpreter;
use crate::hel::error::HlxError;
#[derive(Debug)]
pub enum DispatchResult {
    Parsed(HelixAst),
    Executed(Value),
    #[cfg(feature = "js")]
    ParseError(napi::Error<ParseError>),
    #[cfg(not(feature = "js"))]
    ParseError(ParseError),
    ExecutionError(HlxError),
}
pub struct HelixDispatcher {
    interpreter: Option<HelixInterpreter>,
}
impl HelixDispatcher {
    pub fn new() -> Self {
        Self { interpreter: None }
    }
    pub async fn initialize(&mut self) -> Result<(), HlxError> {
        self.interpreter = Some(HelixInterpreter::new().await?);
        Ok(())
    }
    #[cfg(feature = "js")]
    pub fn parse_only(&self, source: &str) -> Result<HelixAst, napi::Error<ParseError>> {
        crate::parse(source).map_err(|e| e.into())
    }
    #[cfg(not(feature = "js"))]
    pub fn parse_only(&self, source: &str) -> Result<HelixAst, ParseError> {
        crate::parse(source)
    }
    pub async fn parse_and_execute(
        &mut self,
        source: &str,
    ) -> Result<DispatchResult, HlxError> {
        if self.interpreter.is_none() {
            self.initialize().await?;
        }
        let ast = match crate::parse(source) {
            Ok(ast) => ast,
            Err(parse_err) => return Ok(DispatchResult::ParseError(parse_err)),
        };
        if let Some(ref mut interpreter) = self.interpreter {
            match interpreter.execute_ast(&ast).await {
                Ok(value) => Ok(DispatchResult::Executed(value)),
                Err(exec_err) => Ok(DispatchResult::ExecutionError(exec_err)),
            }
        } else {
            Err(
                HlxError::execution_error(
                    "Interpreter not initialized",
                    "Call initialize() before executing code",
                ),
            )
        }
    }
    pub async fn parse_dsl(&mut self, source: &str) -> Result<DispatchResult, HlxError> {
        let tokens_with_loc = tokenize_with_locations(source)
            .map_err(|e| {
                HlxError::invalid_input(
                    format!("Lexer error: {}", e),
                    "Check syntax and try again",
                )
            })?;
        let source_map = SourceMap {
            tokens: tokens_with_loc.clone(),
            source: source.to_string(),
        };
        let mut parser = Parser::new_with_source_map(source_map);
        match parser.parse() {
            Ok(ast) => Ok(DispatchResult::Parsed(ast)),
            Err(parse_err) => {
                let parse_error = ParseError {
                    message: parse_err,
                    location: None,
                    token_index: 0,
                    expected: None,
                    found: String::new(),
                    context: String::new(),
                };
                Ok(DispatchResult::ParseError(parse_error.into()))
            }
        }
    }
    pub fn interpreter(&self) -> Option<&HelixInterpreter> {
        self.interpreter.as_ref()
    }
    pub fn interpreter_mut(&mut self) -> Option<&mut HelixInterpreter> {
        self.interpreter.as_mut()
    }
    pub fn is_ready(&self) -> bool {
        self.interpreter.is_some()
    }
}
#[cfg(feature = "js")]
pub fn parse_helix(source: &str) -> Result<HelixAst, napi::Error<ParseError>> {
    crate::parse(source).map_err(|e| e.into())
}
#[cfg(not(feature = "js"))]
pub fn parse_helix(source: &str) -> Result<HelixAst, ParseError> {
    crate::parse(source)
}
#[cfg(feature = "js")]
pub fn parse_and_validate(
    source: &str,
) -> Result<crate::atp::types::HelixConfig, napi::Error<String>> {
    crate::parse_and_validate(source).map_err(|e| e.into())
}
#[cfg(not(feature = "js"))]
pub fn parse_and_validate(source: &str) -> Result<crate::atp::types::HelixConfig, String> {
    crate::parse_and_validate(source)
}
pub async fn execute_helix(source: &str) -> Result<Value, HlxError> {
    let mut dispatcher = HelixDispatcher::new();
    dispatcher.initialize().await?;
    match dispatcher.parse_and_execute(source).await? {
        DispatchResult::Executed(value) => Ok(value),
        DispatchResult::ParseError(err) => {
            Err(
                HlxError::invalid_input(
                    format!("Parse error: {}", err),
                    "Check syntax and try again",
                ),
            )
        }
        DispatchResult::ExecutionError(err) => Err(err),
        DispatchResult::Parsed(_) => {
            Err(
                HlxError::execution_error(
                    "Parsed AST but no execution result",
                    "Use parse_helix for parsing only",
                ),
            )
        }
    }
}
#[cfg(feature = "js")]
pub async fn parse_helix_dsl(source: &str) -> Result<HelixAst, napi::Error<ParseError>> {
    let mut dispatcher = HelixDispatcher::new();
    match dispatcher.parse_dsl(source).await {
        Ok(DispatchResult::Parsed(ast)) => Ok(ast),
        Ok(DispatchResult::ParseError(err)) => Err(err),
        _ => {
            let parse_error = ParseError {
                message: "Unexpected dispatch result".to_string(),
                location: None,
                token_index: 0,
                expected: None,
                found: String::new(),
                context: String::new(),
            };
            Err(parse_error.into())
        }
    }
}
#[cfg(not(feature = "js"))]
pub async fn parse_helix_dsl(source: &str) -> Result<HelixAst, ParseError> {
    let mut dispatcher = HelixDispatcher::new();
    match dispatcher.parse_dsl(source).await {
        Ok(DispatchResult::Parsed(ast)) => Ok(ast),
        Ok(DispatchResult::ParseError(err)) => Err(err),
        _ => {
            let parse_error = ParseError {
                message: "Unexpected dispatch result".to_string(),
                location: None,
                token_index: 0,
                expected: None,
                found: String::new(),
                context: String::new(),
            };
            Err(parse_error)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_parse_only() {
        let source = r#"
        agent "test_agent" {
            capabilities ["coding", "analysis"]
            backstory {
                "A helpful AI assistant"
            }
        }
        "#;
        let dispatcher = HelixDispatcher::new();
        let result = dispatcher.parse_only(source);
        if let Err(ref e) = result {
            println!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn test_parse_and_execute() {
        let source = r#"
        @date("Y-m-d")
        "#;
        let mut dispatcher = HelixDispatcher::new();
        let result = dispatcher.parse_and_execute(source).await;
        assert!(result.is_ok());
        if let Ok(DispatchResult::Executed(value)) = result {
            match value {
                Value::String(date_str) => assert!(! date_str.is_empty()),
                _ => panic!("Expected string value"),
            }
        }
    }
    #[tokio::test]
    async fn test_execute_expression() {
        let source = "@date(\"Y-m-d\")";
        let mut dispatcher = HelixDispatcher::new();
        dispatcher.initialize().await.unwrap();
        let result = dispatcher.parse_and_execute(source).await;
        assert!(result.is_ok());
        if let Ok(DispatchResult::Executed(value)) = result {
            match value {
                Value::String(date_str) => assert!(! date_str.is_empty()),
                _ => panic!("Expected string value"),
            }
        }
    }
}