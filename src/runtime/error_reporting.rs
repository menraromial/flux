//! Comprehensive error reporting system for Flux
//! 
//! Provides detailed error messages with source locations, suggestions,
//! error recovery strategies, and stack trace generation.

use crate::position::{Position, Span};
use crate::runtime::result::{FluxError, FluxResult, RuntimeError, RuntimeErrorKind};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Comprehensive error reporter with source location tracking
pub struct ErrorReporter {
    /// Source files for error reporting
    pub source_files: HashMap<String, SourceFile>,
    /// Error recovery strategies
    recovery_strategies: Vec<RecoveryStrategy>,
    /// Stack trace collector
    pub stack_trace_collector: StackTraceCollector,
}

/// Source file information for error reporting
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
    pub lines: Vec<String>,
}

/// Error recovery strategy
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Skip the current statement and continue
    SkipStatement,
    /// Insert a missing token
    InsertToken(String),
    /// Replace an incorrect token
    ReplaceToken(String, String),
    /// Suggest an alternative
    SuggestAlternative(String),
}

/// Stack trace collector for runtime errors
#[derive(Debug, Clone)]
pub struct StackTraceCollector {
    pub frames: Vec<StackFrame>,
    pub max_frames: usize,
}

/// Stack frame information
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function_name: String,
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub source_line: Option<String>,
}

/// Detailed error report with context and suggestions
#[derive(Debug, Clone)]
pub struct ErrorReport {
    pub error: FluxError,
    pub location: Option<Span>,
    pub file_path: Option<String>,
    pub source_context: Option<SourceContext>,
    pub suggestions: Vec<ErrorSuggestion>,
    pub stack_trace: Option<Vec<StackFrame>>,
    pub recovery_hint: Option<String>,
}

/// Source context around an error
#[derive(Debug, Clone)]
pub struct SourceContext {
    pub file_path: String,
    pub lines_before: Vec<(usize, String)>,
    pub error_line: (usize, String),
    pub lines_after: Vec<(usize, String)>,
    pub highlight_start: usize,
    pub highlight_end: usize,
}

/// Error suggestion for fixing issues
#[derive(Debug, Clone)]
pub struct ErrorSuggestion {
    pub message: String,
    pub suggestion_type: SuggestionType,
    pub replacement: Option<String>,
    pub span: Option<Span>,
}

/// Type of error suggestion
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionType {
    /// Fix a syntax error
    SyntaxFix,
    /// Fix a type error
    TypeFix,
    /// Add missing import
    AddImport,
    /// Rename identifier
    Rename,
    /// Add missing semicolon
    AddSemicolon,
    /// Remove unnecessary code
    Remove,
    /// General suggestion
    General,
}

impl ErrorReporter {
    /// Create a new error reporter
    pub fn new() -> Self {
        Self {
            source_files: HashMap::new(),
            recovery_strategies: Vec::new(),
            stack_trace_collector: StackTraceCollector::new(),
        }
    }
    
    /// Add a source file for error reporting
    pub fn add_source_file(&mut self, path: PathBuf, content: String) {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let source_file = SourceFile {
            path: path.clone(),
            content,
            lines,
        };
        self.source_files.insert(path.to_string_lossy().to_string(), source_file);
    }
    
    /// Generate a comprehensive error report
    pub fn generate_report(&self, error: FluxError, location: Option<Span>, file_path: Option<String>) -> ErrorReport {
        let source_context = location.as_ref().and_then(|span| {
            file_path.as_ref().and_then(|path| self.get_source_context(span, path))
        });
        
        let suggestions = self.generate_suggestions(&error, location.as_ref());
        let stack_trace = self.stack_trace_collector.get_current_trace();
        let recovery_hint = self.generate_recovery_hint(&error);
        
        ErrorReport {
            error,
            location,
            file_path,
            source_context,
            suggestions,
            stack_trace,
            recovery_hint,
        }
    }
    
    /// Get source context around an error location
    pub fn get_source_context(&self, span: &Span, file_path: &str) -> Option<SourceContext> {
        let source_file = self.source_files.get(file_path)?;
        
        let error_line_num = span.start.line as usize;
        if error_line_num == 0 || error_line_num > source_file.lines.len() {
            return None;
        }
        
        let context_size = 3; // Show 3 lines before and after
        let start_line = error_line_num.saturating_sub(context_size);
        let end_line = (error_line_num + context_size).min(source_file.lines.len());
        
        let mut lines_before = Vec::new();
        for i in start_line..error_line_num.saturating_sub(1) {
            if i < source_file.lines.len() {
                lines_before.push((i + 1, source_file.lines[i].clone()));
            }
        }
        
        let error_line = if error_line_num <= source_file.lines.len() {
            (error_line_num, source_file.lines[error_line_num - 1].clone())
        } else {
            (error_line_num, String::new())
        };
        
        let mut lines_after = Vec::new();
        for i in error_line_num..end_line {
            if i < source_file.lines.len() {
                lines_after.push((i + 1, source_file.lines[i].clone()));
            }
        }
        
        Some(SourceContext {
            file_path: file_path.to_string(),
            lines_before,
            error_line,
            lines_after,
            highlight_start: span.start.column,
            highlight_end: span.end.column,
        })
    }
    
    /// Generate helpful suggestions based on the error type
    pub fn generate_suggestions(&self, error: &FluxError, location: Option<&Span>) -> Vec<ErrorSuggestion> {
        let mut suggestions = Vec::new();
        
        match error {
            FluxError::Type(type_error) => {
                // Type error suggestions
                if type_error.expected == "int" && type_error.found == "string" {
                    suggestions.push(ErrorSuggestion {
                        message: "Try parsing the string to an integer".to_string(),
                        suggestion_type: SuggestionType::TypeFix,
                        replacement: Some(".parse::<i32>()".to_string()),
                        span: location.cloned(),
                    });
                }
                
                if type_error.expected.contains("Result") {
                    suggestions.push(ErrorSuggestion {
                        message: "Consider using pattern matching or the ? operator".to_string(),
                        suggestion_type: SuggestionType::General,
                        replacement: None,
                        span: location.cloned(),
                    });
                }
            }
            
            FluxError::Runtime(runtime_error) => {
                match runtime_error.kind {
                    RuntimeErrorKind::DivisionByZero => {
                        suggestions.push(ErrorSuggestion {
                            message: "Check if the divisor is zero before dividing".to_string(),
                            suggestion_type: SuggestionType::General,
                            replacement: Some("if divisor != 0 { ... }".to_string()),
                            span: location.cloned(),
                        });
                    }
                    
                    RuntimeErrorKind::StackOverflow => {
                        suggestions.push(ErrorSuggestion {
                            message: "Check for infinite recursion in your functions".to_string(),
                            suggestion_type: SuggestionType::General,
                            replacement: None,
                            span: location.cloned(),
                        });
                    }
                    
                    RuntimeErrorKind::OutOfMemory => {
                        suggestions.push(ErrorSuggestion {
                            message: "Consider reducing memory usage or increasing available memory".to_string(),
                            suggestion_type: SuggestionType::General,
                            replacement: None,
                            span: location.cloned(),
                        });
                    }
                    
                    _ => {}
                }
            }
            
            FluxError::NullPointer(_) => {
                suggestions.push(ErrorSuggestion {
                    message: "Check if the value is null before accessing it".to_string(),
                    suggestion_type: SuggestionType::General,
                    replacement: Some("if value != null { ... }".to_string()),
                    span: location.cloned(),
                });
            }
            
            FluxError::IndexOutOfBounds(index_error) => {
                suggestions.push(ErrorSuggestion {
                    message: format!("Check array bounds before accessing index {}", index_error.index),
                    suggestion_type: SuggestionType::General,
                    replacement: Some(format!("if index < array.len() {{ ... }}")),
                    span: location.cloned(),
                });
            }
            
            _ => {}
        }
        
        suggestions
    }
    
    /// Generate recovery hint for continuing after an error
    pub fn generate_recovery_hint(&self, error: &FluxError) -> Option<String> {
        match error {
            FluxError::Runtime(runtime_error) => {
                match runtime_error.kind {
                    RuntimeErrorKind::DivisionByZero => {
                        Some("The program can continue by handling the division by zero case".to_string())
                    }
                    RuntimeErrorKind::StackOverflow => {
                        Some("The program needs to be restarted due to stack overflow".to_string())
                    }
                    _ => None,
                }
            }
            FluxError::Type(_) => {
                Some("Fix the type mismatch and recompile".to_string())
            }
            _ => None,
        }
    }
    
    /// Add a recovery strategy
    pub fn add_recovery_strategy(&mut self, strategy: RecoveryStrategy) {
        self.recovery_strategies.push(strategy);
    }
    
    /// Get available recovery strategies
    pub fn get_recovery_strategies(&self) -> &[RecoveryStrategy] {
        &self.recovery_strategies
    }
    
    /// Push a stack frame for error tracking
    pub fn push_stack_frame(&mut self, frame: StackFrame) {
        self.stack_trace_collector.push_frame(frame);
    }
    
    /// Pop a stack frame
    pub fn pop_stack_frame(&mut self) -> Option<StackFrame> {
        self.stack_trace_collector.pop_frame()
    }
}

impl StackTraceCollector {
    /// Create a new stack trace collector
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            max_frames: 100, // Limit stack trace depth
        }
    }
    
    /// Push a new stack frame
    pub fn push_frame(&mut self, frame: StackFrame) {
        if self.frames.len() < self.max_frames {
            self.frames.push(frame);
        }
    }
    
    /// Pop the top stack frame
    pub fn pop_frame(&mut self) -> Option<StackFrame> {
        self.frames.pop()
    }
    
    /// Get the current stack trace
    pub fn get_current_trace(&self) -> Option<Vec<StackFrame>> {
        if self.frames.is_empty() {
            None
        } else {
            Some(self.frames.clone())
        }
    }
    
    /// Clear the stack trace
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

impl fmt::Display for ErrorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Error header
        writeln!(f, "Error: {}", self.error)?;
        
        // Location information
        if let Some(location) = &self.location {
            if let Some(file_path) = &self.file_path {
                writeln!(f, "  --> {}:{}:{}", file_path, location.start.line, location.start.column)?;
            } else {
                writeln!(f, "  --> {}:{}", location.start.line, location.start.column)?;
            }
        }
        
        // Source context
        if let Some(context) = &self.source_context {
            writeln!(f)?;
            
            // Lines before error
            for (line_num, line) in &context.lines_before {
                writeln!(f, "{:4} | {}", line_num, line)?;
            }
            
            // Error line with highlighting
            writeln!(f, "{:4} | {}", context.error_line.0, context.error_line.1)?;
            
            // Highlight the error position
            let spaces = " ".repeat(7 + context.highlight_start as usize);
            let carets = "^".repeat((context.highlight_end - context.highlight_start).max(1) as usize);
            writeln!(f, "{}{}", spaces, carets)?;
            
            // Lines after error
            for (line_num, line) in &context.lines_after {
                writeln!(f, "{:4} | {}", line_num, line)?;
            }
        }
        
        // Suggestions
        if !self.suggestions.is_empty() {
            writeln!(f)?;
            writeln!(f, "Suggestions:")?;
            for suggestion in &self.suggestions {
                writeln!(f, "  - {}", suggestion.message)?;
                if let Some(replacement) = &suggestion.replacement {
                    writeln!(f, "    Try: {}", replacement)?;
                }
            }
        }
        
        // Stack trace
        if let Some(stack_trace) = &self.stack_trace {
            writeln!(f)?;
            writeln!(f, "Stack trace:")?;
            for (i, frame) in stack_trace.iter().enumerate() {
                writeln!(f, "  {}: {} at {}:{}:{}", 
                    i, frame.function_name, frame.file_path, frame.line, frame.column)?;
                if let Some(source_line) = &frame.source_line {
                    writeln!(f, "      {}", source_line)?;
                }
            }
        }
        
        // Recovery hint
        if let Some(hint) = &self.recovery_hint {
            writeln!(f)?;
            writeln!(f, "Recovery: {}", hint)?;
        }
        
        Ok(())
    }
}

impl fmt::Display for SuggestionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SuggestionType::SyntaxFix => write!(f, "syntax fix"),
            SuggestionType::TypeFix => write!(f, "type fix"),
            SuggestionType::AddImport => write!(f, "add import"),
            SuggestionType::Rename => write!(f, "rename"),
            SuggestionType::AddSemicolon => write!(f, "add semicolon"),
            SuggestionType::Remove => write!(f, "remove"),
            SuggestionType::General => write!(f, "general"),
        }
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for StackTraceCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Global error reporter instance
static mut GLOBAL_ERROR_REPORTER: Option<ErrorReporter> = None;
static mut ERROR_REPORTER_INITIALIZED: bool = false;

/// Initialize the global error reporter
pub fn initialize_error_reporter() {
    unsafe {
        if !ERROR_REPORTER_INITIALIZED {
            GLOBAL_ERROR_REPORTER = Some(ErrorReporter::new());
            ERROR_REPORTER_INITIALIZED = true;
        }
    }
}

/// Get a reference to the global error reporter
pub fn get_error_reporter() -> Option<&'static mut ErrorReporter> {
    unsafe {
        GLOBAL_ERROR_REPORTER.as_mut()
    }
}

/// Report an error with comprehensive information
pub fn report_error(error: FluxError, location: Option<Span>, file_path: Option<String>) -> ErrorReport {
    initialize_error_reporter();
    
    if let Some(reporter) = get_error_reporter() {
        reporter.generate_report(error, location, file_path.clone())
    } else {
        // Fallback if reporter is not available
        ErrorReport {
            error,
            location,
            file_path,
            source_context: None,
            suggestions: Vec::new(),
            stack_trace: None,
            recovery_hint: None,
        }
    }
}

/// Add a source file to the global error reporter
pub fn add_source_file(path: PathBuf, content: String) {
    initialize_error_reporter();
    
    if let Some(reporter) = get_error_reporter() {
        reporter.add_source_file(path, content);
    }
}

/// Push a stack frame to the global error reporter
pub fn push_stack_frame(frame: StackFrame) {
    initialize_error_reporter();
    
    if let Some(reporter) = get_error_reporter() {
        reporter.push_stack_frame(frame);
    }
}

/// Pop a stack frame from the global error reporter
pub fn pop_stack_frame() -> Option<StackFrame> {
    initialize_error_reporter();
    
    if let Some(reporter) = get_error_reporter() {
        reporter.pop_stack_frame()
    } else {
        None
    }
}

/// Macro for automatic stack frame tracking
#[macro_export]
macro_rules! track_function {
    ($func_name:expr, $file:expr, $line:expr, $col:expr) => {
        let _guard = StackFrameGuard::new($func_name, $file, $line, $col);
    };
}

/// RAII guard for automatic stack frame management
pub struct StackFrameGuard {
    _phantom: std::marker::PhantomData<()>,
}

impl StackFrameGuard {
    pub fn new(func_name: &str, file: &str, line: usize, col: usize) -> Self {
        let frame = StackFrame {
            function_name: func_name.to_string(),
            file_path: file.to_string(),
            line,
            column: col,
            source_line: None,
        };
        
        push_stack_frame(frame);
        
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl Drop for StackFrameGuard {
    fn drop(&mut self) {
        pop_stack_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;
    use std::path::PathBuf;

    #[test]
    fn test_error_reporter_creation() {
        let reporter = ErrorReporter::new();
        assert!(reporter.source_files.is_empty());
        assert!(reporter.recovery_strategies.is_empty());
    }
    
    #[test]
    fn test_add_source_file() {
        let mut reporter = ErrorReporter::new();
        let path = PathBuf::from("test.flux");
        let content = "let x = 42;\nlet y = x + 1;".to_string();
        
        reporter.add_source_file(path.clone(), content.clone());
        
        assert!(reporter.source_files.contains_key("test.flux"));
        let source_file = &reporter.source_files["test.flux"];
        assert_eq!(source_file.content, content);
        assert_eq!(source_file.lines.len(), 2);
        assert_eq!(source_file.lines[0], "let x = 42;");
        assert_eq!(source_file.lines[1], "let y = x + 1;");
    }
    
    #[test]
    fn test_stack_trace_collector() {
        let mut collector = StackTraceCollector::new();
        
        let frame1 = StackFrame {
            function_name: "main".to_string(),
            file_path: "main.flux".to_string(),
            line: 10,
            column: 5,
            source_line: Some("let x = foo();".to_string()),
        };
        
        let frame2 = StackFrame {
            function_name: "foo".to_string(),
            file_path: "lib.flux".to_string(),
            line: 25,
            column: 10,
            source_line: Some("return bar();".to_string()),
        };
        
        collector.push_frame(frame1.clone());
        collector.push_frame(frame2.clone());
        
        let trace = collector.get_current_trace().unwrap();
        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0].function_name, "main");
        assert_eq!(trace[1].function_name, "foo");
        
        let popped = collector.pop_frame().unwrap();
        assert_eq!(popped.function_name, "foo");
        
        let trace = collector.get_current_trace().unwrap();
        assert_eq!(trace.len(), 1);
        assert_eq!(trace[0].function_name, "main");
    }
    
    #[test]
    fn test_error_suggestions() {
        let reporter = ErrorReporter::new();
        
        // Test type error suggestions
        let type_error = FluxError::Type(crate::runtime::result::TypeError {
            message: "Type mismatch".to_string(),
            expected: "int".to_string(),
            found: "string".to_string(),
        });
        
        let suggestions = reporter.generate_suggestions(&type_error, None);
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].message.contains("parsing"));
        assert_eq!(suggestions[0].suggestion_type, SuggestionType::TypeFix);
        
        // Test runtime error suggestions
        let runtime_error = FluxError::Runtime(RuntimeError {
            message: "Division by zero".to_string(),
            kind: RuntimeErrorKind::DivisionByZero,
        });
        
        let suggestions = reporter.generate_suggestions(&runtime_error, None);
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].message.contains("divisor"));
    }
    
    #[test]
    fn test_error_report_generation() {
        let mut reporter = ErrorReporter::new();
        
        // Add source file
        let path = PathBuf::from("test.flux");
        let content = "let x = 42;\nlet y = x / 0;\nlet z = y + 1;".to_string();
        reporter.add_source_file(path.clone(), content);
        
        // Create error with location
        let error = FluxError::Runtime(RuntimeError {
            message: "Division by zero".to_string(),
            kind: RuntimeErrorKind::DivisionByZero,
        });
        
        let location = Span {
            start: Position {
                line: 2,
                column: 9,
                offset: 0,
            },
            end: Position {
                line: 2,
                column: 13,
                offset: 0,
            },
        };
        
        let report = reporter.generate_report(error, Some(location), Some("test.flux".to_string()));
        
        assert!(matches!(report.error, FluxError::Runtime(_)));
        assert!(report.location.is_some());
        assert!(report.source_context.is_some());
        assert!(!report.suggestions.is_empty());
        assert!(report.recovery_hint.is_some());
        
        let context = report.source_context.unwrap();
        assert_eq!(context.error_line.0, 2);
        assert_eq!(context.error_line.1, "let y = x / 0;");
        assert_eq!(context.highlight_start, 9);
        assert_eq!(context.highlight_end, 13);
    }
    
    #[test]
    fn test_error_report_display() {
        let error = FluxError::Runtime(RuntimeError {
            message: "Division by zero".to_string(),
            kind: RuntimeErrorKind::DivisionByZero,
        });
        
        let location = Span {
            start: Position {
                line: 2,
                column: 9,
                offset: 0,
            },
            end: Position {
                line: 2,
                column: 13,
                offset: 0,
            },
        };
        
        let report = ErrorReport {
            error,
            location: Some(location),
            file_path: Some("test.flux".to_string()),
            source_context: None,
            suggestions: vec![ErrorSuggestion {
                message: "Check if the divisor is zero".to_string(),
                suggestion_type: SuggestionType::General,
                replacement: Some("if divisor != 0 { ... }".to_string()),
                span: None,
            }],
            stack_trace: None,
            recovery_hint: Some("Handle the division by zero case".to_string()),
        };
        
        let display = format!("{}", report);
        assert!(display.contains("Error:"));
        assert!(display.contains("test.flux:2:9"));
        assert!(display.contains("Suggestions:"));
        assert!(display.contains("Check if the divisor is zero"));
        assert!(display.contains("Recovery:"));
    }
    
    #[test]
    fn test_stack_frame_guard() {
        // This test verifies that the stack frame guard works correctly
        // In a real scenario, this would be used with the global error reporter
        
        let frame_count_before = {
            initialize_error_reporter();
            get_error_reporter().map(|r| r.stack_trace_collector.frames.len()).unwrap_or(0)
        };
        
        {
            let _guard = StackFrameGuard::new("test_function", "test.flux", 10, 5);
            
            let frame_count_during = {
                get_error_reporter().map(|r| r.stack_trace_collector.frames.len()).unwrap_or(0)
            };
            
            assert_eq!(frame_count_during, frame_count_before + 1);
        }
        
        let frame_count_after = {
            get_error_reporter().map(|r| r.stack_trace_collector.frames.len()).unwrap_or(0)
        };
        
        assert_eq!(frame_count_after, frame_count_before);
    }
}