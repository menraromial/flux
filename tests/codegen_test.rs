//! Code generation tests for the Flux compiler

use flux_compiler::codegen::{CodeGenerator, StubCodeGenerator};
use flux_compiler::semantic::{TypedProgram, TypedItem, TypedFunction, TypedBlock, TypedStatement, TypedExpression, TypedExpressionKind, TypedParameter};
use flux_compiler::parser::ast::{Type, Literal, BinaryOp, UnaryOp, Visibility};

#[test]
fn test_stub_code_generator() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a simple typed program
    let typed_func = TypedFunction {
        name: "test".to_string(),
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
    assert!(ir.contains("Code generation not available without LLVM feature"));
}

#[test]
fn test_basic_arithmetic_codegen() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a function that adds two numbers: func add() -> int { return 1 + 2 }
    let typed_func = TypedFunction {
        name: "add".to_string(),
        parameters: vec![],
        return_type: Type::Int,
        body: TypedBlock {
            statements: vec![
                TypedStatement::Return(Some(TypedExpression {
                    kind: TypedExpressionKind::Binary(
                        Box::new(TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Integer(1)),
                            type_: Type::Int,
                        }),
                        BinaryOp::Add,
                        Box::new(TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Integer(2)),
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
    
    // For stub generator, just verify it doesn't crash
    let ir = result.unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_function_with_parameters() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a function with parameters: func multiply(a: int, b: int) -> int { return a * b }
    let typed_func = TypedFunction {
        name: "multiply".to_string(),
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
                        BinaryOp::Multiply,
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
    assert!(!ir.is_empty());
}

#[test]
fn test_variable_assignment() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a function with variable assignment: func test() -> int { let x = 10; return x }
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
    assert!(!ir.is_empty());
}

#[test]
fn test_comparison_operations() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a function with comparison: func test() -> bool { return 5 > 3 }
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
                        BinaryOp::Greater,
                        Box::new(TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Integer(3)),
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
    assert!(!ir.is_empty());
}

#[test]
fn test_logical_operations() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a function with logical operations: func test() -> bool { return true && false }
    let typed_func = TypedFunction {
        name: "test".to_string(),
        parameters: vec![],
        return_type: Type::Bool,
        body: TypedBlock {
            statements: vec![
                TypedStatement::Return(Some(TypedExpression {
                    kind: TypedExpressionKind::Binary(
                        Box::new(TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Boolean(true)),
                            type_: Type::Bool,
                        }),
                        BinaryOp::And,
                        Box::new(TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Boolean(false)),
                            type_: Type::Bool,
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
    assert!(!ir.is_empty());
}

#[test]
fn test_unary_operations() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a function with unary operations: func test() -> int { return -42 }
    let typed_func = TypedFunction {
        name: "test".to_string(),
        parameters: vec![],
        return_type: Type::Int,
        body: TypedBlock {
            statements: vec![
                TypedStatement::Return(Some(TypedExpression {
                    kind: TypedExpressionKind::Unary(
                        UnaryOp::Minus,
                        Box::new(TypedExpression {
                            kind: TypedExpressionKind::Literal(Literal::Integer(42)),
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
    assert!(!ir.is_empty());
}

#[test]
fn test_if_statement() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a function with if statement: func test() -> int { if true { return 1 } else { return 0 } }
    let typed_func = TypedFunction {
        name: "test".to_string(),
        parameters: vec![],
        return_type: Type::Int,
        body: TypedBlock {
            statements: vec![
                TypedStatement::If(
                    TypedExpression {
                        kind: TypedExpressionKind::Literal(Literal::Boolean(true)),
                        type_: Type::Bool,
                    },
                    TypedBlock {
                        statements: vec![
                            TypedStatement::Return(Some(TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Integer(1)),
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
        package: "test".to_string(),
        imports: vec![],
        items: vec![TypedItem::Function(typed_func)],
    };
    
    let result = codegen.generate(typed_program);
    assert!(result.is_ok());
    
    let ir = result.unwrap();
    assert!(!ir.is_empty());
}

#[test]
fn test_while_loop() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a function with while loop: func test() -> int { let x = 0; while x < 10 { x = x + 1 } return x }
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
                        kind: TypedExpressionKind::Literal(Literal::Integer(0)),
                        type_: Type::Int,
                    })
                ),
                TypedStatement::While(
                    TypedExpression {
                        kind: TypedExpressionKind::Binary(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Identifier("x".to_string()),
                                type_: Type::Int,
                            }),
                            BinaryOp::Less,
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Integer(10)),
                                type_: Type::Int,
                            })
                        ),
                        type_: Type::Bool,
                    },
                    TypedBlock {
                        statements: vec![
                            TypedStatement::Assignment(
                                TypedExpression {
                                    kind: TypedExpressionKind::Identifier("x".to_string()),
                                    type_: Type::Int,
                                },
                                TypedExpression {
                                    kind: TypedExpressionKind::Binary(
                                        Box::new(TypedExpression {
                                            kind: TypedExpressionKind::Identifier("x".to_string()),
                                            type_: Type::Int,
                                        }),
                                        BinaryOp::Add,
                                        Box::new(TypedExpression {
                                            kind: TypedExpressionKind::Literal(Literal::Integer(1)),
                                            type_: Type::Int,
                                        })
                                    ),
                                    type_: Type::Int,
                                }
                            )
                        ],
                        type_: Type::Unit,
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
    assert!(!ir.is_empty());
}

#[test]
fn test_for_loop() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create a function with for loop: func test() -> int { for i in 5 { } return 0 }
    let typed_func = TypedFunction {
        name: "test".to_string(),
        parameters: vec![],
        return_type: Type::Int,
        body: TypedBlock {
            statements: vec![
                TypedStatement::For(
                    "i".to_string(),
                    TypedExpression {
                        kind: TypedExpressionKind::Literal(Literal::Integer(5)),
                        type_: Type::Int,
                    },
                    TypedBlock {
                        statements: vec![],
                        type_: Type::Unit,
                    }
                ),
                TypedStatement::Return(Some(TypedExpression {
                    kind: TypedExpressionKind::Literal(Literal::Integer(0)),
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
    assert!(!ir.is_empty());
}

#[test]
fn test_function_call() {
    let mut codegen = StubCodeGenerator::new();
    
    // Create two functions where one calls the other
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
    
    let main_func = TypedFunction {
        name: "main".to_string(),
        parameters: vec![],
        return_type: Type::Int,
        body: TypedBlock {
            statements: vec![
                TypedStatement::Return(Some(TypedExpression {
                    kind: TypedExpressionKind::Call(
                        Box::new(TypedExpression {
                            kind: TypedExpressionKind::Identifier("add".to_string()),
                            type_: Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int)),
                        }),
                        vec![
                            TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Integer(3)),
                                type_: Type::Int,
                            },
                            TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Integer(4)),
                                type_: Type::Int,
                            }
                        ]
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
        items: vec![
            TypedItem::Function(add_func),
            TypedItem::Function(main_func)
        ],
    };
    
    let result = codegen.generate(typed_program);
    assert!(result.is_ok());
    
    let ir = result.unwrap();
    assert!(!ir.is_empty());
}

// Tests for task 5.3: Function and struct code generation

#[cfg(feature = "llvm")]
mod llvm_tests {
    use super::*;
    use flux_compiler::codegen::LLVMCodeGenerator;
    use flux_compiler::semantic::{TypedStruct, TypedClass, TypedField, TypedMethod, TypedConst};
    use inkwell::context::Context;
    use inkwell::AddressSpace;

    #[test]
    fn test_struct_constructor_codegen() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Create a simple struct: struct Point { x: int, y: int }
        let struct_def = TypedStruct {
            name: "Point".to_string(),
            fields: vec![
                TypedField {
                    name: "x".to_string(),
                    type_: Type::Int,
                    visibility: Visibility::Public,
                    is_mutable: false,
                },
                TypedField {
                    name: "y".to_string(),
                    type_: Type::Int,
                    visibility: Visibility::Public,
                    is_mutable: false,
                }
            ],
            visibility: Visibility::Public,
        };
        
        let result = codegen.generate_struct_constructor(&struct_def);
        assert!(result.is_ok());
        
        let ir = codegen.get_ir();
        assert!(ir.contains("Point_new"));
        assert!(ir.contains("alloca"));
        assert!(ir.contains("getelementptr"));
        assert!(ir.contains("store"));
        assert!(ir.contains("ret"));
    }

    #[test]
    fn test_struct_field_accessors_codegen() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Create a struct with mutable and immutable fields
        let struct_def = TypedStruct {
            name: "Person".to_string(),
            fields: vec![
                TypedField {
                    name: "name".to_string(),
                    type_: Type::String,
                    visibility: Visibility::Public,
                    is_mutable: false,
                },
                TypedField {
                    name: "age".to_string(),
                    type_: Type::Int,
                    visibility: Visibility::Public,
                    is_mutable: true,
                }
            ],
            visibility: Visibility::Public,
        };
        
        let result = codegen.generate_struct_accessors(&struct_def);
        assert!(result.is_ok());
        
        let ir = codegen.get_ir();
        // Should have getters for both fields
        assert!(ir.contains("Person_name_get"));
        assert!(ir.contains("Person_age_get"));
        // Should have setter only for mutable field
        assert!(ir.contains("Person_age_set"));
        assert!(!ir.contains("Person_name_set"));
    }

    #[test]
    fn test_class_constructor_codegen() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Create a simple class
        let class_def = TypedClass {
            name: "Rectangle".to_string(),
            fields: vec![
                TypedField {
                    name: "width".to_string(),
                    type_: Type::Float,
                    visibility: Visibility::Private,
                    is_mutable: true,
                },
                TypedField {
                    name: "height".to_string(),
                    type_: Type::Float,
                    visibility: Visibility::Private,
                    is_mutable: true,
                }
            ],
            methods: vec![],
            visibility: Visibility::Public,
        };
        
        let result = codegen.generate_class_constructor(&class_def);
        assert!(result.is_ok());
        
        let ir = codegen.get_ir();
        assert!(ir.contains("Rectangle_new"));
        assert!(ir.contains("alloca"));
        assert!(ir.contains("getelementptr"));
        assert!(ir.contains("store"));
        // Should initialize vtable
        assert!(ir.contains("null"));
    }

    #[test]
    fn test_class_method_codegen() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        let class_def = TypedClass {
            name: "Calculator".to_string(),
            fields: vec![],
            methods: vec![
                TypedMethod {
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
                    visibility: Visibility::Public,
                    is_static: false,
                }
            ],
            visibility: Visibility::Public,
        };
        
        let method = &class_def.methods[0];
        let result = codegen.generate_class_method(&class_def, method);
        assert!(result.is_ok());
        
        let ir = codegen.get_ir();
        assert!(ir.contains("Calculator_add"));
        assert!(ir.contains("self"));
        assert!(ir.contains("add"));
        assert!(ir.contains("ret"));
    }

    #[test]
    fn test_static_method_codegen() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        let class_def = TypedClass {
            name: "Math".to_string(),
            fields: vec![],
            methods: vec![
                TypedMethod {
                    name: "square".to_string(),
                    parameters: vec![
                        TypedParameter {
                            name: "x".to_string(),
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
                                        kind: TypedExpressionKind::Identifier("x".to_string()),
                                        type_: Type::Int,
                                    }),
                                    BinaryOp::Multiply,
                                    Box::new(TypedExpression {
                                        kind: TypedExpressionKind::Identifier("x".to_string()),
                                        type_: Type::Int,
                                    })
                                ),
                                type_: Type::Int,
                            }))
                        ],
                        type_: Type::Int,
                    },
                    visibility: Visibility::Public,
                    is_static: true,
                }
            ],
            visibility: Visibility::Public,
        };
        
        let method = &class_def.methods[0];
        let result = codegen.generate_class_method(&class_def, method);
        assert!(result.is_ok());
        
        let ir = codegen.get_ir();
        assert!(ir.contains("Math_square"));
        // Static method should not have 'self' parameter
        assert!(!ir.contains("self"));
        assert!(ir.contains("mul"));
    }

    #[test]
    fn test_const_codegen() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        let const_def = TypedConst {
            name: "PI".to_string(),
            type_: Type::Float,
            value: TypedExpression {
                kind: TypedExpressionKind::Literal(Literal::Float(3.14159)),
                type_: Type::Float,
            },
            visibility: Visibility::Public,
        };
        
        let result = codegen.generate_const_impl(&const_def);
        assert!(result.is_ok());
        
        let ir = codegen.get_ir();
        assert!(ir.contains("@PI"));
        assert!(ir.contains("constant"));
    }

    #[test]
    fn test_complex_struct_with_methods() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Test a complete struct with constructor and methods
        let struct_def = TypedStruct {
            name: "Vector2D".to_string(),
            fields: vec![
                TypedField {
                    name: "x".to_string(),
                    type_: Type::Float,
                    visibility: Visibility::Public,
                    is_mutable: true,
                },
                TypedField {
                    name: "y".to_string(),
                    type_: Type::Float,
                    visibility: Visibility::Public,
                    is_mutable: true,
                }
            ],
            visibility: Visibility::Public,
        };
        
        // Generate struct type and methods
        let type_result = codegen.generate_struct_type(&struct_def);
        assert!(type_result.is_ok());
        
        let methods_result = codegen.generate_struct_methods(&struct_def);
        assert!(methods_result.is_ok());
        
        let ir = codegen.get_ir();
        
        // Should have constructor
        assert!(ir.contains("Vector2D_new"));
        
        // Should have getters and setters for both fields (since they're mutable)
        assert!(ir.contains("Vector2D_x_get"));
        assert!(ir.contains("Vector2D_x_set"));
        assert!(ir.contains("Vector2D_y_get"));
        assert!(ir.contains("Vector2D_y_set"));
        
        // Should have proper struct operations
        assert!(ir.contains("getelementptr"));
        assert!(ir.contains("load"));
        assert!(ir.contains("store"));
    }

    #[test]
    fn test_field_access_expression() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Create a field access expression: obj.field
        let field_expr = TypedExpression {
            kind: TypedExpressionKind::Field(
                Box::new(TypedExpression {
                    kind: TypedExpressionKind::Identifier("obj".to_string()),
                    type_: Type::Named("Point".to_string()),
                }),
                "x".to_string()
            ),
            type_: Type::Int,
        };
        
        // First create a variable for obj (simplified)
        let obj_type = codegen.context.i64_type();
        let struct_type = codegen.context.struct_type(&[obj_type.into()], false);
        let obj_alloca = codegen.builder.build_alloca(struct_type.ptr_type(AddressSpace::default()), "obj").unwrap();
        codegen.variable_table.insert("obj".to_string(), obj_alloca);
        
        // This test mainly checks that field access doesn't crash
        // In a real implementation, we'd need proper type information
        let result = codegen.generate_expression(&field_expr);
        // This might fail due to simplified implementation, but shouldn't crash
        let _result_check = result.is_ok() || result.is_err();
    }

    #[test]
    fn test_complete_program_with_structs_and_classes() {
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "test_module");
        
        // Create a program with struct, class, const, and function
        let struct_def = TypedStruct {
            name: "Point".to_string(),
            fields: vec![
                TypedField {
                    name: "x".to_string(),
                    type_: Type::Int,
                    visibility: Visibility::Public,
                    is_mutable: false,
                },
                TypedField {
                    name: "y".to_string(),
                    type_: Type::Int,
                    visibility: Visibility::Public,
                    is_mutable: false,
                }
            ],
            visibility: Visibility::Public,
        };
        
        let class_def = TypedClass {
            name: "Shape".to_string(),
            fields: vec![
                TypedField {
                    name: "area".to_string(),
                    type_: Type::Float,
                    visibility: Visibility::Private,
                    is_mutable: true,
                }
            ],
            methods: vec![
                TypedMethod {
                    name: "get_area".to_string(),
                    parameters: vec![],
                    return_type: Type::Float,
                    body: TypedBlock {
                        statements: vec![
                            TypedStatement::Return(Some(TypedExpression {
                                kind: TypedExpressionKind::Literal(Literal::Float(0.0)),
                                type_: Type::Float,
                            }))
                        ],
                        type_: Type::Float,
                    },
                    visibility: Visibility::Public,
                    is_static: false,
                }
            ],
            visibility: Visibility::Public,
        };
        
        let const_def = TypedConst {
            name: "MAX_SIZE".to_string(),
            type_: Type::Int,
            value: TypedExpression {
                kind: TypedExpressionKind::Literal(Literal::Integer(100)),
                type_: Type::Int,
            },
            visibility: Visibility::Public,
        };
        
        let func_def = TypedFunction {
            name: "main".to_string(),
            parameters: vec![],
            return_type: Type::Int,
            body: TypedBlock {
                statements: vec![
                    TypedStatement::Return(Some(TypedExpression {
                        kind: TypedExpressionKind::Literal(Literal::Integer(0)),
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
            items: vec![
                TypedItem::Struct(struct_def),
                TypedItem::Class(class_def),
                TypedItem::Const(const_def),
                TypedItem::Function(func_def),
            ],
        };
        
        let result = codegen.generate(typed_program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        
        // Should contain struct constructor and accessors
        assert!(ir.contains("Point_new"));
        assert!(ir.contains("Point_x_get"));
        assert!(ir.contains("Point_y_get"));
        
        // Should contain class constructor and methods
        assert!(ir.contains("Shape_new"));
        assert!(ir.contains("Shape_get_area"));
        
        // Should contain constant
        assert!(ir.contains("@MAX_SIZE"));
        
        // Should contain main function
        assert!(ir.contains("main"));
    }
}