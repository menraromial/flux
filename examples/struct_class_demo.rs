//! Demonstration of struct and class code generation
//! 
//! This example shows how the Flux compiler generates LLVM IR for:
//! - Struct definitions with constructors and field accessors
//! - Class definitions with methods and vtables
//! - Constants and their initialization
//! - Field access expressions

use flux_compiler::codegen::{CodeGenerator, StubCodeGenerator};
use flux_compiler::semantic::{
    TypedProgram, TypedItem, TypedFunction, TypedStruct, TypedClass, TypedConst,
    TypedBlock, TypedStatement, TypedExpression, TypedExpressionKind, 
    TypedParameter, TypedField, TypedMethod
};
use flux_compiler::parser::ast::{Type, Literal, BinaryOp, Visibility};

#[cfg(feature = "llvm")]
use flux_compiler::codegen::LLVMCodeGenerator;
#[cfg(feature = "llvm")]
use inkwell::context::Context;

fn main() {
    println!("=== Flux Struct and Class Code Generation Demo ===\n");
    
    // Create a comprehensive program with structs, classes, and functions
    let program = create_demo_program();
    
    #[cfg(feature = "llvm")]
    {
        println!("Generating LLVM IR...\n");
        let context = Context::create();
        let mut codegen = LLVMCodeGenerator::new(&context, "struct_class_demo");
        
        match codegen.generate(program.clone()) {
            Ok(ir) => {
                println!("Generated LLVM IR:");
                println!("{}", ir);
            }
            Err(e) => {
                println!("Error generating LLVM IR: {:?}", e);
            }
        }
    }
    
    #[cfg(not(feature = "llvm"))]
    {
        println!("Using stub code generator (LLVM not available)...\n");
        let mut codegen = StubCodeGenerator::new();
        
        match codegen.generate(program) {
            Ok(output) => {
                println!("Stub generator output:");
                println!("{}", output);
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}

fn create_demo_program() -> TypedProgram {
    // Create a Point struct
    let point_struct = TypedStruct {
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
    
    // Create a Rectangle class with methods
    let rectangle_class = TypedClass {
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
        methods: vec![
            // Instance method: area calculation
            TypedMethod {
                name: "area".to_string(),
                parameters: vec![],
                return_type: Type::Float,
                body: TypedBlock {
                    statements: vec![
                        TypedStatement::Return(Some(TypedExpression {
                            kind: TypedExpressionKind::Binary(
                                Box::new(TypedExpression {
                                    kind: TypedExpressionKind::Field(
                                        Box::new(TypedExpression {
                                            kind: TypedExpressionKind::Identifier("self".to_string()),
                                            type_: Type::Named("Rectangle".to_string()),
                                        }),
                                        "width".to_string()
                                    ),
                                    type_: Type::Float,
                                }),
                                BinaryOp::Multiply,
                                Box::new(TypedExpression {
                                    kind: TypedExpressionKind::Field(
                                        Box::new(TypedExpression {
                                            kind: TypedExpressionKind::Identifier("self".to_string()),
                                            type_: Type::Named("Rectangle".to_string()),
                                        }),
                                        "height".to_string()
                                    ),
                                    type_: Type::Float,
                                })
                            ),
                            type_: Type::Float,
                        }))
                    ],
                    type_: Type::Float,
                },
                visibility: Visibility::Public,
                is_static: false,
            },
            // Static method: create square
            TypedMethod {
                name: "create_square".to_string(),
                parameters: vec![
                    TypedParameter {
                        name: "size".to_string(),
                        type_: Type::Float,
                        is_mutable: false,
                    }
                ],
                return_type: Type::Named("Rectangle".to_string()),
                body: TypedBlock {
                    statements: vec![
                        TypedStatement::Return(Some(TypedExpression {
                            kind: TypedExpressionKind::Call(
                                Box::new(TypedExpression {
                                    kind: TypedExpressionKind::Identifier("Rectangle_new".to_string()),
                                    type_: Type::Function(
                                        vec![Type::Float, Type::Float], 
                                        Box::new(Type::Named("Rectangle".to_string()))
                                    ),
                                }),
                                vec![
                                    TypedExpression {
                                        kind: TypedExpressionKind::Identifier("size".to_string()),
                                        type_: Type::Float,
                                    },
                                    TypedExpression {
                                        kind: TypedExpressionKind::Identifier("size".to_string()),
                                        type_: Type::Float,
                                    }
                                ]
                            ),
                            type_: Type::Named("Rectangle".to_string()),
                        }))
                    ],
                    type_: Type::Named("Rectangle".to_string()),
                },
                visibility: Visibility::Public,
                is_static: true,
            }
        ],
        visibility: Visibility::Public,
    };
    
    // Create constants
    let pi_const = TypedConst {
        name: "PI".to_string(),
        type_: Type::Float,
        value: TypedExpression {
            kind: TypedExpressionKind::Literal(Literal::Float(3.14159)),
            type_: Type::Float,
        },
        visibility: Visibility::Public,
    };
    
    let max_points_const = TypedConst {
        name: "MAX_POINTS".to_string(),
        type_: Type::Int,
        value: TypedExpression {
            kind: TypedExpressionKind::Literal(Literal::Integer(1000)),
            type_: Type::Int,
        },
        visibility: Visibility::Public,
    };
    
    // Create a main function that uses structs and classes
    let main_function = TypedFunction {
        name: "main".to_string(),
        parameters: vec![],
        return_type: Type::Int,
        body: TypedBlock {
            statements: vec![
                // Create a point: let p = Point_new(10, 20)
                TypedStatement::Let(
                    "p".to_string(),
                    Type::Named("Point".to_string()),
                    Some(TypedExpression {
                        kind: TypedExpressionKind::Call(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Identifier("Point_new".to_string()),
                                type_: Type::Function(
                                    vec![Type::Int, Type::Int], 
                                    Box::new(Type::Named("Point".to_string()))
                                ),
                            }),
                            vec![
                                TypedExpression {
                                    kind: TypedExpressionKind::Literal(Literal::Integer(10)),
                                    type_: Type::Int,
                                },
                                TypedExpression {
                                    kind: TypedExpressionKind::Literal(Literal::Integer(20)),
                                    type_: Type::Int,
                                }
                            ]
                        ),
                        type_: Type::Named("Point".to_string()),
                    })
                ),
                // Create a rectangle: let r = Rectangle_new(5.0, 3.0)
                TypedStatement::Let(
                    "r".to_string(),
                    Type::Named("Rectangle".to_string()),
                    Some(TypedExpression {
                        kind: TypedExpressionKind::Call(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Identifier("Rectangle_new".to_string()),
                                type_: Type::Function(
                                    vec![Type::Float, Type::Float], 
                                    Box::new(Type::Named("Rectangle".to_string()))
                                ),
                            }),
                            vec![
                                TypedExpression {
                                    kind: TypedExpressionKind::Literal(Literal::Float(5.0)),
                                    type_: Type::Float,
                                },
                                TypedExpression {
                                    kind: TypedExpressionKind::Literal(Literal::Float(3.0)),
                                    type_: Type::Float,
                                }
                            ]
                        ),
                        type_: Type::Named("Rectangle".to_string()),
                    })
                ),
                // Access point field: let x = Point_x_get(p)
                TypedStatement::Let(
                    "x".to_string(),
                    Type::Int,
                    Some(TypedExpression {
                        kind: TypedExpressionKind::Call(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Identifier("Point_x_get".to_string()),
                                type_: Type::Function(
                                    vec![Type::Named("Point".to_string())], 
                                    Box::new(Type::Int)
                                ),
                            }),
                            vec![
                                TypedExpression {
                                    kind: TypedExpressionKind::Identifier("p".to_string()),
                                    type_: Type::Named("Point".to_string()),
                                }
                            ]
                        ),
                        type_: Type::Int,
                    })
                ),
                // Call rectangle method: let area = Rectangle_area(r)
                TypedStatement::Let(
                    "area".to_string(),
                    Type::Float,
                    Some(TypedExpression {
                        kind: TypedExpressionKind::Call(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Identifier("Rectangle_area".to_string()),
                                type_: Type::Function(
                                    vec![Type::Named("Rectangle".to_string())], 
                                    Box::new(Type::Float)
                                ),
                            }),
                            vec![
                                TypedExpression {
                                    kind: TypedExpressionKind::Identifier("r".to_string()),
                                    type_: Type::Named("Rectangle".to_string()),
                                }
                            ]
                        ),
                        type_: Type::Float,
                    })
                ),
                // Return success
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
    
    // Create utility function that demonstrates field access
    let distance_function = TypedFunction {
        name: "distance".to_string(),
        parameters: vec![
            TypedParameter {
                name: "p1".to_string(),
                type_: Type::Named("Point".to_string()),
                is_mutable: false,
            },
            TypedParameter {
                name: "p2".to_string(),
                type_: Type::Named("Point".to_string()),
                is_mutable: false,
            }
        ],
        return_type: Type::Float,
        body: TypedBlock {
            statements: vec![
                // Calculate dx = p2.x - p1.x (using field access)
                TypedStatement::Let(
                    "dx".to_string(),
                    Type::Int,
                    Some(TypedExpression {
                        kind: TypedExpressionKind::Binary(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Field(
                                    Box::new(TypedExpression {
                                        kind: TypedExpressionKind::Identifier("p2".to_string()),
                                        type_: Type::Named("Point".to_string()),
                                    }),
                                    "x".to_string()
                                ),
                                type_: Type::Int,
                            }),
                            BinaryOp::Subtract,
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Field(
                                    Box::new(TypedExpression {
                                        kind: TypedExpressionKind::Identifier("p1".to_string()),
                                        type_: Type::Named("Point".to_string()),
                                    }),
                                    "x".to_string()
                                ),
                                type_: Type::Int,
                            })
                        ),
                        type_: Type::Int,
                    })
                ),
                // Calculate dy = p2.y - p1.y
                TypedStatement::Let(
                    "dy".to_string(),
                    Type::Int,
                    Some(TypedExpression {
                        kind: TypedExpressionKind::Binary(
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Field(
                                    Box::new(TypedExpression {
                                        kind: TypedExpressionKind::Identifier("p2".to_string()),
                                        type_: Type::Named("Point".to_string()),
                                    }),
                                    "y".to_string()
                                ),
                                type_: Type::Int,
                            }),
                            BinaryOp::Subtract,
                            Box::new(TypedExpression {
                                kind: TypedExpressionKind::Field(
                                    Box::new(TypedExpression {
                                        kind: TypedExpressionKind::Identifier("p1".to_string()),
                                        type_: Type::Named("Point".to_string()),
                                    }),
                                    "y".to_string()
                                ),
                                type_: Type::Int,
                            })
                        ),
                        type_: Type::Int,
                    })
                ),
                // Return simplified distance (dx + dy as float)
                TypedStatement::Return(Some(TypedExpression {
                    kind: TypedExpressionKind::Binary(
                        Box::new(TypedExpression {
                            kind: TypedExpressionKind::Identifier("dx".to_string()),
                            type_: Type::Int,
                        }),
                        BinaryOp::Add,
                        Box::new(TypedExpression {
                            kind: TypedExpressionKind::Identifier("dy".to_string()),
                            type_: Type::Int,
                        })
                    ),
                    type_: Type::Int,
                }))
            ],
            type_: Type::Float,
        },
        is_async: false,
        visibility: Visibility::Public,
    };
    
    TypedProgram {
        package: "struct_class_demo".to_string(),
        imports: vec![],
        items: vec![
            TypedItem::Struct(point_struct),
            TypedItem::Class(rectangle_class),
            TypedItem::Const(pi_const),
            TypedItem::Const(max_points_const),
            TypedItem::Function(main_function),
            TypedItem::Function(distance_function),
        ],
    }
}