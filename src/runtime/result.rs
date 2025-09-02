//! Result type implementation for Flux error handling
//! 
//! Provides the Result<T, E> type and error propagation mechanisms
//! that form the foundation of Flux's error handling system.

use std::fmt;

/// Result type for Flux error handling
/// 
/// This is the core type for explicit error handling in Flux.
/// Functions that can fail return Result<T, E> where T is the success type
/// and E is the error type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FluxResult<T, E> {
    /// Success variant containing the value
    Ok(T),
    /// Error variant containing the error
    Err(E),
}

impl<T, E> FluxResult<T, E> {
    /// Returns true if the result is Ok
    pub fn is_ok(&self) -> bool {
        matches!(self, FluxResult::Ok(_))
    }
    
    /// Returns true if the result is Err
    pub fn is_err(&self) -> bool {
        matches!(self, FluxResult::Err(_))
    }
    
    /// Converts from FluxResult<T, E> to Option<T>
    pub fn ok(self) -> Option<T> {
        match self {
            FluxResult::Ok(t) => Some(t),
            FluxResult::Err(_) => None,
        }
    }
    
    /// Converts from FluxResult<T, E> to Option<E>
    pub fn err(self) -> Option<E> {
        match self {
            FluxResult::Ok(_) => None,
            FluxResult::Err(e) => Some(e),
        }
    }
    
    /// Maps a FluxResult<T, E> to FluxResult<U, E> by applying a function to the Ok value
    pub fn map<U, F>(self, f: F) -> FluxResult<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            FluxResult::Ok(t) => FluxResult::Ok(f(t)),
            FluxResult::Err(e) => FluxResult::Err(e),
        }
    }
    
    /// Maps a FluxResult<T, E> to FluxResult<T, F> by applying a function to the Err value
    pub fn map_err<F, O>(self, f: O) -> FluxResult<T, F>
    where
        O: FnOnce(E) -> F,
    {
        match self {
            FluxResult::Ok(t) => FluxResult::Ok(t),
            FluxResult::Err(e) => FluxResult::Err(f(e)),
        }
    }
    
    /// Returns the contained Ok value or a provided default
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            FluxResult::Ok(t) => t,
            FluxResult::Err(_) => default,
        }
    }
    
    /// Returns the contained Ok value or computes it from a closure
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce(E) -> T,
    {
        match self {
            FluxResult::Ok(t) => t,
            FluxResult::Err(e) => f(e),
        }
    }
    
    /// Returns the contained Ok value, consuming the self value
    /// 
    /// # Panics
    /// 
    /// Panics if the value is an Err, with a panic message provided by the Err's value
    pub fn unwrap(self) -> T
    where
        E: fmt::Debug,
    {
        match self {
            FluxResult::Ok(t) => t,
            FluxResult::Err(e) => panic!("called `FluxResult::unwrap()` on an `Err` value: {:?}", e),
        }
    }
    
    /// Returns the contained Ok value, consuming the self value
    /// 
    /// # Panics
    /// 
    /// Panics if the value is an Err, with a panic message including the passed message
    pub fn expect(self, msg: &str) -> T
    where
        E: fmt::Debug,
    {
        match self {
            FluxResult::Ok(t) => t,
            FluxResult::Err(e) => panic!("{}: {:?}", msg, e),
        }
    }
    
    /// Returns the contained Err value, consuming the self value
    /// 
    /// # Panics
    /// 
    /// Panics if the value is an Ok, with a panic message provided by the Ok's value
    pub fn unwrap_err(self) -> E
    where
        T: fmt::Debug,
    {
        match self {
            FluxResult::Ok(t) => panic!("called `FluxResult::unwrap_err()` on an `Ok` value: {:?}", t),
            FluxResult::Err(e) => e,
        }
    }
    
    /// Calls op if the result is Ok, otherwise returns the Err value of self
    pub fn and_then<U, F>(self, op: F) -> FluxResult<U, E>
    where
        F: FnOnce(T) -> FluxResult<U, E>,
    {
        match self {
            FluxResult::Ok(t) => op(t),
            FluxResult::Err(e) => FluxResult::Err(e),
        }
    }
    
    /// Calls op if the result is Err, otherwise returns the Ok value of self
    pub fn or_else<F, O>(self, op: O) -> FluxResult<T, F>
    where
        O: FnOnce(E) -> FluxResult<T, F>,
    {
        match self {
            FluxResult::Ok(t) => FluxResult::Ok(t),
            FluxResult::Err(e) => op(e),
        }
    }
}

impl<T, E> From<Result<T, E>> for FluxResult<T, E> {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(t) => FluxResult::Ok(t),
            Err(e) => FluxResult::Err(e),
        }
    }
}

impl<T, E> From<FluxResult<T, E>> for Result<T, E> {
    fn from(flux_result: FluxResult<T, E>) -> Self {
        match flux_result {
            FluxResult::Ok(t) => Ok(t),
            FluxResult::Err(e) => Err(e),
        }
    }
}

impl<T, E> fmt::Display for FluxResult<T, E>
where
    T: fmt::Display,
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FluxResult::Ok(t) => write!(f, "Ok({})", t),
            FluxResult::Err(e) => write!(f, "Err({})", e),
        }
    }
}

/// Error propagation support
/// 
/// Since the Try trait is unstable, we provide a manual implementation
/// of the ? operator functionality through a macro and helper methods
impl<T, E> FluxResult<T, E> {
    /// Helper method for the ? operator functionality
    /// Returns the value if Ok, or returns early with the error
    pub fn try_unwrap(self) -> Result<T, E> {
        match self {
            FluxResult::Ok(t) => Ok(t),
            FluxResult::Err(e) => Err(e),
        }
    }
}

/// Macro to simulate the ? operator for FluxResult
#[macro_export]
macro_rules! flux_try {
    ($expr:expr) => {
        match $expr {
            FluxResult::Ok(val) => val,
            FluxResult::Err(err) => return FluxResult::Err(err.into()),
        }
    };
}

/// Error type hierarchy for Flux runtime errors
#[derive(Debug, Clone, PartialEq)]
pub enum FluxError {
    /// Runtime error during execution
    Runtime(RuntimeError),
    /// I/O operation error
    Io(IoError),
    /// Type conversion error
    Type(TypeError),
    /// Null pointer access error
    NullPointer(NullPointerError),
    /// Index out of bounds error
    IndexOutOfBounds(IndexError),
    /// Custom user-defined error
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeError {
    pub message: String,
    pub kind: RuntimeErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErrorKind {
    StackOverflow,
    OutOfMemory,
    DivisionByZero,
    Panic,
    Deadlock,
    ChannelClosed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IoError {
    pub message: String,
    pub kind: IoErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IoErrorKind {
    FileNotFound,
    PermissionDenied,
    ConnectionRefused,
    TimedOut,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub message: String,
    pub expected: String,
    pub found: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NullPointerError {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexError {
    pub message: String,
    pub index: i64,
    pub length: usize,
}

impl fmt::Display for FluxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FluxError::Runtime(e) => write!(f, "Runtime error: {}", e),
            FluxError::Io(e) => write!(f, "I/O error: {}", e),
            FluxError::Type(e) => write!(f, "Type error: {}", e),
            FluxError::NullPointer(e) => write!(f, "Null pointer error: {}", e),
            FluxError::IndexOutOfBounds(e) => write!(f, "Index out of bounds: {}", e),
            FluxError::Custom(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?})", self.message, self.kind)
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?})", self.message, self.kind)
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: expected {}, found {}", self.message, self.expected, self.found)
    }
}

impl fmt::Display for NullPointerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: index {} out of bounds for length {}", self.message, self.index, self.length)
    }
}

impl std::error::Error for FluxError {}
impl std::error::Error for RuntimeError {}
impl std::error::Error for IoError {}
impl std::error::Error for TypeError {}
impl std::error::Error for NullPointerError {}
impl std::error::Error for IndexError {}

/// Convenience macros for creating errors
#[macro_export]
macro_rules! flux_error {
    ($kind:expr, $msg:expr) => {
        FluxResult::Err(FluxError::Custom(format!($msg)))
    };
    ($kind:expr, $msg:expr, $($arg:tt)*) => {
        FluxResult::Err(FluxError::Custom(format!($msg, $($arg)*)))
    };
}

#[macro_export]
macro_rules! runtime_error {
    ($kind:expr, $msg:expr) => {
        FluxResult::Err(FluxError::Runtime(RuntimeError {
            message: $msg.to_string(),
            kind: $kind,
        }))
    };
    ($kind:expr, $msg:expr, $($arg:tt)*) => {
        FluxResult::Err(FluxError::Runtime(RuntimeError {
            message: format!($msg, $($arg)*),
            kind: $kind,
        }))
    };
}

#[macro_export]
macro_rules! io_error {
    ($kind:expr, $msg:expr) => {
        FluxResult::Err(FluxError::Io(IoError {
            message: $msg.to_string(),
            kind: $kind,
        }))
    };
    ($kind:expr, $msg:expr, $($arg:tt)*) => {
        FluxResult::Err(FluxError::Io(IoError {
            message: format!($msg, $($arg)*),
            kind: $kind,
        }))
    };
}

#[macro_export]
macro_rules! type_error {
    ($expected:expr, $found:expr, $msg:expr) => {
        FluxResult::Err(FluxError::Type(TypeError {
            message: $msg.to_string(),
            expected: $expected.to_string(),
            found: $found.to_string(),
        }))
    };
    ($expected:expr, $found:expr, $msg:expr, $($arg:tt)*) => {
        FluxResult::Err(FluxError::Type(TypeError {
            message: format!($msg, $($arg)*),
            expected: $expected.to_string(),
            found: $found.to_string(),
        }))
    };
}

/// Pattern matching support for Result types
pub trait ResultMatch<T, E> {
    /// Match on the Result with Ok and Err patterns
    fn match_result<R, F1, F2>(self, ok_fn: F1, err_fn: F2) -> R
    where
        F1: FnOnce(T) -> R,
        F2: FnOnce(E) -> R;
}

impl<T, E> ResultMatch<T, E> for FluxResult<T, E> {
    fn match_result<R, F1, F2>(self, ok_fn: F1, err_fn: F2) -> R
    where
        F1: FnOnce(T) -> R,
        F2: FnOnce(E) -> R,
    {
        match self {
            FluxResult::Ok(t) => ok_fn(t),
            FluxResult::Err(e) => err_fn(e),
        }
    }
}

/// Error propagation helper functions
pub mod propagation {
    use super::*;
    
    /// Try to execute a function and convert any panic to a FluxError
    pub fn try_catch<T, F>(f: F) -> FluxResult<T, FluxError>
    where
        F: FnOnce() -> T + std::panic::UnwindSafe,
    {
        match std::panic::catch_unwind(f) {
            Ok(result) => FluxResult::Ok(result),
            Err(panic) => {
                let message = if let Some(s) = panic.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                
                FluxResult::Err(FluxError::Runtime(RuntimeError {
                    message,
                    kind: RuntimeErrorKind::Panic,
                }))
            }
        }
    }
    
    /// Chain multiple fallible operations
    pub fn chain<T, E, F>(
        first: FluxResult<T, E>,
        second: F,
    ) -> FluxResult<T, E>
    where
        F: FnOnce() -> FluxResult<T, E>,
    {
        match first {
            FluxResult::Ok(t) => FluxResult::Ok(t),
            FluxResult::Err(_) => second(),
        }
    }
    
    /// Collect multiple Results into a single Result containing a Vec
    pub fn collect<T, E>(results: Vec<FluxResult<T, E>>) -> FluxResult<Vec<T>, E> {
        let mut values = Vec::new();
        
        for result in results {
            match result {
                FluxResult::Ok(t) => values.push(t),
                FluxResult::Err(e) => return FluxResult::Err(e),
            }
        }
        
        FluxResult::Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::propagation::*;

    #[test]
    fn test_result_creation() {
        let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
        let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
        
        assert!(ok_result.is_ok());
        assert!(!ok_result.is_err());
        assert!(!err_result.is_ok());
        assert!(err_result.is_err());
    }
    
    #[test]
    fn test_result_map() {
        let result: FluxResult<i32, String> = FluxResult::Ok(42);
        let mapped = result.map(|x| x * 2);
        
        assert_eq!(mapped, FluxResult::Ok(84));
        
        let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
        let mapped_err = err_result.map(|x| x * 2);
        
        assert_eq!(mapped_err, FluxResult::Err("error".to_string()));
    }
    
    #[test]
    fn test_result_and_then() {
        let result: FluxResult<i32, String> = FluxResult::Ok(42);
        let chained = result.and_then(|x| {
            if x > 0 {
                FluxResult::Ok(x * 2)
            } else {
                FluxResult::Err("negative".to_string())
            }
        });
        
        assert_eq!(chained, FluxResult::Ok(84));
        
        let negative_result: FluxResult<i32, String> = FluxResult::Ok(-5);
        let chained_err = negative_result.and_then(|x| {
            if x > 0 {
                FluxResult::Ok(x * 2)
            } else {
                FluxResult::Err("negative".to_string())
            }
        });
        
        assert_eq!(chained_err, FluxResult::Err("negative".to_string()));
    }
    
    #[test]
    fn test_result_unwrap() {
        let result: FluxResult<i32, String> = FluxResult::Ok(42);
        assert_eq!(result.unwrap(), 42);
        
        let result_with_default: FluxResult<i32, String> = FluxResult::Err("error".to_string());
        assert_eq!(result_with_default.unwrap_or(0), 0);
    }
    
    #[test]
    fn test_error_types() {
        let runtime_err = FluxError::Runtime(RuntimeError {
            message: "Stack overflow".to_string(),
            kind: RuntimeErrorKind::StackOverflow,
        });
        
        let io_err = FluxError::Io(IoError {
            message: "File not found".to_string(),
            kind: IoErrorKind::FileNotFound,
        });
        
        let type_err = FluxError::Type(TypeError {
            message: "Type mismatch".to_string(),
            expected: "int".to_string(),
            found: "string".to_string(),
        });
        
        assert!(format!("{}", runtime_err).contains("Stack overflow"));
        assert!(format!("{}", io_err).contains("File not found"));
        assert!(format!("{}", type_err).contains("expected int, found string"));
    }
    
    #[test]
    fn test_result_match() {
        let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
        let result = ok_result.match_result(
            |x| format!("Success: {}", x),
            |e| format!("Error: {}", e),
        );
        assert_eq!(result, "Success: 42");
        
        let err_result: FluxResult<i32, String> = FluxResult::Err("failed".to_string());
        let result = err_result.match_result(
            |x| format!("Success: {}", x),
            |e| format!("Error: {}", e),
        );
        assert_eq!(result, "Error: failed");
    }
    
    #[test]
    fn test_try_catch() {
        let safe_result = try_catch(|| 42);
        assert_eq!(safe_result, FluxResult::Ok(42));
        
        let panic_result = try_catch(|| panic!("test panic"));
        assert!(panic_result.is_err());
        if let FluxResult::Err(FluxError::Runtime(err)) = panic_result {
            assert_eq!(err.kind, RuntimeErrorKind::Panic);
            assert!(err.message.contains("test panic"));
        }
    }
    
    #[test]
    fn test_collect_results() {
        let results: Vec<FluxResult<i32, String>> = vec![
            FluxResult::Ok(1),
            FluxResult::Ok(2),
            FluxResult::Ok(3),
        ];
        
        let collected = collect(results);
        assert_eq!(collected, FluxResult::Ok(vec![1, 2, 3]));
        
        let results_with_error: Vec<FluxResult<i32, String>> = vec![
            FluxResult::Ok(1),
            FluxResult::Err("error".to_string()),
            FluxResult::Ok(3),
        ];
        
        let collected_err = collect(results_with_error);
        assert_eq!(collected_err, FluxResult::Err("error".to_string()));
    }
    
    #[test]
    fn test_error_macros() {
        let custom_err: FluxResult<(), FluxError> = flux_error!(FluxError::Custom("test".to_string()), "Custom error: {}", "test");
        assert!(custom_err.is_err());
        
        let runtime_err: FluxResult<(), FluxError> = runtime_error!(RuntimeErrorKind::StackOverflow, "Stack overflow occurred");
        assert!(runtime_err.is_err());
        
        let io_err: FluxResult<(), FluxError> = io_error!(IoErrorKind::FileNotFound, "File {} not found", "test.txt");
        assert!(io_err.is_err());
        
        let type_err: FluxResult<(), FluxError> = type_error!("int", "string", "Type mismatch in assignment");
        assert!(type_err.is_err());
    }
}