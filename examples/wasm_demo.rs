//! WebAssembly compilation demo
//! 
//! This example demonstrates how to compile Flux code to WebAssembly
//! and generate JavaScript bindings for web integration.

use flux_compiler::codegen::wasm::WasmCodeGenerator;
use flux_compiler::codegen::js_interop::JsInteropGenerator;
use flux_compiler::codegen::wasm_optimizations::WasmOptimizer;
use flux_compiler::semantic::*;
use flux_compiler::parser::ast::*;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Flux WebAssembly Compilation Demo");
    
    // Create a sample Flux program
    let program = create_sample_program();
    
    // Demonstrate WebAssembly compilation
    #[cfg(feature = "wasm")]
    {
        println!("\n📦 Compiling to WebAssembly...");
        compile_to_wasm(program.clone())?;
        
        println!("\n🔧 Generating JavaScript bindings...");
        generate_js_bindings(&program)?;
        
        println!("\n⚡ Applying WebAssembly optimizations...");
        optimize_for_wasm(program)?;
    }
    
    #[cfg(not(feature = "wasm"))]
    {
        println!("❌ WebAssembly support not enabled. Compile with --features wasm");
    }
    
    println!("\n✅ Demo completed!");
    Ok(())
}

fn create_sample_program() -> TypedProgram {
    println!("Creating sample Flux program...");
    
    TypedProgram {
        package: "wasm_demo".to_string(),
        imports: vec![],
        items: vec![
            // A simple math function
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
            
            // A factorial function
            TypedItem::Function(TypedFunction {
                name: "factorial".to_string(),
                parameters: vec![
                    TypedParameter {
                        name: "n".to_string(),
                        type_: Type::Int,
                    },
                ],
                return_type: Type::Int,
                body: TypedBlock {
                    statements: vec![
                        TypedStatement {
                            kind: TypedStatementKind::If(
                                TypedExpression {
                                    kind: TypedExpressionKind::Binary(
                                        Box::new(TypedExpression {
                                            kind: TypedExpressionKind::Identifier("n".to_string()),
                                            type_: Type::Int,
                                            span: None,
                                        }),
                                        BinaryOp::LessEqual,
                                        Box::new(TypedExpression {
                                            kind: TypedExpressionKind::Literal(Literal::Integer(1)),
                                            type_: Type::Int,
                                            span: None,
                                        }),
                                    ),
                                    type_: Type::Bool,
                                    span: None,
                                },
                                TypedBlock {
                                    statements: vec![
                                        TypedStatement {
                                            kind: TypedStatementKind::Return(Some(TypedExpression {
                                                kind: TypedExpressionKind::Literal(Literal::Integer(1)),
                                                type_: Type::Int,
                                                span: None,
                                            })),
                                            span: None,
                                        },
                                    ],
                                },
                                Some(TypedBlock {
                                    statements: vec![
                                        TypedStatement {
                                            kind: TypedStatementKind::Return(Some(TypedExpression {
                                                kind: TypedExpressionKind::Binary(
                                                    Box::new(TypedExpression {
                                                        kind: TypedExpressionKind::Identifier("n".to_string()),
                                                        type_: Type::Int,
                                                        span: None,
                                                    }),
                                                    BinaryOp::Multiply,
                                                    Box::new(TypedExpression {
                                                        kind: TypedExpressionKind::Call(
                                                            Box::new(TypedExpression {
                                                                kind: TypedExpressionKind::Identifier("factorial".to_string()),
                                                                type_: Type::Function(vec![Type::Int], Box::new(Type::Int)),
                                                                span: None,
                                                            }),
                                                            vec![
                                                                TypedExpression {
                                                                    kind: TypedExpressionKind::Binary(
                                                                        Box::new(TypedExpression {
                                                                            kind: TypedExpressionKind::Identifier("n".to_string()),
                                                                            type_: Type::Int,
                                                                            span: None,
                                                                        }),
                                                                        BinaryOp::Subtract,
                                                                        Box::new(TypedExpression {
                                                                            kind: TypedExpressionKind::Literal(Literal::Integer(1)),
                                                                            type_: Type::Int,
                                                                            span: None,
                                                                        }),
                                                                    ),
                                                                    type_: Type::Int,
                                                                    span: None,
                                                                },
                                                            ],
                                                        ),
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
                                }),
                            ),
                            span: None,
                        },
                    ],
                },
                visibility: Visibility::Public,
                is_async: false,
                span: None,
            }),
            
            // A constant
            TypedItem::Const(TypedConst {
                name: "PI".to_string(),
                type_: Type::Float,
                value: TypedExpression {
                    kind: TypedExpressionKind::Literal(Literal::Float(3.14159265359)),
                    type_: Type::Float,
                    span: None,
                },
                visibility: Visibility::Public,
                span: None,
            }),
            
            // Main function for testing
            TypedItem::Function(TypedFunction {
                name: "main".to_string(),
                parameters: vec![],
                return_type: Type::Int,
                body: TypedBlock {
                    statements: vec![
                        TypedStatement {
                            kind: TypedStatementKind::Return(Some(TypedExpression {
                                kind: TypedExpressionKind::Call(
                                    Box::new(TypedExpression {
                                        kind: TypedExpressionKind::Identifier("add".to_string()),
                                        type_: Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int)),
                                        span: None,
                                    }),
                                    vec![
                                        TypedExpression {
                                            kind: TypedExpressionKind::Literal(Literal::Integer(10)),
                                            type_: Type::Int,
                                            span: None,
                                        },
                                        TypedExpression {
                                            kind: TypedExpressionKind::Literal(Literal::Integer(32)),
                                            type_: Type::Int,
                                            span: None,
                                        },
                                    ],
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
    }
}

#[cfg(feature = "wasm")]
fn compile_to_wasm(program: TypedProgram) -> Result<(), Box<dyn std::error::Error>> {
    let mut generator = WasmCodeGenerator::new();
    
    println!("  • Generating WebAssembly bytecode...");
    let wasm_bytes = generator.generate(program)?;
    
    println!("  • Generated {} bytes of WebAssembly code", wasm_bytes.len());
    
    // Save to file
    fs::write("target/wasm_demo.wasm", &wasm_bytes)?;
    println!("  • Saved WebAssembly module to target/wasm_demo.wasm");
    
    // Convert to WAT (WebAssembly Text format) for inspection
    #[cfg(feature = "wasm")]
    {
        use wat::parse;
        if let Ok(wat_text) = wasmprinter::print_bytes(&wasm_bytes) {
            fs::write("target/wasm_demo.wat", wat_text)?;
            println!("  • Saved WebAssembly text format to target/wasm_demo.wat");
        }
    }
    
    Ok(())
}

#[cfg(feature = "wasm")]
fn generate_js_bindings(program: &TypedProgram) -> Result<(), Box<dyn std::error::Error>> {
    let mut js_gen = JsInteropGenerator::new();
    
    // Add exported functions
    for item in &program.items {
        if let TypedItem::Function(func) = item {
            if func.visibility == Visibility::Public {
                js_gen.add_exported_function(func.clone());
            }
        }
    }
    
    println!("  • Generating JavaScript wrapper...");
    let js_code = js_gen.generate_js_wrapper("wasm_demo")?;
    fs::write("target/wasm_demo.js", js_code)?;
    println!("  • Saved JavaScript wrapper to target/wasm_demo.js");
    
    println!("  • Generating TypeScript definitions...");
    let ts_defs = js_gen.generate_typescript_definitions("wasm_demo")?;
    fs::write("target/wasm_demo.d.ts", ts_defs)?;
    println!("  • Saved TypeScript definitions to target/wasm_demo.d.ts");
    
    println!("  • Generating HTML test page...");
    let html_page = js_gen.generate_html_test_page("wasm_demo");
    fs::write("target/wasm_demo.html", html_page)?;
    println!("  • Saved HTML test page to target/wasm_demo.html");
    
    Ok(())
}

#[cfg(feature = "wasm")]
fn optimize_for_wasm(program: TypedProgram) -> Result<(), Box<dyn std::error::Error>> {
    let mut optimizer = WasmOptimizer::new();
    
    println!("  • Applying WebAssembly optimizations...");
    let optimized_program = optimizer.optimize(program)?;
    
    println!("  • Optimizations applied:");
    println!("    - Function inlining analysis");
    println!("    - Dead code elimination");
    println!("    - Memory access optimization");
    println!("    - Control flow optimization");
    
    // Generate optimized WebAssembly
    let mut generator = WasmCodeGenerator::new();
    let optimized_wasm = generator.generate(optimized_program)?;
    
    fs::write("target/wasm_demo_optimized.wasm", &optimized_wasm)?;
    println!("  • Saved optimized WebAssembly to target/wasm_demo_optimized.wasm");
    
    Ok(())
}

#[cfg(not(feature = "wasm"))]
fn compile_to_wasm(_program: TypedProgram) -> Result<(), Box<dyn std::error::Error>> {
    println!("WebAssembly compilation requires the 'wasm' feature");
    Ok(())
}

#[cfg(not(feature = "wasm"))]
fn generate_js_bindings(_program: &TypedProgram) -> Result<(), Box<dyn std::error::Error>> {
    println!("JavaScript binding generation requires the 'wasm' feature");
    Ok(())
}

#[cfg(not(feature = "wasm"))]
fn optimize_for_wasm(_program: TypedProgram) -> Result<(), Box<dyn std::error::Error>> {
    println!("WebAssembly optimization requires the 'wasm' feature");
    Ok(())
}