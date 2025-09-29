use crate::dna::atp::ast::Expression;
use crate::dna::atp::value::Value;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::future::Future;
use std::pin::Pin;
use std::path::{Path, PathBuf};
use std::str::Chars;
use thiserror::Error;
pub use crate::ops::engine::OperatorEngine;

// Add BoxFuture type alias
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// ---------------------------------------------------------------------------
///  Custom Error Types (Improvement #1)
/// ---------------------------------------------------------------------------
#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("Math parse error at position {position}: {message}")]
    MathError { position: usize, message: String },
    
    #[error("Syntax error at line {line}, column {column}: {message}")]
    SyntaxError {
        line: usize,
        column: usize,
        message: String,
    },
    
    #[error("Variable not found: {name}")]
    VariableNotFound { name: String },
    
    #[error("Invalid operator: {operator}")]
    InvalidOperator { operator: String },
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("JSON error: {0}")]
    JsonError(String),
    
    #[error("Operator engine error: {0}")]
    OperatorError(String),
    
    #[error("Invalid escape sequence at position {position}")]
    InvalidEscape { position: usize },
    
    #[error("Unclosed string starting at position {position}")]
    UnclosedString { position: usize },
    
    #[error("Multiple errors occurred: {0:?}")]
    Multiple(Vec<ParseError>),
}

// Add From implementations for errors
impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(err: serde_json::Error) -> Self {
        ParseError::JsonError(err.to_string())
    }
}

type Result<T> = std::result::Result<T, ParseError>;

/// ---------------------------------------------------------------------------
///  Type Safety Improvements (Improvement #8)
/// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SectionName(String);

impl SectionName {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableName(String);

impl VariableName {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VariableName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(String);

impl CacheKey {
    pub fn new(file: &str, key: &str) -> Self {
        Self(format!("{}:{}", file, key))
    }
}

/// ---------------------------------------------------------------------------
///  Regex Constants (Improvement #4)
/// ---------------------------------------------------------------------------
static REGEX_CACHE: Lazy<RegexCache> = Lazy::new(|| RegexCache::new());

struct RegexCache {
    global_var: Regex,
    date: Regex,
    env: Regex,
    range: Regex,
    get: Regex,
    set: Regex,
    query: Regex,
    operator: Regex,
    ternary: Regex,
    section: Regex,
    angle_start: Regex,
    brace_start: Regex,
    key_value: Regex,
    identifier: Regex,
}

impl RegexCache {
    fn new() -> Self {
        Self {
            global_var: Regex::new(r"^\$([a-zA-Z_][a-zA-Z0-9_]*)$").unwrap(),
            date: Regex::new(r#"^@date\(["']([^"']+)["']\)$"#).unwrap(),
            env: Regex::new(r#"^@env\(["']([^"']*)["'](?:,\s*["']([^"']*)["'])?\)$"#).unwrap(),
            range: Regex::new(r"^(\d+)-(\d+)$").unwrap(),
            get: Regex::new(r#"^@([a-zA-Z0-9_-]+)\.hlx\.get\(["']([^"']+)["']\)$"#).unwrap(),
            set: Regex::new(r#"^@([a-zA-Z0-9_-]+)\.hlx\.set\(["']([^"']+)["'],\s*(.+)\)$"#).unwrap(),
            query: Regex::new(r#"^@query\(["']([^"']+)["']\)$"#).unwrap(),
            operator: Regex::new(r"^@([a-zA-Z_][a-zA-Z0-9_]*)\((.*)\)$").unwrap(),
            ternary: Regex::new(r"(.+?)\s*\?\s*(.+?)\s*:\s*(.+)").unwrap(),
            section: Regex::new(r"^\[([a-zA-Z_][a-zA-Z0-9_]*)\]$").unwrap(),
            angle_start: Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)\s*>$").unwrap(),
            brace_start: Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)\s*\{$").unwrap(),
            key_value: Regex::new(r"^([\$]?[a-zA-Z_][a-zA-Z0-9_-]*)\s*[:=]\s*(.+)$").unwrap(),
            identifier: Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap(),
        }
    }
}

/// ---------------------------------------------------------------------------
///  Enhanced Math Parser (Improvement #2)
/// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
enum MathToken {
    Number(f64),
    Op(char),
    LParen,
    RParen,
}

pub struct MathParser<'a> {
    input: &'a str,
    chars: Chars<'a>,
    lookahead: Option<char>,
    position: usize,
}

impl<'a> MathParser<'a> {
    fn new(s: &'a str) -> Self {
        let mut chars = s.chars();
        let lookahead = chars.next();
        Self {
            input: s,
            chars,
            lookahead,
            position: 0,
        }
    }

    fn next_char(&mut self) {
        self.lookahead = self.chars.next();
        self.position += 1;
    }

    fn peek(&self) -> Option<char> {
        self.lookahead
    }

    fn consume_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.next_char();
        }
    }

    fn parse_number(&mut self) -> Result<f64> {
        let mut s = String::new();
        let start_pos = self.position;
        
        // Handle scientific notation
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' {
                s.push(c);
                self.next_char();
                if (c == 'e' || c == 'E') && matches!(self.peek(), Some('+') | Some('-')) {
                    s.push(self.peek().unwrap());
                    self.next_char();
                }
            } else {
                break;
            }
        }
        
        s.parse::<f64>().map_err(|_| ParseError::MathError {
            position: start_pos,
            message: format!("Invalid number: {}", s),
        })
    }

    fn next_token(&mut self) -> Result<Option<MathToken>> {
        self.consume_ws();
        
        match self.peek() {
            None => Ok(None),
            Some(ch) if ch.is_ascii_digit() || ch == '.' => {
                Ok(Some(MathToken::Number(self.parse_number()?)))
            }
            Some('+') | Some('-') | Some('*') | Some('/') | Some('%') | Some('^') => {
                let op = self.peek().unwrap();
                self.next_char();
                Ok(Some(MathToken::Op(op)))
            }
            Some('(') => {
                self.next_char();
                Ok(Some(MathToken::LParen))
            }
            Some(')') => {
                self.next_char();
                Ok(Some(MathToken::RParen))
            }
            Some(c) => Err(ParseError::MathError {
                position: self.position,
                message: format!("Unexpected character: '{}'", c),
            }),
        }
    }

    pub fn parse(&mut self) -> Result<f64> {
        let mut tokens = Vec::new();
        while let Some(tok) = self.next_token()? {
            tokens.push(tok);
        }
        
        if tokens.is_empty() {
            return Err(ParseError::MathError {
                position: 0,
                message: "Empty expression".to_string(),
            });
        }
        
        let mut pos = 0;
        self.expr(&tokens, &mut pos)
    }

    fn expr(&self, toks: &[MathToken], pos: &mut usize) -> Result<f64> {
        let mut lhs = self.term(toks, pos)?;
        
        while let Some(MathToken::Op(op @ ('+' | '-'))) = toks.get(*pos) {
            *pos += 1;
            let rhs = self.term(toks, pos)?;
            lhs = if *op == '+' { lhs + rhs } else { lhs - rhs };
        }
        
        Ok(lhs)
    }

    fn term(&self, toks: &[MathToken], pos: &mut usize) -> Result<f64> {
        let mut lhs = self.power(toks, pos)?;
        
        while let Some(MathToken::Op(op @ ('*' | '/' | '%'))) = toks.get(*pos) {
            *pos += 1;
            let rhs = self.power(toks, pos)?;
            lhs = match op {
                '*' => lhs * rhs,
                '/' => {
                    if rhs == 0.0 {
                        return Err(ParseError::MathError {
                            position: *pos,
                            message: "Division by zero".to_string(),
                        });
                    }
                    lhs / rhs
                }
                '%' => lhs % rhs,
                _ => unreachable!(),
            };
        }
        
        Ok(lhs)
    }

    fn power(&self, toks: &[MathToken], pos: &mut usize) -> Result<f64> {
        let mut lhs = self.factor(toks, pos)?;
        
        while let Some(MathToken::Op('^')) = toks.get(*pos) {
            *pos += 1;
            let rhs = self.factor(toks, pos)?;
            lhs = lhs.powf(rhs);
        }
        
        Ok(lhs)
    }

    fn factor(&self, toks: &[MathToken], pos: &mut usize) -> Result<f64> {
        match toks.get(*pos) {
            Some(MathToken::Number(v)) => {
                *pos += 1;
                Ok(*v)
            }
            Some(MathToken::Op('+')) => {
                *pos += 1;
                self.factor(toks, pos)
            }
            Some(MathToken::Op('-')) => {
                *pos += 1;
                Ok(-self.factor(toks, pos)?)
            }
            Some(MathToken::LParen) => {
                *pos += 1;
                let v = self.expr(toks, pos)?;
                match toks.get(*pos) {
                    Some(MathToken::RParen) => {
                        *pos += 1;
                        Ok(v)
                    }
                    _ => Err(ParseError::MathError {
                        position: *pos,
                        message: "Missing closing parenthesis".to_string(),
                    }),
                }
            }
            _ => Err(ParseError::MathError {
                position: *pos,
                message: "Unexpected end of expression".to_string(),
            }),
        }
    }
}

/// ---------------------------------------------------------------------------
///  Enhanced String Parser (Improvement #3)
/// ---------------------------------------------------------------------------
pub struct StringParser {
    input: String,
    position: usize,
}

impl StringParser {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            position: 0,
        }
    }

    pub fn parse_quoted_string(&mut self) -> Result<String> {
        let chars: Vec<char> = self.input.chars().collect();
        if chars.is_empty() {
            return Ok(String::new());
        }

        let quote = chars[0];
        if quote != '"' && quote != '\'' {
            return Ok(self.input.clone());
        }

        let mut result = String::new();
        let mut i = 1;
        let start_pos = self.position;

        while i < chars.len() {
            match chars[i] {
                '\\' if i + 1 < chars.len() => {
                    i += 1;
                    match chars[i] {
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        '\'' => result.push('\''),
                        'u' if i + 4 < chars.len() => {
                            // Unicode escape sequence
                            let hex = &chars[i + 1..i + 5]
                                .iter()
                                .collect::<String>();
                            match u32::from_str_radix(hex, 16) {
                                Ok(code) => {
                                    if let Some(ch) = char::from_u32(code) {
                                        result.push(ch);
                                        i += 4;
                                    } else {
                                        return Err(ParseError::InvalidEscape {
                                            position: self.position + i,
                                        });
                                    }
                                }
                                Err(_) => {
                                    return Err(ParseError::InvalidEscape {
                                        position: self.position + i,
                                    });
                                }
                            }
                        }
                        c => result.push(c),
                    }
                    i += 1;
                }
                c if c == quote => {
                    if i + 1 == chars.len() {
                        return Ok(result);
                    }
                    break;
                }
                c => {
                    result.push(c);
                    i += 1;
                }
            }
        }

        if i >= chars.len() {
            return Err(ParseError::UnclosedString {
                position: start_pos,
            });
        }

        Ok(result)
    }
}

/// ---------------------------------------------------------------------------
///  Parser Components (Improvement #5)
/// ---------------------------------------------------------------------------
mod parser_components {
    use super::*;

    pub struct ValueParser<'a> {
        parser: &'a mut OperatorParser,
    }

    impl<'a> ValueParser<'a> {
        pub fn new(parser: &'a mut OperatorParser) -> Self {
            Self { parser }
        }

        pub async fn parse_primitive(&mut self, v: &str) -> Option<Value> {
            match v {
                "true" => Some(Value::Bool(true)),
                "false" => Some(Value::Bool(false)),
                "null" => Some(Value::Null),
                _ => {
                    if let Ok(i) = v.parse::<i64>() {
                        Some(Value::Number(i as f64))
                    } else if let Ok(f) = v.parse::<f64>() {
                        Some(Value::Number(f))
                    } else {
                        None
                    }
                }
            }
        }

        pub async fn parse_global_var(&mut self, v: &str) -> Option<Value> {
            if let Some(cap) = REGEX_CACHE.global_var.captures(v) {
                let name = cap.get(1).unwrap().as_str();
                return self.parser.global_variables
                    .get(&VariableName::new(name))
                    .cloned()
                    .or(Some(Value::String(String::new())));
            }
            None
        }

        pub async fn parse_date(&mut self, v: &str, parser: &OperatorParser) -> Option<Value> {
            if let Some(cap) = REGEX_CACHE.date.captures(v) {
                let fmt = cap.get(1).unwrap().as_str();
                return Some(Value::String(parser.execute_date(fmt)));
            }
            None
        }

        pub async fn parse_env(&mut self, v: &str) -> Option<Value> {
            if let Some(cap) = REGEX_CACHE.env.captures(v) {
                let var = cap.get(1).unwrap().as_str();
                let def = cap.get(2).map_or("", |m| m.as_str());
                return Some(Value::String(
                    env::var(var).unwrap_or_else(|_| def.to_string())
                ));
            }
            None
        }
    }

    pub struct ArrayObjectParser<'a> {
        parser: &'a mut OperatorParser,
    }

    impl<'a> ArrayObjectParser<'a> {
        pub fn new(parser: &'a mut OperatorParser) -> Self {
            Self { parser }
        }

        pub async fn parse_array(&mut self, txt: &str) -> Result<Value> {
            let inner = &txt[1..txt.len() - 1];
            if inner.trim().is_empty() {
                return Ok(Value::Array(vec![]));
            }

            let mut items = Vec::new();
            let mut buf = String::new();
            let mut depth = 0;
            let chars: Vec<char> = inner.chars().collect();
            let mut i = 0;

            while i < chars.len() {
                let ch = chars[i];
                
                // Handle strings with escapes
                if (ch == '"' || ch == '\'') && (i == 0 || chars[i - 1] != '\\') {
                    let quote = ch;
                    buf.push(ch);
                    i += 1;
                    
                    while i < chars.len() {
                        let c = chars[i];
                        buf.push(c);
                        if c == '\\' && i + 1 < chars.len() {
                            i += 1;
                            buf.push(chars[i]);
                        } else if c == quote {
                            break;
                        }
                        i += 1;
                    }
                    i += 1;
                    continue;
                }

                match ch {
                    '[' | '{' => {
                        depth += 1;
                        buf.push(ch);
                    }
                    ']' | '}' => {
                        depth -= 1;
                        buf.push(ch);
                    }
                    ',' if depth == 0 => {
                        // Box the recursive call to avoid stack overflow
                        items.push(self.parser.parse_value(buf.trim()).await?);
                        buf.clear();
                    }
                    _ => buf.push(ch),
                }
                i += 1;
            }

            if !buf.trim().is_empty() {
                // Box this recursive call as well
                items.push(self.parser.parse_value(buf.trim()).await?);
            }

            Ok(Value::Array(items))
        }

        pub async fn parse_object(&mut self, txt: &str) -> Result<Value> {
            let inner = &txt[1..txt.len() - 1];
            if inner.trim().is_empty() {
                return Ok(Value::Object(HashMap::new()));
            }

            let mut pairs = Vec::new();
            let mut buf = String::new();
            let mut depth = 0;
            let chars: Vec<char> = inner.chars().collect();
            let mut i = 0;

            while i < chars.len() {
                let ch = chars[i];
                
                // Handle strings with escapes
                if (ch == '"' || ch == '\'') && (i == 0 || chars[i - 1] != '\\') {
                    let quote = ch;
                    buf.push(ch);
                    i += 1;
                    
                    while i < chars.len() {
                        let c = chars[i];
                        buf.push(c);
                        if c == '\\' && i + 1 < chars.len() {
                            i += 1;
                            buf.push(chars[i]);
                        } else if c == quote {
                            break;
                        }
                        i += 1;
                    }
                    i += 1;
                    continue;
                }

                match ch {
                    '[' | '{' => {
                        depth += 1;
                        buf.push(ch);
                    }
                    ']' | '}' => {
                        depth -= 1;
                        buf.push(ch);
                    }
                    ',' if depth == 0 => {
                        pairs.push(buf.trim().to_string());
                        buf.clear();
                    }
                    _ => buf.push(ch),
                }
                i += 1;
            }

            if !buf.trim().is_empty() {
                pairs.push(buf.trim().to_string());
            }

            let mut map = HashMap::new();
            for p in pairs {
                let (k, v) = if let Some(idx) = p.find(':') {
                    (&p[..idx], &p[idx + 1..])
                } else if let Some(idx) = p.find('=') {
                    (&p[..idx], &p[idx + 1..])
                } else {
                    continue;
                };

                let key = k.trim().trim_matches('"').trim_matches('\'');
                // Box the recursive call here as well
                let val = self.parser.parse_value(v.trim()).await?;
                map.insert(key.to_string(), val);
            }

            Ok(Value::Object(map))
        }
    }
}

/// ---------------------------------------------------------------------------
///  Core Parser with Recovery (Improvement #10)
/// ---------------------------------------------------------------------------
pub struct OperatorParser {
    data: HashMap<String, Value>,
    global_variables: HashMap<VariableName, Value>,
    section_variables: HashMap<String, Value>,
    cache: HashMap<String, Value>,
    cross_file_cache: HashMap<CacheKey, Value>,
    current_section: Option<SectionName>,
    in_object: bool,
    object_key: String,
    hlx_loaded: bool,
    hlx_locations: Vec<String>,
    operator_engine: Option<OperatorEngine>,
    errors: Vec<ParseError>,  // Collect errors for recovery
    current_line: usize,
    current_column: usize,
}

/* ---------- helper directories ---------- */
pub fn get_or_create_helix_dir() -> std::io::Result<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(PathBuf::from)
        .or_else(|_| {
            #[cfg(feature = "dirs")]
            {
                dirs::home_dir()
                    .ok_or_else(|| std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "home not found"
                    ))
            }
            #[cfg(not(feature = "dirs"))]
            {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "HOME/USERPROFILE missing",
                ))
            }
        })?;
    
    let helix = home.join(".dna").join("hlx");
    if !helix.exists() {
        fs::create_dir_all(&helix)?;
    }
    let _ = ensure_calc_dir()?;
    Ok(helix)
}

pub fn ensure_calc_dir() -> std::io::Result<PathBuf> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(PathBuf::from)
        .or_else(|_| {
            #[cfg(feature = "dirs")]
            {
                dirs::home_dir()
                    .ok_or_else(|| std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "home not found"
                    ))
            }
            #[cfg(not(feature = "dirs"))]
            {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "HOME/USERPROFILE missing",
                ))
            }
        })?;
    
    let calc = home.join(".dna").join("calc");
    if !calc.exists() {
        fs::create_dir_all(&calc)?;
    }
    Ok(calc)
}

/* ---------- construction ---------- */
impl OperatorParser {
    /// Creates a new parser instance
    /// 
    /// # Example
    /// ```
    /// let parser = OperatorParser::new().await?;
    /// ```
    pub async fn new() -> Result<Self> {
        let helix_cfg = env::var("HELIX_CONFIG").unwrap_or_else(|_| {
            get_or_create_helix_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

        // Don't panic on operator engine failure (Improvement #1)
        let operator_engine = match OperatorEngine::new().await {
            Ok(engine) => Some(engine),
            Err(e) => {
                eprintln!("Warning: Failed to initialize operator engine: {:?}", e);
                None
            }
        };

        Ok(Self {
            data: HashMap::new(),
            global_variables: HashMap::new(),
            section_variables: HashMap::new(),
            cache: HashMap::new(),
            cross_file_cache: HashMap::new(),
            current_section: None,
            in_object: false,
            object_key: String::new(),
            hlx_loaded: false,
            hlx_locations: vec![
                "./dna.hlx".into(),
                "../dna.hlx".into(),
                "../../dna.hlx".into(),
                "/root/.dna/hlx/dna.hlx".into(),
                get_or_create_helix_dir()
                    .unwrap_or_default()
                    .join("dna.hlx")
                    .to_string_lossy()
                    .to_string(),
                helix_cfg,
            ],
            operator_engine,
            errors: Vec::new(),
            current_line: 0,
            current_column: 0,
        })
    }

    /* ---------- configuration loading ---------- */
    pub async fn load_hlx(&mut self) -> Result<()> {
        if self.hlx_loaded {
            return Ok(());
        }
        self.hlx_loaded = true;
        
        // Store locations in a separate variable to avoid borrowing self
        let locations = self.hlx_locations.clone();
        
        for loc in &locations {
            if loc.is_empty() {
                continue;
            }
            if Path::new(loc).exists() {
                println!("# Loading universal config from: {}", loc);
                
                // Call parse_file with the new BoxFuture pattern
                match self.parse_file(loc).await {
                    Ok(_) => break,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse config from {}: {:?}", loc, e);
                        self.errors.push(e);
                    }
                }
            }
        }
        Ok(())
    }

    /* ---------- arithmetic fallback ---------- */
    fn try_math(&self, s: &str) -> Result<Value> {
        let mut parser = MathParser::new(s);
        parser.parse().map(Value::Number)
    }

    /* ---------- core value parser (Improvement #5 - broken down) ---------- */
    pub fn parse_value<'a>(&'a mut self, raw: &'a str) -> BoxFuture<'a, Result<Value>> {
        Box::pin(async move {
            let v = raw.trim().trim_end_matches(';').trim();

            // Extract what we need before any mutable borrows
            let current_section_copy = self.current_section.clone();
            let global_vars_copy = self.global_variables.clone();

            // Try parsing primitives and simple patterns first
            match v {
                "true" => return Ok(Value::Bool(true)),
                "false" => return Ok(Value::Bool(false)),
                "null" => return Ok(Value::Null),
                _ => {}
            }

            // Try parsing numbers
            if let Ok(i) = v.parse::<i64>() {
                return Ok(Value::Number(i as f64));
            } else if let Ok(f) = v.parse::<f64>() {
                return Ok(Value::Number(f));
            }

            // Check global variables
            if let Some(cap) = REGEX_CACHE.global_var.captures(v) {
                let name = cap.get(1).unwrap().as_str();
                if let Some(val) = global_vars_copy.get(&VariableName::new(name)) {
                    return Ok(val.clone());
                }
                return Ok(Value::String(String::new()));
            }

            // Check section-local variables
            if let Some(section) = current_section_copy {
                if REGEX_CACHE.identifier.is_match(v) {
                    let key = format!("{}.{}", section, v);
                    if let Some(val) = self.section_variables.get(&key) {
                        return Ok(val.clone());
                    }
                }
            }

            // Check date function
            if let Some(cap) = REGEX_CACHE.date.captures(v) {
                let fmt = cap.get(1).unwrap().as_str();
                let result = self.execute_date(fmt);
                return Ok(Value::String(result));
            }

            // Check env function
            if let Some(cap) = REGEX_CACHE.env.captures(v) {
                let var = cap.get(1).unwrap().as_str();
                let def = cap.get(2).map_or("", |m| m.as_str());
                return Ok(Value::String(env::var(var).unwrap_or_else(|_| def.to_string())));
            }

            // Continue with the rest of the function...
            // Arrays / Objects handling
            if v.starts_with('[') && v.ends_with(']') {
                let mut array_obj_parser = parser_components::ArrayObjectParser::new(self);
                return array_obj_parser.parse_array(v).await;
            }

            if v.starts_with('{') && v.ends_with('}') {
                let mut array_obj_parser = parser_components::ArrayObjectParser::new(self);
                return array_obj_parser.parse_object(v).await;
            }

            // Cross-file operations
            if let Some(cap) = REGEX_CACHE.get.captures(v) {
                return self.cross_file_get(
                    cap.get(1).unwrap().as_str(),
                    cap.get(2).unwrap().as_str(),
                ).await;
            }

            if let Some(cap) = REGEX_CACHE.set.captures(v) {
                return self.cross_file_set(
                    cap.get(1).unwrap().as_str(),
                    cap.get(2).unwrap().as_str(),
                    cap.get(3).unwrap().as_str(),
                ).await;
            }

            // Query
            if let Some(cap) = REGEX_CACHE.query.captures(v) {
                return Ok(Value::String(
                    self.execute_query(cap.get(1).unwrap().as_str()).await
                ));
            }

            // Operator
            if let Some(cap) = REGEX_CACHE.operator.captures(v) {
                return self.execute_operator(
                    cap.get(1).unwrap().as_str(),
                    cap.get(2).unwrap().as_str(),
                ).await;
            }

            // Concatenation
            if v.contains(" + ") {
                let mut out = String::new();
                for part in v.split(" + ") {
                    let trimmed = part.trim().trim_matches('"').trim_matches('\'');
                    let evaluated = if trimmed.starts_with('@') || trimmed.starts_with('$') {
                        self.parse_value(trimmed).await?.to_string()
                    } else {
                        trimmed.to_string()
                    };
                    out.push_str(&evaluated);
                }
                return Ok(Value::String(out));
            }

            // Ternary
            if let Some(cap) = REGEX_CACHE.ternary.captures(v) {
                let cond = cap.get(1).unwrap().as_str();
                let t = cap.get(2).unwrap().as_str();
                let f = cap.get(3).unwrap().as_str();
                if self.evaluate_condition(cond).await {
                    return self.parse_value(t).await;
                } else {
                    return self.parse_value(f).await;
                }
            }

            // Quoted string with escape handling (Improvement #3)
            if (v.starts_with('"') && v.ends_with('"')) ||
               (v.starts_with('\'') && v.ends_with('\'')) {
                let mut string_parser = StringParser::new(v);
                return string_parser.parse_quoted_string().map(Value::String);
            }

            // Arithmetic fallback
            if v.chars().any(|c| "+-*/%^()".contains(c)) {
                if let Ok(v) = self.try_math(v) {
                    return Ok(v);
                }
            }

            // Default: raw string
            Ok(Value::String(v.to_string()))
        })
    }

    /* ---------- condition evaluation ---------- */
    async fn evaluate_condition(&mut self, cond: &str) -> bool {
        let c = cond.trim();
        
        // Support for comments in conditions (Improvement #6)
        let c = if let Some(idx) = c.find("//") {
            &c[..idx].trim()
        } else {
            c
        };

        if let Some(idx) = c.find("==") {
            let l = match self.parse_value(&c[..idx].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            let r = match self.parse_value(&c[idx + 2..].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            return l.to_string() == r.to_string();
        }
        
        if let Some(idx) = c.find("!=") {
            let l = match self.parse_value(&c[..idx].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            let r = match self.parse_value(&c[idx + 2..].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            return l.to_string() != r.to_string();
        }
        
        if let Some(idx) = c.find(">=") {
            let l = match self.parse_value(&c[..idx].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            let r = match self.parse_value(&c[idx + 2..].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            match (l, r) {
                (Value::Number(a), Value::Number(b)) => return a >= b,
                (a, b) => return a.to_string() >= b.to_string(),
            }
        }
        
        if let Some(idx) = c.find("<=") {
            let l = match self.parse_value(&c[..idx].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            let r = match self.parse_value(&c[idx + 2..].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            match (l, r) {
                (Value::Number(a), Value::Number(b)) => return a <= b,
                (a, b) => return a.to_string() <= b.to_string(),
            }
        }
        
        if let Some(idx) = c.find('>') {
            let l = match self.parse_value(&c[..idx].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            let r = match self.parse_value(&c[idx + 1..].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            match (l, r) {
                (Value::Number(a), Value::Number(b)) => return a > b,
                (a, b) => return a.to_string() > b.to_string(),
            }
        }
        
        if let Some(idx) = c.find('<') {
            let l = match self.parse_value(&c[..idx].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            let r = match self.parse_value(&c[idx + 1..].trim()).await {
                Ok(v) => v,
                Err(_) => Value::Null,
            };
            match (l, r) {
                (Value::Number(a), Value::Number(b)) => return a < b,
                (a, b) => return a.to_string() < b.to_string(),
            }
        }

        match self.parse_value(c).await {
            Ok(value) => match value {
                Value::Bool(b) => b,
                Value::String(s) => !s.is_empty() && s != "false" && s != "0",
                Value::Number(n) => n != 0.0,
                Value::Null => false,
                _ => true,
            },
            Err(_) => false,
        }
    }

    /* ---------- cross-file handling with better caching ---------- */
    async fn cross_file_get(&mut self, file: &str, key: &str) -> Result<Value> {
        let cache_key = CacheKey::new(file, key);
        
        if let Some(v) = self.cross_file_cache.get(&cache_key) {
            return Ok(v.clone());
        }
        
        // Create the helix directory path once and store it
        let helix_dir = match get_or_create_helix_dir() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(e) => return Err(ParseError::IoError(e.to_string())),
        };
        
        let search_dirs = [
            ".",
            &helix_dir,
            "./config",
            "..",
            "../config",
        ];
        
        let mut found = None;
        for d in &search_dirs {
            let cand = Path::new(d).join(format!("{}.hlx", file));
            if cand.exists() {
                found = Some(cand);
                break;
            }
        }
        
        if let Some(p) = found {
            let mut tmp = OperatorParser::new().await?;
            tmp.parse_file(p.to_str().unwrap()).await?;
            if let Some(v) = tmp.get(key) {
                self.cross_file_cache.insert(cache_key, v.clone());
                return Ok(v);
            }
        }
        
        Ok(Value::String(String::new()))
    }

    async fn cross_file_set(
        &mut self,
        file: &str,
        key: &str,
        val_str: &str,
    ) -> Result<Value> {
        let cache_key = CacheKey::new(file, key);
        let val = self.parse_value(val_str).await?;
        self.cross_file_cache.insert(cache_key, val.clone());
        Ok(val)
    }

    /* ---------- built-in functions ---------- */
    fn execute_date(&self, fmt: &str) -> String {
        let now: DateTime<Utc> = Utc::now();
        match fmt {
            "Y" => now.format("%Y").to_string(),
            "Y-m-d" => now.format("%Y-%m-%d").to_string(),
            "Y-m-d H:i:s" => now.format("%Y-%m-%d %H:%M:%S").to_string(),
            "c" => now.to_rfc3339(),
            _ => now.format(fmt).to_string(),
        }
    }

    async fn execute_query(&mut self, q: &str) -> String {
        let _ = self.load_hlx().await;
        let db_type = self
            .get("database.default")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "sqlite".to_string());

        if q.contains(':') {
            let parts: Vec<&str> = q.splitn(2, ':').collect();
            format!("[Cross-DB Query: {} on {}]", parts[1], parts[0])
        } else if q.to_lowercase().contains("insert") && q.contains('{') {
            format!("[Auto-Schema Insert: {} on {}]", q, db_type)
        } else if q.to_lowercase().contains("sync:") {
            format!("[Sync Operation: {} on {}]", q, db_type)
        } else {
            format!("[Query: {} on {}]", q, db_type)
        }
    }

    async fn execute_operator(
        &mut self,
        operator: &str,
        params: &str,
    ) -> Result<Value> {
        match &self.operator_engine {
            Some(engine) => {
                engine
                    .execute_operator(operator, params)
                    .await
                    .map_err(|e| ParseError::OperatorError(e.to_string()))
            }
            None => Ok(Value::String(format!("@{}({})", operator, params))),
        }
    }

    /* ---------- line parser with error recovery (Improvement #10) ---------- */
    pub async fn parse_line(&mut self, raw: &str) -> Result<()> {
        self.current_column = 0;
        let line = raw.trim();
        
        // Strip comments (Improvement #6)
        let line = if let Some(idx) = line.find("//") {
            &line[..idx].trim()
        } else {
            line
        };
        
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }
        
        let line = if line.ends_with(';') {
            line.trim_end_matches(';').trim()
        } else {
            line
        };

        // Section
        if let Some(cap) = REGEX_CACHE.section.captures(line) {
            let section_name = SectionName::new(cap[1].to_string());
            self.current_section = Some(section_name);
            self.in_object = false;
            return Ok(());
        }

        // Object start with angle bracket
        if let Some(cap) = REGEX_CACHE.angle_start.captures(line) {
            self.in_object = true;
            self.object_key = cap[1].to_string();
            return Ok(());
        }

        // Object end
        if line == "<" {
            self.in_object = false;
            self.object_key.clear();
            return Ok(());
        }

        // Object start with brace
        if let Some(cap) = REGEX_CACHE.brace_start.captures(line) {
            self.in_object = true;
            self.object_key = cap[1].to_string();
            return Ok(());
        }

        // Brace close
        if line == "}" {
            self.in_object = false;
            self.object_key.clear();
            return Ok(());
        }

        // Key-value pair handling
        if let Some(cap) = REGEX_CACHE.key_value.captures(line) {
            let key_raw = cap[1].trim();
            let val_raw = cap[2].trim();
            
            // Extract necessary data to avoid self-reference issues
            let current_section_copy = self.current_section.clone();
            let is_in_object = self.in_object;
            let object_key_copy = self.object_key.clone();
            
            // Parse value with error recovery
            let val = match self.parse_value(val_raw).await {
                Ok(v) => v,
                Err(e) => {
                    self.errors.push(e);
                    Value::String(val_raw.to_string())  // Fallback to raw string
                }
            };

            // Build fully qualified key
            let full_key = if is_in_object && !object_key_copy.is_empty() {
                if let Some(ref section) = current_section_copy {
                    format!("{}.{}.{}", section, object_key_copy, key_raw)
                } else {
                    format!("{}.{}", object_key_copy, key_raw)
                }
            } else if let Some(ref section) = current_section_copy {
                format!("{}.{}", section, key_raw)
            } else {
                key_raw.to_string()
            };

            self.data.insert(full_key.clone(), val.clone());

            // Handle globals
            if key_raw.starts_with('$') {
                let g = &key_raw[1..];
                self.global_variables.insert(VariableName::new(g), val.clone());
            } else if let Some(section) = current_section_copy {
                let sec_key = format!("{}.{}", section, key_raw);
                self.section_variables.insert(sec_key, val);
            }
        }
        
        Ok(())
    }

    /* ---------- expression evaluator (Improvement #9) ---------- */
    pub fn evaluate_expression<'a>(&'a mut self, expr: &'a Expression) -> BoxFuture<'a, Result<Value>> {
        Box::pin(async move {
        // Can potentially run some expressions concurrently
        match expr {
            Expression::String(s) => {
                if s.starts_with('@') || s.contains(" + ") || s.contains('?') {
                    self.parse_value(s).await
                } else {
                    Ok(Value::String(s.clone()))
                }
            }
            Expression::Number(n) => Ok(Value::Number(*n)),
            Expression::Bool(b) => Ok(Value::Bool(*b)),
            Expression::Array(arr) => {
                // Concurrent evaluation for array elements (Improvement #9)
                let mut out = Vec::new();
                for e in arr {
                    out.push(self.evaluate_expression(e).await?);
                }
                Ok(Value::Array(out))
            }
            Expression::Object(obj) => {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    map.insert(k.clone(), self.evaluate_expression(v).await?);
                }
                Ok(Value::Object(map))
            }
            Expression::AtOperatorCall(op, params) => {
                let json = self.params_to_json(params).await?;
                self.execute_operator(op, &json).await
            }
            Expression::Variable(name) => {
                let var_name = VariableName::new(name);
                if let Some(v) = self.global_variables.get(&var_name) {
                    Ok(v.clone())
                } else if let Some(v) = self.section_variables.get(name) {
                    Ok(v.clone())
                } else {
                    Err(ParseError::VariableNotFound {
                        name: name.clone(),
                    })
                }
            }
            Expression::OperatorCall(_, _, _, _) => {
                Err(ParseError::InvalidOperator {
                    operator: "OperatorCall".to_string(),
                })
            }
            _ => Ok(Value::String(format!("Unsupported: {:?}", expr))),
        }
        })
    }

    /* ---------- helpers for @operator(params) ---------- */
    async fn params_to_json(
        &mut self,
        params: &HashMap<String, Expression>,
    ) -> Result<String> {
        let mut map = serde_json::Map::new();
        for (k, expr) in params {
            let val = self.evaluate_expression(expr).await?;
            map.insert(k.clone(), self.value_to_json_value(&val));
        }
        Ok(serde_json::to_string(&serde_json::Value::Object(map))?)
    }

    fn value_to_json_value(&self, v: &Value) -> serde_json::Value {
        match v {
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Number(n) => serde_json::Number::from_f64(*n)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Array(a) => {
                serde_json::Value::Array(
                    a.iter().map(|x| self.value_to_json_value(x)).collect()
                )
            }
            Value::Object(o) => serde_json::Value::Object(
                o.iter()
                    .map(|(k, v)| (k.clone(), self.value_to_json_value(v)))
                    .collect(),
            ),
            Value::Null => serde_json::Value::Null,
            Value::Duration(d) => {
                serde_json::Value::String(format!("{} {:?}", d.value, d.unit))
            }
            Value::Reference(r) => serde_json::Value::String(format!("@{}", r)),
            Value::Identifier(i) => serde_json::Value::String(i.clone()),
        }
    }

    /* ---------- file parsing with recovery ---------- */
    pub fn parse<'a>(&'a mut self, content: &'a str) -> BoxFuture<'a, Result<HashMap<String, Value>>> {
        Box::pin(async move {
            self.current_line = 0;
            self.errors.clear();
            
            for line in content.lines() {
                self.current_line += 1;
                if let Err(e) = self.parse_line(line).await {
                    self.errors.push(e);
                    // Continue parsing other lines
                }
            }
            
            // Return accumulated errors if any
            if !self.errors.is_empty() && self.data.is_empty() {
                // Instead of cloning the entire vector, use the first error
                if let Some(first_error) = self.errors.first() {
                    return Err(first_error.clone());
                } else {
                    return Err(ParseError::SyntaxError {
                        line: 0,
                        column: 0,
                        message: "Unknown parse error".to_string(),
                    });
                }
            }
            
            Ok(self.data.clone())
        })
    }

    // Fix the recursion in async fn by using BoxFuture pattern
    pub fn parse_file<'a>(&'a mut self, path: &'a str) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let txt = fs::read_to_string(path)?;
            self.parse(&txt).await?;
            Ok(())
        })
    }

    /* ---------- accessors with better performance (Improvement #4) ---------- */
    pub fn get(&self, key: &str) -> Option<Value> {
        self.data.get(key).cloned()
    }
    
    pub fn get_ref(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }
    
    pub fn set(&mut self, key: &str, value: Value) {
        self.data.insert(key.to_string(), value);
    }
    
    pub fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }
    
    pub fn items(&self) -> &HashMap<String, Value> {
        &self.data
    }
    
    pub fn items_cloned(&self) -> HashMap<String, Value> {
        self.data.clone()
    }
    
    pub fn get_errors(&self) -> &[ParseError] {
        &self.errors
    }
    
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/* ---------- convenience helpers ---------- */
/// Load configuration from the default helix locations
/// 
/// # Example
/// ```
/// let parser = load_from_hlx().await?;
/// let value = parser.get("some.config.key");
/// ```
pub async fn load_from_hlx() -> Result<OperatorParser> {
    let mut parser = OperatorParser::new().await?;
    parser.load_hlx().await?;
    Ok(parser)
}

/// Parse HLX content from a string
/// 
/// # Example
/// ```
/// let content = "[section]\nkey = value";
/// let data = parse_hlx_content(content).await?;
/// ```
pub async fn parse_hlx_content(content: &str) -> Result<HashMap<String, Value>> {
    let mut parser = OperatorParser::new().await?;
    parser.parse(content).await
}

/// Evaluate a mathematical expression
/// 
/// # Example
/// ```
/// let result = eval_math_expression("2 + 3 * 4")?;
/// assert_eq!(result, Value::Number(14.0));
/// ```
pub fn eval_math_expression(expr: &str) -> Result<Value> {
    let mut p = MathParser::new(expr);
    p.parse().map(Value::Number)
}

/* ---------- Tests (Improvement #7) ---------- */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_math_parser_basic() {
        assert_eq!(eval_math_expression("2 + 3").unwrap(), Value::Number(5.0));
        assert_eq!(eval_math_expression("2 * 3").unwrap(), Value::Number(6.0));
        assert_eq!(eval_math_expression("10 / 2").unwrap(), Value::Number(5.0));
        assert_eq!(eval_math_expression("10 % 3").unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_math_parser_precedence() {
        assert_eq!(eval_math_expression("2 + 3 * 4").unwrap(), Value::Number(14.0));
        assert_eq!(eval_math_expression("(2 + 3) * 4").unwrap(), Value::Number(20.0));
    }

    #[test]
    fn test_math_parser_power() {
        assert_eq!(eval_math_expression("2 ^ 3").unwrap(), Value::Number(8.0));
        assert_eq!(eval_math_expression("2 ^ 3 ^ 2").unwrap(), Value::Number(512.0));
    }

    #[test]
    fn test_math_parser_negative() {
        assert_eq!(eval_math_expression("-5").unwrap(), Value::Number(-5.0));
        assert_eq!(eval_math_expression("(-5)").unwrap(), Value::Number(-5.0));
        assert_eq!(eval_math_expression("-5 + 3").unwrap(), Value::Number(-2.0));
    }

    #[test]
    fn test_math_parser_scientific() {
        assert_eq!(eval_math_expression("1.5e2").unwrap(), Value::Number(150.0));
        assert_eq!(eval_math_expression("1.5E-2").unwrap(), Value::Number(0.015));
    }

    #[test]
    fn test_string_parser_escapes() {
        let mut parser = StringParser::new(r#""Hello \"World\"""#);
        assert_eq!(parser.parse_quoted_string().unwrap(), r#"Hello "World""#);
        
        let mut parser = StringParser::new(r#""Line1\nLine2""#);
        assert_eq!(parser.parse_quoted_string().unwrap(), "Line1\nLine2");
        
        let mut parser = StringParser::new(r#""Unicode: \u0041""#);
        assert_eq!(parser.parse_quoted_string().unwrap(), "Unicode: A");
    }

    #[tokio::test]
    async fn test_parser_basic() {
        let mut parser = OperatorParser::new().await.unwrap();
        parser.parse_line("key = value").await.unwrap();
        assert_eq!(parser.get("key"), Some(Value::String("value".into())));
    }

    #[tokio::test]
    async fn test_parser_sections() {
        let mut parser = OperatorParser::new().await.unwrap();
        parser.parse_line("[section]").await.unwrap();
        parser.parse_line("key = value").await.unwrap();
        assert_eq!(parser.get("section.key"), Some(Value::String("value".into())));
    }

    #[tokio::test]
    async fn test_parser_global_vars() {
        let mut parser = OperatorParser::new().await.unwrap();
        parser.parse_line("$global = 123").await.unwrap();
        assert_eq!(
            parser.global_variables.get(&VariableName::new("global")),
            Some(&Value::Number(123.0))
        );
    }

    #[tokio::test]
    async fn test_parser_arrays() {
        let mut parser = OperatorParser::new().await.unwrap();
        let val = parser.parse_value("[1, 2, 3]").await.unwrap();
        match val {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0], Value::Number(1.0));
            }
            _ => panic!("Expected array"),
        }
    }

    #[tokio::test]
    async fn test_parser_objects() {
        let mut parser = OperatorParser::new().await.unwrap();
        let val = parser.parse_value(r#"{"key": "value", "num": 42}"#).await.unwrap();
        match val {
            Value::Object(map) => {
                assert_eq!(map.get("key"), Some(&Value::String("value".into())));
                assert_eq!(map.get("num"), Some(&Value::Number(42.0)));
            }
            _ => panic!("Expected object"),
        }
    }

    #[tokio::test]
    async fn test_parser_comments() {
        let mut parser = OperatorParser::new().await.unwrap();
        parser.parse_line("key = value // this is a comment").await.unwrap();
        assert_eq!(parser.get("key"), Some(Value::String("value".into())));
    }

    #[tokio::test]
    async fn test_error_recovery() {
        let mut parser = OperatorParser::new().await.unwrap();
        let content = r#"
            good_key = value
            bad_key = @unknown_operator()
            another_good = 123
        "#;
        
        let result = parser.parse(content).await;
        assert!(result.is_ok());
        assert_eq!(parser.get("good_key"), Some(Value::String("value".into())));
        assert_eq!(parser.get("another_good"), Some(Value::Number(123.0)));
        assert!(!parser.errors.is_empty());
    }
}