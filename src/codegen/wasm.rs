//! WebAssembly code generation backend
//! 
//! Provides WebAssembly code generation for Flux programs with JavaScript interop support.

use crate::error::{CodeGenError, CodeGenErrorKind};
use crate::semantic::*;
use crate::parser::ast::{Type, Literal, BinaryOp, UnaryOp};
use std::collections::HashMap;

#[cfg(feature = "wasm")]
use wasm_encoder::{
    Module, CodeSection, DataSection, ExportSection, FunctionSection, ImportSection,
    MemorySection, MemoryType, TypeSection, ValType, Instruction,
    FuncType, EntityType, GlobalSection, GlobalType, ExportKind,
};

/// WebAssembly code generator for Flux
#[cfg(feature = "wasm")]
pub struct WasmCodeGenerator {
    module: Module,
    types: TypeSection,
    imports: ImportSection,
    functions: FunctionSection,
    exports: ExportSection,
    code: CodeSection,
    memory: MemorySection,
    globals: GlobalSection,
    data: DataSection,
    
    // State tracking
    function_types: HashMap<String, u32>,
    function_indices: HashMap<String, u32>,
    global_indices: HashMap<String, u32>,
    local_indices: HashMap<String, u32>,
    type_index_counter: u32,
    function_index_counter: u32,
    global_index_counter: u32,
    
    // Memory management
    memory_offset: u32,
    string_literals: HashMap<String, u32>,
}

/// Stub WebAssembly code generator when WASM feature is not available
#[cfg(not(feature = "wasm"))]
pub struct WasmCodeGenerator;

#[cfg(feature = "wasm")]
impl WasmCodeGenerator {
    /// Create a new WebAssembly code generator
    pub fn new() -> Self {
        let mut generator = Self {
            module: Module::new(),
            types: TypeSection::new(),
            imports: ImportSection::new(),
            functions: FunctionSection::new(),
            exports: ExportSection::new(),
            code: CodeSection::new(),
            memory: MemorySection::new(),
            globals: GlobalSection::new(),
            data: DataSection::new(),
            
            function_types: HashMap::new(),
            function_indices: HashMap::new(),
            global_indices: HashMap::new(),
            local_indices: HashMap::new(),
            type_index_counter: 0,
            function_index_counter: 0,
            global_index_counter: 0,
            
            memory_offset: 0,
            string_literals: HashMap::new(),
        };
        
        // Set up basic memory (1 page = 64KB)
        generator.memory.memory(MemoryType {
            minimum: 1,
            maximum: Some(16), // 1MB max
            memory64: false,
            shared: false,
        });
        
        // Add JavaScript interop imports
        generator.add_js_imports();
        
        generator
    }
    
    /// Add JavaScript interop function imports
    fn add_js_imports(&mut self) {
        // console.log function
        let console_log_type = self.add_function_type(&[ValType::I32], &[]);
        self.imports.import(
            "js",
            "console_log",
            EntityType::Function(console_log_type),
        );
        self.function_indices.insert("js.console_log".to_string(), self.function_index_counter);
        self.function_index_counter += 1;
        
        // Memory allocation function
        let malloc_type = self.add_function_type(&[ValType::I32], &[ValType::I32]);
        self.imports.import(
            "js",
            "malloc",
            EntityType::Function(malloc_type),
        );
        self.function_indices.insert("js.malloc".to_string(), self.function_index_counter);
        self.function_index_counter += 1;
        
        // Memory deallocation function
        let free_type = self.add_function_type(&[ValType::I32], &[]);
        self.imports.import(
            "js",
            "free",
            EntityType::Function(free_type),
        );
        self.function_indices.insert("js.free".to_string(), self.function_index_counter);
        self.function_index_counter += 1;
    }
    
    /// Add a function type and return its index
    fn add_function_type(&mut self, params: &[ValType], results: &[ValType]) -> u32 {
        let func_type = FuncType::new(params.to_vec(), results.to_vec());
        let index = self.type_index_counter;
        self.types.function(params.to_vec(), results.to_vec());
        self.type_index_counter += 1;
        index
    }
    
    /// Convert a Flux type to a WebAssembly value type
    fn flux_type_to_wasm(&self, flux_type: &Type) -> Result<ValType, CodeGenError> {
        match flux_type {
            Type::Int => Ok(ValType::I64),
            Type::Float => Ok(ValType::F64),
            Type::Bool => Ok(ValType::I32),
            Type::Char => Ok(ValType::I32),
            Type::Byte => Ok(ValType::I32),
            Type::String => Ok(ValType::I32), // Pointer to string data
            Type::Array(_) => Ok(ValType::I32), // Pointer to array data
            Type::Nullable(_) => Ok(ValType::I32), // Pointer (null = 0)
            Type::Unit => Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: "Unit type cannot be converted to WASM value type".to_string(),
                },
            }),
            _ => Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: format!("Type conversion for: {:?}", flux_type),
                },
            }),
        }
    }
    
    /// Check if a type is the unit type (void)
    fn is_unit_type(&self, flux_type: &Type) -> bool {
        matches!(flux_type, Type::Unit)
    }
    
    /// Generate WebAssembly module from typed program
    pub fn generate(&mut self, program: TypedProgram) -> Result<Vec<u8>, CodeGenError> {
        // Generate all functions
        for item in &program.items {
            match item {
                TypedItem::Function(func) => {
                    self.generate_function(func)?;
                }
                TypedItem::Const(const_def) => {
                    self.generate_const(const_def)?;
                }
                _ => {
                    // Skip other items for now
                }
            }
        }
        
        // Build the final module
        self.build_module()
    }
    
    /// Generate a function
    fn generate_function(&mut self, func: &TypedFunction) -> Result<(), CodeGenError> {
        // Convert parameter types
        let param_types: Result<Vec<ValType>, _> = func.parameters.iter()
            .map(|p| self.flux_type_to_wasm(&p.type_))
            .collect();
        let param_types = param_types?;
        
        // Convert return type
        let return_types = if self.is_unit_type(&func.return_type) {
            vec![]
        } else {
            vec![self.flux_type_to_wasm(&func.return_type)?]
        };
        
        // Add function type
        let type_index = self.add_function_type(&param_types, &return_types);
        self.function_types.insert(func.name.clone(), type_index);
        
        // Add function to function section
        self.functions.function(type_index);
        let func_index = self.function_index_counter;
        self.function_indices.insert(func.name.clone(), func_index);
        self.function_index_counter += 1;
        
        // Export main function if it exists
        if func.name == "main" {
            self.exports.export(&func.name, ExportKind::Func, func_index);
        }
        
        // Generate function body
        let mut function_body = wasm_encoder::Function::new(vec![]); // No additional locals for now
        
        // Set up local variable mapping
        self.local_indices.clear();
        for (i, param) in func.parameters.iter().enumerate() {
            self.local_indices.insert(param.name.clone(), i as u32);
        }
        
        // Generate function body instructions
        self.generate_block_instructions(&func.body, &mut function_body)?;
        
        // Add return instruction if needed
        if self.is_unit_type(&func.return_type) {
            // Void function - just return
            // No explicit return needed for void functions in WASM
        } else {
            // Non-void function should have a return value on stack
            // The block generation should handle this
        }
        
        self.code.function(&function_body);
        
        Ok(())
    }
    
    /// Generate a constant
    fn generate_const(&mut self, const_def: &TypedConst) -> Result<(), CodeGenError> {
        let wasm_type = self.flux_type_to_wasm(&const_def.type_)?;
        
        // Create a global for the constant
        let global_type = GlobalType {
            val_type: wasm_type,
            mutable: false,
        };
        
        // Generate initialization expression
        let init_expr = self.generate_const_expression(&const_def.value)?;
        
        self.globals.global(global_type, &init_expr);
        self.global_indices.insert(const_def.name.clone(), self.global_index_counter);
        self.global_index_counter += 1;
        
        Ok(())
    }
    
    /// Generate instructions for a block
    fn generate_block_instructions(&mut self, block: &TypedBlock, function: &mut wasm_encoder::Function) -> Result<(), CodeGenError> {
        for stmt in &block.statements {
            self.generate_statement_instructions(stmt, function)?;
        }
        Ok(())
    }
    
    /// Generate instructions for a statement
    fn generate_statement_instructions(&mut self, stmt: &TypedStatement, function: &mut wasm_encoder::Function) -> Result<(), CodeGenError> {
        match &stmt.kind {
            TypedStatementKind::Expression(expr) => {
                self.generate_expression_instructions(expr, function)?;
                // Pop the result if it's not used
                if !self.is_unit_type(&expr.type_) {
                    function.instruction(&Instruction::Drop);
                }
            }
            TypedStatementKind::Let(name, _, Some(init_expr)) => {
                // Generate initialization expression
                self.generate_expression_instructions(init_expr, function)?;
                
                // For now, we'll use locals for let bindings
                // This is a simplified approach - a full implementation would need
                // proper local variable allocation
                if let Some(&local_index) = self.local_indices.get(name) {
                    function.instruction(&Instruction::LocalSet(local_index));
                }
            }
            TypedStatementKind::Return(Some(expr)) => {
                self.generate_expression_instructions(expr, function)?;
                function.instruction(&Instruction::Return);
            }
            TypedStatementKind::Return(None) => {
                function.instruction(&Instruction::Return);
            }
            _ => {
                return Err(CodeGenError {
                    span: stmt.span.clone(),
                    kind: CodeGenErrorKind::UnsupportedFeature {
                        feature: format!("Statement: {:?}", stmt.kind),
                    },
                });
            }
        }
        Ok(())
    }
    
    /// Generate instructions for an expression
    fn generate_expression_instructions(&mut self, expr: &TypedExpression, function: &mut wasm_encoder::Function) -> Result<(), CodeGenError> {
        match &expr.kind {
            TypedExpressionKind::Literal(lit) => {
                self.generate_literal_instructions(lit, function)?;
            }
            TypedExpressionKind::Identifier(name) => {
                if let Some(&local_index) = self.local_indices.get(name) {
                    function.instruction(&Instruction::LocalGet(local_index));
                } else if let Some(&global_index) = self.global_indices.get(name) {
                    function.instruction(&Instruction::GlobalGet(global_index));
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: format!("Undefined variable: {}", name),
                        },
                    });
                }
            }
            TypedExpressionKind::Binary(left, op, right) => {
                self.generate_binary_op_instructions(left, op, right, function)?;
            }
            TypedExpressionKind::Unary(op, operand) => {
                self.generate_unary_op_instructions(op, operand, function)?;
            }
            TypedExpressionKind::Call(func_expr, args) => {
                self.generate_call_instructions(func_expr, args, function)?;
            }
            TypedExpressionKind::Block(block) => {
                self.generate_block_instructions(block, function)?;
            }
            _ => {
                return Err(CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::UnsupportedFeature {
                        feature: format!("Expression: {:?}", expr.kind),
                    },
                });
            }
        }
        Ok(())
    }
    
    /// Generate instructions for a literal
    fn generate_literal_instructions(&mut self, lit: &Literal, function: &mut wasm_encoder::Function) -> Result<(), CodeGenError> {
        match lit {
            Literal::Integer(n) => {
                function.instruction(&Instruction::I64Const(*n));
            }
            Literal::Float(f) => {
                function.instruction(&Instruction::F64Const(*f));
            }
            Literal::Boolean(b) => {
                function.instruction(&Instruction::I32Const(if *b { 1 } else { 0 }));
            }
            Literal::Character(c) => {
                function.instruction(&Instruction::I32Const(*c as i32));
            }
            Literal::String(s) => {
                // Store string in data section and return pointer
                let offset = self.add_string_literal(s);
                function.instruction(&Instruction::I32Const(offset as i32));
            }
            Literal::Null => {
                function.instruction(&Instruction::I32Const(0));
            }
        }
        Ok(())
    }
    
    /// Generate instructions for binary operations
    fn generate_binary_op_instructions(&mut self, left: &TypedExpression, op: &BinaryOp, right: &TypedExpression, function: &mut wasm_encoder::Function) -> Result<(), CodeGenError> {
        // Generate left operand
        self.generate_expression_instructions(left, function)?;
        
        // Generate right operand
        self.generate_expression_instructions(right, function)?;
        
        // Generate operation instruction
        match op {
            BinaryOp::Add => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64Add);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Add);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Addition for non-numeric types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::Subtract => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64Sub);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Sub);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Subtraction for non-numeric types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::Multiply => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64Mul);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Mul);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Multiplication for non-numeric types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::Divide => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64DivS);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Div);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Division for non-numeric types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::Equal => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64Eq);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Eq);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Equality for non-comparable types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::Less => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64LtS);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Lt);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Comparison for non-comparable types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::Greater => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64GtS);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Gt);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Comparison for non-comparable types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::LessEqual => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64LeS);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Le);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Comparison for non-comparable types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::GreaterEqual => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64GeS);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Ge);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Comparison for non-comparable types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::NotEqual => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64Ne);
                } else if matches!(left.type_, Type::Float) {
                    function.instruction(&Instruction::F64Ne);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Comparison for non-comparable types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::Modulo => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64RemS);
                } else if matches!(left.type_, Type::Float) {
                    // Float modulo is not directly supported in WASM, would need to implement
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Float modulo operation".to_string(),
                        },
                    });
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Modulo for non-numeric types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::And => {
                if matches!(left.type_, Type::Bool) {
                    function.instruction(&Instruction::I32And);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Logical AND for non-boolean types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::Or => {
                if matches!(left.type_, Type::Bool) {
                    function.instruction(&Instruction::I32Or);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Logical OR for non-boolean types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::BitwiseAnd => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64And);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Bitwise AND for non-integer types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::BitwiseOr => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64Or);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Bitwise OR for non-integer types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::BitwiseXor => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64Xor);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Bitwise XOR for non-integer types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::LeftShift => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64Shl);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Left shift for non-integer types".to_string(),
                        },
                    });
                }
            }
            BinaryOp::RightShift => {
                if matches!(left.type_, Type::Int) {
                    function.instruction(&Instruction::I64ShrS);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Right shift for non-integer types".to_string(),
                        },
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate instructions for unary operations
    fn generate_unary_op_instructions(&mut self, op: &UnaryOp, operand: &TypedExpression, function: &mut wasm_encoder::Function) -> Result<(), CodeGenError> {
        self.generate_expression_instructions(operand, function)?;
        
        match op {
            UnaryOp::Plus => {
                // Unary plus is a no-op for numeric types
                if !matches!(operand.type_, Type::Int | Type::Float) {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Unary plus for non-numeric types".to_string(),
                        },
                    });
                }
                // Value is already on stack, no additional instruction needed
            }
            UnaryOp::Minus => {
                if matches!(operand.type_, Type::Int) {
                    // Negate by subtracting from 0
                    function.instruction(&Instruction::I64Const(0));
                    function.instruction(&Instruction::I64Sub);
                } else if matches!(operand.type_, Type::Float) {
                    function.instruction(&Instruction::F64Neg);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Negation for non-numeric types".to_string(),
                        },
                    });
                }
            }
            UnaryOp::Not => {
                if matches!(operand.type_, Type::Bool) {
                    // Logical not: XOR with 1
                    function.instruction(&Instruction::I32Const(1));
                    function.instruction(&Instruction::I32Xor);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Logical not for non-boolean types".to_string(),
                        },
                    });
                }
            }
            UnaryOp::BitwiseNot => {
                if matches!(operand.type_, Type::Int) {
                    // Bitwise not: XOR with all 1s
                    function.instruction(&Instruction::I64Const(-1));
                    function.instruction(&Instruction::I64Xor);
                } else {
                    return Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Bitwise not for non-integer types".to_string(),
                        },
                    });
                }
            }
            UnaryOp::Try => {
                return Err(CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::UnsupportedFeature {
                        feature: "Try operator (?) not yet implemented for WebAssembly".to_string(),
                    },
                });
            }
        }
        
        Ok(())
    }
    
    /// Generate instructions for function calls
    fn generate_call_instructions(&mut self, func_expr: &TypedExpression, args: &[TypedExpression], function: &mut wasm_encoder::Function) -> Result<(), CodeGenError> {
        // Generate arguments
        for arg in args {
            self.generate_expression_instructions(arg, function)?;
        }
        
        // Get function name
        if let TypedExpressionKind::Identifier(func_name) = &func_expr.kind {
            if let Some(&func_index) = self.function_indices.get(func_name) {
                function.instruction(&Instruction::Call(func_index));
            } else {
                return Err(CodeGenError {
                    span: None,
                    kind: CodeGenErrorKind::UnsupportedFeature {
                        feature: format!("Undefined function: {}", func_name),
                    },
                });
            }
        } else {
            return Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: "Indirect function calls".to_string(),
                },
            });
        }
        
        Ok(())
    }
    
    /// Generate constant expression for globals
    fn generate_const_expression(&mut self, expr: &TypedExpression) -> Result<wasm_encoder::ConstExpr, CodeGenError> {
        match &expr.kind {
            TypedExpressionKind::Literal(lit) => {
                match lit {
                    Literal::Integer(n) => Ok(wasm_encoder::ConstExpr::i64_const(*n)),
                    Literal::Float(f) => Ok(wasm_encoder::ConstExpr::f64_const(*f)),
                    Literal::Boolean(b) => Ok(wasm_encoder::ConstExpr::i32_const(if *b { 1 } else { 0 })),
                    Literal::Character(c) => Ok(wasm_encoder::ConstExpr::i32_const(*c as i32)),
                    Literal::Null => Ok(wasm_encoder::ConstExpr::i32_const(0)),
                    _ => Err(CodeGenError {
                        span: None,
                        kind: CodeGenErrorKind::UnsupportedFeature {
                            feature: "Complex constant expressions".to_string(),
                        },
                    }),
                }
            }
            _ => Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: "Non-literal constant expressions".to_string(),
                },
            }),
        }
    }
    
    /// Add a string literal to the data section and return its offset
    fn add_string_literal(&mut self, s: &str) -> u32 {
        if let Some(&offset) = self.string_literals.get(s) {
            return offset;
        }
        
        let offset = self.memory_offset;
        let bytes = s.as_bytes();
        
        // Add string data
        self.data.active(0, &wasm_encoder::ConstExpr::i32_const(offset as i32), bytes.to_vec());
        
        self.memory_offset += bytes.len() as u32;
        self.string_literals.insert(s.to_string(), offset);
        
        offset
    }
    
    /// Build the final WebAssembly module
    fn build_module(&mut self) -> Result<Vec<u8>, CodeGenError> {
        // Create a new module for building
        let mut module = Module::new();
        
        // Add all sections to the module
        module.section(&self.types);
        module.section(&self.imports);
        module.section(&self.functions);
        module.section(&self.memory);
        module.section(&self.globals);
        module.section(&self.exports);
        module.section(&self.code);
        module.section(&self.data);
        
        Ok(module.finish())
    }
}

#[cfg(not(feature = "wasm"))]
impl WasmCodeGenerator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn generate(&mut self, _program: TypedProgram) -> Result<Vec<u8>, CodeGenError> {
        Err(CodeGenError {
            span: None,
            kind: CodeGenErrorKind::UnsupportedFeature {
                feature: "WebAssembly support not compiled in. Enable 'wasm' feature.".to_string(),
            },
        })
    }
}

/// WebAssembly runtime with JavaScript interop
#[cfg(feature = "wasm")]
pub struct WasmRuntime {
    engine: wasmtime::Engine,
    store: wasmtime::Store<()>,
}

#[cfg(feature = "wasm")]
impl WasmRuntime {
    /// Create a new WebAssembly runtime
    pub fn new() -> Result<Self, CodeGenError> {
        let engine = wasmtime::Engine::default();
        let store = wasmtime::Store::new(&engine, ());
        
        Ok(Self { engine, store })
    }
    
    /// Load and instantiate a WebAssembly module
    pub fn load_module(&mut self, wasm_bytes: &[u8]) -> Result<wasmtime::Instance, CodeGenError> {
        let module = wasmtime::Module::new(&self.engine, wasm_bytes)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::RuntimeError {
                    message: format!("Failed to load WASM module: {}", e),
                },
            })?;
        
        // Create JavaScript interop functions
        let console_log = wasmtime::Func::wrap(&mut self.store, |ptr: i32| {
            println!("WASM console.log: {}", ptr);
        });
        
        let malloc = wasmtime::Func::wrap(&mut self.store, |size: i32| -> i32 {
            // Simple malloc implementation - in a real implementation,
            // this would manage a heap
            size // Just return the size as a fake pointer
        });
        
        let free = wasmtime::Func::wrap(&mut self.store, |_ptr: i32| {
            // Simple free implementation - no-op for now
        });
        
        let imports = [
            wasmtime::Extern::Func(console_log),
            wasmtime::Extern::Func(malloc),
            wasmtime::Extern::Func(free),
        ];
        
        let instance = wasmtime::Instance::new(&mut self.store, &module, &imports)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::RuntimeError {
                    message: format!("Failed to instantiate WASM module: {}", e),
                },
            })?;
        
        Ok(instance)
    }
    
    /// Call a function in the WebAssembly module
    pub fn call_function(&mut self, instance: &wasmtime::Instance, name: &str, args: &[wasmtime::Val]) -> Result<Vec<wasmtime::Val>, CodeGenError> {
        let func = instance.get_func(&mut self.store, name)
            .ok_or_else(|| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::RuntimeError {
                    message: format!("Function '{}' not found in WASM module", name),
                },
            })?;
        
        let mut results = vec![wasmtime::Val::I32(0); func.ty(&self.store).results().len()];
        
        func.call(&mut self.store, args, &mut results)
            .map_err(|e| CodeGenError {
                span: None,
                kind: CodeGenErrorKind::RuntimeError {
                    message: format!("Failed to call WASM function '{}': {}", name, e),
                },
            })?;
        
        Ok(results)
    }
}

#[cfg(not(feature = "wasm"))]
pub struct WasmRuntime;

#[cfg(not(feature = "wasm"))]
impl WasmRuntime {
    pub fn new() -> Result<Self, CodeGenError> {
        Err(CodeGenError {
            span: None,
            kind: CodeGenErrorKind::UnsupportedFeature {
                feature: "WebAssembly runtime support not compiled in. Enable 'wasm' feature.".to_string(),
            },
        })
    }
}

/// WebAssembly memory management utilities
#[cfg(feature = "wasm")]
pub struct WasmMemoryManager {
    heap_start: u32,
    heap_size: u32,
    free_blocks: Vec<(u32, u32)>, // (offset, size) pairs
}

#[cfg(feature = "wasm")]
impl WasmMemoryManager {
    /// Create a new memory manager
    pub fn new(heap_start: u32, heap_size: u32) -> Self {
        Self {
            heap_start,
            heap_size,
            free_blocks: vec![(heap_start, heap_size)],
        }
    }
    
    /// Allocate memory block
    pub fn allocate(&mut self, size: u32) -> Option<u32> {
        // Find a suitable free block
        for i in 0..self.free_blocks.len() {
            let (offset, block_size) = self.free_blocks[i];
            if block_size >= size {
                // Remove the block
                self.free_blocks.remove(i);
                
                // If there's remaining space, add it back
                if block_size > size {
                    self.free_blocks.push((offset + size, block_size - size));
                }
                
                return Some(offset);
            }
        }
        
        None // Out of memory
    }
    
    /// Deallocate memory block
    pub fn deallocate(&mut self, offset: u32, size: u32) {
        // Add the block back to free list
        self.free_blocks.push((offset, size));
        
        // Coalesce adjacent free blocks
        self.coalesce_free_blocks();
    }
    
    /// Coalesce adjacent free blocks
    fn coalesce_free_blocks(&mut self) {
        if self.free_blocks.is_empty() {
            return;
        }
        
        // Sort by offset
        self.free_blocks.sort_by_key(|&(offset, _)| offset);
        
        let mut i = 0;
        while i < self.free_blocks.len() - 1 {
            let (offset1, size1) = self.free_blocks[i];
            let (offset2, size2) = self.free_blocks[i + 1];
            
            if offset1 + size1 == offset2 {
                // Merge adjacent blocks
                self.free_blocks[i] = (offset1, size1 + size2);
                self.free_blocks.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
    
    /// Get total allocated memory
    pub fn allocated_size(&self) -> u32 {
        let total_free: u32 = self.free_blocks.iter().map(|(_, size)| size).sum();
        self.heap_size - total_free
    }
    
    /// Get available memory
    pub fn available_size(&self) -> u32 {
        self.free_blocks.iter().map(|(_, size)| size).sum()
    }
}

#[cfg(not(feature = "wasm"))]
pub struct WasmMemoryManager;

#[cfg(not(feature = "wasm"))]
impl WasmMemoryManager {
    pub fn new(_heap_start: u32, _heap_size: u32) -> Self {
        Self
    }
    
    pub fn allocate(&mut self, _size: u32) -> Option<u32> {
        None
    }
    
    pub fn deallocate(&mut self, _offset: u32, _size: u32) {
        // No-op
    }
    
    pub fn allocated_size(&self) -> u32 {
        0
    }
    
    pub fn available_size(&self) -> u32 {
        0
    }
}