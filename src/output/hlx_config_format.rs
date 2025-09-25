use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::HlxError;

/// Enhanced HLX Config Parser with proper AST-based parsing
/// Supports full HLX syntax including nested structures, arrays, and complex expressions

/// HLX Config Format (.hlx files) - Text-based configuration
/// This handles human-readable configuration files with .hlx extension

/// HLX Config structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxConfig {
    /// Configuration sections
    pub sections: HashMap<String, HlxSection>,
    /// Global metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HlxSection {
    /// Section properties
    pub properties: HashMap<String, serde_json::Value>,
    /// Section metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Enhanced Parser Structures for AST-based parsing

/// Source location for error reporting
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub position: usize,
}

/// Parse error with location information
#[derive(Debug, Clone)]
pub struct ConfigParseError {
    pub message: String,
    pub location: Option<SourceLocation>,
    pub context: String,
}

impl std::fmt::Display for ConfigParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(loc) = &self.location {
            write!(f, "{} at line {}, column {}", self.message, loc.line, loc.column)?;
            if !self.context.is_empty() {
                write!(f, " (context: {})", self.context)?;
            }
        } else {
            write!(f, "{}", self.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigParseError {}

/// Tokens for config parsing
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigToken {
    Identifier(String),
    String(String),
    Number(f64),
    Bool(bool),
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Equals,
    Comma,
    Comment(String),
    Newline,
    Eof,
}

impl ConfigToken {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            ConfigToken::String(s) => Some(s),
            ConfigToken::Identifier(s) => Some(s),
            _ => None,
        }
    }
}

/// Token with location information
#[derive(Debug, Clone)]
pub struct TokenWithLocation {
    pub token: ConfigToken,
    pub location: SourceLocation,
}

/// AST nodes for config parsing
#[derive(Debug, Clone)]
pub enum ConfigValue {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
    Null,
}

impl ConfigValue {
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            ConfigValue::String(s) => serde_json::Value::String(s.clone()),
            ConfigValue::Number(n) => serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0))),
            ConfigValue::Bool(b) => serde_json::Value::Bool(*b),
            ConfigValue::Array(arr) => serde_json::Value::Array(arr.iter().map(|v| v.to_json_value()).collect()),
            ConfigValue::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, v) in obj {
                    map.insert(k.clone(), v.to_json_value());
                }
                serde_json::Value::Object(map)
            }
            ConfigValue::Null => serde_json::Value::Null,
        }
    }
}

/// Configuration block AST
#[derive(Debug, Clone)]
pub struct ConfigBlock {
    pub name: String,
    pub properties: HashMap<String, ConfigValue>,
    pub location: SourceLocation,
}

/// Enhanced config parser with lexer and AST
pub struct EnhancedConfigParser {
    tokens: Vec<TokenWithLocation>,
    current: usize,
}

impl EnhancedConfigParser {
    /// Create a new parser from source text
    pub fn new(source: &str) -> Result<Self, ConfigParseError> {
        let tokens = Self::tokenize(source)?;
        Ok(Self { tokens, current: 0 })
    }

    /// Tokenize the source text
    fn tokenize(source: &str) -> Result<Vec<TokenWithLocation>, ConfigParseError> {
        let mut tokens = Vec::new();
        let mut chars = source.chars().peekable();
        let mut line = 1;
        let mut column = 1;
        let mut position = 0;

        while let Some(&ch) = chars.peek() {
            let start_location = SourceLocation { line, column, position };

            match ch {
                '#' => {
                    // Comment - consume until end of line
                    let mut comment = String::new();
                    chars.next(); // consume '#'
                    column += 1;
                    position += 1;
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        comment.push(c);
                        chars.next();
                        column += 1;
                        position += 1;
                    }
                    tokens.push(TokenWithLocation {
                        token: ConfigToken::Comment(comment),
                        location: start_location,
                    });
                }
                '"' => {
                    // String literal
                    chars.next(); // consume opening quote
                    column += 1;
                    position += 1;
                    let mut string = String::new();
                    let mut escaped = false;
                    while let Some(c) = chars.next() {
                        column += 1;
                        position += 1;
                        if escaped {
                            match c {
                                'n' => string.push('\n'),
                                't' => string.push('\t'),
                                'r' => string.push('\r'),
                                '"' => string.push('"'),
                                '\\' => string.push('\\'),
                                _ => string.push(c),
                            }
                            escaped = false;
                        } else if c == '\\' {
                            escaped = true;
                        } else if c == '"' {
                            break;
                        } else {
                            string.push(c);
                        }
                    }
                    tokens.push(TokenWithLocation {
                        token: ConfigToken::String(string),
                        location: start_location,
                    });
                }
                '0'..='9' | '-' => {
                    // Number
                    let mut num_str = String::new();
                    let mut has_dot = false;
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() || c == '.' || c == '-' {
                            if c == '.' {
                                if has_dot {
                                    break;
                                }
                                has_dot = true;
                            }
                            num_str.push(c);
                            chars.next();
                            column += 1;
                            position += 1;
                        } else {
                            break;
                        }
                    }
                    match num_str.parse::<f64>() {
                        Ok(num) => tokens.push(TokenWithLocation {
                            token: ConfigToken::Number(num),
                            location: start_location,
                        }),
                        Err(_) => {
                            return Err(ConfigParseError {
                                message: format!("Invalid number: {}", num_str),
                                location: Some(start_location),
                                context: "expected valid numeric value".to_string(),
                            });
                        }
                    }
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    // Identifier or keyword
                    let mut ident = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            ident.push(c);
                            chars.next();
                            column += 1;
                            position += 1;
                        } else {
                            break;
                        }
                    }
                    let token = match ident.as_str() {
                        "true" => ConfigToken::Bool(true),
                        "false" => ConfigToken::Bool(false),
                        "null" => ConfigToken::Identifier("null".to_string()),
                        _ => ConfigToken::Identifier(ident),
                    };
                    tokens.push(TokenWithLocation {
                        token,
                        location: start_location,
                    });
                }
                '=' => {
                    tokens.push(TokenWithLocation {
                        token: ConfigToken::Equals,
                        location: start_location,
                    });
                    chars.next();
                    column += 1;
                    position += 1;
                }
                '{' => {
                    tokens.push(TokenWithLocation {
                        token: ConfigToken::LeftBrace,
                        location: start_location,
                    });
                    chars.next();
                    column += 1;
                    position += 1;
                }
                '}' => {
                    tokens.push(TokenWithLocation {
                        token: ConfigToken::RightBrace,
                        location: start_location,
                    });
                    chars.next();
                    column += 1;
                    position += 1;
                }
                '[' => {
                    tokens.push(TokenWithLocation {
                        token: ConfigToken::LeftBracket,
                        location: start_location,
                    });
                    chars.next();
                    column += 1;
                    position += 1;
                }
                ']' => {
                    tokens.push(TokenWithLocation {
                        token: ConfigToken::RightBracket,
                        location: start_location,
                    });
                    chars.next();
                    column += 1;
                    position += 1;
                }
                ',' => {
                    tokens.push(TokenWithLocation {
                        token: ConfigToken::Comma,
                        location: start_location,
                    });
                    chars.next();
                    column += 1;
                    position += 1;
                }
                '\n' => {
                    tokens.push(TokenWithLocation {
                        token: ConfigToken::Newline,
                        location: start_location,
                    });
                    chars.next();
                    line += 1;
                    column = 1;
                    position += 1;
                }
                ' ' | '\t' | '\r' => {
                    // Skip whitespace
                    chars.next();
                    column += 1;
                    position += 1;
                }
                _ => {
                    return Err(ConfigParseError {
                        message: format!("Unexpected character: {}", ch),
                        location: Some(start_location),
                        context: "expected valid token".to_string(),
                    });
                }
            }
        }

        tokens.push(TokenWithLocation {
            token: ConfigToken::Eof,
            location: SourceLocation { line, column, position },
        });

        Ok(tokens)
    }

    /// Parse the configuration
    pub fn parse(&mut self) -> Result<HlxConfig, ConfigParseError> {
        let mut config = HlxConfig {
            sections: HashMap::new(),
            metadata: HashMap::new(),
        };

        while !self.is_at_end() {
            match self.current_token().token.clone() {
                ConfigToken::Comment(_) => {
                    self.advance();
                }
                ConfigToken::Newline => {
                    self.advance();
                }
                ConfigToken::Identifier(ref ident) => {
                    if self.peek_next().map(|t| t.token == ConfigToken::LeftBrace).unwrap_or(false) {
                        // Block syntax: identifier { ... }
                        let block = self.parse_block()?;
                        let section = HlxSection {
                            properties: block.properties.into_iter()
                                .map(|(k, v)| (k, v.to_json_value()))
                                .collect(),
                            metadata: None,
                        };
                        config.sections.insert(block.name, section);
                    } else if self.peek_next().map(|t| t.token == ConfigToken::Equals).unwrap_or(false) {
                        // Property syntax: key = value
                        let key = ident.clone();
                        self.advance(); // consume identifier
                        self.advance(); // consume equals
                        let value = self.parse_value()?;
                        config.metadata.insert(key, value.to_json_value());
                    } else {
                        self.advance();
                    }
                }
                ConfigToken::String(ref s) => {
                    if self.peek_next().map(|t| t.token == ConfigToken::LeftBrace).unwrap_or(false) {
                        // Block syntax with quoted name: "name" { ... }
                        let block = self.parse_block()?;
                        let section = HlxSection {
                            properties: block.properties.into_iter()
                                .map(|(k, v)| (k, v.to_json_value()))
                                .collect(),
                            metadata: None,
                        };
                        config.sections.insert(block.name, section);
                    } else if self.peek_next().map(|t| t.token == ConfigToken::Equals).unwrap_or(false) {
                        // Property syntax with quoted key: "key" = value
                        let key = s.clone();
                        self.advance(); // consume string
                        self.advance(); // consume equals
                        let value = self.parse_value()?;
                        config.metadata.insert(key, value.to_json_value());
                    } else {
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }

        Ok(config)
    }

    /// Parse a block: identifier { properties }
    fn parse_block(&mut self) -> Result<ConfigBlock, ConfigParseError> {
        let start_location = self.current_token().location.clone();
        let name = match &self.current_token().token {
            ConfigToken::Identifier(s) => s.clone(),
            ConfigToken::String(s) => s.clone(),
            _ => {
                return Err(ConfigParseError {
                    message: "Expected identifier or string for block name".to_string(),
                    location: Some(self.current_token().location.clone()),
                    context: "block syntax: name { ... }".to_string(),
                });
            }
        };

        self.advance(); // consume name
        self.expect_token(ConfigToken::LeftBrace)?;

        let mut properties = HashMap::new();

        while !self.is_at_end() && self.current_token().token != ConfigToken::RightBrace {
            match &self.current_token().token {
                ConfigToken::Comment(_) | ConfigToken::Newline => {
                    self.advance();
                    continue;
                }
                ConfigToken::Identifier(key) | ConfigToken::String(key) => {
                    let prop_key = key.clone();
                    self.advance(); // consume key

                    if self.current_token().token == ConfigToken::LeftBrace {
                        // Nested block
                        let nested_block = self.parse_block()?;
                        let mut nested_props = HashMap::new();
                        for (k, v) in nested_block.properties {
                            nested_props.insert(k, v);
                        }
                        properties.insert(prop_key, ConfigValue::Object(nested_props));
                    } else {
                        self.expect_token(ConfigToken::Equals)?;
                        let value = self.parse_value()?;
                        properties.insert(prop_key, value);
                    }
                }
                _ => {
                    return Err(ConfigParseError {
                        message: "Expected property key or closing brace".to_string(),
                        location: Some(self.current_token().location.clone()),
                        context: "property syntax: key = value".to_string(),
                    });
                }
            }
        }

        self.expect_token(ConfigToken::RightBrace)?;

        Ok(ConfigBlock {
            name,
            properties,
            location: start_location,
        })
    }

    /// Parse a value (string, number, bool, array, object)
    fn parse_value(&mut self) -> Result<ConfigValue, ConfigParseError> {
        match &self.current_token().token {
            ConfigToken::String(s) => {
                let value = ConfigValue::String(s.clone());
                self.advance();
                Ok(value)
            }
            ConfigToken::Number(n) => {
                let value = ConfigValue::Number(*n);
                self.advance();
                Ok(value)
            }
            ConfigToken::Bool(b) => {
                let value = ConfigValue::Bool(*b);
                self.advance();
                Ok(value)
            }
            ConfigToken::Identifier(s) if s == "null" => {
                self.advance();
                Ok(ConfigValue::Null)
            }
            ConfigToken::LeftBracket => {
                self.parse_array()
            }
            ConfigToken::LeftBrace => {
                self.parse_object()
            }
            _ => {
                Err(ConfigParseError {
                    message: "Expected value (string, number, bool, array, or object)".to_string(),
                    location: Some(self.current_token().location.clone()),
                    context: "supported value types: strings, numbers, booleans, arrays [...], objects {...}".to_string(),
                })
            }
        }
    }

    /// Parse an array: [ value1, value2, ... ]
    fn parse_array(&mut self) -> Result<ConfigValue, ConfigParseError> {
        self.expect_token(ConfigToken::LeftBracket)?;
        let mut values = Vec::new();

        while !self.is_at_end() && self.current_token().token != ConfigToken::RightBracket {
            if matches!(self.current_token().token, ConfigToken::Comment(_) | ConfigToken::Newline | ConfigToken::Comma) {
                self.advance();
                continue;
            }

            values.push(self.parse_value()?);
        }

        self.expect_token(ConfigToken::RightBracket)?;
        Ok(ConfigValue::Array(values))
    }

    /// Parse an object: { key1 = value1, key2 = value2, ... }
    fn parse_object(&mut self) -> Result<ConfigValue, ConfigParseError> {
        self.expect_token(ConfigToken::LeftBrace)?;
        let mut properties = HashMap::new();

        while !self.is_at_end() && self.current_token().token != ConfigToken::RightBrace {
            if matches!(self.current_token().token, ConfigToken::Comment(_) | ConfigToken::Newline | ConfigToken::Comma) {
                self.advance();
                continue;
            }

            let key = match &self.current_token().token {
                ConfigToken::Identifier(s) | ConfigToken::String(s) => s.clone(),
                _ => {
                    return Err(ConfigParseError {
                        message: "Expected property key".to_string(),
                        location: Some(self.current_token().location.clone()),
                        context: "object property syntax: key = value".to_string(),
                    });
                }
            };

            self.advance(); // consume key
            self.expect_token(ConfigToken::Equals)?;
            let value = self.parse_value()?;
            properties.insert(key, value);
        }

        self.expect_token(ConfigToken::RightBrace)?;
        Ok(ConfigValue::Object(properties))
    }

    /// Get current token
    fn current_token(&self) -> &TokenWithLocation {
        &self.tokens[self.current]
    }

    /// Check if at end of tokens
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || matches!(self.current_token().token, ConfigToken::Eof)
    }

    /// Advance to next token
    fn advance(&mut self) {
        if !self.is_at_end() {
            self.current += 1;
        }
    }

    /// Peek at next token without advancing
    fn peek_next(&self) -> Option<&TokenWithLocation> {
        if self.current + 1 < self.tokens.len() {
            Some(&self.tokens[self.current + 1])
        } else {
            None
        }
    }

    /// Expect a specific token, advance if found
    fn expect_token(&mut self, expected: ConfigToken) -> Result<(), ConfigParseError> {
        if self.current_token().token == expected {
            self.advance();
            Ok(())
        } else {
            Err(ConfigParseError {
                message: format!("Expected {:?}, found {:?}", expected, self.current_token().token),
                location: Some(self.current_token().location.clone()),
                context: format!("expected token: {:?}", expected),
            })
        }
    }
}

/// HLX Config Reader/Writer
pub struct HlxConfigHandler;

impl HlxConfigHandler {
    /// Read HLX config from file
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<HlxConfig, HlxError> {
        let content = fs::read_to_string(&path)
            .map_err(|e| HlxError::io_error(
                format!("Failed to read HLX config file: {}", e),
                format!("Check if file exists and is readable: {}", path.as_ref().display())
            ))?;

        Self::parse_content(&content)
    }

    /// Write HLX config to file
    pub fn write_to_file<P: AsRef<Path>>(config: &HlxConfig, path: P) -> Result<(), HlxError> {
        let content = Self::serialize_config(config)?;

        fs::write(&path, content)
            .map_err(|e| HlxError::io_error(
                format!("Failed to write HLX config file: {}", e),
                format!("Check write permissions: {}", path.as_ref().display())
            ))
    }

    /// Parse HLX config content with enhanced AST-based parsing
    pub fn parse_content(content: &str) -> Result<HlxConfig, HlxError> {
        // Try enhanced parser first (full HLX syntax)
        match EnhancedConfigParser::new(content) {
            Ok(mut parser) => {
                match parser.parse() {
                    Ok(config) => return Ok(config),
                    Err(parse_err) => {
                        // Fall back to legacy parser for backward compatibility
                        eprintln!("Enhanced parser failed: {}, falling back to legacy parser", parse_err);
                        return Self::parse_content_legacy(content);
                    }
                }
            }
            Err(lex_err) => {
                // Fall back to legacy parser for backward compatibility
                eprintln!("Enhanced parser lexer failed: {}, falling back to legacy parser", lex_err);
                return Self::parse_content_legacy(content);
            }
        }
    }

    /// Legacy TOML-like parsing for backward compatibility
    fn parse_content_legacy(content: &str) -> Result<HlxConfig, HlxError> {
        let mut sections = HashMap::new();
        let mut metadata = HashMap::new();
        let mut current_section = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                // Section header
                let section_name = &line[1..line.len() - 1];
                current_section = Some(section_name.to_string());
                sections.insert(section_name.to_string(), HlxSection {
                    properties: HashMap::new(),
                    metadata: None,
                });
            } else if let Some(section_name) = &current_section {
                // Property in section
                if let Some((key, value)) = Self::parse_property(line) {
                    if let Some(section) = sections.get_mut(section_name) {
                        section.properties.insert(key, value);
                    }
                }
            } else {
                // Global property
                if let Some((key, value)) = Self::parse_property(line) {
                    metadata.insert(key, value);
                }
            }
        }

        Ok(HlxConfig { sections, metadata })
    }

    /// Serialize config to string
    pub fn serialize_config(config: &HlxConfig) -> Result<String, HlxError> {
        let mut output = String::new();

        // Write metadata
        for (key, value) in &config.metadata {
            output.push_str(&format!("{} = {}\n", key, Self::serialize_value(value)));
        }

        // Write sections
        for (section_name, section) in &config.sections {
            output.push_str(&format!("\n[{}]\n", section_name));

            for (key, value) in &section.properties {
                output.push_str(&format!("{} = {}\n", key, Self::serialize_value(value)));
            }
        }

        Ok(output)
    }

    /// Parse a property line (key = value)
    fn parse_property(line: &str) -> Option<(String, serde_json::Value)> {
        let parts: Vec<&str> = line.splitn(2, '=').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let key = parts[0].to_string();
            let value_str = parts[1];

            // Try to parse as JSON, fallback to string
            if let Ok(value) = serde_json::from_str(value_str) {
                Some((key, value))
            } else {
                Some((key, serde_json::Value::String(value_str.to_string())))
            }
        } else {
            None
        }
    }

    /// Serialize a value to string
    fn serialize_value(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => format!("\"{}\"", s),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_config() {
        let content = r#"
# Global metadata
version = "1.0"
name = "test"

[database]
host = "localhost"
port = 5432

[logging]
level = "info"
file = "/var/log/app.log"
"#;

        let config = HlxConfigHandler::parse_content(content).unwrap();

        assert_eq!(config.metadata.get("version").unwrap().as_str().unwrap(), "1.0");
        assert_eq!(config.metadata.get("name").unwrap().as_str().unwrap(), "test");

        assert!(config.sections.contains_key("database"));
        assert!(config.sections.contains_key("logging"));

        let db_section = &config.sections["database"];
        assert_eq!(db_section.properties.get("host").unwrap().as_str().unwrap(), "localhost");
        assert_eq!(db_section.properties.get("port").unwrap().as_i64().unwrap(), 5432);
    }

    #[test]
    fn test_parse_enhanced_hlx_syntax() {
        let content = r#"
# Enhanced HLX syntax test
version = "2.0"
enabled = true

project "test-project" {
    name = "Test Project"
    version = "1.0.0"
    description = "A test project"

    database {
        host = "localhost"
        port = 5432
        credentials = {
            username = "admin"
            password = "secret"
        }
        features = ["ssl", "pooling", "metrics"]
    }

    agents = [
        "agent1",
        "agent2",
        "agent3"
    ]

    settings {
        debug = true
        timeout = 30.5
        retries = 3
    }
}

logging {
    level = "info"
    file = "/var/log/app.log"
    format = "json"
}
"#;

        let config = HlxConfigHandler::parse_content(content).unwrap();

        // Test global metadata
        assert_eq!(config.metadata.get("version").unwrap().as_str().unwrap(), "2.0");
        assert_eq!(config.metadata.get("enabled").unwrap().as_bool().unwrap(), true);

        // Test project section
        assert!(config.sections.contains_key("test-project"));
        let project = &config.sections["test-project"];
        assert_eq!(project.properties.get("name").unwrap().as_str().unwrap(), "Test Project");
        assert_eq!(project.properties.get("version").unwrap().as_str().unwrap(), "1.0.0");

        // Test nested database object
        let db_obj = project.properties.get("database").unwrap().as_object().unwrap();
        assert_eq!(db_obj.get("host").unwrap().as_str().unwrap(), "localhost");
        assert_eq!(db_obj.get("port").unwrap().as_i64().unwrap(), 5432);

        // Test nested credentials object
        let creds = db_obj.get("credentials").unwrap().as_object().unwrap();
        assert_eq!(creds.get("username").unwrap().as_str().unwrap(), "admin");
        assert_eq!(creds.get("password").unwrap().as_str().unwrap(), "secret");

        // Test array
        let features = db_obj.get("features").unwrap().as_array().unwrap();
        assert_eq!(features.len(), 3);
        assert_eq!(features[0].as_str().unwrap(), "ssl");
        assert_eq!(features[1].as_str().unwrap(), "pooling");
        assert_eq!(features[2].as_str().unwrap(), "metrics");

        // Test agents array
        let agents = project.properties.get("agents").unwrap().as_array().unwrap();
        assert_eq!(agents.len(), 3);
        assert_eq!(agents[0].as_str().unwrap(), "agent1");
        assert_eq!(agents[1].as_str().unwrap(), "agent2");
        assert_eq!(agents[2].as_str().unwrap(), "agent3");

        // Test settings object
        let settings = project.properties.get("settings").unwrap().as_object().unwrap();
        assert_eq!(settings.get("debug").unwrap().as_bool().unwrap(), true);
        assert_eq!(settings.get("timeout").unwrap().as_f64().unwrap(), 30.5);
        assert_eq!(settings.get("retries").unwrap().as_i64().unwrap(), 3);

        // Test logging section
        assert!(config.sections.contains_key("logging"));
        let logging = &config.sections["logging"];
        assert_eq!(logging.properties.get("level").unwrap().as_str().unwrap(), "info");
        assert_eq!(logging.properties.get("file").unwrap().as_str().unwrap(), "/var/log/app.log");
        assert_eq!(logging.properties.get("format").unwrap().as_str().unwrap(), "json");
    }

    #[test]
    fn test_enhanced_parser_error_reporting() {
        // Test invalid syntax
        let content = r#"
project "test" {
    name = "test"
    invalid_syntax = [
"#;

        let result = EnhancedConfigParser::new(content);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.message.contains("Unexpected character") || err.message.contains("Expected"));
        }
    }

    #[test]
    fn test_backward_compatibility() {
        // Test that legacy TOML-like syntax still works
        let content = r#"
# Legacy format
version = "1.0"
name = "legacy"

[database]
host = "localhost"
port = 5432

[logging]
level = "info"
"#;

        let config = HlxConfigHandler::parse_content(content).unwrap();

        // Should parse correctly with legacy parser fallback
        assert_eq!(config.metadata.get("version").unwrap().as_str().unwrap(), "1.0");
        assert_eq!(config.metadata.get("name").unwrap().as_str().unwrap(), "legacy");
        assert!(config.sections.contains_key("database"));
        assert!(config.sections.contains_key("logging"));
    }

    #[test]
    fn test_serialize_config() {
        let mut config = HlxConfig::default();
        config.metadata.insert("version".to_string(), serde_json::Value::String("1.0".to_string()));

        let mut db_section = HlxSection::default();
        db_section.properties.insert("host".to_string(), serde_json::Value::String("localhost".to_string()));

        config.sections.insert("database".to_string(), db_section);

        let output = HlxConfigHandler::serialize_config(&config).unwrap();
        assert!(output.contains("version = \"1.0\""));
        assert!(output.contains("[database]"));
        assert!(output.contains("host = \"localhost\""));
    }

    #[test]
    fn test_complex_hlx_config() {
        let content = r#"
# Complex HLX configuration
app_name = "ComplexApp"
version = "3.0.0"

server "web-server" {
    host = "0.0.0.0"
    port = 8080
    ssl = true

    endpoints = [
        "/api/v1",
        "/api/v2",
        "/health"
    ]

    middleware {
        cors = {
            enabled = true
            origins = ["*"]
            methods = ["GET", "POST", "PUT", "DELETE"]
        }

        rate_limit = {
            requests_per_minute = 1000
            burst_limit = 100
        }
    }

    database {
        primary = {
            host = "db1.example.com"
            port = 5432
            name = "app_db"
        }

        replicas = [
            {
                host = "db2.example.com"
                port = 5432
            },
            {
                host = "db3.example.com"
                port = 5432
            }
        ]
    }
}

workers = [
    {
        name = "queue-worker"
        type = "async"
        concurrency = 10
    },
    {
        name = "cache-worker"
        type = "sync"
        concurrency = 5
    }
]
"#;

        let config = HlxConfigHandler::parse_content(content).unwrap();

        // Test complex nested structures
        assert!(config.sections.contains_key("web-server"));
        let server = &config.sections["web-server"];

        // Test nested middleware.cors
        let middleware = server.properties.get("middleware").unwrap().as_object().unwrap();
        let cors = middleware.get("cors").unwrap().as_object().unwrap();
        assert_eq!(cors.get("enabled").unwrap().as_bool().unwrap(), true);
        let origins = cors.get("origins").unwrap().as_array().unwrap();
        assert_eq!(origins[0].as_str().unwrap(), "*");

        // Test nested database.primary
        let db = server.properties.get("database").unwrap().as_object().unwrap();
        let primary = db.get("primary").unwrap().as_object().unwrap();
        assert_eq!(primary.get("host").unwrap().as_str().unwrap(), "db1.example.com");

        // Test array of objects (replicas)
        let replicas = db.get("replicas").unwrap().as_array().unwrap();
        assert_eq!(replicas.len(), 2);
        let replica1 = replicas[0].as_object().unwrap();
        assert_eq!(replica1.get("host").unwrap().as_str().unwrap(), "db2.example.com");

        // Test global workers array
        let workers = config.metadata.get("workers").unwrap().as_array().unwrap();
        assert_eq!(workers.len(), 2);
        let worker1 = workers[0].as_object().unwrap();
        assert_eq!(worker1.get("name").unwrap().as_str().unwrap(), "queue-worker");
        assert_eq!(worker1.get("concurrency").unwrap().as_i64().unwrap(), 10);
    }
}

impl Default for HlxConfig {
    fn default() -> Self {
        Self {
            sections: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

impl Default for HlxSection {
    fn default() -> Self {
        Self {
            properties: HashMap::new(),
            metadata: None,
        }
    }
}
