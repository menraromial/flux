//! Integration tests for the Flux compiler

use flux_compiler::*;
use flux_compiler::lexer::{FluxLexer, Token};
use flux_compiler::parser::{FluxParser, Parser};
use flux_compiler::semantic::{FluxSemanticAnalyzer, SemanticAnalyzer};
use flux_compiler::position::Position;

#[test]
fn test_basic_lexer() {
    let input = "let x = 42".to_string();
    let mut lexer = FluxLexer::new(input);
    
    assert_eq!(lexer.next_token().unwrap(), Token::Let);
    assert_eq!(lexer.next_token().unwrap(), Token::Identifier("x".to_string()));
    assert_eq!(lexer.next_token().unwrap(), Token::Assign);
    assert_eq!(lexer.next_token().unwrap(), Token::Integer(42));
    assert_eq!(lexer.next_token().unwrap(), Token::Eof);
}

#[test]
fn test_basic_parser() {
    let input = "func main() { let x = 42 }".to_string();
    let lexer = FluxLexer::new(input);
    let mut parser = FluxParser::new(lexer).unwrap();
    
    let program = parser.parse_program().unwrap();
    assert_eq!(program.items.len(), 1);
    
    match &program.items[0] {
        flux_compiler::parser::ast::Item::Function(func) => {
            assert_eq!(func.name, "main");
            assert_eq!(func.parameters.len(), 0);
        }
        _ => panic!("Expected function item"),
    }
}

#[test]
fn test_semantic_analysis() {
    let input = "func main() { return 42 }".to_string();
    let lexer = FluxLexer::new(input);
    let mut parser = FluxParser::new(lexer).unwrap();
    let program = parser.parse_program().unwrap();
    
    let mut analyzer = FluxSemanticAnalyzer::new();
    let typed_program = analyzer.analyze(program).unwrap();
    
    assert_eq!(typed_program.items.len(), 1);
}

#[test]
fn test_position_tracking() {
    let mut pos = Position::start();
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 1);
    assert_eq!(pos.offset, 0);
    
    pos.advance('a');
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 2);
    assert_eq!(pos.offset, 1);
    
    pos.advance('\n');
    assert_eq!(pos.line, 2);
    assert_eq!(pos.column, 1);
    assert_eq!(pos.offset, 2);
}

#[cfg(feature = "llvm")]
mod llvm_tests {
    use super::*;
    use flux_compiler::codegen::{LLVMCodeGenerator, CodeGenerator};
    use flux_compiler::semantic::{TypedProgram, TypedItem, TypedFunction, TypedBlock, TypedStatement, TypedExpression, TypedExpressionKind, TypedParameter};
    use flux_compiler::parser::ast::{Type, Literal, BinaryOp, Visibility};
    use inkwell::context::Context;

    #[test]
    fn test_llvm_context_creation() {
        let context = Context::create();
        let codegen = LLVMCodeGenerator::new(&context, "test_module");
        let ir = codegen.get_ir();
        assert!(ir.contains("test_module"));
    }

    #[test]
    fn test_simple_function_generation() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
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
            package: "test".to_string(),
            imports: vec![],
            items: vec![TypedItem::Function(typed_func)],
        };
        
        let result = codegen.generate(typed_program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        assert!(ir.contains("define"));
        assert!(ir.contains("add"));
        assert!(ir.contains("ret"));
    }

    #[test]
    fn test_literal_generation() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Create a simple function that returns a literal
        let typed_func = TypedFunction {
            name: "get_answer".to_string(),
            parameters: vec![],
            return_type: Type::Int,
            body: TypedBlock {
                statements: vec![
                    TypedStatement::Return(Some(TypedExpression {
                        kind: TypedExpressionKind::Literal(Literal::Integer(42)),
                        type_: Type::Int,
                    }))
                ],
                type_: Type::Int,
            },
            is_async: false,
            visibility: Visibility::Public,
        };
        
        let typed_program = TypedProgram {
            package: "test".to_string(),
            imports: vec![],
            items: vec![TypedItem::Function(typed_func)],
        };
        
        let result = codegen.generate(typed_program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        assert!(ir.contains("42"));
        assert!(ir.contains("ret i64"));
    }

    #[test]
    fn test_variable_assignment() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Create a function with variable assignment: func test() { let x = 10; x = 20; return x }
        let typed_func = TypedFunction {
            name: "test".to_string(),
            parameters: vec![],
            return_type: Type::Int,
            body: TypedBlock {
                statements: vec![
                    TypedStatement::Let(
                        "x".to_string(),
                        Type::Int,
                        Some(TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Integer(10)),
                            type_: Type::Int,
                        })
                    ),
                    TypedStatement::Assignment(
                        TypedExpression {
                            kind: TypedExpressionKind::Identifier("x".to_string()),
                            type_: Type::Int,
                        },
                        TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Integer(20)),
                            type_: Type::Int,
                        }
                    ),
                    TypedStatement::Return(Some(TypedExpression {
                        kind: TypedExpressionKind::Identifier("x".to_string()),
                        type_: Type::Int,
                    }))
                ],
                type_: Type::Int,
            },
            is_async: false,
            visibility: Visibility::Public,
        };
        
        let typed_program = TypedProgram {
            package: "test".to_string(),
            imports: vec![],
            items: vec![TypedItem::Function(typed_func)],
        };
        
        let result = codegen.generate(typed_program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        assert!(ir.contains("alloca"));
        assert!(ir.contains("store"));
        assert!(ir.contains("load"));
    }

    #[test]
    fn test_boolean_operations() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Create a function that returns a boolean comparison: func test() -> bool { return 5 < 10 }
        let typed_func = TypedFunction {
            name: "test".to_string(),
            parameters: vec![],
            return_type: Type::Bool,
            body: TypedBlock {
                statements: vec![
                    TypedStatement::Return(Some(TypedExpression {
                        kind: TypedExpressionKind::Binary(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Integer(5)),
                                type_: Type::Int,
                            }),
                            BinaryOp::Less,
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Integer(10)),
                                type_: Type::Int,
                            })
                        ),
                        type_: Type::Bool,
                    }))
                ],
                type_: Type::Bool,
            },
            is_async: false,
            visibility: Visibility::Public,
        };
        
        let typed_program = TypedProgram {
            package: "test".to_string(),
            imports: vec![],
            items: vec![TypedItem::Function(typed_func)],
        };
        
        let result = codegen.generate(typed_program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        assert!(ir.contains("icmp"));
        assert!(ir.contains("ret i1"));
    }

    #[test]
    fn test_float_operations() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Create a function with float arithmetic: func test() -> float { return 3.14 + 2.86 }
        let typed_func = TypedFunction {
            name: "test".to_string(),
            parameters: vec![],
            return_type: Type::Float,
            body: TypedBlock {
                statements: vec![
                    TypedStatement::Return(Some(TypedExpression {
                        kind: TypedExpressionKind::Binary(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Float(3.14)),
                                type_: Type::Float,
                            }),
                            BinaryOp::Add,
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Float(2.86)),
                                type_: Type::Float,
                            })
                        ),
                        type_: Type::Float,
                    }))
                ],
                type_: Type::Float,
            },
            is_async: false,
            visibility: Visibility::Public,
        };
        
        let typed_program = TypedProgram {
            package: "test".to_string(),
            imports: vec![],
            items: vec![TypedItem::Function(typed_func)],
        };
        
        let result = codegen.generate(typed_program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        assert!(ir.contains("fadd"));
        assert!(ir.contains("ret double"));
    }
}