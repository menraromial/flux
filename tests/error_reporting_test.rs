//! Tests for Flux error reporting system

use flux_compiler::runtime::error_reporting::*;
use flux_compiler::runtime::result::*;
use flux_compiler::position::{Position, Span};
use std::path::PathBuf;

#[test]
fn test_error_reporter_basic_functionality() {
    let mut reporter = ErrorReporter::new();
    
    // Test adding source files
    let path = PathBuf::from("test.flux");
    let content = "let x = 42;\nlet y = x + 1;\nlet z = y * 2;".to_string();
    reporter.add_source_file(path.clone(), content.clone());
    
    // Verify source file was added
    assert!(reporter.source_files.contains_key("test.flux"));
    let source_file = &reporter.source_files["test.flux"];
    assert_eq!(source_file.content, content);
    assert_eq!(source_file.lines.len(), 3);
    assert_eq!(source_file.lines[0], "let x = 42;");
    assert_eq!(source_file.lines[1], "let y = x + 1;");
    assert_eq!(source_file.lines[2], "let z = y * 2;");
}

#[test]
fn test_source_context_generation() {
    let mut reporter = ErrorReporter::new();
    
    // Add a multi-line source file
    let path = PathBuf::from("example.flux");
    let content = vec![
        "func main() {",
        "    let x = 10;",
        "    let y = 0;",
        "    let result = x / y;", // Error on this line
        "    println(result);",
        "    return result;",
        "}",
    ].join("\n");
    
    reporter.add_source_file(path, content);
    
    // Create a span pointing to the division operation
    let span = Span {
        start: Position {
            line: 4,
            column: 17,
            offset: 0,
        },
        end: Position {
            line: 4,
            column: 22,
            offset: 0,
        },
    };
    
    let context = reporter.get_source_context(&span, "example.flux").unwrap();
    
    // Verify context structure
    assert_eq!(context.file_path, "example.flux");
    assert_eq!(context.error_line.0, 4);
    assert_eq!(context.error_line.1, "    let result = x / y;");
    assert_eq!(context.highlight_start, 17);
    assert_eq!(context.highlight_end, 22);
    
    // Check lines before and after
    assert!(!context.lines_before.is_empty());
    assert!(!context.lines_after.is_empty());
    
    // Verify specific lines
    assert!(context.lines_before.iter().any(|(_, line)| line.contains("let y = 0;")));
    assert!(context.lines_after.iter().any(|(_, line)| line.contains("println(result);")));
}

#[test]
fn test_error_suggestions_generation() {
    let reporter = ErrorReporter::new();
    
    // Test type error suggestions
    let type_error = FluxError::Type(TypeError {
        message: "Cannot assign string to int variable".to_string(),
        expected: "int".to_string(),
        found: "string".to_string(),
    });
    
    let suggestions = reporter.generate_suggestions(&type_error, None);
    assert!(!suggestions.is_empty());
    
    let first_suggestion = &suggestions[0];
    assert!(first_suggestion.message.contains("parsing"));
    assert_eq!(first_suggestion.suggestion_type, SuggestionType::TypeFix);
    assert!(first_suggestion.replacement.is_some());
    
    // Test runtime error suggestions
    let runtime_error = FluxError::Runtime(RuntimeError {
        message: "Division by zero occurred".to_string(),
        kind: RuntimeErrorKind::DivisionByZero,
    });
    
    let suggestions = reporter.generate_suggestions(&runtime_error, None);
    assert!(!suggestions.is_empty());
    
    let div_suggestion = &suggestions[0];
    assert!(div_suggestion.message.contains("divisor"));
    assert!(div_suggestion.replacement.is_some());
    
    // Test null pointer error suggestions
    let null_error = FluxError::NullPointer(NullPointerError {
        message: "Attempted to access null value".to_string(),
    });
    
    let suggestions = reporter.generate_suggestions(&null_error, None);
    assert!(!suggestions.is_empty());
    assert!(suggestions[0].message.contains("null"));
    
    // Test index out of bounds suggestions
    let index_error = FluxError::IndexOutOfBounds(IndexError {
        message: "Array index out of bounds".to_string(),
        index: 5,
        length: 3,
    });
    
    let suggestions = reporter.generate_suggestions(&index_error, None);
    assert!(!suggestions.is_empty());
    assert!(suggestions[0].message.contains("5"));
    assert!(suggestions[0].replacement.is_some());
}

#[test]
fn test_recovery_hints() {
    let reporter = ErrorReporter::new();
    
    // Test division by zero recovery hint
    let div_error = FluxError::Runtime(RuntimeError {
        message: "Division by zero".to_string(),
        kind: RuntimeErrorKind::DivisionByZero,
    });
    
    let hint = reporter.generate_recovery_hint(&div_error);
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("continue"));
    
    // Test stack overflow recovery hint
    let stack_error = FluxError::Runtime(RuntimeError {
        message: "Stack overflow".to_string(),
        kind: RuntimeErrorKind::StackOverflow,
    });
    
    let hint = reporter.generate_recovery_hint(&stack_error);
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("restarted"));
    
    // Test type error recovery hint
    let type_error = FluxError::Type(TypeError {
        message: "Type mismatch".to_string(),
        expected: "int".to_string(),
        found: "string".to_string(),
    });
    
    let hint = reporter.generate_recovery_hint(&type_error);
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("recompile"));
}

#[test]
fn test_stack_trace_collector() {
    let mut collector = StackTraceCollector::new();
    
    // Test empty collector
    assert!(collector.get_current_trace().is_none());
    
    // Add stack frames
    let frame1 = StackFrame {
        function_name: "main".to_string(),
        file_path: "main.flux".to_string(),
        line: 10,
        column: 5,
        source_line: Some("let result = calculate();".to_string()),
    };
    
    let frame2 = StackFrame {
        function_name: "calculate".to_string(),
        file_path: "math.flux".to_string(),
        line: 25,
        column: 10,
        source_line: Some("return divide(a, b);".to_string()),
    };
    
    let frame3 = StackFrame {
        function_name: "divide".to_string(),
        file_path: "math.flux".to_string(),
        line: 45,
        column: 15,
        source_line: Some("return a / b;".to_string()),
    };
    
    collector.push_frame(frame1.clone());
    collector.push_frame(frame2.clone());
    collector.push_frame(frame3.clone());
    
    // Verify stack trace
    let trace = collector.get_current_trace().unwrap();
    assert_eq!(trace.len(), 3);
    assert_eq!(trace[0].function_name, "main");
    assert_eq!(trace[1].function_name, "calculate");
    assert_eq!(trace[2].function_name, "divide");
    
    // Test popping frames
    let popped = collector.pop_frame().unwrap();
    assert_eq!(popped.function_name, "divide");
    
    let trace = collector.get_current_trace().unwrap();
    assert_eq!(trace.len(), 2);
    
    // Test clearing
    collector.clear();
    assert!(collector.get_current_trace().is_none());
}

#[test]
fn test_comprehensive_error_report() {
    let mut reporter = ErrorReporter::new();
    
    // Set up source file
    let path = PathBuf::from("error_example.flux");
    let content = vec![
        "func divide(a: int, b: int) -> int {",
        "    if b == 0 {",
        "        panic(\"Division by zero\");",
        "    }",
        "    return a / b;",
        "}",
        "",
        "func main() {",
        "    let x = 10;",
        "    let y = 0;",
        "    let result = divide(x, y);",
        "    println(result);",
        "}",
    ].join("\n");
    
    reporter.add_source_file(path, content);
    
    // Add stack frames
    reporter.push_stack_frame(StackFrame {
        function_name: "main".to_string(),
        file_path: "error_example.flux".to_string(),
        line: 11,
        column: 18,
        source_line: Some("    let result = divide(x, y);".to_string()),
    });
    
    reporter.push_stack_frame(StackFrame {
        function_name: "divide".to_string(),
        file_path: "error_example.flux".to_string(),
        line: 3,
        column: 9,
        source_line: Some("        panic(\"Division by zero\");".to_string()),
    });
    
    // Create error with location
    let error = FluxError::Runtime(RuntimeError {
        message: "Division by zero in divide function".to_string(),
        kind: RuntimeErrorKind::DivisionByZero,
    });
    
    let location = Span {
        start: Position {
            line: 3,
            column: 9,
            offset: 0,
        },
        end: Position {
            line: 3,
            column: 34,
            offset: 0,
        },
    };
    
    // Generate comprehensive report
    let report = reporter.generate_report(error, Some(location), Some("error_example.flux".to_string()));
    
    // Verify all components are present
    assert!(matches!(report.error, FluxError::Runtime(_)));
    assert!(report.location.is_some());
    assert!(report.source_context.is_some());
    assert!(!report.suggestions.is_empty());
    assert!(report.stack_trace.is_some());
    assert!(report.recovery_hint.is_some());
    
    // Verify source context
    let context = report.source_context.unwrap();
    assert_eq!(context.error_line.1, "        panic(\"Division by zero\");");
    assert!(context.lines_before.iter().any(|(_, line)| line.contains("if b == 0")));
    
    // Verify stack trace
    let stack_trace = report.stack_trace.unwrap();
    assert_eq!(stack_trace.len(), 2);
    assert_eq!(stack_trace[0].function_name, "main");
    assert_eq!(stack_trace[1].function_name, "divide");
    
    // Verify suggestions
    assert!(report.suggestions.iter().any(|s| s.message.contains("divisor")));
}

#[test]
fn test_error_report_display_formatting() {
    let error = FluxError::Runtime(RuntimeError {
        message: "Division by zero".to_string(),
        kind: RuntimeErrorKind::DivisionByZero,
    });
    
    let location = Span {
        start: Position {
            line: 5,
            column: 12,
            offset: 0,
        },
        end: Position {
            line: 5,
            column: 17,
            offset: 0,
        },
    };
    
    let source_context = SourceContext {
        file_path: "test.flux".to_string(),
        lines_before: vec![
            (3, "func calculate() {".to_string()),
            (4, "    let x = 10;".to_string()),
        ],
        error_line: (5, "    let y = x / 0;".to_string()),
        lines_after: vec![
            (6, "    return y;".to_string()),
            (7, "}".to_string()),
        ],
        highlight_start: 12,
        highlight_end: 17,
    };
    
    let suggestions = vec![
        ErrorSuggestion {
            message: "Check if the divisor is zero before dividing".to_string(),
            suggestion_type: SuggestionType::General,
            replacement: Some("if divisor != 0 { ... }".to_string()),
            span: Some(location.clone()),
        }
    ];
    
    let stack_trace = vec![
        StackFrame {
            function_name: "calculate".to_string(),
            file_path: "test.flux".to_string(),
            line: 5,
            column: 12,
            source_line: Some("    let y = x / 0;".to_string()),
        }
    ];
    
    let report = ErrorReport {
        error,
        location: Some(location),
        file_path: Some("test.flux".to_string()),
        source_context: Some(source_context),
        suggestions,
        stack_trace: Some(stack_trace),
        recovery_hint: Some("The program can continue by handling the division by zero case".to_string()),
    };
    
    let display = format!("{}", report);
    
    // Verify all sections are present
    assert!(display.contains("Error:"));
    assert!(display.contains("test.flux:5:12"));
    assert!(display.contains("   3 | func calculate() {"));
    assert!(display.contains("   5 |     let y = x / 0;"));  // Updated to match actual format
    assert!(display.contains("^^^^"));  // Error highlighting
    assert!(display.contains("Suggestions:"));
    assert!(display.contains("Check if the divisor is zero"));
    assert!(display.contains("Try: if divisor != 0"));
    assert!(display.contains("Stack trace:"));
    assert!(display.contains("0: calculate at test.flux:5:12"));
    assert!(display.contains("Recovery:"));
    assert!(display.contains("continue by handling"));
}

#[test]
fn test_recovery_strategies() {
    let mut reporter = ErrorReporter::new();
    
    // Test adding recovery strategies
    reporter.add_recovery_strategy(RecoveryStrategy::SkipStatement);
    reporter.add_recovery_strategy(RecoveryStrategy::InsertToken(";".to_string()));
    reporter.add_recovery_strategy(RecoveryStrategy::ReplaceToken("=".to_string(), "==".to_string()));
    reporter.add_recovery_strategy(RecoveryStrategy::SuggestAlternative("Use 'let' instead of 'var'".to_string()));
    
    let strategies = reporter.get_recovery_strategies();
    assert_eq!(strategies.len(), 4);
    
    assert!(matches!(strategies[0], RecoveryStrategy::SkipStatement));
    assert!(matches!(strategies[1], RecoveryStrategy::InsertToken(_)));
    assert!(matches!(strategies[2], RecoveryStrategy::ReplaceToken(_, _)));
    assert!(matches!(strategies[3], RecoveryStrategy::SuggestAlternative(_)));
}

#[test]
fn test_global_error_reporter_functions() {
    // Test initialization
    initialize_error_reporter();
    
    // Test adding source file globally
    let path = PathBuf::from("global_test.flux");
    let content = "let x = 42;".to_string();
    add_source_file(path, content);
    
    // Test stack frame operations
    let frame = StackFrame {
        function_name: "test_function".to_string(),
        file_path: "global_test.flux".to_string(),
        line: 1,
        column: 5,
        source_line: Some("let x = 42;".to_string()),
    };
    
    push_stack_frame(frame.clone());
    
    let popped = pop_stack_frame();
    assert!(popped.is_some());
    assert_eq!(popped.unwrap().function_name, "test_function");
    
    // Test error reporting
    let error = FluxError::Custom("Test error".to_string());
    let report = report_error(error, None, None);
    
    assert!(matches!(report.error, FluxError::Custom(_)));
}

#[test]
fn test_stack_frame_guard() {
    initialize_error_reporter();
    
    // Get initial frame count
    let initial_count = get_error_reporter()
        .map(|r| r.stack_trace_collector.frames.len())
        .unwrap_or(0);
    
    {
        // Create a stack frame guard
        let _guard = StackFrameGuard::new("test_function", "test.flux", 10, 5);
        
        // Verify frame was added
        let current_count = get_error_reporter()
            .map(|r| r.stack_trace_collector.frames.len())
            .unwrap_or(0);
        
        assert_eq!(current_count, initial_count + 1);
        
        // Verify frame details
        if let Some(reporter) = get_error_reporter() {
            let trace = reporter.stack_trace_collector.get_current_trace().unwrap();
            let last_frame = trace.last().unwrap();
            assert_eq!(last_frame.function_name, "test_function");
            assert_eq!(last_frame.file_path, "test.flux");
            assert_eq!(last_frame.line, 10);
            assert_eq!(last_frame.column, 5);
        }
    }
    
    // Verify frame was removed when guard went out of scope
    let final_count = get_error_reporter()
        .map(|r| r.stack_trace_collector.frames.len())
        .unwrap_or(0);
    
    assert_eq!(final_count, initial_count);
}

#[test]
fn test_suggestion_types_display() {
    assert_eq!(format!("{}", SuggestionType::SyntaxFix), "syntax fix");
    assert_eq!(format!("{}", SuggestionType::TypeFix), "type fix");
    assert_eq!(format!("{}", SuggestionType::AddImport), "add import");
    assert_eq!(format!("{}", SuggestionType::Rename), "rename");
    assert_eq!(format!("{}", SuggestionType::AddSemicolon), "add semicolon");
    assert_eq!(format!("{}", SuggestionType::Remove), "remove");
    assert_eq!(format!("{}", SuggestionType::General), "general");
}

#[test]
fn test_edge_cases() {
    let mut reporter = ErrorReporter::new();
    
    // Test source context with invalid line numbers
    let path = PathBuf::from("edge_case.flux");
    let content = "single line".to_string();
    reporter.add_source_file(path, content);
    
    let span = Span {
        start: Position {
            line: 10, // Line doesn't exist
            column: 1,
            offset: 0,
        },
        end: Position {
            line: 10,
            column: 5,
            offset: 0,
        },
    };
    
    let context = reporter.get_source_context(&span, "edge_case.flux");
    assert!(context.is_none());
    
    // Test with non-existent file
    let span2 = Span {
        start: Position {
            line: 1,
            column: 1,
            offset: 0,
        },
        end: Position {
            line: 1,
            column: 5,
            offset: 0,
        },
    };
    
    let context2 = reporter.get_source_context(&span2, "nonexistent.flux");
    assert!(context2.is_none());
    
    // Test stack trace collector with max frames
    let mut collector = StackTraceCollector::new();
    collector.max_frames = 2; // Set low limit for testing
    
    for i in 0..5 {
        collector.push_frame(StackFrame {
            function_name: format!("func_{}", i),
            file_path: "test.flux".to_string(),
            line: i + 1,
            column: 1,
            source_line: None,
        });
    }
    
    let trace = collector.get_current_trace().unwrap();
    assert_eq!(trace.len(), 2); // Should be limited to max_frames
}

#[test]
fn test_complex_error_scenarios() {
    let mut reporter = ErrorReporter::new();
    
    // Set up a complex source file with multiple potential errors
    let path = PathBuf::from("complex.flux");
    let content = vec![
        "import std/io;",
        "",
        "struct Point {",
        "    x: int,",
        "    y: int,",
        "}",
        "",
        "func distance(p1: Point, p2: Point) -> float {",
        "    let dx = p1.x - p2.x;",
        "    let dy = p1.y - p2.y;",
        "    return sqrt(dx * dx + dy * dy);",
        "}",
        "",
        "func main() {",
        "    let p1 = Point { x: 0, y: 0 };",
        "    let p2 = Point { x: 3, y: 4 };",
        "    let dist = distance(p1, p2);",
        "    println(\"Distance: {}\", dist);",
        "",
        "    // This will cause a type error",
        "    let invalid: int = \"not a number\";",
        "",
        "    // This will cause a runtime error",
        "    let zero = 0;",
        "    let result = 42 / zero;",
        "}",
    ].join("\n");
    
    reporter.add_source_file(path, content);
    
    // Test type error reporting
    let type_error = FluxError::Type(TypeError {
        message: "Cannot assign string to int variable".to_string(),
        expected: "int".to_string(),
        found: "string".to_string(),
    });
    
    let type_error_location = Span {
        start: Position {
            line: 21,
            column: 24,
            offset: 0,
        },
        end: Position {
            line: 21,
            column: 39,
            offset: 0,
        },
    };
    
    let type_report = reporter.generate_report(type_error, Some(type_error_location), Some("complex.flux".to_string()));
    
    // Verify type error report
    assert!(matches!(type_report.error, FluxError::Type(_)));
    assert!(type_report.source_context.is_some());
    assert!(!type_report.suggestions.is_empty());
    
    let type_context = type_report.source_context.unwrap();
    assert!(type_context.error_line.1.contains("\"not a number\""));
    
    // Test runtime error reporting with stack trace
    reporter.push_stack_frame(StackFrame {
        function_name: "main".to_string(),
        file_path: "complex.flux".to_string(),
        line: 25,
        column: 18,
        source_line: Some("    let result = 42 / zero;".to_string()),
    });
    
    let runtime_error = FluxError::Runtime(RuntimeError {
        message: "Division by zero".to_string(),
        kind: RuntimeErrorKind::DivisionByZero,
    });
    
    let runtime_error_location = Span {
        start: Position {
            line: 25,
            column: 18,
            offset: 0,
        },
        end: Position {
            line: 25,
            column: 28,
            offset: 0,
        },
    };
    
    let runtime_report = reporter.generate_report(runtime_error, Some(runtime_error_location), Some("complex.flux".to_string()));
    
    // Verify runtime error report
    assert!(matches!(runtime_report.error, FluxError::Runtime(_)));
    assert!(runtime_report.source_context.is_some());
    assert!(runtime_report.stack_trace.is_some());
    assert!(!runtime_report.suggestions.is_empty());
    assert!(runtime_report.recovery_hint.is_some());
    
    let runtime_context = runtime_report.source_context.unwrap();
    assert!(runtime_context.error_line.1.contains("42 / zero"));
    
    let stack_trace = runtime_report.stack_trace.unwrap();
    assert_eq!(stack_trace.len(), 1);
    assert_eq!(stack_trace[0].function_name, "main");
}