//! Demonstration of LLVM code generation
//! 
//! This example shows how to use the Flux compiler's code generation
//! to produce LLVM IR from a simple Flux program.

use flux_compiler::codegen::CodeGenerator;

#[cfg(feature = "llvm")]
use flux_compiler::codegen::LLVMCodeGenerator;

#[cfg(feature = "llvm")]
use flux_compiler::semantic::{
    TypedProgram, TypedItem, TypedFunction, TypedBlock, TypedStatement, 
    TypedExpression, TypedExpressionKind, TypedParameter
};

#[cfg(feature = "llvm")]
use flux_compiler::parser::ast::{Type, Literal, BinaryOp, Visibility};

#[cfg(feature = "llvm")]
use inkwell::context::Context;

fn main() {
    #[cfg(feature = "llvm")]
    {
        println!("=== Flux LLVM Code Generation Demo ===\n");
        
        // Create LLVM context
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "flux_demo");
        
        // Create a simple function: func add(a: int, b: int) -> int { return a + b }
        let add_func = TypedFunction {
            name: "add".to_string(),
            parameters: vec![
                TypedParameter {
                    name: "a".to_string(),
                    type_: Type::Int,
                    is_mutable: false,
                },
                TypedParameter {
                    name: "b".to_string(),
                    type_: Type::Int,
                    is_mutable: false,
                }
            ],
            return_type: Type::Int,
            body: TypedBlock {
                statements: vec![
                    TypedStatement::Return(Some(TypedExpression {
                        kind: TypedExpressionKind::Binary(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Identifier("a".to_string()),
                                type_: Type::Int,
                            }),
                            BinaryOp::Add,
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Identifier("b".to_string()),
                                type_: Type::Int,
                            })
                        ),
                        type_: Type::Int,
                    }))
                ],
                type_: Type::Int,
            },
            is_async: false,
            visibility: Visibility::Public,
        };
        
        // Create a main function that calls add
        let main_func = TypedFunction {
            name: "main".to_string(),
            parameters: vec![],
            return_type: Type::Int,
            body: TypedBlock {
                statements: vec![
                    TypedStatement::Let(
                        "x".to_string(),
                        Type::Int,
                        Some(TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Integer(5)),
                            type_: Type::Int,
                        })
                    ),
                    TypedStatement::Let(
                        "y".to_string(),
                        Type::Int,
                        Some(TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Integer(3)),
                            type_: Type::Int,
                        })
                    ),
                    TypedStatement::If(
                        TypedExpression {
                            kind: TypedExpressionKind::Binary(
                                Box::new(TypedExpression {
                                    kind: TypedExpressionKind::Identifier("x".to_string()),
                                    type_: Type::Int,
                                }),
                                BinaryOp::Greater,
                                Box::new(TypedExpression {
                                    kind: TypedExpressionKind::Identifier("y".to_string()),
                                    type_: Type::Int,
                                })
                            ),
                            type_: Type::Bool,
                        },
                        TypedBlock {
                            statements: vec![
                                TypedStatement::Return(Some(TypedExpression {
                                    kind: TypedExpressionKind::Call(
                                        Box::new(TypedExpression {
                                            kind: TypedExpressionKind::Identifier("add".to_string()),
                                            type_: Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int)),
                                        }),
                                        vec![
                                            TypedExpression {
                                                kind: TypedExpressionKind::Identifier("x".to_string()),
                                                type_: Type::Int,
                                            },
                                            TypedExpression {
                                                kind: TypedExpressionKind::Identifier("y".to_string()),
                                                type_: Type::Int,
                                            }
                                        ]
                                    ),
                                    type_: Type::Int,
                                }))
                            ],
                            type_: Type::Int,
                        },
                        Some(TypedBlock {
                            statements: vec![
                                TypedStatement::Return(Some(TypedExpression {
                                    kind: TypedExpressionKind::Literal(Literal::Integer(0)),
                                    type_: Type::Int,
                                }))
                            ],
                            type_: Type::Int,
                        })
                    )
                ],
                type_: Type::Int,
            },
            is_async: false,
            visibility: Visibility::Public,
        };
        
        let typed_program = TypedProgram {
            package: "demo".to_string(),
            imports: vec![],
            items: vec![
                TypedItem::Function(add_func),
                TypedItem::Function(main_func)
            ],
        };
        
        // Generate LLVM IR
        match codegen.generate(typed_program) {
            Ok(ir) => {
                println!("Generated LLVM IR:");
                println!("{}", ir);
                println!("\n=== Code generation successful! ===");
            }
            Err(e) => {
                eprintln!("Code generation failed: {:?}", e);
            }
        }
    }
    
    #[cfg(not(feature = "llvm"))]
    {
        println!("LLVM feature not enabled. Run with: cargo run --example codegen_demo --features llvm");
    }
}