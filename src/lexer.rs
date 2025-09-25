use std::str::Chars;
use std::iter::Peekable;
use std::fmt;
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    String(String),
    Number(f64),
    Bool(bool),
    Duration(u64, TimeUnit),
    Identifier(String),
    Keyword(Keyword),
    Assign,
    Arrow,
    Pipe,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    Comma,
    Dot,
    Comment(String),
    Variable(String),
    Reference(String),
    Newline,
    Eof,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    Project,
    Agent,
    Workflow,
    Memory,
    Context,
    Crew,
    Plugin,
    Database,
    Step,
    Pipeline,
    Trigger,
    Capabilities,
    Backstory,
    Secrets,
    Variables,
    Embeddings,
    True,
    False,
    Null,
    DependsOn,
    Parallel,
    Timeout,
    Load,
    // Dynamic section keywords - these are handled generically
    Section,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub position: usize,
}
#[derive(Debug, Clone)]
pub struct TokenWithLocation {
    pub token: Token,
    pub location: SourceLocation,
}
#[derive(Debug, Clone)]
pub enum LexError {
    UnterminatedString { location: SourceLocation },
    InvalidNumber { location: SourceLocation, text: String },
    UnexpectedCharacter { location: SourceLocation, char: char },
    InvalidEscape { location: SourceLocation, char: char },
}
impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnterminatedString { location } => {
                write!(
                    f, "Unterminated string at line {}, column {}", location.line,
                    location.column
                )
            }
            LexError::InvalidNumber { location, text } => {
                write!(
                    f, "Invalid number '{}' at line {}, column {}", text, location.line,
                    location.column
                )
            }
            LexError::UnexpectedCharacter { location, char } => {
                write!(
                    f, "Unexpected character '{}' at line {}, column {}", char, location
                    .line, location.column
                )
            }
            LexError::InvalidEscape { location, char } => {
                write!(
                    f, "Invalid escape sequence '\\{}' at line {}, column {}", char,
                    location.line, location.column
                )
            }
        }
    }
}
pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    current_char: Option<char>,
    position: usize,
    line: usize,
    column: usize,
}
impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input: input.chars().peekable(),
            current_char: None,
            position: 0,
            line: 1,
            column: 0,
        };
        lexer.advance();
        lexer
    }
    fn current_location(&self) -> SourceLocation {
        SourceLocation {
            line: self.line,
            column: self.column,
            position: self.position,
        }
    }
    fn advance(&mut self) {
        self.current_char = self.input.next();
        self.position += 1;
        if let Some(ch) = self.current_char {
            if ch == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
    }
    fn peek(&mut self) -> Option<&char> {
        self.input.peek()
    }
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else if ch == '\\' && self.peek() == Some(&'\n') {
                self.advance();
                self.advance();
                self.line += 1;
                self.column = 0;
            } else {
                break;
            }
        }
    }
    fn read_string(&mut self) -> Result<String, LexError> {
        let mut result = String::new();
        self.advance();
        while let Some(ch) = self.current_char {
            if ch == '"' {
                self.advance();
                return Ok(result);
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char {
                    result.push(
                        match escaped {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            _ => {
                                return Err(LexError::InvalidEscape {
                                    location: self.current_location(),
                                    char: escaped,
                                });
                            }
                        },
                    );
                    self.advance();
                }
            } else {
                result.push(ch);
                self.advance();
            }
        }
        // If we reach here, we hit EOF without finding a closing quote
        Err(LexError::UnterminatedString {
            location: self.current_location(),
        })
    }
    fn read_number(&mut self) -> f64 {
        let mut num_str = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_numeric() || ch == '.' || ch == '_' {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        // Remove underscores for parsing
        let clean_num_str = num_str.replace('_', "");
        clean_num_str.parse().unwrap_or(0.0)
    }
    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        ident
    }
    fn read_comment(&mut self) -> String {
        let mut comment = String::new();
        self.advance();
        while let Some(ch) = self.current_char {
            if ch == '\n' {
                break;
            }
            comment.push(ch);
            self.advance();
        }
        comment.trim().to_string()
    }
    fn read_variable(&mut self) -> String {
        let mut var = String::new();
        self.advance();
        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' {
                var.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        var
    }
    fn read_reference(&mut self) -> String {
        let mut reference = String::new();
        self.advance();
        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                reference.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        reference
    }
    fn check_keyword(&self, ident: &str) -> Option<Keyword> {
        match ident {
            "project" => Some(Keyword::Project),
            "agent" => Some(Keyword::Agent),
            "workflow" => Some(Keyword::Workflow),
            "memory" => Some(Keyword::Memory),
            "context" => Some(Keyword::Context),
            "crew" => Some(Keyword::Crew),
            "plugin" => Some(Keyword::Plugin),
            "database" => Some(Keyword::Database),
            "step" => Some(Keyword::Step),
            "pipeline" => Some(Keyword::Pipeline),
            "trigger" => Some(Keyword::Trigger),
            "capabilities" => Some(Keyword::Capabilities),
            "backstory" => Some(Keyword::Backstory),
            "secrets" => Some(Keyword::Secrets),
            "variables" => Some(Keyword::Variables),
            "embeddings" => Some(Keyword::Embeddings),
            "true" => Some(Keyword::True),
            "false" => Some(Keyword::False),
            "null" => Some(Keyword::Null),
            "depends_on" => Some(Keyword::DependsOn),
            "parallel" => Some(Keyword::Parallel),
            "timeout" => Some(Keyword::Timeout),
            "load" => Some(Keyword::Load),
            "section" => Some(Keyword::Section),
            _ => None,
        }
    }
    fn read_duration(&mut self, num: f64) -> Option<Token> {
        let mut unit_str = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_alphabetic() {
                unit_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        let unit = match unit_str.as_str() {
            "s" | "sec" | "seconds" => Some(TimeUnit::Seconds),
            "m" | "min" | "minutes" => Some(TimeUnit::Minutes),
            "h" | "hr" | "hours" => Some(TimeUnit::Hours),
            "d" | "days" => Some(TimeUnit::Days),
            _ => None,
        };
        if let Some(u) = unit { Some(Token::Duration(num as u64, u)) } else { None }
    }
    pub fn next_token_with_location(&mut self) -> TokenWithLocation {
        self.skip_whitespace();
        let location = self.current_location();
        let token = self.next_token_internal();
        TokenWithLocation {
            token,
            location,
        }
    }
    pub fn next_token(&mut self) -> Token {
        self.next_token_internal()
    }
    fn next_token_internal(&mut self) -> Token {
        self.skip_whitespace();
        match self.current_char {
            None => Token::Eof,
            Some('\n') => {
                self.advance();
                Token::Newline
            }
            Some('#') => {
                let comment = self.read_comment();
                Token::Comment(comment)
            }
            Some('"') => {
                match self.read_string() {
                    Ok(string) => Token::String(string),
                    Err(_) => Token::String("".to_string()), // For now, return empty string on error
                }
            }
            Some('$') => {
                let var = self.read_variable();
                Token::Variable(var)
            }
            Some('@') => {
                let reference = self.read_reference();
                Token::Reference(reference)
            }
            Some('{') => {
                self.advance();
                Token::LeftBrace
            }
            Some('}') => {
                self.advance();
                Token::RightBrace
            }
            Some('[') => {
                self.advance();
                Token::LeftBracket
            }
            Some(']') => {
                self.advance();
                Token::RightBracket
            }
            Some('(') => {
                self.advance();
                Token::LeftParen
            }
            Some(')') => {
                self.advance();
                Token::RightParen
            }
            Some(',') => {
                self.advance();
                Token::Comma
            }
            Some('.') => {
                self.advance();
                Token::Dot
            }
            Some('=') => {
                self.advance();
                Token::Assign
            }
            Some('-') => {
                self.advance();
                if self.current_char == Some('>') {
                    self.advance();
                    Token::Arrow
                } else {
                    if let Some(ch) = self.current_char {
                        if ch.is_numeric() {
                            let num = -self.read_number();
                            Token::Number(num)
                        } else {
                            let mut ident = String::from("-");
                            ident.push_str(&self.read_identifier());
                            if let Some(keyword) = self.check_keyword(&ident) {
                                Token::Keyword(keyword)
                            } else {
                                Token::Identifier(ident)
                            }
                        }
                    } else {
                        Token::Identifier("-".to_string())
                    }
                }
            }
            Some('|') => {
                self.advance();
                Token::Pipe
            }
            Some(ch) if ch.is_numeric() => {
                let num = self.read_number();
                // Look ahead past any whitespace that belongs to the same "token"
                while let Some(' ') | Some('\t') = self.current_char {
                    self.advance();
                }
                if let Some(duration_token) = self.read_duration(num) {
                    duration_token
                } else {
                    Token::Number(num)
                }
            }
            Some(ch) if ch.is_alphabetic() || ch == '_' => {
                let ident = self.read_identifier();
                if let Some(keyword) = self.check_keyword(&ident) {
                    match keyword {
                        Keyword::True => Token::Bool(true),
                        Keyword::False => Token::Bool(false),
                        _ => Token::Keyword(keyword),
                    }
                } else {
                    Token::Identifier(ident)
                }
            }
            Some(ch) => {
                self.advance();
                Token::Identifier(ch.to_string())
            }
        }
    }
}
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        match &token {
            Token::Eof => {
                tokens.push(token);
                break;
            }
            Token::Comment(_) => {}
            _ => {
                tokens.push(token);
            }
        }
    }
    Ok(tokens)
}
pub fn tokenize_with_locations(input: &str) -> Result<Vec<TokenWithLocation>, LexError> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        let token_with_loc = lexer.next_token_with_location();
        match &token_with_loc.token {
            Token::Eof => {
                tokens.push(token_with_loc);
                break;
            }
            Token::Comment(_) => {}
            _ => {
                tokens.push(token_with_loc);
            }
        }
    }
    Ok(tokens)
}
pub struct SourceMap {
    pub tokens: Vec<TokenWithLocation>,
    pub source: String,
}
impl SourceMap {
    pub fn new(source: String) -> Result<Self, LexError> {
        let tokens = tokenize_with_locations(&source)?;
        Ok(SourceMap { tokens, source })
    }
    pub fn get_line(&self, line_num: usize) -> Option<&str> {
        self.source.lines().nth(line_num - 1)
    }
    pub fn get_context(
        &self,
        location: &SourceLocation,
        context_lines: usize,
    ) -> String {
        let mut result = String::new();
        let start_line = location.line.saturating_sub(context_lines);
        let end_line = location.line + context_lines;
        for (i, line) in self.source.lines().enumerate() {
            let line_num = i + 1;
            if line_num >= start_line && line_num <= end_line {
                if line_num == location.line {
                    result.push_str(&format!("{:4} | {}\n", line_num, line));
                    result
                        .push_str(
                            &format!(
                                "     | {}^\n", " ".repeat(location.column
                                .saturating_sub(1))
                            ),
                        );
                } else {
                    result.push_str(&format!("{:4} | {}\n", line_num, line));
                }
            }
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic_tokens() {
        let input = r#"project "test" { }"#;
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Keyword(Keyword::Project));
        assert_eq!(tokens[1], Token::String("test".to_string()));
        assert_eq!(tokens[2], Token::LeftBrace);
        assert_eq!(tokens[3], Token::RightBrace);
    }
    #[test]
    fn test_duration() {
        let input = "timeout = 30m";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Keyword(Keyword::Timeout));
        assert_eq!(tokens[1], Token::Assign);
        assert_eq!(tokens[2], Token::Duration(30, TimeUnit::Minutes));
    }

    #[test]
    fn test_duration_with_space() {
        let input = "timeout = 30 m";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Keyword(Keyword::Timeout));
        assert_eq!(tokens[1], Token::Assign);
        assert_eq!(tokens[2], Token::Duration(30, TimeUnit::Minutes));
    }

    #[test]
    fn test_section_keyword() {
        let input = "section test { }";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Keyword(Keyword::Section));
        assert_eq!(tokens[1], Token::Identifier("test".to_string()));
        assert_eq!(tokens[2], Token::LeftBrace);
        assert_eq!(tokens[3], Token::RightBrace);
    }
    #[test]
    fn test_variables_and_references() {
        let input = "$API_KEY @memory.context";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0], Token::Variable("API_KEY".to_string()));
        assert_eq!(tokens[1], Token::Reference("memory.context".to_string()));
    }
}