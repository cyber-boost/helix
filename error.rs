use std::fmt;
use std::path::PathBuf;
use crate::lexer::SourceLocation;
#[derive(Debug)]
pub enum HelixError {
    Lexer(LexerError),
    Parser(ParserError),
    Semantic(SemanticError),
    Compilation(CompilationError),
    Runtime(RuntimeError),
    Io(IoError),
}
#[derive(Debug)]
pub struct LexerError {
    pub message: String,
    pub location: SourceLocation,
    pub source_line: String,
    pub suggestion: Option<String>,
}
#[derive(Debug)]
pub struct ParserError {
    pub message: String,
    pub location: SourceLocation,
    pub expected: Vec<String>,
    pub found: String,
    pub source_line: String,
    pub suggestion: Option<String>,
}
#[derive(Debug)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub location: SourceLocation,
    pub entity: String,
    pub context: Vec<String>,
}
#[derive(Debug)]
pub enum SemanticErrorKind {
    UndefinedReference,
    DuplicateDefinition,
    TypeMismatch { expected: String, found: String },
    CircularDependency,
    InvalidValue,
    MissingRequired,
    DeprecatedFeature,
}
#[derive(Debug)]
pub struct CompilationError {
    pub stage: CompilationStage,
    pub message: String,
    pub file: Option<PathBuf>,
    pub recoverable: bool,
}
#[derive(Debug)]
pub enum CompilationStage {
    Parsing,
    Validation,
    Optimization,
    CodeGeneration,
    Serialization,
    Bundling,
}
#[derive(Debug)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub message: String,
    pub stack_trace: Vec<String>,
}
#[derive(Debug, PartialEq)]
pub enum RuntimeErrorKind {
    InvalidInstruction,
    StackUnderflow,
    StackOverflow,
    MemoryAccessViolation,
    DivisionByZero,
    TypeConversion,
    ResourceNotFound,
}
#[derive(Debug)]
pub struct IoError {
    pub operation: IoOperation,
    pub path: PathBuf,
    pub message: String,
}
#[derive(Debug)]
pub enum IoOperation {
    Read,
    Write,
    Create,
    Delete,
    Rename,
    Metadata,
}
impl fmt::Display for HelixError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HelixError::Lexer(e) => write!(f, "{}", e),
            HelixError::Parser(e) => write!(f, "{}", e),
            HelixError::Semantic(e) => write!(f, "{}", e),
            HelixError::Compilation(e) => write!(f, "{}", e),
            HelixError::Runtime(e) => write!(f, "{}", e),
            HelixError::Io(e) => write!(f, "{}", e),
        }
    }
}
impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Lexer error at {}:{}", self.location.line, self.location.column)?;
        writeln!(f, "  {}", self.message)?;
        writeln!(f, "  {}", self.source_line)?;
        writeln!(f, "  {}^", " ".repeat(self.location.column))?;
        if let Some(suggestion) = &self.suggestion {
            writeln!(f, "  Suggestion: {}", suggestion)?;
        }
        Ok(())
    }
}
impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Parser error at {}:{}", self.location.line, self.location.column)?;
        writeln!(f, "  {}", self.message)?;
        writeln!(f, "  {}", self.source_line)?;
        writeln!(f, "  {}^", " ".repeat(self.location.column))?;
        if !self.expected.is_empty() {
            writeln!(f, "  Expected: {}", self.expected.join(" | "))?;
        }
        writeln!(f, "  Found: {}", self.found)?;
        if let Some(suggestion) = &self.suggestion {
            writeln!(f, "  Suggestion: {}", suggestion)?;
        }
        Ok(())
    }
}
impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Semantic error: ")?;
        match &self.kind {
            SemanticErrorKind::UndefinedReference => {
                writeln!(f, "Undefined reference to '{}'", self.entity)?;
            }
            SemanticErrorKind::DuplicateDefinition => {
                writeln!(f, "Duplicate definition of '{}'", self.entity)?;
            }
            SemanticErrorKind::TypeMismatch { expected, found } => {
                writeln!(
                    f, "Type mismatch for '{}': expected {}, found {}", self.entity,
                    expected, found
                )?;
            }
            SemanticErrorKind::CircularDependency => {
                writeln!(f, "Circular dependency involving '{}'", self.entity)?;
            }
            SemanticErrorKind::InvalidValue => {
                writeln!(f, "Invalid value for '{}'", self.entity)?;
            }
            SemanticErrorKind::MissingRequired => {
                writeln!(f, "Missing required field '{}'", self.entity)?;
            }
            SemanticErrorKind::DeprecatedFeature => {
                writeln!(f, "Use of deprecated feature '{}'", self.entity)?;
            }
        }
        writeln!(f, "  at {}:{}", self.location.line, self.location.column)?;
        if !self.context.is_empty() {
            writeln!(f, "  Context:")?;
            for ctx in &self.context {
                writeln!(f, "    - {}", ctx)?;
            }
        }
        Ok(())
    }
}
impl fmt::Display for CompilationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Compilation error during {:?}: {}", self.stage, self.message)?;
        if let Some(file) = &self.file {
            write!(f, " in file {:?}", file)?;
        }
        if self.recoverable {
            write!(f, " (recoverable)")?;
        }
        Ok(())
    }
}
impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Runtime error: {:?}", self.kind)?;
        writeln!(f, "  {}", self.message)?;
        if !self.stack_trace.is_empty() {
            writeln!(f, "  Stack trace:")?;
            for frame in &self.stack_trace {
                writeln!(f, "    {}", frame)?;
            }
        }
        Ok(())
    }
}
impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f, "IO error during {:?} operation on {:?}: {}", self.operation, self.path,
            self.message
        )
    }
}
impl std::error::Error for HelixError {}
impl std::error::Error for LexerError {}
impl std::error::Error for ParserError {}
impl std::error::Error for SemanticError {}
impl std::error::Error for CompilationError {}
impl std::error::Error for RuntimeError {}
impl std::error::Error for IoError {}
impl From<std::io::Error> for HelixError {
    fn from(err: std::io::Error) -> Self {
        HelixError::Io(IoError {
            operation: IoOperation::Read,
            path: PathBuf::new(),
            message: err.to_string(),
        })
    }
}
pub type Result<T> = std::result::Result<T, HelixError>;
pub struct ErrorRecovery;
impl ErrorRecovery {
    pub fn suggest_for_undefined_reference(name: &str) -> Option<String> {
        if name == "agnet" {
            return Some("Did you mean 'agent'?".to_string());
        }
        if name == "worfklow" || name == "workfow" {
            return Some("Did you mean 'workflow'?".to_string());
        }
        None
    }
    pub fn suggest_for_syntax_error(found: &str, expected: &[String]) -> Option<String> {
        if expected.contains(&"=".to_string()) && found == ":" {
            return Some("Use '=' for assignment, not ':'".to_string());
        }
        if expected.contains(&"{".to_string()) && found == "(" {
            return Some("Use '{' for block start, not '('".to_string());
        }
        None
    }
}