//! Basic code generation example for Flux compiler
//! 
//! This example demonstrates how to use the LLVM code generator
//! to generate IR from typed Flux AST nodes.

use flux_compiler::codegen::{CodeGenerator, StubCodeGenerator};
use flux_compiler::semantic::{TypedProgram, TypedItem, TypedFunction, TypedBlock, TypedStatement, TypedExpression, TypedExpressionKind, TypedParameter};
use flux_compiler::parser::ast::{Type, Literal, BinaryOp, Visibility};

fn main() {
    println!("Flux Compiler - Basic Code Generation Example");
    println!("==============================================");
    
    // Create a simple function: func add(a: int, b: int) -> int { return a + b }
    let typed_func = TypedFunction {
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
    
    let typed_program = TypedProgram {
        package: "example".to_string(),
        imports: vec![],
        items: vec![TypedItem::Function(typed_func)],
    };
    
    // Generate code using the stub generator (LLVM not available in this environment)
    let mut codegen = StubCodeGenerator::new();
    
    match codegen.generate(typed_program) {
        Ok(ir) => {
            println!("Generated IR:");
            println!("{}", ir);
        }
        Err(e) => {
            eprintln!("Code generation failed: {}", e);
        }
    }
    
    println!("\nNote: This example uses the stub code generator.");
    println!("To use LLVM code generation, compile with --features llvm");
    println!("and ensure LLVM 17 is installed on your system.");
}