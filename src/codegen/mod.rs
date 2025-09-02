//! Code generation module
//! 
//! Provides LLVM-based and WebAssembly code generation for Flux programs.

use crate::error::{CodeGenError, CodeGenErrorKind};
use crate::semantic::*;
use crate::parser::ast::{Type, Literal, BinaryOp, UnaryOp};
use std::collections::HashMap;

pub mod wasm;
pub mod js_interop;
pub mod wasm_optimizations;

#[cfg(feature = "llvm")]
use inkwell::context::Context;
#[cfg(feature = "llvm")]
use inkwell::module::Module;
#[cfg(feature = "llvm")]
use inkwell::builder::Builder;
#[cfg(feature = "llvm")]
use inkwell::values::{FunctionValue, BasicValueEnum, BasicValue, PointerValue, IntValue, FloatValue};
#[cfg(feature = "llvm")]
use inkwell::types::{BasicTypeEnum, FunctionType, BasicType};
#[cfg(feature = "llvm")]
use inkwell::{IntPredicate, FloatPredicate, AddressSpace};

/// Core code generator trait
pub trait CodeGenerator {
    /// Generate code for a typed program
    fn generate(&mut self, program: TypedProgram) -> Result<String, CodeGenError>;
}

/// LLVM-based code generator for Flux
#[cfg(feature = "llvm")]
pub struct LLVMCodeGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    function_table: HashMap<String, FunctionValue<'ctx>>,
    current_function: Option<FunctionValue<'ctx>>,
    variable_table: HashMap<String, PointerValue<'ctx>>,
}

/// Stub code generator when LLVM is not available
#[cfg(not(feature = "llvm"))]
pub struct StubCodeGenerator;

#[cfg(feature = "llvm")]
impl<'ctx> LLVMCodeGenerator<'ctx> {
    /// Create a new LLVM code generator
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        
        Self {
            context,
            module,
            builder,
            function_table: HashMap::new(),
            current_function: None,
            variable_table: HashMap::new(),
        }
    }
    
    /// Get the generated LLVM IR as a string
    pub fn get_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }
    
    /// Convert a Flux type to an LLVM type
    fn flux_type_to_llvm(&self, flux_type: &Type) -> Result<BasicTypeEnum<'ctx>, CodeGenError> {
        match flux_type {
            Type::Int => Ok(self.context.i64_type().into()),
            Type::Float => Ok(self.context.f64_type().into()),
            Type::Bool => Ok(self.context.bool_type().into()),
            Type::Char => Ok(self.context.i8_type().into()),
            Type::Byte => Ok(self.context.i8_type().into()),
            Type::String => {
                // String as i8* (pointer to char)
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).into())
            }
            Type::Array(elem_type) => {
                let elem_llvm_type = self.flux_type_to_llvm(elem_type)?;
                // For now, represent arrays as pointers to the element type
                Ok(elem_llvm_type.ptr_type(AddressSpace::default()).into())
            }
            Type::Nullable(inner_type) => {
                // Nullable types are represented as pointers (null = nullptr)
                let inner_llvm_type = self.flux_type_to_llvm(inner_type)?;
                Ok(inner_llvm_type.ptr_type(AddressSpace::default()).into())
            }
            Type::Unit => {
                // Unit type - we'll handle this specially in function returns
                // For now, use i1 as a placeholder when we need a BasicTypeEnum
                Ok(self.context.bool_type().into())
            }
            _ => Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: format!("Type: {:?}", flux_type),
                },
            }),
        }
    }
    
    /// Check if a type is the unit type (void)
    fn is_unit_type(&self, flux_type: &Type) -> bool {
        matches!(flux_type, Type::Unit)
    }
    
    /// Create a function type from parameter and return types
    fn create_function_type(&self, params: &[Type], return_type: &Type) -> Result<FunctionType<'ctx>, CodeGenError> {
        let param_types: Result<Vec<BasicTypeEnum>, _> = params.iter()
            .map(|t| self.flux_type_to_llvm(t))
            .collect();
        
        let param_types = param_types?;
        
        if self.is_unit_type(return_type) {
            Ok(self.context.void_type().fn_type(&param_types, false))
        } else {
            let ret_type = self.flux_type_to_llvm(return_type)?;
            Ok(ret_type.fn_type(&param_types, false))
        }
    }
}

#[cfg(feature = "llvm")]
impl<'ctx> CodeGenerator for LLVMCodeGenerator<'ctx> {
    fn generate(&mut self, program: TypedProgram) -> Result<String, CodeGenError> {
        // First pass: Generate struct types
        for item in &program.items {
            if let TypedItem::Struct(struct_def) = item {
                self.generate_struct_type(struct_def)?;
            }
        }
        
        // Second pass: Generate all other items
        for item in &program.items {
            match item {
                TypedItem::Function(func) => {
                    self.generate_function_impl(func)?;
                }
                TypedItem::Struct(struct_def) => {
                    self.generate_struct_methods(struct_def)?;
                }
                TypedItem::Class(class_def) => {
                    self.generate_class_impl(class_def)?;
                }
                TypedItem::Const(const_def) => {
                    self.generate_const_impl(const_def)?;
                }
                TypedItem::ExternFunction(extern_func) => {
                    self.generate_extern_function_impl(extern_func)?;
                }
            }
        }
        
        Ok(self.get_ir())
    }
}

#[cfg(feature = "llvm")]
impl<'ctx> LLVMCodeGenerator<'ctx> {
    fn generate_function_impl(&mut self, func: &TypedFunction) -> Result<(), CodeGenError> {
        // Clear variable table for new function
        self.variable_table.clear();
        
        // Extract parameter types
        let param_types: Vec<Type> = func.parameters.iter()
            .map(|p| p.type_.clone())
            .collect();
        
        // Create function type
        let fn_type = self.create_function_type(&param_types, &func.return_type)?;
        
        // Create function
        let function = self.module.add_function(&func.name, fn_type, None);
        self.function_table.insert(func.name.clone(), function);
        self.current_function = Some(function);
        
        // Create entry basic block
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);
        
        // Create allocas for parameters
        for (i, param) in func.parameters.iter().enumerate() {
            let param_type = self.flux_type_to_llvm(&param.type_)?;
            let alloca = self.builder.build_alloca(param_type, &param.name)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build alloca for parameter {}: {:?}", param.name, e),
                    },
                })?;
            
            // Store parameter value
            let param_value = function.get_nth_param(i as u32).unwrap();
            self.builder.build_store(alloca, param_value)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to store parameter {}: {:?}", param.name, e),
                    },
                })?;
            
            self.variable_table.insert(param.name.clone(), alloca);
        }
        
        // Generate function body
        self.generate_block(&func.body)?;
        
        // Add return if function doesn't end with one and is void
        if self.is_unit_type(&func.return_type) {
            // Check if the last instruction is already a return
            let current_block = self.builder.get_insert_block().unwrap();
            if current_block.get_terminator().is_none() {
                self.builder.build_return(None)
                    .map_err(|e| CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::LlvmError {
                            message: format!("Failed to build return: {:?}", e),
                        },
                    })?;
            }
        }
        
        self.current_function = None;
        Ok(())
    }
    
    /// Generate code for a typed block
    fn generate_block(&mut self, block: &TypedBlock) -> Result<Option<BasicValueEnum<'ctx>>, CodeGenError> {
        let mut last_value = None;
        
        for stmt in &block.statements {
            if let Some(value) = self.generate_statement(stmt)? {
                last_value = Some(value);
            }
        }
        
        Ok(last_value)
    }
    
    /// Generate code for a typed expression
    fn generate_expression(&mut self, expr: &TypedExpression) -> Result<BasicValueEnum<'ctx>, CodeGenError> {
        match &expr.kind {
            TypedExpressionKind::Literal(lit) => {
                self.generate_literal(lit)
            }
            TypedExpressionKind::Identifier(name) => {
                // Load variable value
                if let Some(alloca) = self.variable_table.get(name) {
                    self.builder.build_load(alloca.get_type().get_element_type(), *alloca, name)
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to load variable {}: {:?}", name, e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: format!("Undefined variable: {}", name),
                        },
                    })
                }
            }
            TypedExpressionKind::Binary(left, op, right) => {
                self.generate_binary_op(left, op, right)
            }
            TypedExpressionKind::Unary(op, operand) => {
                self.generate_unary_op(op, operand)
            }
            TypedExpressionKind::Call(func, args) => {
                self.generate_call(func, args)
            }
            TypedExpressionKind::Field(obj, field_name) => {
                self.generate_field_access(obj, field_name)
            }
            TypedExpressionKind::Block(block) => {
                if let Some(value) = self.generate_block(block)? {
                    Ok(value)
                } else {
                    // Block with no value returns unit (represented as i1 false)
                    Ok(self.context.bool_type().const_int(0, false).into())
                }
            }
            TypedExpressionKind::If(cond, then_block, else_block) => {
                if let Some(value) = self.generate_if_statement(cond, then_block, else_block)? {
                    Ok(value)
                } else {
                    // If expression with no value returns unit
                    Ok(self.context.bool_type().const_int(0, false).into())
                }
            }
            _ => Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: format!("Expression: {:?}", expr.kind),
                },
            }),
        }
    }
    
    /// Generate code for a literal
    fn generate_literal(&self, lit: &Literal) -> Result<BasicValueEnum<'ctx>, CodeGenError> {
        match lit {
            Literal::Integer(n) => {
                Ok(self.context.i64_type().const_int(*n as u64, true).into())
            }
            Literal::Float(f) => {
                Ok(self.context.f64_type().const_float(*f).into())
            }
            Literal::Boolean(b) => {
                Ok(self.context.bool_type().const_int(if *b { 1 } else { 0 }, false).into())
            }
            Literal::Character(c) => {
                Ok(self.context.i8_type().const_int(*c as u64, false).into())
            }
            Literal::String(_s) => {
                // String literals are more complex - for now, return a placeholder
                Err(CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::UnsupportedFeature {
                        feature: "String literals".to_string(),
                    },
                })
            }
            Literal::Null => {
                // Null pointer
                Ok(self.context.i8_type().ptr_type(AddressSpace::default()).const_null().into())
            }
        }
    }
    
    /// Generate code for binary operations
    fn generate_binary_op(&mut self, left: &TypedExpression, op: &BinaryOp, right: &TypedExpression) -> Result<BasicValueEnum<'ctx>, CodeGenError> {
        let left_val = self.generate_expression(left)?;
        let right_val = self.generate_expression(right)?;
        
        match op {
            BinaryOp::Add => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_add(left_val.into_int_value(), right_val.into_int_value(), "add")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int add: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_add(left_val.into_float_value(), right_val.into_float_value(), "fadd")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float add: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type addition".to_string(),
                        },
                    })
                }
            }
            BinaryOp::Subtract => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_sub(left_val.into_int_value(), right_val.into_int_value(), "sub")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int sub: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_sub(left_val.into_float_value(), right_val.into_float_value(), "fsub")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float sub: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type subtraction".to_string(),
                        },
                    })
                }
            }
            BinaryOp::Multiply => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_mul(left_val.into_int_value(), right_val.into_int_value(), "mul")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int mul: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_mul(left_val.into_float_value(), right_val.into_float_value(), "fmul")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float mul: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type multiplication".to_string(),
                        },
                    })
                }
            }
            BinaryOp::Divide => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_signed_div(left_val.into_int_value(), right_val.into_int_value(), "div")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int div: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_div(left_val.into_float_value(), right_val.into_float_value(), "fdiv")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float div: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type division".to_string(),
                        },
                    })
                }
            }
            BinaryOp::Equal => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::EQ, left_val.into_int_value(), right_val.into_int_value(), "eq")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int compare: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_compare(FloatPredicate::OEQ, left_val.into_float_value(), right_val.into_float_value(), "feq")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float compare: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type equality".to_string(),
                        },
                    })
                }
            }
            BinaryOp::Less => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::SLT, left_val.into_int_value(), right_val.into_int_value(), "lt")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int compare: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_compare(FloatPredicate::OLT, left_val.into_float_value(), right_val.into_float_value(), "flt")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float compare: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type comparison".to_string(),
                        },
                    })
                }
            }
            BinaryOp::Greater => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::SGT, left_val.into_int_value(), right_val.into_int_value(), "gt")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int compare: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_compare(FloatPredicate::OGT, left_val.into_float_value(), right_val.into_float_value(), "fgt")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float compare: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type comparison".to_string(),
                        },
                    })
                }
            }
            BinaryOp::LessEqual => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::SLE, left_val.into_int_value(), right_val.into_int_value(), "le")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int compare: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_compare(FloatPredicate::OLE, left_val.into_float_value(), right_val.into_float_value(), "fle")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float compare: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type comparison".to_string(),
                        },
                    })
                }
            }
            BinaryOp::GreaterEqual => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::SGE, left_val.into_int_value(), right_val.into_int_value(), "ge")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int compare: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_compare(FloatPredicate::OGE, left_val.into_float_value(), right_val.into_float_value(), "fge")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float compare: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type comparison".to_string(),
                        },
                    })
                }
            }
            BinaryOp::NotEqual => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::NE, left_val.into_int_value(), right_val.into_int_value(), "ne")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int compare: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_compare(FloatPredicate::ONE, left_val.into_float_value(), right_val.into_float_value(), "fne")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float compare: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type equality".to_string(),
                        },
                    })
                }
            }
            BinaryOp::And => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_and(left_val.into_int_value(), right_val.into_int_value(), "and")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build logical and: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Logical and of non-boolean types".to_string(),
                        },
                    })
                }
            }
            BinaryOp::Or => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_or(left_val.into_int_value(), right_val.into_int_value(), "or")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build logical or: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Logical or of non-boolean types".to_string(),
                        },
                    })
                }
            }
            BinaryOp::Modulo => {
                if left_val.is_int_value() && right_val.is_int_value() {
                    self.builder.build_int_signed_rem(left_val.into_int_value(), right_val.into_int_value(), "mod")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int modulo: {:?}", e),
                            },
                        })
                } else if left_val.is_float_value() && right_val.is_float_value() {
                    self.builder.build_float_rem(left_val.into_float_value(), right_val.into_float_value(), "fmod")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float modulo: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Mixed type modulo".to_string(),
                        },
                    })
                }
            }
            _ => Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: format!("Binary operator: {:?}", op),
                },
            }),
        }
    }
    
    /// Generate code for unary operations
    fn generate_unary_op(&mut self, op: &UnaryOp, operand: &TypedExpression) -> Result<BasicValueEnum<'ctx>, CodeGenError> {
        let operand_val = self.generate_expression(operand)?;
        
        match op {
            UnaryOp::Minus => {
                if operand_val.is_int_value() {
                    let zero = self.context.i64_type().const_int(0, false);
                    self.builder.build_int_sub(zero, operand_val.into_int_value(), "neg")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build int negation: {:?}", e),
                            },
                        })
                } else if operand_val.is_float_value() {
                    self.builder.build_float_neg(operand_val.into_float_value(), "fneg")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build float negation: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Negation of non-numeric type".to_string(),
                        },
                    })
                }
            }
            UnaryOp::Not => {
                if operand_val.is_int_value() {
                    // Logical not for boolean (i1) values
                    self.builder.build_not(operand_val.into_int_value(), "not")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build logical not: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Logical not of non-boolean type".to_string(),
                        },
                    })
                }
            }
            UnaryOp::Plus => {
                // Unary plus is essentially a no-op, just return the operand
                Ok(operand_val)
            }
            UnaryOp::BitwiseNot => {
                if operand_val.is_int_value() {
                    // Bitwise not (complement)
                    let all_ones = self.context.i64_type().const_all_ones();
                    self.builder.build_int_xor(operand_val.into_int_value(), all_ones, "bnot")
                        .map(|v| v.into())
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build bitwise not: {:?}", e),
                            },
                        })
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Bitwise not of non-integer type".to_string(),
                        },
                    })
                }
            }
            _ => Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: format!("Unary operator: {:?}", op),
                },
            }),
        }
    }
    
    /// Generate code for function calls
    fn generate_call(&mut self, func: &TypedExpression, args: &[TypedExpression]) -> Result<BasicValueEnum<'ctx>, CodeGenError> {
        // For now, only support direct function calls by name
        if let TypedExpressionKind::Identifier(func_name) = &func.kind {
            if let Some(function) = self.function_table.get(func_name).copied() {
                let mut arg_values = Vec::new();
                for arg in args {
                    arg_values.push(self.generate_expression(arg)?.into());
                }
                
                self.builder.build_call(function, &arg_values, "call")
                    .map_err(|e| CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::LlvmError {
                            message: format!("Failed to build function call: {:?}", e),
                        },
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::InternalError {
                            message: "Function call returned void but expected value".to_string(),
                        },
                    })
            } else {
                Err(CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::UnsupportedFeature {
                        feature: format!("Undefined function: {}", func_name),
                    },
                })
            }
        } else {
            Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: "Indirect function calls".to_string(),
                },
            })
        }
    }
    
    /// Generate code for a statement, returning optional value for expression statements
    fn generate_statement(&mut self, stmt: &TypedStatement) -> Result<Option<BasicValueEnum<'ctx>>, CodeGenError> {
        match stmt {
            TypedStatement::Expression(expr) => {
                let value = self.generate_expression(expr)?;
                Ok(Some(value))
            }
            TypedStatement::Let(name, type_, init) => {
                // Create alloca for the variable
                let var_type = self.flux_type_to_llvm(type_)?;
                let alloca = self.builder.build_alloca(var_type, name)
                    .map_err(|e| CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::LlvmError {
                            message: format!("Failed to build alloca for variable {}: {:?}", name, e),
                        },
                    })?;
                
                // Store initial value if provided
                if let Some(init_expr) = init {
                    let init_value = self.generate_expression(init_expr)?;
                    self.builder.build_store(alloca, init_value)
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to store initial value for {}: {:?}", name, e),
                            },
                        })?;
                }
                
                self.variable_table.insert(name.clone(), alloca);
                Ok(None)
            }
            TypedStatement::Assignment(target, value) => {
                // For now, only support simple variable assignment
                if let TypedExpressionKind::Identifier(var_name) = &target.kind {
                    if let Some(alloca) = self.variable_table.get(var_name) {
                        let new_value = self.generate_expression(value)?;
                        self.builder.build_store(*alloca, new_value)
                            .map_err(|e| CodeGenError {
                                span: None,
                                kind: CodeGenErrorKind::LlvmError {
                                    message: format!("Failed to store value to {}: {:?}", var_name, e),
                                },
                            })?;
                        Ok(None)
                    } else {
                        Err(CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::UnsupportedFeature {
                                feature: format!("Undefined variable in assignment: {}", var_name),
                            },
                        })
                    }
                } else {
                    Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Complex assignment targets".to_string(),
                        },
                    })
                }
            }
            TypedStatement::Return(expr) => {
                if let Some(e) = expr {
                    let value = self.generate_expression(e)?;
                    self.builder.build_return(Some(&value))
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build return: {:?}", e),
                            },
                        })?;
                } else {
                    self.builder.build_return(None)
                        .map_err(|e| CodeGenError {
                            span: None,
                            kind: CodeGenErrorKind::LlvmError {
                                message: format!("Failed to build return: {:?}", e),
                            },
                        })?;
                }
                Ok(None)
            }
            TypedStatement::If(cond, then_block, else_block) => {
                self.generate_if_statement(cond, then_block, else_block)
            }
            TypedStatement::While(cond, body) => {
                self.generate_while_loop(cond, body)
            }
            TypedStatement::For(var, iter, body) => {
                self.generate_for_loop(var, iter, body)
            }
            _ => Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: format!("Statement: {:?}", stmt),
                },
            }),
        }
    }
    
    /// Generate code for if statements
    fn generate_if_statement(&mut self, cond: &TypedExpression, then_block: &TypedBlock, else_block: &Option<TypedBlock>) -> Result<Option<BasicValueEnum<'ctx>>, CodeGenError> {
        let current_function = self.current_function.ok_or_else(|| CodeGenError {
            span: None,
            kind: CodeGenErrorKind::InternalError {
                message: "No current function for if statement".to_string(),
            },
        })?;
        
        // Generate condition
        let cond_val = self.generate_expression(cond)?;
        
        // Create basic blocks
        let then_bb = self.context.append_basic_block(current_function, "then");
        let else_bb = if else_block.is_some() {
            Some(self.context.append_basic_block(current_function, "else"))
        } else {
            None
        };
        let merge_bb = self.context.append_basic_block(current_function, "ifcont");
        
        // Build conditional branch
        if let Some(else_bb) = else_bb {
            self.builder.build_conditional_branch(cond_val.into_int_value(), then_bb, else_bb)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build conditional branch: {:?}", e),
                    },
                })?;
        } else {
            self.builder.build_conditional_branch(cond_val.into_int_value(), then_bb, merge_bb)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build conditional branch: {:?}", e),
                    },
                })?;
        }
        
        // Generate then block
        self.builder.position_at_end(then_bb);
        let then_value = self.generate_block(then_block)?;
        
        // Branch to merge block if no terminator
        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
            self.builder.build_unconditional_branch(merge_bb)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build branch to merge: {:?}", e),
                    },
                })?;
        }
        
        let then_end_bb = self.builder.get_insert_block().unwrap();
        
        // Generate else block if present
        let (else_value, else_end_bb) = if let (Some(else_bb), Some(else_block)) = (else_bb, else_block) {
            self.builder.position_at_end(else_bb);
            let else_val = self.generate_block(else_block)?;
            
            // Branch to merge block if no terminator
            if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                self.builder.build_unconditional_branch(merge_bb)
                    .map_err(|e| CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::LlvmError {
                            message: format!("Failed to build branch to merge: {:?}", e),
                        },
                    })?;
            }
            
            let else_end = self.builder.get_insert_block().unwrap();
            (else_val, Some(else_end))
        } else {
            (None, None)
        };
        
        // Position at merge block
        self.builder.position_at_end(merge_bb);
        
        // If both branches return values, create a phi node
        if let (Some(then_val), Some(else_val)) = (then_value, else_value) {
            let phi = self.builder.build_phi(then_val.get_type(), "ifphi")
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build phi node: {:?}", e),
                    },
                })?;
            
            phi.add_incoming(&[(&then_val, then_end_bb)]);
            if let Some(else_end) = else_end_bb {
                phi.add_incoming(&[(&else_val, else_end)]);
            }
            
            Ok(Some(phi.as_basic_value()))
        } else {
            Ok(None)
        }
    }
    
    /// Generate code for while loops
    fn generate_while_loop(&mut self, cond: &TypedExpression, body: &TypedBlock) -> Result<Option<BasicValueEnum<'ctx>>, CodeGenError> {
        let current_function = self.current_function.ok_or_else(|| CodeGenError {
            span: None,
            kind: CodeGenErrorKind::InternalError {
                message: "No current function for while loop".to_string(),
            },
        })?;
        
        // Create basic blocks
        let loop_cond_bb = self.context.append_basic_block(current_function, "loopcond");
        let loop_body_bb = self.context.append_basic_block(current_function, "loopbody");
        let loop_end_bb = self.context.append_basic_block(current_function, "loopend");
        
        // Branch to condition check
        self.builder.build_unconditional_branch(loop_cond_bb)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to build branch to loop condition: {:?}", e),
                },
            })?;
        
        // Generate condition check
        self.builder.position_at_end(loop_cond_bb);
        let cond_val = self.generate_expression(cond)?;
        self.builder.build_conditional_branch(cond_val.into_int_value(), loop_body_bb, loop_end_bb)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to build conditional branch: {:?}", e),
                },
            })?;
        
        // Generate loop body
        self.builder.position_at_end(loop_body_bb);
        self.generate_block(body)?;
        
        // Branch back to condition if no terminator
        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
            self.builder.build_unconditional_branch(loop_cond_bb)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build branch back to condition: {:?}", e),
                    },
                })?;
        }
        
        // Position at loop end
        self.builder.position_at_end(loop_end_bb);
        
        Ok(None)
    }
    
    /// Generate code for for loops (simplified implementation)
    fn generate_for_loop(&mut self, var: &str, iter: &TypedExpression, body: &TypedBlock) -> Result<Option<BasicValueEnum<'ctx>>, CodeGenError> {
        // For now, implement a simple for loop as a while loop
        // In a complete implementation, this would handle iterators properly
        
        // This is a placeholder implementation that treats the iterator as a range
        // For example: for i in 0..10 would be implemented as:
        // let i = 0; while i < 10 { body; i = i + 1; }
        
        let current_function = self.current_function.ok_or_else(|| CodeGenError {
            span: None,
            kind: CodeGenErrorKind::InternalError {
                message: "No current function for for loop".to_string(),
            },
        })?;
        
        // Create basic blocks
        let loop_cond_bb = self.context.append_basic_block(current_function, "forcond");
        let loop_body_bb = self.context.append_basic_block(current_function, "forbody");
        let loop_end_bb = self.context.append_basic_block(current_function, "forend");
        
        // For simplicity, assume iter is a literal integer representing the upper bound
        // and start from 0
        let iter_val = self.generate_expression(iter)?;
        
        // Create loop variable
        let loop_var_type = self.context.i64_type();
        let loop_var_alloca = self.builder.build_alloca(loop_var_type, var)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to build alloca for loop variable: {:?}", e),
                },
            })?;
        
        // Initialize loop variable to 0
        let zero = self.context.i64_type().const_int(0, false);
        self.builder.build_store(loop_var_alloca, zero)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to initialize loop variable: {:?}", e),
                },
            })?;
        
        // Add to variable table
        self.variable_table.insert(var.to_string(), loop_var_alloca);
        
        // Branch to condition check
        self.builder.build_unconditional_branch(loop_cond_bb)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to build branch to for condition: {:?}", e),
                },
            })?;
        
        // Generate condition check (i < iter_val)
        self.builder.position_at_end(loop_cond_bb);
        let current_val = self.builder.build_load(loop_var_type, loop_var_alloca, "loopvar")
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to load loop variable: {:?}", e),
                },
            })?;
        
        let cond_val = self.builder.build_int_compare(
            IntPredicate::SLT,
            current_val.into_int_value(),
            iter_val.into_int_value(),
            "forcond"
        ).map_err(|e| CodeGenError {
            span: None,
            kind: CodeGenErrorKind::LlvmError {
                message: format!("Failed to build for loop condition: {:?}", e),
            },
        })?;
        
        self.builder.build_conditional_branch(cond_val, loop_body_bb, loop_end_bb)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to build conditional branch: {:?}", e),
                },
            })?;
        
        // Generate loop body
        self.builder.position_at_end(loop_body_bb);
        self.generate_block(body)?;
        
        // Increment loop variable
        let current_val = self.builder.build_load(loop_var_type, loop_var_alloca, "loopvar")
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to load loop variable for increment: {:?}", e),
                },
            })?;
        
        let one = self.context.i64_type().const_int(1, false);
        let incremented = self.builder.build_int_add(current_val.into_int_value(), one, "inc")
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to increment loop variable: {:?}", e),
                },
            })?;
        
        self.builder.build_store(loop_var_alloca, incremented)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to store incremented loop variable: {:?}", e),
                },
            })?;
        
        // Branch back to condition if no terminator
        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
            self.builder.build_unconditional_branch(loop_cond_bb)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build branch back to condition: {:?}", e),
                    },
                })?;
        }
        
        // Position at loop end
        self.builder.position_at_end(loop_end_bb);
        
        // Remove loop variable from table
        self.variable_table.remove(var);
        
        Ok(None)
    }
    
    /// Generate LLVM struct type for a Flux struct
    fn generate_struct_type(&mut self, struct_def: &TypedStruct) -> Result<(), CodeGenError> {
        let mut field_types = Vec::new();
        
        for field in &struct_def.fields {
            let field_type = self.flux_type_to_llvm(&field.type_)?;
            field_types.push(field_type);
        }
        
        // Create struct type
        let struct_type = self.context.struct_type(&field_types, false);
        
        // Store struct type for later use (we'd need a struct_types HashMap)
        // For now, we'll just create the type
        
        Ok(())
    }
    
    /// Generate struct methods (constructors, etc.)
    fn generate_struct_methods(&mut self, struct_def: &TypedStruct) -> Result<(), CodeGenError> {
        // Generate constructor function
        self.generate_struct_constructor(struct_def)?;
        
        // Generate field accessors if needed
        self.generate_struct_accessors(struct_def)?;
        
        Ok(())
    }
    
    /// Generate struct constructor
    fn generate_struct_constructor(&mut self, struct_def: &TypedStruct) -> Result<(), CodeGenError> {
        // Create constructor function type
        let param_types: Result<Vec<BasicTypeEnum>, _> = struct_def.fields.iter()
            .map(|f| self.flux_type_to_llvm(&f.type_))
            .collect();
        let param_types = param_types?;
        
        // Constructor returns a pointer to the struct
        let struct_field_types: Result<Vec<BasicTypeEnum>, _> = struct_def.fields.iter()
            .map(|f| self.flux_type_to_llvm(&f.type_))
            .collect();
        let struct_field_types = struct_field_types?;
        let struct_type = self.context.struct_type(&struct_field_types, false);
        let struct_ptr_type = struct_type.ptr_type(AddressSpace::default());
        
        let constructor_name = format!("{}_new", struct_def.name);
        let fn_type = struct_ptr_type.fn_type(&param_types, false);
        
        // Create constructor function
        let constructor = self.module.add_function(&constructor_name, fn_type, None);
        self.function_table.insert(constructor_name.clone(), constructor);
        
        // Create entry block
        let entry_block = self.context.append_basic_block(constructor, "entry");
        self.builder.position_at_end(entry_block);
        
        // Allocate memory for struct
        let struct_alloca = self.builder.build_alloca(struct_type, "struct_instance")
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to allocate struct: {:?}", e),
                },
            })?;
        
        // Initialize fields with parameter values
        for (i, field) in struct_def.fields.iter().enumerate() {
            let param_value = constructor.get_nth_param(i as u32).unwrap();
            
            // Get field pointer
            let field_ptr = self.builder.build_struct_gep(struct_type, struct_alloca, i as u32, &format!("field_{}", field.name))
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to get field pointer: {:?}", e),
                    },
                })?;
            
            // Store parameter value in field
            self.builder.build_store(field_ptr, param_value)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to store field value: {:?}", e),
                    },
                })?;
        }
        
        // Return pointer to struct
        self.builder.build_return(Some(&struct_alloca))
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to build return: {:?}", e),
                },
            })?;
        
        Ok(())
    }
    
    /// Generate struct field accessors
    fn generate_struct_accessors(&mut self, struct_def: &TypedStruct) -> Result<(), CodeGenError> {
        let struct_field_types: Result<Vec<BasicTypeEnum>, _> = struct_def.fields.iter()
            .map(|f| self.flux_type_to_llvm(&f.type_))
            .collect();
        let struct_field_types = struct_field_types?;
        let struct_type = self.context.struct_type(&struct_field_types, false);
        let struct_ptr_type = struct_type.ptr_type(AddressSpace::default());
        
        for (field_index, field) in struct_def.fields.iter().enumerate() {
            // Generate getter
            let getter_name = format!("{}_{}_get", struct_def.name, field.name);
            let field_type = self.flux_type_to_llvm(&field.type_)?;
            let getter_type = field_type.fn_type(&[struct_ptr_type.into()], false);
            
            let getter = self.module.add_function(&getter_name, getter_type, None);
            self.function_table.insert(getter_name.clone(), getter);
            
            let entry_block = self.context.append_basic_block(getter, "entry");
            self.builder.position_at_end(entry_block);
            
            let struct_param = getter.get_nth_param(0).unwrap().into_pointer_value();
            
            // Get field pointer and load value
            let field_ptr = self.builder.build_struct_gep(struct_type, struct_param, field_index as u32, &format!("field_{}", field.name))
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to get field pointer: {:?}", e),
                    },
                })?;
            
            let field_value = self.builder.build_load(field_type, field_ptr, &format!("field_{}_value", field.name))
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to load field value: {:?}", e),
                    },
                })?;
            
            self.builder.build_return(Some(&field_value))
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build return: {:?}", e),
                    },
                })?;
            
            // Generate setter if field is mutable
            if field.is_mutable {
                let setter_name = format!("{}_{}_set", struct_def.name, field.name);
                let setter_type = self.context.void_type().fn_type(&[struct_ptr_type.into(), field_type], false);
                
                let setter = self.module.add_function(&setter_name, setter_type, None);
                self.function_table.insert(setter_name.clone(), setter);
                
                let entry_block = self.context.append_basic_block(setter, "entry");
                self.builder.position_at_end(entry_block);
                
                let struct_param = setter.get_nth_param(0).unwrap().into_pointer_value();
                let value_param = setter.get_nth_param(1).unwrap();
                
                let field_ptr = self.builder.build_struct_gep(struct_type, struct_param, field_index as u32, &format!("field_{}", field.name))
                    .map_err(|e| CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::LlvmError {
                            message: format!("Failed to get field pointer: {:?}", e),
                        },
                    })?;
                
                self.builder.build_store(field_ptr, value_param)
                    .map_err(|e| CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::LlvmError {
                            message: format!("Failed to store field value: {:?}", e),
                        },
                    })?;
                
                self.builder.build_return(None)
                    .map_err(|e| CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::LlvmError {
                            message: format!("Failed to build return: {:?}", e),
                        },
                    })?;
            }
        }
        
        Ok(())
    }
    
    /// Generate class implementation
    fn generate_class_impl(&mut self, class_def: &TypedClass) -> Result<(), CodeGenError> {
        // Generate class struct type (similar to struct)
        self.generate_class_type(class_def)?;
        
        // Generate constructor
        self.generate_class_constructor(class_def)?;
        
        // Generate methods
        for method in &class_def.methods {
            self.generate_class_method(class_def, method)?;
        }
        
        Ok(())
    }
    
    /// Generate class struct type
    fn generate_class_type(&mut self, class_def: &TypedClass) -> Result<(), CodeGenError> {
        let mut field_types = Vec::new();
        
        // Add vtable pointer for dynamic dispatch (simplified)
        let vtable_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        field_types.push(vtable_ptr_type.into());
        
        // Add regular fields
        for field in &class_def.fields {
            let field_type = self.flux_type_to_llvm(&field.type_)?;
            field_types.push(field_type);
        }
        
        // Create class struct type
        let _class_type = self.context.struct_type(&field_types, false);
        
        Ok(())
    }
    
    /// Generate class constructor
    fn generate_class_constructor(&mut self, class_def: &TypedClass) -> Result<(), CodeGenError> {
        // Similar to struct constructor but with vtable initialization
        let param_types: Result<Vec<BasicTypeEnum>, _> = class_def.fields.iter()
            .map(|f| self.flux_type_to_llvm(&f.type_))
            .collect();
        let param_types = param_types?;
        
        // Create class type with vtable
        let mut field_types = Vec::new();
        let vtable_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        field_types.push(vtable_ptr_type.into());
        
        for field in &class_def.fields {
            let field_type = self.flux_type_to_llvm(&field.type_)?;
            field_types.push(field_type);
        }
        
        let class_type = self.context.struct_type(&field_types, false);
        let class_ptr_type = class_type.ptr_type(AddressSpace::default());
        
        let constructor_name = format!("{}_new", class_def.name);
        let fn_type = class_ptr_type.fn_type(&param_types, false);
        
        let constructor = self.module.add_function(&constructor_name, fn_type, None);
        self.function_table.insert(constructor_name.clone(), constructor);
        
        let entry_block = self.context.append_basic_block(constructor, "entry");
        self.builder.position_at_end(entry_block);
        
        // Allocate memory for class instance
        let class_alloca = self.builder.build_alloca(class_type, "class_instance")
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to allocate class: {:?}", e),
                },
            })?;
        
        // Initialize vtable (simplified - just null for now)
        let vtable_ptr = self.builder.build_struct_gep(class_type, class_alloca, 0, "vtable_ptr")
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to get vtable pointer: {:?}", e),
                },
            })?;
        
        let null_vtable = vtable_ptr_type.const_null();
        self.builder.build_store(vtable_ptr, null_vtable)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to store vtable: {:?}", e),
                },
            })?;
        
        // Initialize fields with parameter values (offset by 1 for vtable)
        for (i, field) in class_def.fields.iter().enumerate() {
            let param_value = constructor.get_nth_param(i as u32).unwrap();
            
            let field_ptr = self.builder.build_struct_gep(class_type, class_alloca, (i + 1) as u32, &format!("field_{}", field.name))
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to get field pointer: {:?}", e),
                    },
                })?;
            
            self.builder.build_store(field_ptr, param_value)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to store field value: {:?}", e),
                    },
                })?;
        }
        
        self.builder.build_return(Some(&class_alloca))
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::LlvmError {
                    message: format!("Failed to build return: {:?}", e),
                },
            })?;
        
        Ok(())
    }
    
    /// Generate class method
    fn generate_class_method(&mut self, class_def: &TypedClass, method: &TypedMethod) -> Result<(), CodeGenError> {
        // Clear variable table for new method
        self.variable_table.clear();
        
        // Create class type for 'self' parameter
        let mut field_types = Vec::new();
        let vtable_ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        field_types.push(vtable_ptr_type.into());
        
        for field in &class_def.fields {
            let field_type = self.flux_type_to_llvm(&field.type_)?;
            field_types.push(field_type);
        }
        
        let class_type = self.context.struct_type(&field_types, false);
        let class_ptr_type = class_type.ptr_type(AddressSpace::default());
        
        // Extract parameter types
        let mut param_types = Vec::new();
        
        // Add 'self' parameter if not static
        if !method.is_static {
            param_types.push(class_ptr_type.into());
        }
        
        // Add method parameters
        for param in &method.parameters {
            let param_type = self.flux_type_to_llvm(&param.type_)?;
            param_types.push(param_type);
        }
        
        // Create method function type
        let method_name = format!("{}_{}", class_def.name, method.name);
        let fn_type = self.create_function_type(
            &method.parameters.iter().map(|p| p.type_.clone()).collect::<Vec<_>>(),
            &method.return_type
        )?;
        
        // For methods, we need to manually create the function type with self parameter
        let actual_fn_type = if self.is_unit_type(&method.return_type) {
            self.context.void_type().fn_type(&param_types, false)
        } else {
            let ret_type = self.flux_type_to_llvm(&method.return_type)?;
            ret_type.fn_type(&param_types, false)
        };
        
        // Create method function
        let function = self.module.add_function(&method_name, actual_fn_type, None);
        self.function_table.insert(method_name.clone(), function);
        self.current_function = Some(function);
        
        // Create entry basic block
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);
        
        let mut param_index = 0;
        
        // Create alloca for 'self' parameter if not static
        if !method.is_static {
            let self_alloca = self.builder.build_alloca(class_ptr_type, "self")
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build alloca for self: {:?}", e),
                    },
                })?;
            
            let self_param = function.get_nth_param(param_index).unwrap();
            self.builder.build_store(self_alloca, self_param)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to store self parameter: {:?}", e),
                    },
                })?;
            
            self.variable_table.insert("self".to_string(), self_alloca);
            param_index += 1;
        }
        
        // Create allocas for method parameters
        for param in &method.parameters {
            let param_type = self.flux_type_to_llvm(&param.type_)?;
            let alloca = self.builder.build_alloca(param_type, &param.name)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to build alloca for parameter {}: {:?}", param.name, e),
                    },
                })?;
            
            let param_value = function.get_nth_param(param_index).unwrap();
            self.builder.build_store(alloca, param_value)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to store parameter {}: {:?}", param.name, e),
                    },
                })?;
            
            self.variable_table.insert(param.name.clone(), alloca);
            param_index += 1;
        }
        
        // Generate method body
        self.generate_block(&method.body)?;
        
        // Add return if method doesn't end with one and is void
        if self.is_unit_type(&method.return_type) {
            let current_block = self.builder.get_insert_block().unwrap();
            if current_block.get_terminator().is_none() {
                self.builder.build_return(None)
                    .map_err(|e| CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::LlvmError {
                            message: format!("Failed to build return: {:?}", e),
                        },
                    })?;
            }
        }
        
        self.current_function = None;
        Ok(())
    }
    
    /// Generate constant implementation
    fn generate_const_impl(&mut self, const_def: &TypedConst) -> Result<(), CodeGenError> {
        // Generate global constant
        let const_type = self.flux_type_to_llvm(&const_def.type_)?;
        let global = self.module.add_global(const_type, Some(AddressSpace::default()), &const_def.name);
        
        // For now, set to zero/null - in a real implementation we'd evaluate the constant expression
        let init_value = match &const_def.type_ {
            Type::Int => const_type.into_int_type().const_zero().into(),
            Type::Float => const_type.into_float_type().const_zero().into(),
            Type::Bool => const_type.into_int_type().const_zero().into(),
            _ => const_type.const_zero(),
        };
        
        global.set_initializer(&init_value);
        global.set_constant(true);
        
        Ok(())
    }
    
    /// Generate extern function declaration
    fn generate_extern_function_impl(&mut self, extern_func: &TypedExternFunction) -> Result<(), CodeGenError> {
        // Extract parameter types
        let param_types: Vec<Type> = extern_func.parameters.iter()
            .map(|p| p.type_.clone())
            .collect();
        
        // Create function type
        let fn_type = self.create_function_type(&param_types, &extern_func.return_type)?;
        
        // Declare external function (no body)
        let function = self.module.add_function(&extern_func.name, fn_type, None);
        
        // Set linkage to external
        function.set_linkage(inkwell::module::Linkage::External);
        
        // Store in function table for later calls
        self.function_table.insert(extern_func.name.clone(), function);
        
        Ok(())
    }
    
    /// Generate field access expression
    fn generate_field_access(&mut self, obj_expr: &TypedExpression, field_name: &str) -> Result<BasicValueEnum<'ctx>, CodeGenError> {
        let obj_value = self.generate_expression(obj_expr)?;
        
        // For now, assume obj_value is a pointer to a struct
        if let BasicValueEnum::PointerValue(ptr) = obj_value {
            // This is a simplified implementation - in reality we'd need type information
            // to determine the correct field index and struct type
            
            // For demonstration, assume it's accessing field 0
            let field_index = 0u32; // This should be looked up based on field_name and struct type
            
            // Create a dummy struct type for demonstration
            let dummy_field_type = self.context.i64_type();
            let dummy_struct_type = self.context.struct_type(&[dummy_field_type.into()], false);
            
            let field_ptr = self.builder.build_struct_gep(dummy_struct_type, ptr, field_index, field_name)
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to get field pointer: {:?}", e),
                    },
                })?;
            
            self.builder.build_load(dummy_field_type, field_ptr, &format!("{}_value", field_name))
                .map_err(|e| CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::LlvmError {
                        message: format!("Failed to load field value: {:?}", e),
                    },
                })
        } else {
            Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: "Field access on non-pointer value".to_string(),
                },
            })
        }
    }
}
#[cfg(not(
feature = "llvm"))]
impl StubCodeGenerator {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "llvm"))]
impl CodeGenerator for StubCodeGenerator {
    fn generate(&mut self, _program: TypedProgram) -> Result<String, CodeGenError> {
        Ok("// Code generation not available without LLVM feature".to_string())
    }
}