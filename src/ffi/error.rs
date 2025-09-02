//! FFI-specific error types and handling

use std::fmt;

/// FFI-specific errors
#[derive(Debug, Clone)]
pub enum FFIError {
    /// Type conversion error
    TypeConversion {
        from: String,
        to: String,
        reason: String,
    },
    
    /// Marshaling error
    Marshaling {
        operation: String,
        details: String,
    },
    
    /// Safety violation
    SafetyViolation {
        function: String,
        violation: String,
    },
    
    /// Library loading error
    LibraryLoad {
        library: String,
        error: String,
    },
    
    /// Symbol resolution error
    SymbolNotFound {
        symbol: String,
        library: Option<String>,
    },
    
    /// Function signature mismatch
    SignatureMismatch {
        function: String,
        expected: String,
        found: String,
    },
    
    /// Runtime FFI error
    Runtime {
        function: String,
        error: String,
    },
    
    /// Invalid FFI declaration
    InvalidDeclaration {
        function: String,
        reason: String,
    },
}

impl fmt::Display for FFIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FFIError::TypeConversion { from, to, reason } => {
                write!(f, "Cannot convert type '{}' to '{}': {}", from, to, reason)
            }
            
            FFIError::Marshaling { operation, details } => {
                write!(f, "Marshaling error in {}: {}", operation, details)
            }
            
            FFIError::SafetyViolation { function, violation } => {
                write!(f, "Safety violation in function '{}': {}", function, violation)
            }
            
            FFIError::LibraryLoad { library, error } => {
                write!(f, "Failed to load library '{}': {}", library, error)
            }
            
            FFIError::SymbolNotFound { symbol, library } => {
                match library {
                    Some(lib) => write!(f, "Symbol '{}' not found in library '{}'", symbol, lib),
                    None => write!(f, "Symbol '{}' not found", symbol),
                }
            }
            
            FFIError::SignatureMismatch { function, expected, found } => {
                write!(f, "Function '{}' signature mismatch: expected '{}', found '{}'", 
                       function, expected, found)
            }
            
            FFIError::Runtime { function, error } => {
                write!(f, "Runtime error in FFI function '{}': {}", function, error)
            }
            
            FFIError::InvalidDeclaration { function, reason } => {
                write!(f, "Invalid FFI declaration for '{}': {}", function, reason)
            }
        }
    }
}

impl std::error::Error for FFIError {}

/// Result type for FFI operations
pub type FFIResult<T> = Result<T, FFIError>;

/// Helper functions for creating common FFI errors
impl FFIError {
    pub fn type_conversion(from: &str, to: &str, reason: &str) -> Self {
        Self::TypeConversion {
            from: from.to_string(),
            to: to.to_string(),
            reason: reason.to_string(),
        }
    }
    
    pub fn marshaling(operation: &str, details: &str) -> Self {
        Self::Marshaling {
            operation: operation.to_string(),
            details: details.to_string(),
        }
    }
    
    pub fn safety_violation(function: &str, violation: &str) -> Self {
        Self::SafetyViolation {
            function: function.to_string(),
            violation: violation.to_string(),
        }
    }
    
    pub fn library_load(library: &str, error: &str) -> Self {
        Self::LibraryLoad {
            library: library.to_string(),
            error: error.to_string(),
        }
    }
    
    pub fn symbol_not_found(symbol: &str, library: Option<&str>) -> Self {
        Self::SymbolNotFound {
            symbol: symbol.to_string(),
            library: library.map(|s| s.to_string()),
        }
    }
    
    pub fn signature_mismatch(function: &str, expected: &str, found: &str) -> Self {
        Self::SignatureMismatch {
            function: function.to_string(),
            expected: expected.to_string(),
            found: found.to_string(),
        }
    }
    
    pub fn runtime(function: &str, error: &str) -> Self {
        Self::Runtime {
            function: function.to_string(),
            error: error.to_string(),
        }
    }
    
    pub fn invalid_declaration(function: &str, reason: &str) -> Self {
        Self::InvalidDeclaration {
            function: function.to_string(),
            reason: reason.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = FFIError::type_conversion("int", "string", "incompatible types");
        assert_eq!(
            error.to_string(),
            "Cannot convert type 'int' to 'string': incompatible types"
        );
    }

    #[test]
    fn test_safety_violation_error() {
        let error = FFIError::safety_violation("dangerous_func", "null pointer dereference");
        assert_eq!(
            error.to_string(),
            "Safety violation in function 'dangerous_func': null pointer dereference"
        );
    }

    #[test]
    fn test_symbol_not_found() {
        let error = FFIError::symbol_not_found("missing_func", Some("libtest.so"));
        assert_eq!(
            error.to_string(),
            "Symbol 'missing_func' not found in library 'libtest.so'"
        );
    }
}