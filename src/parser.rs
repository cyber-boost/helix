use super::lexer::{
    Token, Keyword, TimeUnit, TokenWithLocation, SourceLocation, SourceMap,
};
use super::ast::*;
use crate::types::Duration;
use crate::operators::OperatorEngine;
use crate::error::HlxError;
use std::collections::HashMap;
use regex;
#[cfg(feature = "js")]
use napi;
pub struct Parser {
    tokens: Vec<TokenWithLocation>,
    source_map: Option<SourceMap>,
    current: usize,
    errors: Vec<ParseError>,
    recovery_points: Vec<usize>,
    operator_engine: Option<OperatorEngine>,
}
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub location: Option<SourceLocation>,
    pub token_index: usize,
    pub expected: Option<String>,
    pub found: String,
    pub context: String,
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(expected) = &self.expected {
            write!(f, " (expected: {}, found: {})", expected, self.found)?;
        }
        if !self.context.is_empty() {
            write!(f, " in {}", self.context)?;
        }
        Ok(())
    }
}
impl std::error::Error for ParseError {}
impl AsRef<str> for ParseError {
    fn as_ref(&self) -> &str {
        &self.message
    }
}

#[cfg(feature = "js")]
impl From<ParseError> for napi::Error<ParseError> {
    fn from(err: ParseError) -> Self {
        // Create a typed NAPI error using the ParseError
        napi::Error::new(err, napi::Status::GenericFailure)
    }
}


impl ParseError {
    pub fn format_with_source(&self, source: &str, tokens: &[Token]) -> String {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in source.chars().enumerate() {
            if i >= self.token_index {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        let lines: Vec<&str> = source.lines().collect();
        let error_line = if line > 0 && line <= lines.len() {
            lines[line - 1]
        } else {
            ""
        };
        let mut result = format!("Error at line {}, column {}:\n", line, col);
        result.push_str(&format!("    {}\n", error_line));
        result.push_str(&format!("    {}^\n", " ".repeat(col - 1)));
        let token_info = if self.token_index < tokens.len() {
            format!("{:?}", tokens[self.token_index])
        } else {
            "<EOF>".to_string()
        };
        if let Some(expected) = &self.expected {
            result
                .push_str(
                    &format!("Expected {}, found token {}\n", expected, token_info),
                );
        } else {
            result.push_str(&format!("{}\n", self.message));
        }
        if !self.context.is_empty() {
            result.push_str(&format!("Context: {}\n", self.context));
        }
        result
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
enum Precedence {
    Lowest = 0,
    Pipeline = 1,
    Logical = 2,
    Equality = 3,
    Comparison = 4,
    Addition = 5,
    Multiplication = 6,
    Unary = 7,
    Call = 8,
    Index = 9,
    Highest = 10,
}
impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let tokens_with_location = tokens
            .into_iter()
            .enumerate()
            .map(|(i, token)| {
                TokenWithLocation {
                    token,
                    location: SourceLocation {
                        line: 1,
                        column: i + 1,
                        position: i,
                    },
                }
            })
            .collect();
        Parser {
            tokens: tokens_with_location,
            source_map: None,
            current: 0,
            errors: Vec::new(),
            recovery_points: Vec::new(),
            operator_engine: None,
        }
    }
    pub fn new_enhanced(tokens: Vec<TokenWithLocation>) -> Self {
        Parser {
            tokens,
            source_map: None,
            current: 0,
            errors: Vec::new(),
            recovery_points: Vec::new(),
            operator_engine: None,
        }
    }
    pub fn new_with_source_map(source_map: SourceMap) -> Self {
        let tokens = source_map.tokens.clone();
        Parser {
            tokens,
            source_map: Some(source_map),
            current: 0,
            errors: Vec::new(),
            recovery_points: Vec::new(),
            operator_engine: None,
        }
    }
    fn add_error(&mut self, message: String, expected: Option<String>) {
        let error = ParseError {
            message,
            location: self.current_location(),
            token_index: self.current,
            expected,
            found: format!("{:?}", self.current_token()),
            context: self.get_enhanced_context(),
        };
        self.errors.push(error);
    }
    fn get_context(&self) -> String {
        if self.recovery_points.is_empty() {
            "top-level".to_string()
        } else {
            match self.recovery_points.last() {
                Some(_) => "inside declaration".to_string(),
                None => "unknown".to_string(),
            }
        }
    }
    fn get_enhanced_context(&self) -> String {
        let basic_context = self.get_context();
        if let (Some(source_map), Some(location)) = (
            &self.source_map,
            &self.current_location(),
        ) {
            let source_context = source_map.get_context(location, 2);
            format!("{} - Source context:\n{}", basic_context, source_context)
        } else {
            basic_context
        }
    }
    fn recover_to_next_declaration(&mut self) {
        while self.current_token() != &Token::Eof {
            match self.current_token() {
                Token::Keyword(k) => {
                    match k {
                        Keyword::Agent
                        | Keyword::Workflow
                        | Keyword::Memory
                        | Keyword::Context
                        | Keyword::Crew
                        | Keyword::Project
                        | Keyword::Pipeline
                        | Keyword::Load => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            self.advance();
        }
    }
    #[allow(dead_code)]
    fn recover_to_closing_brace(&mut self) {
        let mut brace_depth = 1;
        while self.current_token() != &Token::Eof && brace_depth > 0 {
            match self.current_token() {
                Token::LeftBrace => brace_depth += 1,
                Token::RightBrace => brace_depth -= 1,
                _ => {}
            }
            if brace_depth > 0 {
                self.advance();
            }
        }
    }
    fn current_token(&self) -> &Token {
        self.tokens
            .get(self.current)
            .map(|token_with_loc| &token_with_loc.token)
            .unwrap_or(&Token::Eof)
    }
    fn current_location(&self) -> Option<SourceLocation> {
        self.tokens
            .get(self.current)
            .map(|token_with_loc| token_with_loc.location.clone())
    }
    fn peek_token(&self) -> &Token {
        self.tokens
            .get(self.current + 1)
            .map(|token_with_loc| &token_with_loc.token)
            .unwrap_or(&Token::Eof)
    }
    fn parse_enhanced_expression(&mut self) -> Result<Expression, String> {
        let next_token = self.peek_token().clone();
        match (self.current_token().clone(), &next_token) {
            (Token::Identifier(name), Token::Assign) => {
                self.advance();
                Ok(Expression::Identifier(name))
            }
            (Token::LeftParen, _) => {
                self.advance();
                let expr = self.parse_enhanced_expression()?;
                self.expect(Token::RightParen)?;
                Ok(expr)
            }
            (Token::LeftBracket, _) => {
                self.advance();
                let mut elements = Vec::new();
                while self.current_token() != &Token::RightBracket
                    && self.current_token() != &Token::Eof
                {
                    self.skip_newlines();
                    if self.current_token() == &Token::RightBracket {
                        break;
                    }
                    elements.push(self.parse_enhanced_expression()?);
                    self.skip_newlines();
                    if self.current_token() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(Token::RightBracket)?;
                Ok(Expression::Array(elements))
            }
            _ => self.parse_primary_expression(),
        }
    }
    fn advance(&mut self) -> Token {
        let token = self.current_token().clone();
        if self.current < self.tokens.len() {
            self.current += 1;
        }
        token
    }
    fn expect(&mut self, expected: Token) -> Result<(), String> {
        let token = self.current_token().clone();
        if token == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, found {:?}", expected, token))
        }
    }
    fn expect_identifier(&mut self) -> Result<String, String> {
        match self.current_token() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            Token::String(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(format!("Expected identifier, found {:?}", self.current_token())),
        }
    }
    fn skip_newlines(&mut self) {
        while self.current_token() == &Token::Newline {
            self.advance();
        }
    }
    pub fn parse(&mut self) -> Result<HelixAst, String> {
        let mut ast = HelixAst::new();
        while self.current_token() != &Token::Eof {
            self.skip_newlines();
            let current_token = self.current_token().clone();
            match current_token {
                Token::Keyword(keyword) => {
                    self.recovery_points.push(self.current);
                    match self.parse_declaration(keyword.clone()) {
                        Ok(decl) => {
                            ast.add_declaration(decl);
                            self.recovery_points.pop();
                        }
                        Err(err) => {
                            self.add_error(
                                err.clone(),
                                Some(format!("valid {:?} declaration", keyword)),
                            );
                            self.recover_to_next_declaration();
                            self.recovery_points.pop();
                        }
                    }
                }
                Token::Identifier(section_name) => {
                    // Handle arbitrary identifiers as section declarations
                    self.recovery_points.push(self.current);
                    self.advance(); // consume the identifier
                    self.expect(Token::LeftBrace)?;
                    let properties = self.parse_properties()?;
                    self.expect(Token::RightBrace)?;
                    ast.add_declaration(Declaration::Section(SectionDecl {
                        name: section_name.clone(),
                        properties,
                    }));
                    self.recovery_points.pop();
                }
                Token::Eof => break,
                _ => {
                    self.add_error(
                        format!("Unexpected token: {:?}", current_token),
                        Some("declaration keyword or identifier".to_string()),
                    );
                    self.recover_to_next_declaration();
                }
            }
            self.skip_newlines();
        }
        if !self.errors.is_empty() {
            let error_summary = self
                .errors
                .iter()
                .map(|e| format!("{} at token {}", e.message, e.token_index))
                .collect::<Vec<_>>()
                .join("; ");
            Err(format!("Parse errors: {}", error_summary))
        } else {
            Ok(ast)
        }
    }
    fn parse_declaration(&mut self, keyword: Keyword) -> Result<Declaration, String> {
        match keyword {
            Keyword::Project => {
                self.advance();
                let name = self.expect_identifier()?;
                self.expect(Token::LeftBrace)?;
                let properties = self.parse_properties()?;
                self.expect(Token::RightBrace)?;
                Ok(Declaration::Project(ProjectDecl { name, properties }))
            }
            Keyword::Agent => {
                self.advance();
                let name = self.expect_identifier()?;
                self.expect(Token::LeftBrace)?;
                let mut properties = HashMap::new();
                let mut capabilities = None;
                let mut backstory = None;
                let tools = None;
                while self.current_token() != &Token::RightBrace {
                    self.skip_newlines();
                    match self.current_token() {
                        Token::Keyword(Keyword::Capabilities) => {
                            self.advance();
                            capabilities = Some(self.parse_string_array()?);
                        }
                        Token::Keyword(Keyword::Backstory) => {
                            self.advance();
                            backstory = Some(self.parse_backstory_block()?);
                        }
                        Token::Identifier(key) => {
                            let key = key.clone();
                            self.advance();
                            self.expect(Token::Assign)?;
                            let value = self.parse_expression()?;
                            properties.insert(key, value);
                        }
                        Token::Keyword(keyword) => {
                            match keyword {
                                Keyword::Capabilities | Keyword::Backstory => {
                                    return Err(
                                        format!(
                                            "Unexpected token in agent: {:?}", self.current_token()
                                        ),
                                    );
                                }
                                _ => {
                                    let key = format!("{:?}", keyword).to_lowercase();
                                    self.advance();
                                    self.expect(Token::Assign)?;
                                    let value = self.parse_expression()?;
                                    properties.insert(key, value);
                                }
                            }
                        }
                        Token::RightBrace => break,
                        _ => {
                            return Err(
                                format!(
                                    "Unexpected token in agent: {:?}", self.current_token()
                                ),
                            );
                        }
                    }
                    self.skip_newlines();
                }
                self.expect(Token::RightBrace)?;
                Ok(
                    Declaration::Agent(AgentDecl {
                        name,
                        properties,
                        capabilities,
                        backstory,
                        tools,
                    }),
                )
            }
            Keyword::Workflow => {
                self.advance();
                let name = self.expect_identifier()?;
                self.expect(Token::LeftBrace)?;
                let mut trigger = None;
                let mut steps = Vec::new();
                let mut pipeline = None;
                let mut properties = HashMap::new();
                while self.current_token() != &Token::RightBrace {
                    self.skip_newlines();
                    match self.current_token() {
                        Token::Keyword(Keyword::Trigger) => {
                            self.advance();
                            self.expect(Token::Assign)?;
                            trigger = Some(self.parse_trigger_config()?);
                        }
                        Token::Keyword(Keyword::Step) => {
                            steps.push(self.parse_step()?);
                        }
                        Token::Keyword(Keyword::Pipeline) => {
                            self.advance();
                            pipeline = Some(self.parse_pipeline_block()?);
                        }
                        Token::Keyword(Keyword::Timeout) => {
                            self.advance();
                            self.expect(Token::Assign)?;
                            let timeout_value = self.parse_expression()?;
                            properties.insert("timeout".to_string(), timeout_value);
                        }
                        Token::Identifier(key) => {
                            let key = key.clone();
                            self.advance();
                            self.expect(Token::Assign)?;
                            let value = self.parse_expression()?;
                            properties.insert(key, value);
                        }
                        Token::RightBrace => break,
                        _ => {
                            return Err(
                                format!(
                                    "Unexpected token in workflow: {:?}", self.current_token()
                                ),
                            );
                        }
                    }
                    self.skip_newlines();
                }
                self.expect(Token::RightBrace)?;
                Ok(
                    Declaration::Workflow(WorkflowDecl {
                        name,
                        trigger,
                        steps,
                        pipeline,
                        properties,
                    }),
                )
            }
            Keyword::Memory => {
                self.advance();
                self.expect(Token::LeftBrace)?;
                let mut provider = String::new();
                let mut connection = String::new();
                let mut embeddings = None;
                let mut properties = HashMap::new();
                while self.current_token() != &Token::RightBrace {
                    self.skip_newlines();
                    match self.current_token() {
                        Token::Keyword(Keyword::Embeddings) => {
                            self.advance();
                            embeddings = Some(self.parse_embeddings_block()?);
                        }
                        Token::Identifier(key) => {
                            let key = key.clone();
                            self.advance();
                            self.expect(Token::Assign)?;
                            let value = self.parse_expression()?;
                            match key.as_str() {
                                "provider" => {
                                    provider = value.as_string().unwrap_or_default();
                                }
                                "connection" => {
                                    connection = value.as_string().unwrap_or_default();
                                }
                                _ => {
                                    properties.insert(key, value);
                                }
                            }
                        }
                        Token::RightBrace => break,
                        _ => {
                            return Err(
                                format!(
                                    "Unexpected token in memory: {:?}", self.current_token()
                                ),
                            );
                        }
                    }
                    self.skip_newlines();
                }
                self.expect(Token::RightBrace)?;
                Ok(
                    Declaration::Memory(MemoryDecl {
                        provider,
                        connection,
                        embeddings,
                        properties,
                    }),
                )
            }
            Keyword::Context => {
                self.advance();
                let name = self.expect_identifier()?;
                self.expect(Token::LeftBrace)?;
                let mut environment = String::new();
                let mut secrets = None;
                let mut variables = None;
                let mut properties = HashMap::new();
                while self.current_token() != &Token::RightBrace {
                    self.skip_newlines();
                    match self.current_token() {
                        Token::Keyword(Keyword::Secrets) => {
                            self.advance();
                            secrets = Some(self.parse_secrets_block()?);
                        }
                        Token::Keyword(Keyword::Variables) => {
                            self.advance();
                            variables = Some(self.parse_variables_block()?);
                        }
                        Token::Identifier(key) => {
                            let key = key.clone();
                            self.advance();
                            self.expect(Token::Assign)?;
                            let value = self.parse_expression()?;
                            if key == "environment" {
                                environment = value.as_string().unwrap_or_default();
                            } else {
                                properties.insert(key, value);
                            }
                        }
                        Token::RightBrace => break,
                        _ => {
                            return Err(
                                format!(
                                    "Unexpected token in context: {:?}", self.current_token()
                                ),
                            );
                        }
                    }
                    self.skip_newlines();
                }
                self.expect(Token::RightBrace)?;
                Ok(
                    Declaration::Context(ContextDecl {
                        name,
                        environment,
                        secrets,
                        variables,
                        properties,
                    }),
                )
            }
            Keyword::Crew => {
                self.advance();
                let name = self.expect_identifier()?;
                self.expect(Token::LeftBrace)?;
                let mut agents = Vec::new();
                let mut process_type = None;
                let mut properties = HashMap::new();
                while self.current_token() != &Token::RightBrace {
                    self.skip_newlines();
                    match self.current_token() {
                        Token::Identifier(key) => {
                            let key = key.clone();
                            self.advance();
                            if key == "agents" {
                                agents = self.parse_string_array()?;
                            } else {
                                self.expect(Token::Assign)?;
                                let value = self.parse_expression()?;
                                if key == "process" {
                                    process_type = value.as_string();
                                } else {
                                    properties.insert(key, value);
                                }
                            }
                        }
                        Token::RightBrace => break,
                        _ => {
                            return Err(
                                format!(
                                    "Unexpected token in crew: {:?}", self.current_token()
                                ),
                            );
                        }
                    }
                    self.skip_newlines();
                }
                self.expect(Token::RightBrace)?;
                Ok(
                    Declaration::Crew(CrewDecl {
                        name,
                        agents,
                        process_type,
                        properties,
                    }),
                )
            }
            Keyword::Pipeline => {
                self.advance();
                self.expect(Token::LeftBrace)?;
                let pipeline = self.parse_pipeline_block()?;
                self.expect(Token::RightBrace)?;
                Ok(Declaration::Pipeline(pipeline))
            }
            Keyword::Load => {
                self.advance();
                let file_name = self.expect_identifier()?;
                self.expect(Token::LeftBrace)?;
                let properties = self.parse_properties()?;
                self.expect(Token::RightBrace)?;
                Ok(Declaration::Load(LoadDecl { file_name, properties }))
            }
            _ => Err(format!("Unexpected keyword: {:?}", keyword)),
        }
    }
    fn parse_step(&mut self) -> Result<StepDecl, String> {
        self.advance();
        let name = self.expect_identifier()?;
        self.expect(Token::LeftBrace)?;
        let mut agent = None;
        let mut crew = None;
        let mut task = None;
        let mut properties = HashMap::new();
        while self.current_token() != &Token::RightBrace {
            self.skip_newlines();
            match self.current_token() {
                Token::Keyword(Keyword::Timeout) => {
                    self.advance();
                    self.expect(Token::Assign)?;
                    let timeout_value = self.parse_expression()?;
                    properties.insert("timeout".to_string(), timeout_value);
                }
                Token::Identifier(key) => {
                    let key = key.clone();
                    self.advance();
                    match key.as_str() {
                        "agent" => {
                            self.expect(Token::Assign)?;
                            agent = self.parse_expression()?.as_string();
                        }
                        "crew" => {
                            self.expect(Token::Assign)?;
                            if self.current_token() == &Token::LeftBracket {
                                crew = Some(self.parse_string_array()?);
                            }
                        }
                        "task" => {
                            self.expect(Token::Assign)?;
                            task = self.parse_expression()?.as_string();
                        }
                        "retry" => {
                            let retry_config = self.parse_retry_block()?;
                            properties
                                .insert(
                                    "retry".to_string(),
                                    Expression::Object(retry_config),
                                );
                        }
                        _ => {
                            self.expect(Token::Assign)?;
                            let value = self.parse_expression()?;
                            properties.insert(key, value);
                        }
                    }
                }
                Token::Keyword(keyword) => {
                    let key = format!("{:?}", keyword).to_lowercase();
                    self.advance();
                    self.expect(Token::Assign)?;
                    match key.as_str() {
                        "agent" => {
                            agent = self.parse_expression()?.as_string();
                        }
                        _ => {
                            let value = self.parse_expression()?;
                            properties.insert(key, value);
                        }
                    }
                }
                Token::RightBrace => break,
                _ => {
                    return Err(
                        format!("Unexpected token in step: {:?}", self.current_token()),
                    );
                }
            }
            self.skip_newlines();
        }
        self.expect(Token::RightBrace)?;
        Ok(StepDecl {
            name,
            agent,
            crew,
            task,
            properties,
        })
    }
    fn parse_retry_block(&mut self) -> Result<HashMap<String, Expression>, String> {
        self.expect(Token::LeftBrace)?;
        let mut retry_config = HashMap::new();
        while self.current_token() != &Token::RightBrace {
            self.skip_newlines();
            if self.current_token() == &Token::RightBrace {
                break;
            }
            let key = self.expect_identifier()?;
            if self.peek_token() != &Token::Assign
                && self.current_token() != &Token::Assign
            {
                return Err(
                    format!(
                        "Expected '=' after property key '{}', found {:?}", key, self
                        .current_token()
                    ),
                );
            }
            self.expect(Token::Assign)?;
            let value = self.parse_expression()?;
            retry_config.insert(key, value);
            self.skip_newlines();
        }
        self.expect(Token::RightBrace)?;
        Ok(retry_config)
    }
    fn parse_trigger_config(&mut self) -> Result<Expression, String> {
        if self.current_token() == &Token::LeftBrace {
            self.advance();
            let trigger_obj = self.parse_object()?;
            Ok(Expression::Object(trigger_obj))
        } else {
            self.parse_expression()
        }
    }
    fn parse_expression(&mut self) -> Result<Expression, String> {
        match self.current_token() {
            Token::LeftParen | Token::LeftBracket => self.parse_enhanced_expression(),
            _ => self.parse_expression_with_precedence(Precedence::Lowest),
        }
    }
    fn parse_expression_with_precedence(
        &mut self,
        min_precedence: Precedence,
    ) -> Result<Expression, String> {
        let mut left = self.parse_primary_expression()?;
        while !self.is_at_end() {
            let precedence = self.get_token_precedence(self.current_token());
            if precedence < min_precedence {
                break;
            }
            match self.current_token() {
                Token::Arrow => {
                    self.advance();
                    let mut pipeline = vec![];
                    if let Expression::Identifier(id) = left {
                        pipeline.push(id);
                    } else if let Expression::Pipeline(mut p) = left {
                        pipeline.append(&mut p);
                    } else {
                        return Err(format!("Invalid left side of pipeline: {:?}", left));
                    }
                    let right = self
                        .parse_expression_with_precedence(Precedence::Pipeline)?;
                    if let Expression::Identifier(id) = right {
                        pipeline.push(id);
                    } else if let Expression::Pipeline(mut p) = right {
                        pipeline.append(&mut p);
                    } else {
                        return Err(
                            format!("Invalid right side of pipeline: {:?}", right),
                        );
                    }
                    left = Expression::Pipeline(pipeline);
                }
                _ => {
                    break;
                }
            }
        }
        Ok(left)
    }
    fn parse_primary_expression(&mut self) -> Result<Expression, String> {
        match self.current_token() {
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::String(s))
            }
            Token::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expression::Number(n))
            }
            Token::Bool(b) => {
                let b = *b;
                self.advance();
                Ok(Expression::Bool(b))
            }
            Token::Duration(value, unit) => {
                let duration = Duration {
                    value: *value,
                    unit: match unit {
                        TimeUnit::Seconds => crate::types::TimeUnit::Seconds,
                        TimeUnit::Minutes => crate::types::TimeUnit::Minutes,
                        TimeUnit::Hours => crate::types::TimeUnit::Hours,
                        TimeUnit::Days => crate::types::TimeUnit::Days,
                    },
                };
                self.advance();
                Ok(Expression::Duration(duration))
            }
            Token::Variable(v) => {
                let v = v.clone();
                self.advance();
                Ok(Expression::Variable(v))
            }
            Token::Reference(r) => {
                let r = r.clone();
                self.advance();
                if self.current_token() == &Token::LeftBracket {
                    self.advance();
                    let key = self.expect_identifier()?;
                    self.expect(Token::RightBracket)?;
                    Ok(Expression::IndexedReference(r, key))
                } else {
                    Ok(Expression::Reference(r))
                }
            }
            Token::Identifier(i) => {
                let i = i.clone();
                self.advance();
                Ok(Expression::Identifier(i))
            }
            Token::LeftBracket => {
                self.advance();
                let array = self.parse_array()?;
                Ok(Expression::Array(array))
            }
            Token::LeftBrace => {
                self.advance();
                let object = self.parse_object()?;
                Ok(Expression::Object(object))
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RightParen)?;
                Ok(expr)
            }
            _ => {
                Err(
                    format!("Unexpected token in expression: {:?}", self.current_token()),
                )
            }
        }
    }
    fn get_token_precedence(&self, token: &Token) -> Precedence {
        match token {
            Token::Arrow => Precedence::Pipeline,
            _ => Precedence::Lowest,
        }
    }
    fn is_at_end(&self) -> bool {
        self.current_token() == &Token::Eof
    }
    fn parse_array(&mut self) -> Result<Vec<Expression>, String> {
        let mut elements = Vec::new();
        while self.current_token() != &Token::RightBracket {
            self.skip_newlines();
            if self.current_token() == &Token::RightBracket {
                break;
            }
            elements.push(self.parse_expression()?);
            if self.current_token() == &Token::Comma {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(Token::RightBracket)?;
        Ok(elements)
    }
    fn parse_object(&mut self) -> Result<HashMap<String, Expression>, String> {
        let mut object = HashMap::new();
        while self.current_token() != &Token::RightBrace {
            self.skip_newlines();
            if self.current_token() == &Token::RightBrace {
                break;
            }
            let key = self.expect_identifier()?;
            if self.peek_token() != &Token::Assign
                && self.current_token() != &Token::Assign
            {
                return Err(
                    format!(
                        "Expected '=' after property key '{}', found {:?}", key, self
                        .current_token()
                    ),
                );
            }
            self.expect(Token::Assign)?;
            let value = self.parse_expression()?;
            object.insert(key, value);
            if self.current_token() == &Token::Comma {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(Token::RightBrace)?;
        Ok(object)
    }
    fn parse_string_array(&mut self) -> Result<Vec<String>, String> {
        self.expect(Token::LeftBracket)?;
        let mut items = Vec::new();
        while self.current_token() != &Token::RightBracket {
            self.skip_newlines();
            if self.current_token() == &Token::RightBracket {
                break;
            }
            items.push(self.expect_identifier()?);
            if self.current_token() == &Token::Comma {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(Token::RightBracket)?;
        Ok(items)
    }
    fn parse_properties(&mut self) -> Result<HashMap<String, Expression>, String> {
        let mut properties = HashMap::new();
        while self.current_token() != &Token::RightBrace {
            self.skip_newlines();
            if self.current_token() == &Token::RightBrace {
                break;
            }
            let key = self.expect_identifier()?;
            if self.peek_token() != &Token::Assign
                && self.current_token() != &Token::Assign
            {
                return Err(
                    format!(
                        "Expected '=' after property key '{}', found {:?}", key, self
                        .current_token()
                    ),
                );
            }
            self.expect(Token::Assign)?;
            let value = self.parse_expression()?;
            properties.insert(key, value);
            self.skip_newlines();
        }
        Ok(properties)
    }
    fn parse_backstory_block(&mut self) -> Result<BackstoryBlock, String> {
        self.expect(Token::LeftBrace)?;
        let mut lines = Vec::new();
        while self.current_token() != &Token::RightBrace {
            self.skip_newlines();
            match self.current_token() {
                Token::Identifier(text) | Token::String(text) => {
                    lines.push(text.clone());
                    self.advance();
                }
                Token::RightBrace => break,
                _ => {
                    self.advance();
                }
            }
        }
        self.expect(Token::RightBrace)?;
        Ok(BackstoryBlock { lines })
    }
    fn parse_pipeline_block(&mut self) -> Result<PipelineDecl, String> {
        self.expect(Token::LeftBrace)?;
        let mut flow = Vec::new();
        while self.current_token() != &Token::RightBrace {
            self.skip_newlines();
            if let Token::Identifier(step) = self.current_token() {
                flow.push(PipelineNode::Step(step.clone()));
                self.advance();
                if self.current_token() == &Token::Arrow {
                    self.advance();
                }
            } else if self.current_token() == &Token::RightBrace {
                break;
            } else {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(Token::RightBrace)?;
        Ok(PipelineDecl { flow })
    }
    fn parse_embeddings_block(&mut self) -> Result<EmbeddingsDecl, String> {
        self.expect(Token::LeftBrace)?;
        let mut model = String::new();
        let mut dimensions = 0;
        let mut properties = HashMap::new();
        while self.current_token() != &Token::RightBrace {
            self.skip_newlines();
            let key = self.expect_identifier()?;
            if self.peek_token() != &Token::Assign
                && self.current_token() != &Token::Assign
            {
                return Err(
                    format!(
                        "Expected '=' after property key '{}', found {:?}", key, self
                        .current_token()
                    ),
                );
            }
            self.expect(Token::Assign)?;
            let value = self.parse_expression()?;
            match key.as_str() {
                "model" => model = value.as_string().unwrap_or_default(),
                "dimensions" => dimensions = value.as_number().unwrap_or(0.0) as u32,
                _ => {
                    properties.insert(key, value);
                }
            }
            self.skip_newlines();
        }
        self.expect(Token::RightBrace)?;
        Ok(EmbeddingsDecl {
            model,
            dimensions,
            properties,
        })
    }
    fn parse_variables_block(&mut self) -> Result<HashMap<String, Expression>, String> {
        self.expect(Token::LeftBrace)?;
        let mut variables = HashMap::new();
        while self.current_token() != &Token::RightBrace {
            self.skip_newlines();
            if self.current_token() == &Token::RightBrace {
                break;
            }
            let key = match self.current_token().clone() {
                Token::Identifier(id) => {
                    self.advance();
                    id.clone()
                }
                Token::Keyword(kw) => {
                    self.advance();
                    format!("{:?}", kw).to_lowercase()
                }
                _ => {
                    return Err(
                        format!(
                            "Expected identifier or keyword for variable name, found {:?}",
                            self.current_token()
                        ),
                    );
                }
            };
            self.expect(Token::Assign)?;
            let value = self.parse_expression()?;
            variables.insert(key, value);
            self.skip_newlines();
        }
        self.expect(Token::RightBrace)?;
        Ok(variables)
    }
    fn parse_secrets_block(&mut self) -> Result<HashMap<String, SecretRef>, String> {
        self.expect(Token::LeftBrace)?;
        let mut secrets = HashMap::new();
        while self.current_token() != &Token::RightBrace {
            self.skip_newlines();
            let key = self.expect_identifier()?;
            self.expect(Token::Assign)?;
            let secret_ref = match self.current_token() {
                Token::Variable(var) => {
                    let var = var.clone();
                    self.advance();
                    SecretRef::Environment(var)
                }
                Token::String(path) if path.starts_with("vault:") => {
                    let path = path.clone();
                    self.advance();
                    SecretRef::Vault(path.trim_start_matches("vault:").to_string())
                }
                Token::String(path) if path.starts_with("file:") => {
                    let path = path.clone();
                    self.advance();
                    SecretRef::File(path.trim_start_matches("file:").to_string())
                }
                _ => {
                    return Err(
                        format!("Invalid secret reference: {:?}", self.current_token()),
                    );
                }
            };
            secrets.insert(key, secret_ref);
            self.skip_newlines();
        }
        self.expect(Token::RightBrace)?;
        Ok(secrets)
    }

    /// Execute an operator with the given parameters
    pub async fn execute_operator(&mut self, operator: &str, params: &str) -> Result<crate::value::Value, Box<dyn std::error::Error + Send + Sync>> {
        // Initialize operator engine if not already done
        if self.operator_engine.is_none() {
            self.operator_engine = Some(OperatorEngine::new().await.map_err(|e| {
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?);
        }

        if let Some(ref engine) = self.operator_engine {
            match engine.execute_operator(operator, params).await {
                Ok(value) => Ok(value),
                Err(e) => {
                    eprintln!("Operator execution error: {:?}", e);
                    Ok(crate::value::Value::String(format!("@{}({})", operator, params)))
                }
            }
        } else {
            Ok(crate::value::Value::String(format!("@{}({})", operator, params)))
        }
    }

    /// Convert expression parameters to JSON string for operator execution
    pub async fn params_to_json(&mut self, params: &HashMap<String, Expression>) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut json_map = serde_json::Map::new();
        for (key, expr) in params {
            let value = Box::pin(self.evaluate_expression(expr)).await?;
            let json_value = self.value_to_json_value(&value);
            json_map.insert(key.clone(), json_value);
        }
        let json_obj = serde_json::Value::Object(json_map);
        Ok(serde_json::to_string(&json_obj)?)
    }

    /// Convert our Value type to serde_json::Value
    fn value_to_json_value(&self, value: &crate::value::Value) -> serde_json::Value {
        match value {
            crate::value::Value::String(s) => serde_json::Value::String(s.clone()),
            crate::value::Value::Number(n) => serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            crate::value::Value::Bool(b) => serde_json::Value::Bool(*b),
            crate::value::Value::Array(arr) => {
                let values: Vec<serde_json::Value> = arr.iter()
                    .map(|v| self.value_to_json_value(v))
                    .collect();
                serde_json::Value::Array(values)
            },
            crate::value::Value::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, v) in obj {
                    map.insert(k.clone(), self.value_to_json_value(v));
                }
                serde_json::Value::Object(map)
            },
            crate::value::Value::Null => serde_json::Value::Null,
        }
    }

    /// Evaluate an AST expression with operator support
    pub async fn evaluate_expression(&mut self, expr: &Expression) -> Result<crate::value::Value, Box<dyn std::error::Error + Send + Sync>> {
        match expr {
            Expression::String(s) => {
                // Check if string contains special operators
                if s.starts_with('@') || s.contains(" + ") || s.contains('?') {
                    Ok(self.parse_value(s).await?)
                } else {
                    Ok(crate::value::Value::String(s.clone()))
                }
            }
            Expression::Number(n) => Ok(crate::value::Value::Number(*n)),
            Expression::Bool(b) => Ok(crate::value::Value::Bool(*b)),
            Expression::Array(arr) => {
                let mut values = Vec::new();
                for item in arr {
                    values.push(Box::pin(self.evaluate_expression(item)).await?);
                }
                Ok(crate::value::Value::Array(values))
            }
            Expression::Object(obj) => {
                let mut map = HashMap::new();
                for (key, expr) in obj {
                    map.insert(key.clone(), Box::pin(self.evaluate_expression(expr)).await?);
                }
                Ok(crate::value::Value::Object(map))
            }
            Expression::OperatorCall(operator, params) => {
                Err(Box::new(HlxError::validation_error("OperatorCall not supported", "Use @ prefixed operators instead")))
            }
            Expression::AtOperatorCall(operator, params) => {
                let json_params = self.params_to_json(params).await?;
                Ok(self.execute_operator(operator, &json_params).await?)
            }
            Expression::Identifier(name) => {
                // Could be an operator call with no parameters
                let params = HashMap::new();
                let json_params = self.params_to_json(&params).await?;
                match self.execute_operator(&name, &json_params).await {
                    Ok(value) => Ok(value),
                    Err(_) => Ok(crate::value::Value::String(name.clone())),
                }
            }
            _ => Ok(crate::value::Value::String(format!("Unsupported expression: {:?}", expr))),
        }
    }

    /// Parse value with operator support (similar to ops.rs)
    pub async fn parse_value(&mut self, value: &str) -> Result<crate::value::Value, Box<dyn std::error::Error + Send + Sync>> {
        let value = value.trim();

        // Remove optional semicolon
        let value = if value.ends_with(';') {
            value.trim_end_matches(';').trim()
        } else {
            value
        };

        // Basic types
        match value {
            "true" => return Ok(crate::value::Value::Bool(true)),
            "false" => return Ok(crate::value::Value::Bool(false)),
            "null" => return Ok(crate::value::Value::Null),
            _ => {}
        }

        // Numbers
        if let Ok(num) = value.parse::<i64>() {
            return Ok(crate::value::Value::Number(num as f64));
        }
        if let Ok(num) = value.parse::<f64>() {
            return Ok(crate::value::Value::Number(num));
        }

        // @ operators
        let operator_re = regex::Regex::new(r"^@([a-zA-Z_][a-zA-Z0-9_]*)\((.+)\)$").unwrap();
        if let Some(captures) = operator_re.captures(value) {
            let operator = captures.get(1).unwrap().as_str();
            let params = captures.get(2).unwrap().as_str();
            return self.execute_operator(&format!("@{}", operator), params).await;
        }

        // String concatenation
        if value.contains(" + ") {
            let parts: Vec<&str> = value.split(" + ").collect();
            let mut result = String::new();
            for part in parts {
                let part = part.trim().trim_matches('"').trim_matches('\'');
                result.push_str(&part);
            }
            return Ok(crate::value::Value::String(result));
        }

        // Conditional/ternary: condition ? true_val : false_val
        let ternary_re = regex::Regex::new(r"(.+?)\s*\?\s*(.+?)\s*:\s*(.+)").unwrap();
        if let Some(captures) = ternary_re.captures(value) {
            let condition = captures.get(1).unwrap().as_str().trim();
            let true_val = captures.get(2).unwrap().as_str().trim();
            let false_val = captures.get(3).unwrap().as_str().trim();

            if self.evaluate_condition(condition).await {
                return Box::pin(self.parse_value(true_val)).await;
            } else {
                return Box::pin(self.parse_value(false_val)).await;
            }
        }

        // Remove quotes from strings
        if (value.starts_with('"') && value.ends_with('"')) ||
           (value.starts_with('\'') && value.ends_with('\'')) {
            return Ok(crate::value::Value::String(value[1..value.len()-1].to_string()));
        }

        // Return as string
        Ok(crate::value::Value::String(value.to_string()))
    }

    /// Evaluate conditions for ternary expressions
    async fn evaluate_condition(&mut self, condition: &str) -> bool {
        let condition = condition.trim();

        // Simple equality check
        if let Some(eq_pos) = condition.find("==") {
            let left = Box::pin(self.parse_value(condition[..eq_pos].trim())).await.unwrap_or(crate::value::Value::String("".to_string()));
            let right = Box::pin(self.parse_value(condition[eq_pos+2..].trim())).await.unwrap_or(crate::value::Value::String("".to_string()));
            return left.to_string() == right.to_string();
        }

        // Not equal
        if let Some(ne_pos) = condition.find("!=") {
            let left = Box::pin(self.parse_value(condition[..ne_pos].trim())).await.unwrap_or(crate::value::Value::String("".to_string()));
            let right = Box::pin(self.parse_value(condition[ne_pos+2..].trim())).await.unwrap_or(crate::value::Value::String("".to_string()));
            return left.to_string() != right.to_string();
        }

        // Greater than
        if let Some(gt_pos) = condition.find('>') {
            let left = Box::pin(self.parse_value(condition[..gt_pos].trim())).await.unwrap_or(crate::value::Value::String("".to_string()));
            let right = Box::pin(self.parse_value(condition[gt_pos+1..].trim())).await.unwrap_or(crate::value::Value::String("".to_string()));

            if let (crate::value::Value::Number(l), crate::value::Value::Number(r)) = (&left, &right) {
                return l > r;
            }
            return left.to_string() > right.to_string();
        }

        // Default: check if truthy
        let value = Box::pin(self.parse_value(condition)).await.unwrap_or(crate::value::Value::String("".to_string()));
        match value {
            crate::value::Value::Bool(b) => b,
            crate::value::Value::String(s) => !s.is_empty() && s != "false" && s != "null" && s != "0",
            crate::value::Value::Number(n) => n != 0.0,
            crate::value::Value::Null => false,
            _ => true,
        }
    }
}
pub fn parse(tokens: Vec<Token>) -> Result<HelixAst, ParseError> {
    let mut parser = Parser::new(tokens);
    parser
        .parse()
        .map_err(|msg| ParseError {
            message: msg,
            location: None,
            token_index: 0,
            expected: None,
            found: String::new(),
            context: String::new(),
        })
}