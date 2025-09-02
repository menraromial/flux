//! Tests for Flux Result type and error handling system

use flux_compiler::runtime::result::*;
use flux_compiler::runtime::result::propagation::*;
use flux_compiler::{flux_error, runtime_error, io_error, type_error};

#[test]
fn test_result_basic_operations() {
    // Test Ok creation and methods
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    assert!(ok_result.is_ok());
    assert!(!ok_result.is_err());
    assert_eq!(ok_result.clone().ok(), Some(42));
    assert_eq!(ok_result.clone().err(), None);
    assert_eq!(ok_result.unwrap(), 42);
    
    // Test Err creation and methods
    let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    assert!(!err_result.is_ok());
    assert!(err_result.is_err());
    assert_eq!(err_result.clone().ok(), None);
    assert_eq!(err_result.clone().err(), Some("error".to_string()));
    assert_eq!(err_result.unwrap_or(0), 0);
}

#[test]
fn test_result_map_operations() {
    // Test map on Ok
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    let mapped = ok_result.map(|x| x * 2);
    assert_eq!(mapped, FluxResult::Ok(84));
    
    // Test map on Err
    let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    let mapped_err = err_result.map(|x| x * 2);
    assert_eq!(mapped_err, FluxResult::Err("error".to_string()));
    
    // Test map_err on Ok
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    let mapped_ok = ok_result.map_err(|e| format!("Error: {}", e));
    assert_eq!(mapped_ok, FluxResult::Ok(42));
    
    // Test map_err on Err
    let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    let mapped_err = err_result.map_err(|e| format!("Error: {}", e));
    assert_eq!(mapped_err, FluxResult::Err("Error: error".to_string()));
}

#[test]
fn test_result_and_then() {
    // Test and_then with Ok -> Ok
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    let chained = ok_result.and_then(|x| FluxResult::Ok(x * 2));
    assert_eq!(chained, FluxResult::Ok(84));
    
    // Test and_then with Ok -> Err
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(-5);
    let chained_err = ok_result.and_then(|x| {
        if x > 0 {
            FluxResult::Ok(x * 2)
        } else {
            FluxResult::Err("negative number".to_string())
        }
    });
    assert_eq!(chained_err, FluxResult::Err("negative number".to_string()));
    
    // Test and_then with Err
    let err_result: FluxResult<i32, String> = FluxResult::Err("initial error".to_string());
    let chained = err_result.and_then(|x| FluxResult::Ok(x * 2));
    assert_eq!(chained, FluxResult::Err("initial error".to_string()));
}

#[test]
fn test_result_or_else() {
    // Test or_else with Ok
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    let recovered = ok_result.or_else(|_| FluxResult::<i32, String>::Ok(0));
    assert_eq!(recovered, FluxResult::Ok(42));
    
    // Test or_else with Err -> Ok
    let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    let recovered = err_result.or_else(|_| FluxResult::<i32, String>::Ok(0));
    assert_eq!(recovered, FluxResult::Ok(0));
    
    // Test or_else with Err -> Err
    let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    let still_err = err_result.or_else(|e| FluxResult::Err(format!("Still: {}", e)));
    assert_eq!(still_err, FluxResult::Err("Still: error".to_string()));
}

#[test]
fn test_result_unwrap_variants() {
    // Test unwrap_or
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    assert_eq!(ok_result.unwrap_or(0), 42);
    
    let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    assert_eq!(err_result.unwrap_or(0), 0);
    
    // Test unwrap_or_else
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    assert_eq!(ok_result.unwrap_or_else(|_| 0), 42);
    
    let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    assert_eq!(err_result.unwrap_or_else(|e| e.len() as i32), 5);
}

#[test]
#[should_panic(expected = "called `FluxResult::unwrap()` on an `Err` value")]
fn test_result_unwrap_panic() {
    let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    err_result.unwrap();
}

#[test]
#[should_panic(expected = "Test panic: \"error\"")]
fn test_result_expect_panic() {
    let err_result: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    err_result.expect("Test panic");
}

#[test]
#[should_panic(expected = "called `FluxResult::unwrap_err()` on an `Ok` value")]
fn test_result_unwrap_err_panic() {
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    ok_result.unwrap_err();
}

#[test]
fn test_error_types() {
    // Test RuntimeError
    let runtime_err = FluxError::Runtime(RuntimeError {
        message: "Stack overflow occurred".to_string(),
        kind: RuntimeErrorKind::StackOverflow,
    });
    
    let display = format!("{}", runtime_err);
    assert!(display.contains("Runtime error"));
    assert!(display.contains("Stack overflow occurred"));
    assert!(display.contains("StackOverflow"));
    
    // Test IoError
    let io_err = FluxError::Io(IoError {
        message: "File not found".to_string(),
        kind: IoErrorKind::FileNotFound,
    });
    
    let display = format!("{}", io_err);
    assert!(display.contains("I/O error"));
    assert!(display.contains("File not found"));
    
    // Test TypeError
    let type_err = FluxError::Type(TypeError {
        message: "Type mismatch in assignment".to_string(),
        expected: "int".to_string(),
        found: "string".to_string(),
    });
    
    let display = format!("{}", type_err);
    assert!(display.contains("Type error"));
    assert!(display.contains("expected int, found string"));
    
    // Test NullPointerError
    let null_err = FluxError::NullPointer(NullPointerError {
        message: "Attempted to access null pointer".to_string(),
    });
    
    let display = format!("{}", null_err);
    assert!(display.contains("Null pointer error"));
    assert!(display.contains("Attempted to access null pointer"));
    
    // Test IndexError
    let index_err = FluxError::IndexOutOfBounds(IndexError {
        message: "Array access out of bounds".to_string(),
        index: 5,
        length: 3,
    });
    
    let display = format!("{}", index_err);
    assert!(display.contains("Index out of bounds"));
    assert!(display.contains("index 5 out of bounds for length 3"));
    
    // Test Custom error
    let custom_err = FluxError::Custom("Custom error message".to_string());
    let display = format!("{}", custom_err);
    assert!(display.contains("Error: Custom error message"));
}

#[test]
fn test_result_match() {
    // Test match on Ok
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    let result = ok_result.match_result(
        |x| format!("Success: {}", x),
        |e| format!("Error: {}", e),
    );
    assert_eq!(result, "Success: 42");
    
    // Test match on Err
    let err_result: FluxResult<i32, String> = FluxResult::Err("failed".to_string());
    let result = err_result.match_result(
        |x| format!("Success: {}", x),
        |e| format!("Error: {}", e),
    );
    assert_eq!(result, "Error: failed");
}

#[test]
fn test_try_catch() {
    // Test successful operation
    let safe_result = try_catch(|| 42);
    assert_eq!(safe_result, FluxResult::Ok(42));
    
    // Test panic handling
    let panic_result = try_catch(|| panic!("test panic"));
    assert!(panic_result.is_err());
    
    if let FluxResult::Err(FluxError::Runtime(err)) = panic_result {
        assert_eq!(err.kind, RuntimeErrorKind::Panic);
        assert!(err.message.contains("test panic"));
    } else {
        panic!("Expected runtime error with panic");
    }
    
    // Test string panic
    let string_panic_result = try_catch(|| panic!("string panic"));
    assert!(string_panic_result.is_err());
    
    // Test unknown panic type
    let unknown_panic_result = try_catch(|| {
        let boxed: Box<dyn std::any::Any + Send> = Box::new(42i32);
        std::panic::panic_any(boxed);
    });
    assert!(unknown_panic_result.is_err());
}

#[test]
fn test_chain_operations() {
    // Test chain with first success
    let first_ok: FluxResult<i32, String> = FluxResult::Ok(42);
    let chained = chain(first_ok, || FluxResult::<i32, String>::Ok(0));
    assert_eq!(chained, FluxResult::Ok(42));
    
    // Test chain with first failure, second success
    let first_err: FluxResult<i32, String> = FluxResult::Err("first error".to_string());
    let chained = chain(first_err, || FluxResult::Ok(100));
    assert_eq!(chained, FluxResult::Ok(100));
    
    // Test chain with both failures
    let first_err: FluxResult<i32, String> = FluxResult::Err("first error".to_string());
    let chained = chain(first_err, || FluxResult::Err("second error".to_string()));
    assert_eq!(chained, FluxResult::Err("second error".to_string()));
}

#[test]
fn test_collect_results() {
    // Test collecting all successful results
    let results = vec![
        FluxResult::<i32, String>::Ok(1),
        FluxResult::<i32, String>::Ok(2),
        FluxResult::<i32, String>::Ok(3),
    ];
    
    let collected = collect(results);
    assert_eq!(collected, FluxResult::Ok(vec![1, 2, 3]));
    
    // Test collecting with one error
    let results_with_error = vec![
        FluxResult::Ok(1),
        FluxResult::Err("error".to_string()),
        FluxResult::Ok(3),
    ];
    
    let collected_err = collect(results_with_error);
    assert_eq!(collected_err, FluxResult::Err("error".to_string()));
    
    // Test collecting empty vector
    let empty_results: Vec<FluxResult<i32, String>> = vec![];
    let collected_empty = collect(empty_results);
    assert_eq!(collected_empty, FluxResult::Ok(vec![]));
}

#[test]
fn test_conversion_from_std_result() {
    // Test conversion from std::Result::Ok
    let std_ok: Result<i32, String> = Ok(42);
    let flux_result: FluxResult<i32, String> = std_ok.into();
    assert_eq!(flux_result, FluxResult::Ok(42));
    
    // Test conversion from std::Result::Err
    let std_err: Result<i32, String> = Err("error".to_string());
    let flux_result: FluxResult<i32, String> = std_err.into();
    assert_eq!(flux_result, FluxResult::Err("error".to_string()));
}

#[test]
fn test_conversion_to_std_result() {
    // Test conversion to std::Result::Ok
    let flux_ok: FluxResult<i32, String> = FluxResult::Ok(42);
    let std_result: Result<i32, String> = flux_ok.into();
    assert_eq!(std_result, Ok(42));
    
    // Test conversion to std::Result::Err
    let flux_err: FluxResult<i32, String> = FluxResult::Err("error".to_string());
    let std_result: Result<i32, String> = flux_err.into();
    assert_eq!(std_result, Err("error".to_string()));
}

#[test]
fn test_error_macros() {
    // Test flux_error macro
    let custom_err: FluxResult<(), FluxError> = flux_error!(FluxError::Custom("test".to_string()), "Custom error: {}", "test");
    assert!(custom_err.is_err());
    if let FluxResult::Err(FluxError::Custom(msg)) = custom_err {
        assert_eq!(msg, "Custom error: test");
    }
    
    // Test runtime_error macro
    let runtime_err: FluxResult<(), FluxError> = runtime_error!(RuntimeErrorKind::StackOverflow, "Stack overflow occurred");
    assert!(runtime_err.is_err());
    if let FluxResult::Err(FluxError::Runtime(err)) = runtime_err {
        assert_eq!(err.kind, RuntimeErrorKind::StackOverflow);
        assert_eq!(err.message, "Stack overflow occurred");
    }
    
    // Test io_error macro
    let io_err: FluxResult<(), FluxError> = io_error!(IoErrorKind::FileNotFound, "File {} not found", "test.txt");
    assert!(io_err.is_err());
    if let FluxResult::Err(FluxError::Io(err)) = io_err {
        assert_eq!(err.kind, IoErrorKind::FileNotFound);
        assert_eq!(err.message, "File test.txt not found");
    }
    
    // Test type_error macro
    let type_err: FluxResult<(), FluxError> = type_error!("int", "string", "Type mismatch in assignment");
    assert!(type_err.is_err());
    if let FluxResult::Err(FluxError::Type(err)) = type_err {
        assert_eq!(err.expected, "int");
        assert_eq!(err.found, "string");
        assert_eq!(err.message, "Type mismatch in assignment");
    }
}

#[test]
fn test_result_display() {
    // Test Ok display
    let ok_result: FluxResult<i32, String> = FluxResult::Ok(42);
    assert_eq!(format!("{}", ok_result), "Ok(42)");
    
    // Test Err display
    let err_result: FluxResult<i32, String> = FluxResult::Err("error message".to_string());
    assert_eq!(format!("{}", err_result), "Err(error message)");
}

#[test]
fn test_complex_error_propagation_scenario() {
    // Simulate a complex operation that can fail at multiple points
    fn divide_and_multiply(a: i32, b: i32, c: i32) -> FluxResult<i32, FluxError> {
        if b == 0 {
            return runtime_error!(RuntimeErrorKind::DivisionByZero, "Division by zero");
        }
        
        let division_result = a / b;
        
        if division_result > 1000 {
            return FluxResult::Err(FluxError::Custom("Result too large".to_string()));
        }
        
        let final_result = division_result * c;
        FluxResult::Ok(final_result)
    }
    
    // Test successful case
    let success = divide_and_multiply(10, 2, 3);
    assert_eq!(success, FluxResult::Ok(15));
    
    // Test division by zero
    let div_zero = divide_and_multiply(10, 0, 3);
    assert!(div_zero.is_err());
    if let FluxResult::Err(FluxError::Runtime(err)) = div_zero {
        assert_eq!(err.kind, RuntimeErrorKind::DivisionByZero);
    }
    
    // Test result too large
    let too_large = divide_and_multiply(10000, 1, 3);
    assert!(too_large.is_err());
    if let FluxResult::Err(FluxError::Custom(msg)) = too_large {
        assert_eq!(msg, "Result too large");
    }
}

#[test]
fn test_nested_result_operations() {
    // Test chaining multiple operations that can fail
    fn parse_and_double(s: &str) -> FluxResult<i32, FluxError> {
        let parsed = match s.parse::<i32>() {
            Ok(val) => val,
            Err(_) => return FluxResult::Err(FluxError::Type(TypeError {
                message: "Failed to parse integer".to_string(),
                expected: "integer".to_string(),
                found: "string".to_string(),
            })),
        };
        
        if parsed < 0 {
            return runtime_error!(RuntimeErrorKind::Panic, "Negative numbers not allowed");
        }
        
        FluxResult::Ok(parsed * 2)
    }
    
    // Test successful parsing and doubling
    let success = parse_and_double("21");
    assert_eq!(success, FluxResult::Ok(42));
    
    // Test parse failure
    let parse_fail = parse_and_double("not_a_number");
    assert!(parse_fail.is_err());
    if let FluxResult::Err(FluxError::Type(_)) = parse_fail {
        // Expected
    } else {
        panic!("Expected type error");
    }
    
    // Test negative number
    let negative = parse_and_double("-5");
    assert!(negative.is_err());
    if let FluxResult::Err(FluxError::Runtime(err)) = negative {
        assert_eq!(err.kind, RuntimeErrorKind::Panic);
    }
}