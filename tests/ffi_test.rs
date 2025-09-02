//! Tests for the FFI (Foreign Function Interface) system

use flux_compiler::ffi::*;
use flux_compiler::ffi::marshaling::*;
use flux_compiler::ffi::safety::*;
use flux_compiler::ffi::c_types::*;
use flux_compiler::ffi::error::*;
use flux_compiler::parser::ast::{Type as FluxType, ExternFunction as ASTExternFunction, Parameter, Visibility};

#[test]
fn test_ffi_registry_basic() {
    let mut registry = FFIRegistry::new();
    
    let extern_func = ASTExternFunction {
        name: "strlen".to_string(),
        parameters: vec![Parameter {
            name: "str".to_string(),
            type_: FluxType::String,
            is_mutable: false,
        }],
        return_type: Some(FluxType::Int),
        library: Some("C".to_string()),
        is_variadic: false,
        visibility: Visibility::Public,
    };
    
    // Convert to FFI representation
    let ffi_func = ExternFunction {
        name: extern_func.name.clone(),
        parameters: vec![CParameter {
            name: "str".to_string(),
            c_type: CType::Pointer(Box::new(CType::Char)),
        }],
        return_type: CType::Int,
        library: extern_func.library.clone(),
        is_variadic: extern_func.is_variadic,
    };
    
    assert!(registry.register_function(ffi_func).is_ok());
    assert!(registry.get_function("strlen").is_some());
}

#[test]
fn test_type_conversion() {
    // Test Flux to C type conversion
    assert_eq!(flux_to_c_type(&FluxType::Int).unwrap(), CType::Int);
    assert_eq!(flux_to_c_type(&FluxType::Float).unwrap(), CType::Double);
    assert_eq!(flux_to_c_type(&FluxType::String).unwrap(), CType::Pointer(Box::new(CType::Char)));
    assert_eq!(flux_to_c_type(&FluxType::Bool).unwrap(), CType::UChar);
    
    // Test C to Flux type conversion
    assert_eq!(c_to_flux_type(&CType::Int).unwrap(), FluxType::Int);
    assert_eq!(c_to_flux_type(&CType::Double).unwrap(), FluxType::Float);
    assert_eq!(c_to_flux_type(&CType::Pointer(Box::new(CType::Char))).unwrap(), FluxType::String);
}

#[test]
fn test_marshaling_primitives() {
    let mut ctx = MarshalingContext::new();
    
    // Test integer marshaling
    let int_val = FluxValue::Int(42);
    let marshaled = ctx.marshal_value(&int_val, &CType::Int).unwrap();
    match marshaled {
        MarshaledValue::Int(i) => assert_eq!(i, 42),
        _ => panic!("Expected marshaled int"),
    }
    
    // Test string marshaling
    let string_val = FluxValue::String("hello".to_string());
    let char_ptr_type = CType::Pointer(Box::new(CType::Char));
    let marshaled = ctx.marshal_value(&string_val, &char_ptr_type).unwrap();
    match marshaled {
        MarshaledValue::Pointer(ptr) => assert!(!ptr.is_null()),
        _ => panic!("Expected marshaled pointer"),
    }
}

#[test]
fn test_safety_checker() {
    let checker = SafetyChecker::new(SafetyLevel::Safe);
    
    // Test safe function
    let safe_func = ExternFunction {
        name: "add".to_string(),
        parameters: vec![
            CParameter { name: "a".to_string(), c_type: CType::Int },
            CParameter { name: "b".to_string(), c_type: CType::Int },
        ],
        return_type: CType::Int,
        library: None,
        is_variadic: false,
    };
    
    let report = checker.check_function_safety(&safe_func).unwrap();
    assert!(report.warnings.is_empty());
    
    // Test dangerous function with void pointer
    let dangerous_func = ExternFunction {
        name: "dangerous".to_string(),
        parameters: vec![
            CParameter { 
                name: "ptr".to_string(), 
                c_type: CType::Pointer(Box::new(CType::Void)) 
            },
        ],
        return_type: CType::Void,
        library: None,
        is_variadic: false,
    };
    
    let result = checker.check_function_safety(&dangerous_func);
    assert!(result.is_err());
}

#[test]
fn test_variadic_function_safety() {
    let checker = SafetyChecker::new(SafetyLevel::Safe);
    
    let printf_func = ExternFunction {
        name: "printf".to_string(),
        parameters: vec![
            CParameter { 
                name: "format".to_string(), 
                c_type: CType::Pointer(Box::new(CType::Char)) 
            },
        ],
        return_type: CType::Int,
        library: Some("C".to_string()),
        is_variadic: true,
    };
    
    let result = checker.check_function_safety(&printf_func);
    assert!(result.is_err()); // Variadic functions are considered dangerous
}

#[test]
fn test_trusted_function() {
    let checker = SafetyChecker::new(SafetyLevel::Strict);
    
    let strlen_func = ExternFunction {
        name: "strlen".to_string(),
        parameters: vec![
            CParameter { 
                name: "str".to_string(), 
                c_type: CType::Pointer(Box::new(CType::Char)) 
            },
        ],
        return_type: CType::Int,
        library: Some("C".to_string()),
        is_variadic: false,
    };
    
    let report = checker.check_function_safety(&strlen_func).unwrap();
    assert_eq!(report.trust_level, TrustLevel::Trusted);
}

#[test]
fn test_parameter_validation() {
    let ctx = MarshalingContext::new();
    
    let args = vec![
        FluxValue::Int(42),
        FluxValue::String("test".to_string()),
    ];
    
    let params = vec![
        CParameter { name: "x".to_string(), c_type: CType::Int },
        CParameter { 
            name: "s".to_string(), 
            c_type: CType::Pointer(Box::new(CType::Char)) 
        },
    ];
    
    assert!(ctx.validate_parameters(&args, &params).is_ok());
    
    // Test parameter count mismatch
    let wrong_args = vec![FluxValue::Int(42)];
    assert!(ctx.validate_parameters(&wrong_args, &params).is_err());
}

#[test]
fn test_null_pointer_safety() {
    let checker = SafetyChecker::new(SafetyLevel::Safe);
    
    let func = ExternFunction {
        name: "test".to_string(),
        parameters: vec![
            CParameter { 
                name: "ptr".to_string(), 
                c_type: CType::Pointer(Box::new(CType::Int)) 
            },
        ],
        return_type: CType::Void,
        library: None,
        is_variadic: false,
    };
    
    let args = vec![FluxValue::Null];
    let result = checker.check_call_safety(&func, &args);
    assert!(result.is_err());
}

#[test]
fn test_string_with_null_byte() {
    let checker = SafetyChecker::new(SafetyLevel::Safe);
    
    let func = ExternFunction {
        name: "test".to_string(),
        parameters: vec![
            CParameter { 
                name: "str".to_string(), 
                c_type: CType::Pointer(Box::new(CType::Char)) 
            },
        ],
        return_type: CType::Void,
        library: None,
        is_variadic: false,
    };
    
    let args = vec![FluxValue::String("hello\0world".to_string())];
    let result = checker.check_call_safety(&func, &args);
    assert!(result.is_err());
}

#[test]
fn test_type_sizes() {
    assert_eq!(c_type_size(&CType::Char), 1);
    assert_eq!(c_type_size(&CType::Int), 4);
    assert_eq!(c_type_size(&CType::Double), 8);
    assert_eq!(c_type_size(&CType::Pointer(Box::new(CType::Int))), std::mem::size_of::<*const ()>());
}

#[test]
fn test_array_marshaling() {
    let mut ctx = MarshalingContext::new();
    
    let array_val = FluxValue::Array(vec![
        FluxValue::Int(1),
        FluxValue::Int(2),
        FluxValue::Int(3),
    ]);
    
    let int_ptr_type = CType::Pointer(Box::new(CType::Int));
    let marshaled = ctx.marshal_value(&array_val, &int_ptr_type).unwrap();
    
    match marshaled {
        MarshaledValue::Pointer(ptr) => assert!(!ptr.is_null()),
        _ => panic!("Expected marshaled pointer"),
    }
}

#[test]
fn test_return_value_unmarshaling() {
    let ctx = MarshalingContext::new();
    
    // Test integer return
    let int_val = 42i32;
    let ptr = &int_val as *const i32 as *const std::ffi::c_void;
    let result = ctx.unmarshal_return(ptr, &CType::Int).unwrap();
    
    match result {
        FluxValue::Int(i) => assert_eq!(i, 42),
        _ => panic!("Expected unmarshaled int"),
    }
}

#[test]
fn test_complex_function_signature() {
    let mut registry = FFIRegistry::new();
    
    // Test a complex function like: int complex_func(char* str, int* arr, double val, ...);
    let complex_func = ExternFunction {
        name: "complex_func".to_string(),
        parameters: vec![
            CParameter { 
                name: "str".to_string(), 
                c_type: CType::Pointer(Box::new(CType::Char)) 
            },
            CParameter { 
                name: "arr".to_string(), 
                c_type: CType::Pointer(Box::new(CType::Int)) 
            },
            CParameter { 
                name: "val".to_string(), 
                c_type: CType::Double 
            },
        ],
        return_type: CType::Int,
        library: Some("mylib".to_string()),
        is_variadic: true,
    };
    
    // This should register successfully but trigger safety warnings
    let result = registry.register_function(complex_func);
    assert!(result.is_ok());
}

#[test]
fn test_error_messages() {
    use flux_compiler::ffi::error::FFIError;
    
    let error = FFIError::type_conversion("int", "string", "incompatible types");
    assert!(error.to_string().contains("Cannot convert type 'int' to 'string'"));
    
    let error = FFIError::safety_violation("dangerous_func", "null pointer dereference");
    assert!(error.to_string().contains("Safety violation in function 'dangerous_func'"));
    
    let error = FFIError::symbol_not_found("missing_func", Some("libtest.so"));
    assert!(error.to_string().contains("Symbol 'missing_func' not found in library 'libtest.so'"));
}

#[test]
fn test_ffi_caller_creation() {
    let caller = FFICaller::new(SafetyLevel::Safe);
    // Just test that it can be created without panicking
    drop(caller);
}

#[test]
fn test_ffi_function_resolution() {
    let mut caller = FFICaller::new(SafetyLevel::Safe);
    
    // Test resolving a simple function
    let simple_func = ExternFunction {
        name: "test_func".to_string(),
        parameters: vec![
            CParameter { name: "x".to_string(), c_type: CType::Int },
        ],
        return_type: CType::Int,
        library: Some("C".to_string()),
        is_variadic: false,
    };
    
    // This will likely fail since the function doesn't exist, but we test the mechanism
    let result = caller.resolve_function(simple_func);
    // We expect this to fail for a non-existent function
    assert!(result.is_err());
}

#[test]
fn test_unsafe_function_in_strict_mode() {
    let mut caller = FFICaller::new(SafetyLevel::Strict);
    
    let unsafe_func = ExternFunction {
        name: "unsafe_func".to_string(),
        parameters: vec![
            CParameter { 
                name: "ptr".to_string(), 
                c_type: CType::Pointer(Box::new(CType::Void)) 
            },
        ],
        return_type: CType::Void,
        library: Some("C".to_string()),
        is_variadic: false,
    };
    
    // Should fail due to safety checks
    let result = caller.resolve_function(unsafe_func);
    assert!(result.is_err());
}

#[test]
fn test_library_loading_error() {
    let mut caller = FFICaller::new(SafetyLevel::Safe);
    
    // Try to load a non-existent library
    let result = caller.load_library("nonexistent", "/path/to/nonexistent.so");
    assert!(result.is_err());
    
    match result.unwrap_err() {
        FFIError::LibraryLoad { library, .. } => {
            assert_eq!(library, "nonexistent");
        }
        _ => panic!("Expected LibraryLoad error"),
    }
}

#[test]
fn test_ffi_error_types() {
    // Test different error types
    let type_error = FFIError::type_conversion("int", "string", "incompatible");
    assert!(matches!(type_error, FFIError::TypeConversion { .. }));
    
    let marshaling_error = FFIError::marshaling("test", "details");
    assert!(matches!(marshaling_error, FFIError::Marshaling { .. }));
    
    let safety_error = FFIError::safety_violation("func", "violation");
    assert!(matches!(safety_error, FFIError::SafetyViolation { .. }));
}