use crate::lexer::{tokenize_with_locations, SourceMap};
use crate::parser::{Parser, ParseError};
use crate::ast::HelixAst;
use crate::interpreter::HelixInterpreter;
use crate::value::Value;
use crate::error::HlxError;

/// DispatchResult represents the outcome of parsing and executing Helix code
#[derive(Debug)]
pub enum DispatchResult {
    /// Successfully parsed AST without execution
    Parsed(HelixAst),
    /// Successfully executed and returned a value
    Executed(Value),
    /// Parse error occurred
    #[cfg(feature = "js")]
    ParseError(napi::Error<ParseError>),
    #[cfg(not(feature = "js"))]
    ParseError(ParseError),
    /// Execution error occurred
    ExecutionError(HlxError),
}

/// HelixDispatcher coordinates parsing, semantic analysis, and execution
pub struct HelixDispatcher {
    interpreter: Option<HelixInterpreter>,
}

impl HelixDispatcher {
    /// Create a new dispatcher
    pub fn new() -> Self {
        Self {
            interpreter: None,
        }
    }

    /// Initialize the interpreter (async operation)
    pub async fn initialize(&mut self) -> Result<(), HlxError> {
        self.interpreter = Some(HelixInterpreter::new().await?);
        Ok(())
    }

    /// Parse source code into AST only
    #[cfg(feature = "js")]
    pub fn parse_only(&self, source: &str) -> Result<HelixAst, napi::Error<ParseError>> {
        crate::parse(source).map_err(|e| e.into())
    }

    /// Parse source code into AST only
    #[cfg(not(feature = "js"))]
    pub fn parse_only(&self, source: &str) -> Result<HelixAst, ParseError> {
        crate::parse(source)
    }

    /// Parse and execute source code
    pub async fn parse_and_execute(&mut self, source: &str) -> Result<DispatchResult, HlxError> {
        // Ensure interpreter is initialized
        if self.interpreter.is_none() {
            self.initialize().await?;
        }

        // First try to parse
        let ast = match crate::parse(source) {
            Ok(ast) => ast,
            Err(parse_err) => return Ok(DispatchResult::ParseError(parse_err.into())),
        };

        // Then execute
        if let Some(ref mut interpreter) = self.interpreter {
            match interpreter.execute_ast(&ast).await {
                Ok(value) => Ok(DispatchResult::Executed(value)),
                Err(exec_err) => Ok(DispatchResult::ExecutionError(exec_err)),
            }
        } else {
            Err(HlxError::execution_error(
                "Interpreter not initialized",
                "Call initialize() before executing code"
            ))
        }
    }

    /// Parse Helix DSL source code with enhanced operator support
    pub async fn parse_dsl(&mut self, source: &str) -> Result<DispatchResult, HlxError> {
        // Use enhanced parsing with source map for better error reporting
        let tokens_with_loc = tokenize_with_locations(source).map_err(|e| {
            HlxError::invalid_input(
                format!("Lexer error: {}", e),
                "Check syntax and try again"
            )
        })?;

        let source_map = SourceMap {
            tokens: tokens_with_loc.clone(),
            source: source.to_string(),
        };

        let mut parser = Parser::new_with_source_map(source_map);

        match parser.parse() {
            Ok(ast) => {
                // For DSL parsing, we might want to execute certain parts
                // For now, just return the parsed AST
                Ok(DispatchResult::Parsed(ast))
            }
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
            },
        }
    }


    /// Get access to the underlying interpreter
    pub fn interpreter(&self) -> Option<&HelixInterpreter> {
        self.interpreter.as_ref()
    }

    /// Get mutable access to the underlying interpreter
    pub fn interpreter_mut(&mut self) -> Option<&mut HelixInterpreter> {
        self.interpreter.as_mut()
    }

    /// Check if dispatcher is ready for execution
    pub fn is_ready(&self) -> bool {
        self.interpreter.is_some()
    }
}

/// Convenience functions for one-off operations

/// Parse source code into AST
#[cfg(feature = "js")]
pub fn parse_helix(source: &str) -> Result<HelixAst, napi::Error<ParseError>> {
    crate::parse(source).map_err(|e| e.into())
}

/// Parse source code into AST
#[cfg(not(feature = "js"))]
pub fn parse_helix(source: &str) -> Result<HelixAst, ParseError> {
    crate::parse(source)
}

/// Parse and validate Helix configuration
#[cfg(feature = "js")]
pub fn parse_and_validate(source: &str) -> Result<crate::types::HelixConfig, napi::Error<crate::NapiStringError>> {
    crate::parse_and_validate(source).map_err(|e| e.into())
}

/// Parse and validate Helix configuration
#[cfg(not(feature = "js"))]
pub fn parse_and_validate(source: &str) -> Result<crate::types::HelixConfig, String> {
    crate::parse_and_validate(source)
}

/// Execute Helix code with full operator support
pub async fn execute_helix(source: &str) -> Result<Value, HlxError> {
    let mut dispatcher = HelixDispatcher::new();
    dispatcher.initialize().await?;

    match dispatcher.parse_and_execute(source).await? {
        DispatchResult::Executed(value) => Ok(value),
        DispatchResult::ParseError(err) => Err(HlxError::invalid_input(
            format!("Parse error: {}", err),
            "Check syntax and try again"
        )),
        DispatchResult::ExecutionError(err) => Err(err),
        DispatchResult::Parsed(_) => Err(HlxError::execution_error(
            "Parsed AST but no execution result",
            "Use parse_helix for parsing only"
        )),
    }
}

/// Parse Helix DSL with enhanced operator integration
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
        },
    }
}

/// Parse Helix DSL with enhanced operator integration
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
        },
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
            // Should return a date string
            match value {
                Value::String(date_str) => assert!(!date_str.is_empty()),
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
            // Should return a date string
            match value {
                Value::String(date_str) => assert!(!date_str.is_empty()),
                _ => panic!("Expected string value"),
            }
        }
    }
}
