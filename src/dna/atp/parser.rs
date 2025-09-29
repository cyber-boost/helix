use crate::atp::lexer::{
    Token, Keyword, TimeUnit, TokenWithLocation, SourceLocation, SourceMap,
};
pub use crate::atp::types::SecretRef;
use crate::atp::types::Duration;
use crate::atp::ast::*;
pub use crate::atp::types::Value;
use crate::ops::engine::OperatorEngine;
use crate::hel::error::HlxError;
use std::collections::HashMap;
use regex;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectBlockType {
    Brace,
    Angle,
    Bracket,
    Colon,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Brace,
    Angle,
    Bracket,
    Colon,
}
#[cfg(feature = "js")]
use napi;
pub struct Parser {
    tokens: Vec<TokenWithLocation>,
    source_map: Option<SourceMap>,
    current: usize,
    errors: Vec<ParseError>,
    recovery_points: Vec<usize>,
    operator_engine: Option<OperatorEngine>,
    runtime_context: HashMap<String, String>,
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
            runtime_context: HashMap::new(),
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
            runtime_context: HashMap::new(),
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
            runtime_context: HashMap::new(),
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
                let (clean_name, is_var) = Self::peel_markers(&name);
                if is_var {
                    Ok(self.resolve_variable(&clean_name, true))
                } else {
                    Ok(name)
                }
            }
            Token::String(name) => {
                let name = name.clone();
                self.advance();
                let (clean_name, is_var) = Self::peel_markers(&name);
                if is_var {
                    Ok(self.resolve_variable(&clean_name, true))
                } else {
                    Ok(name)
                }
            }
            _ => Err(format!("Expected identifier, found {:?}", self.current_token())),
        }
    }
    fn skip_newlines(&mut self) {
        while self.current_token() == &Token::Newline {
            self.advance();
        }
    }
    /// Remove leading/trailing '!' and tell whether the token was marked
    fn peel_markers(raw: &str) -> (String, bool) {
        let mut s = raw.to_string();
        let mut is_var = false;
        if s.starts_with('!') {
            s.remove(0);
            is_var = true;
        }
        if s.ends_with('!') {
            s.pop();
            is_var = true;
        }
        (s, is_var)
    }
    /// Resolve a variable-marker name. Returns the concrete string.
    fn resolve_variable(&self, name: &str, is_marked: bool) -> String {
        if !is_marked {
            return name.to_string();
        }
        if let Some(v) = self.runtime_context.get(name) {
            return v.clone();
        }
        std::env::var(name).unwrap_or_else(|_| name.to_string())
    }
    fn expect_identifier_or_string(&mut self) -> Result<(String, bool), String> {
        match self.current_token() {
            Token::Identifier(raw) => {
                let raw = raw.clone();
                self.advance();
                Ok(Self::peel_markers(&raw))
            }
            Token::String(raw) => {
                let raw = raw.clone();
                self.advance();
                Ok(Self::peel_markers(&raw))
            }
            _ => {
                Err(
                    format!(
                        "Expected identifier or string, found {:?}", self.current_token()
                    ),
                )
            }
        }
    }
    fn build_full_name(&self, base: &str, sub: Option<String>) -> String {
        if let Some(s) = sub { format!("{}.{}", base, s) } else { base.to_string() }
    }
    fn parse_at_operator_with_chain(
        &mut self,
        operator: String,
        arg: String,
    ) -> Result<Expression, String> {
        self.advance();
        let first_key = self.expect_identifier()?;
        let mut value_opt = None;
        if self.current_token() == &Token::LeftBracket {
            self.advance();
            let v = self.expect_identifier()?;
            self.expect(Token::RightBracket)?;
            value_opt = Some(v);
        }
        Ok(Expression::OperatorCall(operator, arg, Some(first_key), value_opt))
    }
    pub fn set_runtime_context(&mut self, context: HashMap<String, String>) {
        self.runtime_context = context;
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
                Token::Tilde => {
                    self.advance();
                    match self.current_token() {
                        Token::Identifier(actual_name) => {
                            let actual_name = actual_name.clone();
                            self.recovery_points.push(self.current);
                            match self.parse_generic_declaration(actual_name.clone()) {
                                Ok(decl) => {
                                    ast.add_declaration(decl);
                                    self.recovery_points.pop();
                                }
                                Err(err) => {
                                    self.add_error(
                                        err.clone(),
                                        Some(format!("valid ~{} declaration", actual_name)),
                                    );
                                    self.recover_to_next_declaration();
                                    self.recovery_points.pop();
                                }
                            }
                        }
                        _ => {
                            self.add_error(
                                format!(
                                    "Expected identifier after '~', found {:?}", self
                                    .current_token()
                                ),
                                Some("identifier after tilde".to_string()),
                            );
                            self.recover_to_next_declaration();
                        }
                    }
                }
                Token::Identifier(identifier) => {
                    self.advance(); // Advance past the identifier like keywords do
                    self.recovery_points.push(self.current);
                    match self.parse_generic_declaration(identifier.clone()) {
                        Ok(decl) => {
                            ast.add_declaration(decl);
                            self.recovery_points.pop();
                        }
                        Err(err) => {
                            self.add_error(
                                err.clone(),
                                Some(format!("valid {} declaration", identifier)),
                            );
                            self.recover_to_next_declaration();
                            self.recovery_points.pop();
                        }
                    }
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
                let (name, block_kind) = self.parse_project_variations()?;
                match block_kind {
                    BlockKind::Brace => {
                        self.expect(Token::LeftBrace)?;
                    }
                    BlockKind::Angle => {
                        self.expect(Token::LessThan)?;
                    }
                    BlockKind::Bracket => {
                        self.expect(Token::LeftBracket)?;
                    }
                    BlockKind::Colon => {
                        self.expect(Token::Colon)?;
                        let properties = self.parse_properties()?;
                        self.expect(Token::Semicolon)?;
                        return Ok(Declaration::Project(ProjectDecl { name, properties }));
                    }
                }
                let properties = self.parse_properties()?;
                match block_kind {
                    BlockKind::Brace => {
                        self.expect(Token::RightBrace)?;
                    }
                    BlockKind::Angle => {
                        self.expect(Token::GreaterThan)?;
                    }
                    BlockKind::Bracket => {
                        self.expect(Token::RightBracket)?;
                    }
                    BlockKind::Colon => {}
                }
                Ok(Declaration::Project(ProjectDecl { name, properties }))
            }
            Keyword::Agent => {
                self.advance();
                let name = if self.current_token() == &Token::LeftBrace {
                    String::new()
                } else {
                    self.expect_identifier()?
                };
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
            Keyword::Task => {
                self.advance();
                let (name, block_kind) = self.parse_generic_variations("task".to_string())?;
                match block_kind {
                    BlockKind::Brace => {
                        self.expect(Token::LeftBrace)?;
                    }
                    BlockKind::Angle => {
                        self.expect(Token::LessThan)?;
                    }
                    BlockKind::Bracket => {
                        self.expect(Token::LeftBracket)?;
                    }
                    BlockKind::Colon => {
                        self.expect(Token::Colon)?;
                        if self.current_token() == &Token::Semicolon {
                            self.advance();
                        }
                        return Ok(
                            Declaration::Task(TaskDecl {
                                name,
                                properties: HashMap::new(),
                            }),
                        );
                    }
                }
                let properties = self.parse_properties()?;
                match block_kind {
                    BlockKind::Brace => {
                        self.expect(Token::RightBrace)?;
                    }
                    BlockKind::Angle => {
                        self.expect(Token::GreaterThan)?;
                    }
                    BlockKind::Bracket => {
                        self.expect(Token::RightBracket)?;
                    }
                    BlockKind::Colon => {}
                }
                Ok(Declaration::Task(TaskDecl { name, properties }))
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
                Token::Plus => {
                    self.advance();
                    let right = self.parse_expression_with_precedence(Precedence::Addition)?;
                    left = Expression::BinaryOp(Box::new(left), BinaryOperator::Add, Box::new(right));
                }
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
                let (clean_s, is_var) = Self::peel_markers(&s);
                if is_var {
                    let resolved_value = self.resolve_variable(&clean_s, true);
                    Ok(Expression::String(resolved_value))
                } else {
                    Ok(Expression::String(s))
                }
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
                        TimeUnit::Seconds => crate::atp::types::TimeUnit::Seconds,
                        TimeUnit::Minutes => crate::atp::types::TimeUnit::Minutes,
                        TimeUnit::Hours => crate::atp::types::TimeUnit::Hours,
                        TimeUnit::Days => crate::atp::types::TimeUnit::Days,
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
                let operator = r.clone();
                self.advance();
                if self.current_token() == &Token::LeftBracket {
                    self.advance();
                    let arg = match self.current_token() {
                        Token::String(s) => {
                            let s = s.clone();
                            self.advance();
                            let (clean_s, is_var) = Self::peel_markers(&s);
                            if is_var {
                                self.resolve_variable(&clean_s, true)
                            } else {
                                s
                            }
                        }
                        Token::Identifier(id) => {
                            let id = id.clone();
                            self.advance();
                            let (clean_id, is_var) = Self::peel_markers(&id);
                            if is_var {
                                self.resolve_variable(&clean_id, true)
                            } else {
                                id
                            }
                        }
                        _ => {
                            return Err(
                                format!(
                                    "Expected string or identifier inside @{}[ … ], found {:?}",
                                    operator, self.current_token()
                                ),
                            );
                        }
                    };
                    if self.current_token() != &Token::RightBracket {
                        return Err(
                            format!(
                                "Expected ']' after argument of @{}, found {:?}", operator,
                                self.current_token()
                            ),
                        );
                    }
                    self.advance();
                    if self.current_token() == &Token::Dot {
                        return self.parse_at_operator_with_chain(operator, arg);
                    }
                    let mut params = HashMap::new();
                    params.insert("arg".to_string(), Expression::String(arg));
                    Ok(Expression::AtOperatorCall(operator, params))
                } else {
                    if self.current_token() == &Token::LeftBracket {
                        self.advance();
                        let key = self.expect_identifier()?;
                        self.expect(Token::RightBracket)?;
                        if self.current_token() == &Token::Dot {
                            self.advance();
                            let sub_key = self.expect_identifier()?;
                            if self.current_token() == &Token::LeftBracket {
                                self.advance();
                                let value = self.expect_identifier()?;
                                self.expect(Token::RightBracket)?;
                                Ok(
                                    Expression::OperatorCall(
                                        operator,
                                        key,
                                        Some(sub_key),
                                        Some(value),
                                    ),
                                )
                            } else {
                                Ok(
                                    Expression::OperatorCall(operator, key, Some(sub_key), None),
                                )
                            }
                        } else {
                            Ok(Expression::IndexedReference(operator, key))
                        }
                    } else if self.current_token() == &Token::Dot {
                        self.advance();
                        let key = self.expect_identifier()?;
                        if self.current_token() == &Token::LeftBracket {
                            self.advance();
                            let value = self.expect_identifier()?;
                            self.expect(Token::RightBracket)?;
                            Ok(
                                Expression::OperatorCall(operator, key, None, Some(value)),
                            )
                        } else {
                            Ok(Expression::OperatorCall(operator, key, None, None))
                        }
                    } else {
                        Ok(Expression::Reference(operator))
                    }
                }
            }
            Token::Identifier(i) => {
                let i = i.clone();
                self.advance();
                let (clean_i, is_var) = Self::peel_markers(&i);
                if is_var {
                    let resolved_value = self.resolve_variable(&clean_i, true);
                    Ok(Expression::String(resolved_value))
                } else {
                    Ok(Expression::Identifier(i))
                }
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
            Token::Plus => Precedence::Addition,
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
        while self.current_token() != &Token::RightBrace
            && self.current_token() != &Token::GreaterThan
            && self.current_token() != &Token::RightBracket
            && self.current_token() != &Token::Semicolon
        {
            self.skip_newlines();
            if self.current_token() == &Token::RightBrace
                || self.current_token() == &Token::GreaterThan
                || self.current_token() == &Token::RightBracket
                || self.current_token() == &Token::Semicolon
            {
                break;
            }
            let key = match self.current_token() {
                Token::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    name
                }
                Token::Keyword(keyword) => {
                    let keyword = keyword.clone();
                    self.advance();
                    format!("{:?}", keyword).to_lowercase()
                }
                _ => {
                    return Err(
                        format!(
                            "Expected property key (identifier or keyword), found {:?}",
                            self.current_token()
                        ),
                    );
                }
            };
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
    pub async fn execute_operator(
        &mut self,
        operator: &str,
        params: &str,
    ) -> Result<crate::atp::value::Value, Box<dyn std::error::Error + Send + Sync>> {
        if self.operator_engine.is_none() {
            self.operator_engine = Some(
                OperatorEngine::new()
                    .await
                    .map_err(|e| {
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    })?,
            );
        }
        if let Some(ref engine) = self.operator_engine {
            match engine.execute_operator(operator, params).await {
                Ok(value) => Ok(value),
                Err(e) => {
                    eprintln!("Operator execution error: {:?}", e);
                    Ok(crate::atp::value::Value::String(format!("@{}({})", operator, params)))
                }
            }
        } else {
            Ok(crate::atp::value::Value::String(format!("@{}({})", operator, params)))
        }
    }
    pub async fn params_to_json(
        &mut self,
        params: &HashMap<String, Expression>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut json_map = serde_json::Map::new();
        for (key, expr) in params {
            let value = Box::pin(self.evaluate_expression(expr)).await?;
            let json_value = self.value_to_json_value(&value);
            json_map.insert(key.clone(), json_value);
        }
        let json_obj = serde_json::Value::Object(json_map);
        Ok(serde_json::to_string(&json_obj)?)
    }
    fn value_to_json_value(&self, value: &crate::atp::value::Value) -> serde_json::Value {
        match value {
            crate::atp::value::Value::String(s) => serde_json::Value::String(s.clone()),
            crate::atp::value::Value::Number(n) => {
                serde_json::Number::from_f64(*n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            crate::atp::value::Value::Bool(b) => serde_json::Value::Bool(*b),
            crate::atp::value::Value::Array(arr) => {
                let values: Vec<serde_json::Value> = arr
                    .iter()
                    .map(|v| self.value_to_json_value(v))
                    .collect();
                serde_json::Value::Array(values)
            }
            crate::atp::value::Value::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, v) in obj {
                    map.insert(k.clone(), self.value_to_json_value(v));
                }
                serde_json::Value::Object(map)
            }
            crate::atp::value::Value::Null => serde_json::Value::Null,
            crate::atp::value::Value::Duration(d) => serde_json::Value::String(format!("{} {:?}", d.value, d.unit)),
            crate::atp::value::Value::Reference(r) => serde_json::Value::String(format!("@{}", r)),
            crate::atp::value::Value::Identifier(i) => serde_json::Value::String(i.clone()),
        }
    }
    pub async fn evaluate_expression(
        &mut self,
        expr: &Expression,
    ) -> Result<crate::atp::value::Value, Box<dyn std::error::Error + Send + Sync>> {
        match expr {
            Expression::String(s) => {
                if s.starts_with('@') || s.contains(" + ") || s.contains('?') {
                    Ok(self.parse_value(s).await?)
                } else {
                    Ok(crate::atp::value::Value::String(s.clone()))
                }
            }
            Expression::Number(n) => Ok(crate::atp::value::Value::Number(*n)),
            Expression::Bool(b) => Ok(crate::atp::value::Value::Bool(*b)),
            Expression::Array(arr) => {
                let mut values = Vec::new();
                for item in arr {
                    values.push(Box::pin(self.evaluate_expression(item)).await?);
                }
                Ok(crate::atp::value::Value::Array(values))
            }
            Expression::Object(obj) => {
                let mut map = HashMap::new();
                for (key, expr) in obj {
                    map.insert(
                        key.clone(),
                        Box::pin(self.evaluate_expression(expr)).await?,
                    );
                }
                Ok(crate::atp::value::Value::Object(map))
            }
            Expression::OperatorCall(_operator, _key, _sub_key, _value) => {
                Err(
                    Box::new(
                        HlxError::validation_error(
                            "OperatorCall not supported",
                            "Use @ prefixed operators instead",
                        ),
                    ),
                )
            }
            Expression::AtOperatorCall(operator, params) => {
                match operator.as_str() {
                    "env" => {
                        let arg_expr = params
                            .get("arg")
                            .ok_or_else(|| HlxError::validation_error(
                                "Missing argument for @env",
                                "",
                            ))?;
                        let var_name = match arg_expr {
                            Expression::String(s) => s.clone(),
                            _ => {
                                Box::pin(self.evaluate_expression(arg_expr))
                                    .await?
                                    .to_string()
                            }
                        };
                        let val = self
                            .runtime_context
                            .get(&var_name)
                            .cloned()
                            .or_else(|| std::env::var(&var_name).ok())
                            .ok_or_else(|| HlxError::validation_error(
                                &format!("Env var '{}' not set", var_name),
                                "",
                            ))?;
                        Ok(crate::atp::value::Value::String(val))
                    }
                    other => {
                        let json_params = self.params_to_json(params).await?;
                        self.execute_operator(&format!("@{}", other), &json_params).await
                    }
                }
            }
            Expression::Identifier(name) => {
                let params = HashMap::new();
                let json_params = self.params_to_json(&params).await?;
                match self.execute_operator(&name, &json_params).await {
                    Ok(value) => Ok(value),
                    Err(_) => {
                        let (clean_name, is_var) = Self::peel_markers(&name);
                        Ok(
                            crate::atp::value::Value::String(
                                self.resolve_variable(&clean_name, is_var),
                            ),
                        )
                    }
                }
            }
            _ => {
                Ok(
                    crate::atp::value::Value::String(
                        format!("Unsupported expression: {:?}", expr),
                    ),
                )
            }
        }
    }
    pub async fn parse_value(
        &mut self,
        value: &str,
    ) -> Result<crate::atp::value::Value, Box<dyn std::error::Error + Send + Sync>> {
        let value = value.trim();
        let value = if value.ends_with(';') {
            value.trim_end_matches(';').trim()
        } else {
            value
        };
        match value {
            "true" => return Ok(crate::atp::value::Value::Bool(true)),
            "false" => return Ok(crate::atp::value::Value::Bool(false)),
            "null" => return Ok(crate::atp::value::Value::Null),
            _ => {}
        }
        if let Ok(num) = value.parse::<i64>() {
            return Ok(crate::atp::value::Value::Number(num as f64));
        }
        if let Ok(num) = value.parse::<f64>() {
            return Ok(crate::atp::value::Value::Number(num));
        }
        let operator_re = regex::Regex::new(r"^@([a-zA-Z_][a-zA-Z0-9_]*)\((.+)\)$")
            .unwrap();
        if let Some(captures) = operator_re.captures(value) {
            let operator = captures.get(1).unwrap().as_str();
            let params = captures.get(2).unwrap().as_str();
            return Ok(self.execute_operator(&format!("@{}", operator), params).await?);
        }
        if value.contains(" + ") {
            let parts: Vec<&str> = value.split(" + ").collect();
            let mut result = String::new();
            for part in parts {
                let part = part.trim().trim_matches('"').trim_matches('\'');
                result.push_str(&part);
            }
            return Ok(crate::atp::value::Value::String(result));
        }
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
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            return Ok(
                crate::atp::value::Value::String(value[1..value.len() - 1].to_string()),
            );
        }
        Ok(crate::atp::value::Value::String(value.to_string()))
    }
    async fn evaluate_condition(&mut self, condition: &str) -> bool {
        let condition = condition.trim();
        if let Some(eq_pos) = condition.find("==") {
            let left = Box::pin(self.parse_value(condition[..eq_pos].trim()))
                .await
                .unwrap_or(crate::atp::value::Value::String("".to_string()));
            let right = Box::pin(self.parse_value(condition[eq_pos + 2..].trim()))
                .await
                .unwrap_or(crate::atp::value::Value::String("".to_string()));
            return left.to_string() == right.to_string();
        }
        if let Some(ne_pos) = condition.find("!=") {
            let left = Box::pin(self.parse_value(condition[..ne_pos].trim()))
                .await
                .unwrap_or(crate::atp::value::Value::String("".to_string()));
            let right = Box::pin(self.parse_value(condition[ne_pos + 2..].trim()))
                .await
                .unwrap_or(crate::atp::value::Value::String("".to_string()));
            return left.to_string() != right.to_string();
        }
        if let Some(gt_pos) = condition.find('>') {
            let left = Box::pin(self.parse_value(condition[..gt_pos].trim()))
                .await
                .unwrap_or(crate::atp::value::Value::String("".to_string()));
            let right = Box::pin(self.parse_value(condition[gt_pos + 1..].trim()))
                .await
                .unwrap_or(crate::atp::value::Value::String("".to_string()));
            if let (crate::atp::value::Value::Number(l), crate::atp::value::Value::Number(r)) = (
                &left,
                &right,
            ) {
                return l > r;
            }
            return left.to_string() > right.to_string();
        }
        let value = Box::pin(self.parse_value(condition))
            .await
            .unwrap_or(crate::atp::value::Value::String("".to_string()));
        match value {
            crate::atp::value::Value::Bool(b) => b,
            crate::atp::value::Value::String(s) => {
                !s.is_empty() && s != "false" && s != "null" && s != "0"
            }
            crate::atp::value::Value::Number(n) => n != 0.0,
            crate::atp::value::Value::Null => false,
            _ => true,
        }
    }
    fn parse_generic_declaration(
        &mut self,
        identifier: String,
    ) -> Result<Declaration, String> {
        // Parse optional subname like keywords do
        let (name, block_kind) = self.parse_generic_variations(identifier)?;
        match block_kind {
            BlockKind::Brace => {
                self.expect(Token::LeftBrace)?;
            }
            BlockKind::Angle => {
                self.expect(Token::LessThan)?;
            }
            BlockKind::Bracket => {
                self.expect(Token::LeftBracket)?;
            }
            BlockKind::Colon => {
                self.expect(Token::Colon)?;
                let properties = self.parse_properties()?;
                self.expect(Token::Semicolon)?;
                return Ok(Declaration::Section(SectionDecl { name, properties }));
            }
        }
        let properties = self.parse_properties()?;
        match block_kind {
            BlockKind::Brace => {
                self.expect(Token::RightBrace)?;
            }
            BlockKind::Angle => {
                self.expect(Token::GreaterThan)?;
            }
            BlockKind::Bracket => {
                self.expect(Token::RightBracket)?;
            }
            BlockKind::Colon => {}
        }
        Ok(Declaration::Section(SectionDecl { name, properties }))
    }
    fn parse_generic_variations(
        &mut self,
        identifier: String,
    ) -> Result<(String, BlockKind), String> {
        let sub_name = match self.current_token() {
            Token::Identifier(s) => {
                let v = s.clone();
                self.advance();
                Some(v)
            }
            Token::String(s) => {
                let v = s.clone();
                self.advance();
                Some(v)
            }
            _ => None,
        };
        self.skip_newlines();
        let kind = match self.current_token() {
            Token::LeftBrace => BlockKind::Brace,
            Token::LessThan => BlockKind::Angle,
            Token::LeftBracket => BlockKind::Bracket,
            Token::Colon => BlockKind::Colon,
            _ => {
                return Err(
                    format!(
                        "Expected block delimiter after '{}', found {:?}", identifier,
                        self.current_token()
                    ),
                );
            }
        };
        match kind {
            BlockKind::Brace => {}
            BlockKind::Angle => {}
            BlockKind::Bracket => {}
            BlockKind::Colon => {
                // Don't advance here - let parse_generic_declaration handle it
            }
        }
        Ok((self.build_full_name(&identifier, sub_name), kind))
    }
    fn parse_project_variations(&mut self) -> Result<(String, BlockKind), String> {
        self.parse_generic_variations("project".to_string())
    }
    fn convert_ops_value_to_types_value(&self, ops_value: crate::atp::value::Value) -> Value {
        match ops_value {
            crate::atp::value::Value::String(s) => Value::String(s),
            crate::atp::value::Value::Number(n) => Value::Number(n),
            crate::atp::value::Value::Bool(b) => Value::Bool(b),
            crate::atp::value::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(|v| self.convert_ops_value_to_types_value(v)).collect())
            }
            crate::atp::value::Value::Object(obj) => {
                Value::Object(obj.into_iter().map(|(k, v)| (k, self.convert_ops_value_to_types_value(v))).collect())
            }
            crate::atp::value::Value::Null => Value::Null,
            crate::atp::value::Value::Duration(d) => Value::Duration(d),
            crate::atp::value::Value::Reference(r) => Value::Reference(r),
            crate::atp::value::Value::Identifier(i) => Value::Identifier(i),
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
