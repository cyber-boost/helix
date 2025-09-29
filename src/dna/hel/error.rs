use std::path::PathBuf;
use std::fmt;
use thiserror::Error;
use crate::dna::atp::lexer::SourceLocation;
#[derive(Error, Debug, Clone)]
pub enum HlxError {
    #[error("Configuration conversion failed: {field} - {details}")]
    ConfigConversion { field: String, details: String, suggestion: String },
    #[error("Dataset processing failed: {message}")]
    DatasetProcessing { message: String, suggestion: String },
    #[error("Dataset quality validation failed: score {score:.2}")]
    QualityValidation { score: f64, issues: Vec<String>, suggestions: Vec<String> },
    #[error("Format conversion failed: {from} → {to}")]
    FormatConversion { from: String, to: String, suggestion: String },
    #[error("Algorithm '{algorithm}' not supported")]
    UnsupportedAlgorithm { algorithm: String, supported: Vec<String> },
    #[error("Dataset not found: {path}")]
    DatasetNotFound { path: PathBuf, suggestion: String },
    #[error("HLX processing failed: {message}")]
    HlxProcessing { message: String, suggestion: String },
    #[error("Forge integration failed: {message}")]
    ForgeIntegration { message: String, suggestion: String },
    #[error("Configuration validation failed: {field} = {value}")]
    ConfigValidation { field: String, value: String, suggestion: String },
    #[error("Invalid input: {message}")]
    InvalidInput { message: String, suggestion: String },
    #[error("Execution error: {message}")]
    ExecutionError { message: String, suggestion: String },
    #[error("Invalid parameters: {message}")]
    InvalidParameters { message: String, suggestion: String },
    #[error("Unknown operator: {message}")]
    UnknownOperator { message: String, suggestion: String },
    #[error("Validation error: {message}")]
    ValidationError { message: String, suggestion: String },
    #[error("Hash error: {message}")]
    HashError { message: String, suggestion: String },
    #[error("JSON error: {message}")]
    JsonError { message: String, suggestion: String },
    #[error("Base64 error: {message}")]
    Base64Error { message: String, suggestion: String },
    #[error("Unknown error: {message}")]
    Unknown { message: String, suggestion: String },
    #[error("Compilation error: {message}")]
    Compilation { message: String, suggestion: String },
    #[error("IO error: {message}")]
    Io { message: String, suggestion: String },
    #[error("Serialization error: {message}")]
    SerializationError { message: String, suggestion: String },
    #[error("Deserialization error: {message}")]
    DeserializationError { message: String, suggestion: String },
    #[error("Compression error: {message}")]
    CompressionError { message: String, suggestion: String },
    #[error("Decompression error: {message}")]
    DecompressionError { message: String, suggestion: String },
    #[error("Feature not available: {feature}")]
    FeatureError { feature: String, message: String },
}
impl HlxError {
    pub fn config_conversion(
        field: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        let field = field.into();
        let details = details.into();
        let suggestion = format!(
            "Check your Forge.toml configuration for the '{}' field", field
        );
        Self::ConfigConversion {
            field,
            details,
            suggestion,
        }
    }
    pub fn dataset_processing(message: impl Into<String>) -> Self {
        let message = message.into();
        let suggestion = "Try running 'forge hlx dataset validate' to check dataset compatibility"
            .to_string();
        Self::DatasetProcessing {
            message,
            suggestion,
        }
    }
    pub fn quality_validation(score: f64, issues: Vec<String>) -> Self {
        let suggestions = vec![
            "Run 'forge hlx dataset analyze' for detailed quality metrics".to_string(),
            "Consider filtering or augmenting low-quality samples".to_string(),
            "Check dataset format and required columns".to_string(),
        ];
        Self::QualityValidation {
            score,
            issues,
            suggestions,
        }
    }
    pub fn format_conversion(from: impl Into<String>, to: impl Into<String>) -> Self {
        let from = from.into();
        let to = to.into();
        let suggestion = format!(
            "Ensure your dataset contains the required fields for {} format", to
        );
        Self::FormatConversion {
            from,
            to,
            suggestion,
        }
    }
    pub fn unsupported_algorithm(algorithm: impl Into<String>) -> Self {
        let algorithm = algorithm.into();
        let supported = vec!["bco", "dpo", "ppo", "sft"]
            .into_iter()
            .map(String::from)
            .collect();
        Self::UnsupportedAlgorithm {
            algorithm,
            supported,
        }
    }
    pub fn dataset_not_found(path: PathBuf) -> Self {
        let suggestion = format!(
            "Ensure the dataset file exists at: {}", path.display()
        );
        Self::DatasetNotFound {
            path,
            suggestion,
        }
    }
    pub fn execution_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::ExecutionError {
            message,
            suggestion,
        }
    }
    pub fn invalid_input(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::InvalidInput {
            message,
            suggestion,
        }
    }
    pub fn invalid_parameters(operator: &str, params: &str) -> Self {
        let message = format!(
            "Invalid parameters for operator '{}': {}", operator, params
        );
        let suggestion = format!("Check the operator '{}' parameter format", operator);
        Self::InvalidParameters {
            message,
            suggestion,
        }
    }
    pub fn unknown_operator(operator: impl Into<String>) -> Self {
        let operator = operator.into();
        let message = format!("Unknown operator: {}", operator);
        let suggestion = "Check the operator name and ensure it is supported"
            .to_string();
        Self::UnknownOperator {
            message,
            suggestion,
        }
    }
    pub fn validation_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::ValidationError {
            message,
            suggestion,
        }
    }
    pub fn hash_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::HashError {
            message,
            suggestion,
        }
    }
    pub fn json_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::JsonError {
            message,
            suggestion,
        }
    }
    pub fn base64_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::Base64Error {
            message,
            suggestion,
        }
    }
    pub fn unknown_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::Unknown {
            message,
            suggestion,
        }
    }
    pub fn compilation_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::Compilation {
            message,
            suggestion,
        }
    }
    pub fn io_error(message: impl Into<String>, suggestion: impl Into<String>) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::Io { message, suggestion }
    }
    pub fn serialization_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::SerializationError {
            message,
            suggestion,
        }
    }
    pub fn deserialization_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::DeserializationError {
            message,
            suggestion,
        }
    }
    pub fn compression_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::CompressionError {
            message,
            suggestion,
        }
    }
    pub fn decompression_error(
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        let message = message.into();
        let suggestion = suggestion.into();
        Self::DecompressionError {
            message,
            suggestion,
        }
    }
    pub fn feature_error(
        feature: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let feature = feature.into();
        let message = message.into();
        Self::FeatureError {
            feature,
            message,
        }
    }
    pub fn suggestions(&self) -> Vec<String> {
        match self {
            Self::ConfigConversion { suggestion, .. } => vec![suggestion.clone()],
            Self::DatasetProcessing { suggestion, .. } => vec![suggestion.clone()],
            Self::QualityValidation { suggestions, .. } => suggestions.clone(),
            Self::FormatConversion { suggestion, .. } => vec![suggestion.clone()],
            Self::UnsupportedAlgorithm { supported, .. } => {
                vec![format!("Supported algorithms: {}", supported.join(", "))]
            }
            Self::DatasetNotFound { suggestion, .. } => vec![suggestion.clone()],
            Self::HlxProcessing { suggestion, .. } => vec![suggestion.clone()],
            Self::ForgeIntegration { suggestion, .. } => vec![suggestion.clone()],
            Self::ConfigValidation { suggestion, .. } => vec![suggestion.clone()],
            Self::InvalidInput { suggestion, .. } => vec![suggestion.clone()],
            Self::ExecutionError { suggestion, .. } => vec![suggestion.clone()],
            Self::InvalidParameters { suggestion, .. } => vec![suggestion.clone()],
            Self::UnknownOperator { suggestion, .. } => vec![suggestion.clone()],
            Self::ValidationError { suggestion, .. } => vec![suggestion.clone()],
            Self::HashError { suggestion, .. } => vec![suggestion.clone()],
            Self::JsonError { suggestion, .. } => vec![suggestion.clone()],
            Self::Base64Error { suggestion, .. } => vec![suggestion.clone()],
            Self::Unknown { suggestion, .. } => vec![suggestion.clone()],
            Self::Compilation { suggestion, .. } => vec![suggestion.clone()],
            Self::Io { suggestion, .. } => vec![suggestion.clone()],
            Self::SerializationError { suggestion, .. } => vec![suggestion.clone()],
            Self::DeserializationError { suggestion, .. } => vec![suggestion.clone()],
            Self::CompressionError { suggestion, .. } => vec![suggestion.clone()],
            Self::DecompressionError { suggestion, .. } => vec![suggestion.clone()],
            Self::FeatureError { message, .. } => vec![message.clone()],
        }
    }
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::ConfigConversion { .. } => true,
            Self::DatasetProcessing { .. } => true,
            Self::QualityValidation { score, .. } => *score > 0.3,
            Self::FormatConversion { .. } => true,
            Self::UnsupportedAlgorithm { .. } => false,
            Self::DatasetNotFound { .. } => false,
            Self::HlxProcessing { .. } => true,
            Self::ForgeIntegration { .. } => true,
            Self::ConfigValidation { .. } => true,
            Self::InvalidInput { .. } => true,
            Self::ExecutionError { .. } => true,
            Self::InvalidParameters { .. } => true,
            Self::UnknownOperator { .. } => true,
            Self::ValidationError { .. } => true,
            Self::HashError { .. } => true,
            Self::JsonError { .. } => true,
            Self::Base64Error { .. } => true,
            Self::Unknown { .. } => true,
            Self::Compilation { .. } => true,
            Self::Io { .. } => true,
            Self::SerializationError { .. } => true,
            Self::DeserializationError { .. } => true,
            Self::CompressionError { .. } => true,
            Self::DecompressionError { .. } => true,
            Self::FeatureError { .. } => false,
        }
    }
}
impl From<std::io::Error> for HlxError {
    fn from(err: std::io::Error) -> Self {
        Self::io_error(err.to_string(), "Check file permissions and paths")
    }
}
impl From<serde_json::Error> for HlxError {
    fn from(err: serde_json::Error) -> Self {
        Self::json_error(err.to_string(), "Check JSON format and structure")
    }
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
impl std::error::Error for LexerError {}
impl std::error::Error for ParserError {}
impl std::error::Error for SemanticError {}
impl std::error::Error for CompilationError {}
impl std::error::Error for RuntimeError {}
impl std::error::Error for IoError {}
pub type Result<T> = std::result::Result<T, HlxError>;
pub type HlxResult<T> = std::result::Result<T, HlxError>;
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