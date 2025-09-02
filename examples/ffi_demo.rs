//! FFI (Foreign Function Interface) demonstration
//! 
//! This example shows how to use the Flux FFI system to call C functions.

use flux_compiler::ffi::*;
use flux_compiler::ffi::marshaling::*;
use flux_compiler::ffi::safety::*;
use flux_compiler::parser::ast::{Type as FluxType, ExternFunction as ASTExternFunction, Parameter, Visibility};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Flux FFI System Demo ===\n");

    // Create FFI registry
    let mut registry = FFIRegistry::new();
    println!("✓ Created FFI registry");

    // Create an extern function declaration for strlen
    let strlen_extern = ASTExternFunction {
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
    let strlen_ffi = ExternFunction {
        name: strlen_extern.name.clone(),
        parameters: vec![CParameter {
            name: "str".to_string(),
            c_type: CType::Pointer(Box::new(CType::Char)),
        }],
        return_type: CType::Int,
        library: strlen_extern.library.clone(),
        is_variadic: strlen_extern.is_variadic,
    };

    // Register the function
    registry.register_function(strlen_ffi)?;
    println!("✓ Registered extern function: strlen");

    // Demonstrate type conversion
    println!("\n=== Type Conversion Demo ===");
    
    let flux_int = FluxType::Int;
    let c_int = flux_to_c_type(&flux_int)?;
    println!("Flux int -> C type: {:?}", c_int);
    
    let flux_string = FluxType::String;
    let c_string = flux_to_c_type(&flux_string)?;
    println!("Flux string -> C type: {:?}", c_string);
    
    let c_double = CType::Double;
    let flux_double = c_to_flux_type(&c_double)?;
    println!("C double -> Flux type: {:?}", flux_double);

    // Demonstrate marshaling
    println!("\n=== Marshaling Demo ===");
    
    let mut marshaling_ctx = MarshalingContext::new();
    
    let flux_value = FluxValue::Int(42);
    let marshaled = marshaling_ctx.marshal_value(&flux_value, &CType::Int)?;
    println!("Marshaled int value: {:?}", marshaled);
    
    let flux_string_val = FluxValue::String("Hello, FFI!".to_string());
    let char_ptr_type = CType::Pointer(Box::new(CType::Char));
    let marshaled_string = marshaling_ctx.marshal_value(&flux_string_val, &char_ptr_type)?;
    println!("Marshaled string value: {:?}", marshaled_string);

    // Demonstrate safety checking
    println!("\n=== Safety Checking Demo ===");
    
    let safety_checker = SafetyChecker::new(SafetyLevel::Safe);
    
    // Safe function
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
    
    let safety_report = safety_checker.check_function_safety(&safe_func)?;
    println!("Safe function '{}' - warnings: {}", safe_func.name, safety_report.warnings.len());
    
    // Potentially dangerous function
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
    
    match safety_checker.check_function_safety(&dangerous_func) {
        Ok(report) => println!("Function '{}' passed safety check with {} warnings", 
                              dangerous_func.name, report.warnings.len()),
        Err(e) => println!("Function '{}' failed safety check: {}", dangerous_func.name, e),
    }

    // Demonstrate FFI caller
    println!("\n=== FFI Caller Demo ===");
    
    let mut caller = FFICaller::new(SafetyLevel::Safe);
    println!("✓ Created FFI caller with Safe security level");
    
    // Try to resolve a function (this will likely fail since we don't have the actual library)
    match caller.resolve_function(safe_func.clone()) {
        Ok(_) => println!("✓ Successfully resolved function: {}", safe_func.name),
        Err(e) => println!("⚠ Failed to resolve function '{}': {}", safe_func.name, e),
    }

    // Demonstrate error handling
    println!("\n=== Error Handling Demo ===");
    
    let type_error = FFIError::type_conversion("int", "string", "incompatible types");
    println!("Type conversion error: {}", type_error);
    
    let safety_error = FFIError::safety_violation("dangerous_func", "null pointer dereference");
    println!("Safety violation error: {}", safety_error);
    
    let symbol_error = FFIError::symbol_not_found("missing_func", Some("libtest.so"));
    println!("Symbol not found error: {}", symbol_error);

    println!("\n=== Demo Complete ===");
    println!("The Flux FFI system provides:");
    println!("• Type-safe marshaling between Flux and C types");
    println!("• Configurable safety levels (Unsafe, Safe, Strict)");
    println!("• Dynamic library loading and symbol resolution");
    println!("• Comprehensive error handling and reporting");
    println!("• Support for extern \"C\" function declarations");

    Ok(())
}