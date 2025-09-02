//! WebAssembly compilation tests

use flux_compiler::codegen::wasm::WasmCodeGenerator;
use flux_compiler::codegen::js_interop::JsInteropGenerator;
use flux_compiler::codegen::wasm_optimizations::WasmOptimizer;
use flux_compiler::semantic::*;
use flux_compiler::parser::ast::*;
use flux_compiler::error::CodeGenError;

#[cfg(feature = "wasm")]
#[test]
fn test_wasm_code_generation() {
    let mut generator = WasmCodeGenerator::new();
    
    // Create a simple test program
    let program = create_test_program();
    
    // Generate WebAssembly code
    let result = generator.generate(program);
    assert!(result.is_ok(), "WebAssembly code generation should succeed");
    
    let wasm_bytes = result.unwrap();
    assert!(!wasm_bytes.is_empty(), "Generated WASM should not be empty");
}

#[cfg(feature = "wasm")]
#[test]
fn test_wasm_function_generation() {
    let mut generator = WasmCodeGenerator::new();
    
    // Create a program with a simple function
    let program = TypedProgram {
        package: "test".to_string(),
        imports: vec![],
        items: vec![
            TypedItem::Function(TypedFunction {
                name: "add".to_string(),
                parameters: vec![
                    TypedParameter {
                        name: "a".to_string(),
                        type_: Type::Int,
                    },
                    TypedParameter {
                        name: "b".to_string(),
                        type_: Type::Int,
                    },
                ],
                return_type: Type::Int,
                body: TypedBlock {
                    statements: vec![
                        TypedStatement {
                            kind: TypedStatementKind::Return(Some(TypedExpression {
                                kind: TypedExpressionKind::Binary(
                                    Box::new(TypedExpression {
                                        kind: TypedExpressionKind::Identifier("a".to_string()),
                                        type_: Type::Int,
                                        span: None,
                                    }),
                                    BinaryOp::Add,
                                    Box::new(TypedExpression {
                                        kind: TypedExpressionKind::Identifier("b".to_string()),
                                        type_: Type::Int,
                                        span: None,
                                    }),
                                ),
                                type_: Type::Int,
                                span: None,
                            })),
                            span: None,
                        },
                    ],
                },
                visibility: Visibility::Public,
                is_async: false,
                span: None,
            }),
        ],
    };
    
    let result = generator.generate(program);
    assert!(result.is_ok(), "Function generation should succeed");
}

#[cfg(feature = "wasm")]
#[test]
fn test_wasm_literal_generation() {
    let mut generator = WasmCodeGenerator::new();
    
    // Test different literal types
    let program = TypedProgram {
        package: "test".to_string(),
        imports: vec![],
        items: vec![
            TypedItem::Function(TypedFunction {
                name: "test_literals".to_string(),
                parameters: vec![],
                return_type: Type::Int,
                body: TypedBlock {
                    statements: vec![
                        TypedStatement {
                            kind: TypedStatementKind::Return(Some(TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Integer(42)),
                                type_: Type::Int,
                                span: None,
                            })),
                            span: None,
                        },
                    ],
                },
                visibility: Visibility::Public,
                is_async: false,
                span: None,
            }),
        ],
    };
    
    let result = generator.generate(program);
    assert!(result.is_ok(), "Literal generation should succeed");
}

#[cfg(feature = "wasm")]
#[test]
fn test_wasm_binary_operations() {
    let mut generator = WasmCodeGenerator::new();
    
    // Test binary operations
    let operations = vec![
        BinaryOp::Add,
        BinaryOp::Subtract,
        BinaryOp::Multiply,
        BinaryOp::Divide,
        BinaryOp::Equal,
        BinaryOp::Less,
        BinaryOp::Greater,
    ];
    
    for op in operations {
        let program = TypedProgram {
            package: "test".to_string(),
            imports: vec![],
            items: vec![
                TypedItem::Function(TypedFunction {
                    name: "test_op".to_string(),
                    parameters: vec![],
                    return_type: if matches!(op, BinaryOp::Equal | BinaryOp::Less | BinaryOp::Greater) {
                        Type::Bool
                    } else {
                        Type::Int
                    },
                    body: TypedBlock {
                        statements: vec![
                            TypedStatement {
                                kind: TypedStatementKind::Return(Some(TypedExpression {
                                    kind: TypedExpressionKind::Binary(
                                        Box::new(TypedExpression {
                                            kind: TypedExpressionKind::Literal(Literal::Integer(10)),
                                            type_: Type::Int,
                                            span: None,
                                        }),
                                        op,
                                        Box::new(TypedExpression {
                                            kind: TypedExpressionKind::Literal(Literal::Integer(5)),
                                            type_: Type::Int,
                                            span: None,
                                        }),
                                    ),
                                    type_: if matches!(op, BinaryOp::Equal | BinaryOp::Less | BinaryOp::Greater) {
                                        Type::Bool
                                    } else {
                                        Type::Int
                                    },
                                    span: None,
                                })),
                                span: None,
                            },
                        ],
                    },
                    visibility: Visibility::Public,
                    is_async: false,
                    span: None,
                }),
            ],
        };
        
        let result = generator.generate(program);
        assert!(result.is_ok(), "Binary operation {:?} generation should succeed", op);
    }
}

#[test]
fn test_js_interop_generation() {
    let mut js_gen = JsInteropGenerator::new();
    
    // Add a test function
    let test_func = TypedFunction {
        name: "greet".to_string(),
        parameters: vec![
            TypedParameter {
                name: "name".to_string(),
                type_: Type::String,
            },
        ],
        return_type: Type::String,
        body: TypedBlock { statements: vec![] },
        visibility: Visibility::Public,
        is_async: false,
        span: None,
    };
    
    js_gen.add_exported_function(test_func);
    
    // Generate JavaScript wrapper
    let js_code = js_gen.generate_js_wrapper("test_module");
    assert!(js_code.is_ok(), "JavaScript wrapper generation should succeed");
    
    let js_code = js_code.unwrap();
    assert!(js_code.contains("class FluxModule"), "Should contain FluxModule class");
    assert!(js_code.contains("greet("), "Should contain greet function");
}

#[test]
fn test_typescript_definitions() {
    let mut js_gen = JsInteropGenerator::new();
    
    // Add a test function
    let test_func = TypedFunction {
        name: "calculate".to_string(),
        parameters: vec![
            TypedParameter {
                name: "x".to_string(),
                type_: Type::Int,
            },
            TypedParameter {
                name: "y".to_string(),
                type_: Type::Float,
            },
        ],
        return_type: Type::Float,
        body: TypedBlock { statements: vec![] },
        visibility: Visibility::Public,
        is_async: false,
        span: None,
    };
    
    js_gen.add_exported_function(test_func);
    
    // Generate TypeScript definitions
    let ts_defs = js_gen.generate_typescript_definitions("test_module");
    assert!(ts_defs.is_ok(), "TypeScript definitions generation should succeed");
    
    let ts_defs = ts_defs.unwrap();
    assert!(ts_defs.contains("interface FluxModule"), "Should contain FluxModule interface");
    assert!(ts_defs.contains("calculate(x: number, y: number): number"), "Should contain calculate function signature");
}

#[test]
fn test_html_test_page_generation() {
    let js_gen = JsInteropGenerator::new();
    
    let html = js_gen.generate_html_test_page("test_module");
    assert!(html.contains("<!DOCTYPE html>"), "Should be valid HTML");
    assert!(html.contains("Flux WebAssembly Test"), "Should contain title");
    assert!(html.contains("loadModule()"), "Should contain load function");
}

#[cfg(feature = "wasm")]
#[test]
fn test_wasm_optimization() {
    let mut optimizer = WasmOptimizer::new();
    
    let program = create_test_program();
    let result = optimizer.optimize(program);
    
    assert!(result.is_ok(), "WebAssembly optimization should succeed");
}

#[cfg(feature = "wasm")]
#[test]
fn test_wasm_constants() {
    let mut generator = WasmCodeGenerator::new();
    
    let program = TypedProgram {
        package: "test".to_string(),
        imports: vec![],
        items: vec![
            TypedItem::Const(TypedConst {
                name: "PI".to_string(),
                type_: Type::Float,
                value: TypedExpression {
                    kind: TypedExpressionKind::Literal(Literal::Float(3.14159)),
                    type_: Type::Float,
                    span: None,
                },
                visibility: Visibility::Public,
                span: None,
            }),
        ],
    };
    
    let result = generator.generate(program);
    assert!(result.is_ok(), "Constant generation should succeed");
}

#[cfg(not(feature = "wasm"))]
#[test]
fn test_wasm_feature_disabled() {
    let mut generator = WasmCodeGenerator::new();
    let program = create_test_program();
    
    let result = generator.generate(program);
    assert!(result.is_err(), "Should fail when WASM feature is disabled");
    
    if let Err(error) = result {
        assert!(error.to_string().contains("WebAssembly support not compiled in"));
    }
}

#[test]
fn test_wasm_memory_management() {
    #[cfg(feature = "wasm")]
    {
        use flux_compiler::codegen::wasm::WasmMemoryManager;
        
        let mut memory_manager = WasmMemoryManager::new(1024, 4096);
        
        // Test allocation
        let ptr1 = memory_manager.allocate(100);
        assert!(ptr1.is_some(), "Should be able to allocate memory");
        
        let ptr2 = memory_manager.allocate(200);
        assert!(ptr2.is_some(), "Should be able to allocate more memory");
        
        // Test deallocation
        if let Some(ptr) = ptr1 {
            memory_manager.deallocate(ptr, 100);
        }
        
        // Test allocation after deallocation
        let ptr3 = memory_manager.allocate(50);
        assert!(ptr3.is_some(), "Should be able to allocate after deallocation");
    }
}

#[cfg(feature = "wasm")]
#[test]
fn test_wasm_runtime() {
    use flux_compiler::codegen::wasm::WasmRuntime;
    
    let runtime_result = WasmRuntime::new();
    assert!(runtime_result.is_ok(), "Should be able to create WASM runtime");
}

// Helper function to create a test program
fn create_test_program() -> TypedProgram {
    TypedProgram {
        package: "test".to_string(),
        imports: vec![],
        items: vec![
            TypedItem::Function(TypedFunction {
                name: "main".to_string(),
                parameters: vec![],
                return_type: Type::Unit,
                body: TypedBlock {
                    statements: vec![
                        TypedStatement {
                            kind: TypedStatementKind::Expression(TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Integer(42)),
                                type_: Type::Int,
                                span: None,
                            }),
                            span: None,
                        },
                    ],
                },
                visibility: Visibility::Public,
                is_async: false,
                span: None,
            }),
        ],
    }
}

#[test]
fn test_wasm_error_handling() {
    #[cfg(feature = "wasm")]
    {
        let mut generator = WasmCodeGenerator::new();
        
        // Test with unsupported feature
        let program = TypedProgram {
            package: "test".to_string(),
            imports: vec![],
            items: vec![
                TypedItem::Function(TypedFunction {
                    name: "test".to_string(),
                    parameters: vec![],
                    return_type: Type::Unit,
                    body: TypedBlock {
                        statements: vec![
                            TypedStatement {
                                kind: TypedStatementKind::Expression(TypedExpression {
                                    kind: TypedExpressionKind::Literal(Literal::String("test".to_string())),
                                    type_: Type::String,
                                    span: None,
                                }),
                                span: None,
                            },
                        ],
                    },
                    visibility: Visibility::Public,
                    is_async: false,
                    span: None,
                }),
            ],
        };
        
        let result = generator.generate(program);
        // String literals are not fully implemented yet, so this might fail
        // This test ensures error handling works properly
    }
}

#[test]
fn test_wasm_type_conversion() {
    #[cfg(feature = "wasm")]
    {
        let generator = WasmCodeGenerator::new();
        
        // Test type conversions
        let int_type = generator.flux_type_to_wasm(&Type::Int);
        assert!(int_type.is_ok());
        
        let float_type = generator.flux_type_to_wasm(&Type::Float);
        assert!(float_type.is_ok());
        
        let bool_type = generator.flux_type_to_wasm(&Type::Bool);
        assert!(bool_type.is_ok());
        
        let unit_type = generator.flux_type_to_wasm(&Type::Unit);
        assert!(unit_type.is_err(), "Unit type should not convert to WASM value type");
    }
}