//! Error types for the Flux compiler
//! 
//! Defines all error types used throughout the compilation pipeline,
//! from lexical analysis to code generation.

use crate::position::{Position, Span};
use std::fmt;
use thiserror::Error;

/// Top-level error type for all Flux compiler errors
#[derive(Debug, Error)]
pub enum FluxError {
    #[error("Lexical error: {0}")]
    Lexical(#[from] LexError),
    
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),
    
    #[error("Semantic error: {0}")]
    Semantic(#[from] SemanticError),
    
    #[error("Code generation error: {0}")]
    CodeGen(#[from] CodeGenError),
    
    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),
    
    #[error("Package error: {0}")]
    Package(PackageError),
    
    #[error("CLI error: {0}")]
    Cli(String),
    
    #[error("I/O error: {0}")]
    Io(String),
    
    #[error("FFI error: {0}")]
    FFI(#[from] crate::ffi::error::FFIError),
}

/// Lexical analysis errors
#[derive(Debug, Error)]
pub struct LexError {
    pub position: Position,
    pub kind: LexErrorKind,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.position, self.kind)
    }
}

#[derive(Debug, Error)]
pub enum LexErrorKind {
    #[error("Unexpected character: '{0}'")]
    UnexpectedCharacter(char),
    
    #[error("Unterminated string literal")]
    UnterminatedString,
    
    #[error("Invalid number format")]
    InvalidNumber,
    
    #[error("Invalid escape sequence")]
    InvalidEscape,
    
    #[error("Invalid character literal")]
    InvalidCharacter,
    
    #[error("Comment not closed")]
    UnterminatedComment,
}

/// Parse errors
#[derive(Debug, Error)]
pub struct ParseError {
    pub span: Span,
    pub kind: ParseErrorKind,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.span, self.kind)
    }
}

#[derive(Debug, Error)]
pub enum ParseErrorKind {
    #[error("Expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String },
    
    #[error("Unexpected end of file")]
    UnexpectedEof,
    
    #[error("Invalid syntax: {message}")]
    InvalidSyntax { message: String },
    
    #[error("Missing semicolon")]
    MissingSemicolon,
    
    #[error("Invalid expression")]
    InvalidExpression,
    
    #[error("Invalid statement")]
    InvalidStatement,
}

/// Semantic analysis errors
#[derive(Debug, Error)]
pub struct SemanticError {
    pub span: Span,
    pub kind: SemanticErrorKind,
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.span, self.kind)
    }
}

#[derive(Debug, Error)]
pub enum SemanticErrorKind {
    #[error("Undefined variable: '{name}'")]
    UndefinedVariable { name: String },
    
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },
    
    #[error("Duplicate definition: '{name}'")]
    DuplicateDefinition { name: String },
    
    #[error("Invalid operation: {message}")]
    InvalidOperation { message: String },
    
    #[error("Null pointer access")]
    NullPointerAccess,
    
    #[error("Cannot infer type")]
    CannotInferType,
    
    #[error("Invalid function call")]
    InvalidFunctionCall,
    
    #[error("Return type mismatch")]
    ReturnTypeMismatch,
    
    #[error("Unsupported feature: {feature}")]
    UnsupportedFeature { feature: String },
}

/// Code generation errors
#[derive(Debug, Error)]
pub struct CodeGenError {
    pub span: Option<Span>,
    pub kind: CodeGenErrorKind,
}

impl fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.span {
            Some(span) => write!(f, "{}: {}", span, self.kind),
            None => write!(f, "{}", self.kind),
        }
    }
}

#[derive(Debug, Error)]
pub enum CodeGenErrorKind {
    #[error("LLVM error: {message}")]
    LlvmError { message: String },
    
    #[error("Unsupported feature: {feature}")]
    UnsupportedFeature { feature: String },
    
    #[error("Internal compiler error: {message}")]
    InternalError { message: String },
    
    #[error("Target not supported: {target}")]
    UnsupportedTarget { target: String },
    
    #[error("Runtime error: {message}")]
    RuntimeError { message: String },
}

/// Runtime errors
#[derive(Debug, Error)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

#[derive(Debug, Error)]
pub enum RuntimeErrorKind {
    #[error("Stack overflow")]
    StackOverflow,
    
    #[error("Out of memory")]
    OutOfMemory,
    
    #[error("Division by zero")]
    DivisionByZero,
    
    #[error("Index out of bounds")]
    IndexOutOfBounds,
    
    #[error("Null pointer dereference")]
    NullPointerDereference,
    
    #[error("Deadlock detected")]
    Deadlock,
    
    #[error("Channel closed")]
    ChannelClosed,
    
    #[error("Panic: {message}")]
    Panic { message: String },
}

/// Package management errors
#[derive(Debug, Error)]
pub enum PackageError {
    #[error("Configuration not found: {0}")]
    ConfigNotFound(std::path::PathBuf),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Dependency resolution failed: {0}")]
    DependencyResolutionFailed(String),
    
    #[error("Project creation failed: {0}")]
    ProjectCreationFailed(String),
    
    #[error("I/O error: {0}")]
    IoError(String),
    
    #[error("Version parsing error: {0}")]
    VersionParseError(String),
    
    #[error("Lock file error: {0}")]
    LockFileError(String),
}

/// Result type alias for Flux operations
pub type FluxResult<T> = Result<T, FluxError>;

impl FluxError {
    /// Create an FFI error from a string (for backward compatibility)
    pub fn ffi_string(message: String) -> Self {
        Self::FFI(crate::ffi::error::FFIError::Runtime {
            function: "unknown".to_string(),
            error: message,
        })
    }
}