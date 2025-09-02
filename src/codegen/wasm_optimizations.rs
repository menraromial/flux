//! WebAssembly-specific runtime optimizations
//! 
//! Provides optimization passes specifically for WebAssembly targets.

use crate::error::{CodeGenError, CodeGenErrorKind};
use crate::semantic::*;
use std::collections::{HashMap, HashSet};

/// WebAssembly optimization pass manager
pub struct WasmOptimizer {
    /// Functions that have been analyzed
    analyzed_functions: HashSet<String>,
    /// Function call graph
    call_graph: HashMap<String, Vec<String>>,
    /// Inline candidates
    inline_candidates: HashSet<String>,
}

impl WasmOptimizer {
    /// Create a new WebAssembly optimizer
    pub fn new() -> Self {
        Self {
            analyzed_functions: HashSet::new(),
            call_graph: HashMap::new(),
            inline_candidates: HashSet::new(),
        }
    }
    
    /// Optimize a typed program for WebAssembly
    pub fn optimize(&mut self, mut program: TypedProgram) -> Result<TypedProgram, CodeGenError> {
        // Build call graph
        self.build_call_graph(&program)?;
        
        // Apply optimization passes
        program = self.inline_small_functions(program)?;
        program = self.eliminate_dead_code(program)?;
        program = self.optimize_memory_access(program)?;
        program = self.optimize_control_flow(program)?;
        
        Ok(program)
    }
    
    /// Build function call graph
    fn build_call_graph(&mut self, program: &TypedProgram) -> Result<(), CodeGenError> {
        for item in &program.items {
            if let TypedItem::Function(func) = item {
                let mut calls = Vec::new();
                self.collect_function_calls(&func.body, &mut calls);
                self.call_graph.insert(func.name.clone(), calls);
            }
        }
        Ok(())
    }
    
    /// Collect function calls from a block
    fn collect_function_calls(&self, block: &TypedBlock, calls: &mut Vec<String>) {
        for stmt in &block.statements {
            self.collect_function_calls_from_statement(stmt, calls);
        }
    }
    
    /// Collect function calls from a statement
    fn collect_function_calls_from_statement(&self, stmt: &TypedStatement, calls: &mut Vec<String>) {
        match &stmt.kind {
            TypedStatementKind::Expression(expr) => {
                self.collect_function_calls_from_expression(expr, calls);
            }
            TypedStatementKind::Let(_, _, Some(expr)) => {
                self.collect_function_calls_from_expression(expr, calls);
            }
            TypedStatementKind::Return(Some(expr)) => {
                self.collect_function_calls_from_expression(expr, calls);
            }
            TypedStatementKind::If(cond, then_block, else_block) => {
                self.collect_function_calls_from_expression(cond, calls);
                self.collect_function_calls(then_block, calls);
                if let Some(else_block) = else_block {
                    self.collect_function_calls(else_block, calls);
                }
            }
            _ => {}
        }
    }
    
    /// Collect function calls from an expression
    fn collect_function_calls_from_expression(&self, expr: &TypedExpression, calls: &mut Vec<String>) {
        match &expr.kind {
            TypedExpressionKind::Call(func_expr, args) => {
                if let TypedExpressionKind::Identifier(func_name) = &func_expr.kind {
                    calls.push(func_name.clone());
                }
                for arg in args {
                    self.collect_function_calls_from_expression(arg, calls);
                }
            }
            TypedExpressionKind::Binary(left, _, right) => {
                self.collect_function_calls_from_expression(left, calls);
                self.collect_function_calls_from_expression(right, calls);
            }
            TypedExpressionKind::Unary(_, operand) => {
                self.collect_function_calls_from_expression(operand, calls);
            }
            TypedExpressionKind::Block(block) => {
                self.collect_function_calls(block, calls);
            }
            TypedExpressionKind::If(cond, then_block, else_block) => {
                self.collect_function_calls_from_expression(cond, calls);
                self.collect_function_calls(then_block, calls);
                if let Some(else_block) = else_block {
                    self.collect_function_calls(else_block, calls);
                }
            }
            _ => {}
        }
    }
    
    /// Inline small functions
    fn inline_small_functions(&mut self, mut program: TypedProgram) -> Result<TypedProgram, CodeGenError> {
        // Identify small functions that are good candidates for inlining
        for item in &program.items {
            if let TypedItem::Function(func) = item {
                if self.is_inline_candidate(func) {
                    self.inline_candidates.insert(func.name.clone());
                }
            }
        }
        
        // Apply inlining (simplified - a full implementation would need more sophisticated analysis)
        for item in &mut program.items {
            if let TypedItem::Function(func) = item {
                self.inline_function_calls(&mut func.body)?;
            }
        }
        
        Ok(program)
    }
    
    /// Check if a function is a good candidate for inlining
    fn is_inline_candidate(&self, func: &TypedFunction) -> bool {
        // Simple heuristics for inlining:
        // - Small functions (< 5 statements)
        // - No recursive calls
        // - Called only a few times
        
        let statement_count = self.count_statements(&func.body);
        if statement_count > 5 {
            return false;
        }
        
        // Check for recursion
        if let Some(calls) = self.call_graph.get(&func.name) {
            if calls.contains(&func.name) {
                return false; // Recursive function
            }
        }
        
        // Check call frequency (simplified)
        let call_count = self.call_graph.values()
            .map(|calls| calls.iter().filter(|&name| name == &func.name).count())
            .sum::<usize>();
        
        call_count <= 3 // Only inline if called 3 times or fewer
    }
    
    /// Count statements in a block
    fn count_statements(&self, block: &TypedBlock) -> usize {
        let mut count = block.statements.len();
        
        for stmt in &block.statements {
            match &stmt.kind {
                TypedStatementKind::If(_, then_block, else_block) => {
                    count += self.count_statements(then_block);
                    if let Some(else_block) = else_block {
                        count += self.count_statements(else_block);
                    }
                }
                _ => {}
            }
        }
        
        count
    }
    
    /// Inline function calls in a block (simplified implementation)
    fn inline_function_calls(&self, _block: &mut TypedBlock) -> Result<(), CodeGenError> {
        // This is a placeholder for function inlining
        // A full implementation would need to:
        // 1. Find function calls to inline candidates
        // 2. Replace the call with the function body
        // 3. Handle parameter substitution
        // 4. Manage variable scoping
        
        Ok(())
    }
    
    /// Eliminate dead code
    fn eliminate_dead_code(&mut self, mut program: TypedProgram) -> Result<TypedProgram, CodeGenError> {
        // Remove unused functions
        let mut used_functions = HashSet::new();
        
        // Start with main function and exported functions
        for item in &program.items {
            if let TypedItem::Function(func) = item {
                if func.name == "main" || func.visibility == crate::parser::ast::Visibility::Public {
                    used_functions.insert(func.name.clone());
                    self.mark_used_functions(&func.name, &mut used_functions);
                }
            }
        }
        
        // Remove unused functions
        program.items.retain(|item| {
            match item {
                TypedItem::Function(func) => used_functions.contains(&func.name),
                _ => true, // Keep non-function items
            }
        });
        
        Ok(program)
    }
    
    /// Mark functions as used based on call graph
    fn mark_used_functions(&self, func_name: &str, used_functions: &mut HashSet<String>) {
        if let Some(calls) = self.call_graph.get(func_name) {
            for called_func in calls {
                if used_functions.insert(called_func.clone()) {
                    // Recursively mark called functions
                    self.mark_used_functions(called_func, used_functions);
                }
            }
        }
    }
    
    /// Optimize memory access patterns
    fn optimize_memory_access(&mut self, mut program: TypedProgram) -> Result<TypedProgram, CodeGenError> {
        // WebAssembly-specific memory optimizations:
        // - Combine adjacent memory operations
        // - Use more efficient load/store instructions
        // - Optimize string operations
        
        for item in &mut program.items {
            if let TypedItem::Function(func) = item {
                self.optimize_function_memory_access(func)?;
            }
        }
        
        Ok(program)
    }
    
    /// Optimize memory access in a function
    fn optimize_function_memory_access(&self, func: &mut TypedFunction) -> Result<(), CodeGenError> {
        self.optimize_block_memory_access(&mut func.body)?;
        Ok(())
    }
    
    /// Optimize memory access in a block
    fn optimize_block_memory_access(&self, block: &mut TypedBlock) -> Result<(), CodeGenError> {
        // Look for patterns like:
        // - Multiple string concatenations -> single allocation
        // - Array access patterns -> bulk operations
        // - Repeated field access -> local caching
        
        for stmt in &mut block.statements {
            self.optimize_statement_memory_access(stmt)?;
        }
        
        Ok(())
    }
    
    /// Optimize memory access in a statement
    fn optimize_statement_memory_access(&self, stmt: &mut TypedStatement) -> Result<(), CodeGenError> {
        match &mut stmt.kind {
            TypedStatementKind::Expression(expr) => {
                self.optimize_expression_memory_access(expr)?;
            }
            TypedStatementKind::Let(_, _, Some(expr)) => {
                self.optimize_expression_memory_access(expr)?;
            }
            TypedStatementKind::Return(Some(expr)) => {
                self.optimize_expression_memory_access(expr)?;
            }
            TypedStatementKind::If(cond, then_block, else_block) => {
                self.optimize_expression_memory_access(cond)?;
                self.optimize_block_memory_access(then_block)?;
                if let Some(else_block) = else_block {
                    self.optimize_block_memory_access(else_block)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Optimize memory access in an expression
    fn optimize_expression_memory_access(&self, expr: &mut TypedExpression) -> Result<(), CodeGenError> {
        match &mut expr.kind {
            TypedExpressionKind::Binary(left, _, right) => {
                self.optimize_expression_memory_access(left)?;
                self.optimize_expression_memory_access(right)?;
            }
            TypedExpressionKind::Unary(_, operand) => {
                self.optimize_expression_memory_access(operand)?;
            }
            TypedExpressionKind::Call(func_expr, args) => {
                self.optimize_expression_memory_access(func_expr)?;
                for arg in args {
                    self.optimize_expression_memory_access(arg)?;
                }
            }
            TypedExpressionKind::Block(block) => {
                self.optimize_block_memory_access(block)?;
            }
            TypedExpressionKind::If(cond, then_block, else_block) => {
                self.optimize_expression_memory_access(cond)?;
                self.optimize_block_memory_access(then_block)?;
                if let Some(else_block) = else_block {
                    self.optimize_block_memory_access(else_block)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Optimize control flow
    fn optimize_control_flow(&mut self, mut program: TypedProgram) -> Result<TypedProgram, CodeGenError> {
        // WebAssembly-specific control flow optimizations:
        // - Simplify conditional branches
        // - Eliminate unreachable code
        // - Optimize loop structures
        
        for item in &mut program.items {
            if let TypedItem::Function(func) = item {
                self.optimize_function_control_flow(func)?;
            }
        }
        
        Ok(program)
    }
    
    /// Optimize control flow in a function
    fn optimize_function_control_flow(&self, func: &mut TypedFunction) -> Result<(), CodeGenError> {
        self.optimize_block_control_flow(&mut func.body)?;
        Ok(())
    }
    
    /// Optimize control flow in a block
    fn optimize_block_control_flow(&self, block: &mut TypedBlock) -> Result<(), CodeGenError> {
        let mut optimized_statements = Vec::new();
        let mut i = 0;
        
        while i < block.statements.len() {
            let stmt = &mut block.statements[i];
            
            // Check for optimization opportunities
            match &mut stmt.kind {
                TypedStatementKind::If(cond, then_block, else_block) => {
                    // Optimize the condition and blocks
                    self.optimize_expression_control_flow(cond)?;
                    self.optimize_block_control_flow(then_block)?;
                    if let Some(else_block) = else_block {
                        self.optimize_block_control_flow(else_block)?;
                    }
                    
                    // Check for constant conditions
                    if let TypedExpressionKind::Literal(literal) = &cond.kind {
                        match literal {
                            crate::parser::ast::Literal::Boolean(true) => {
                                // Always true - replace with then block
                                optimized_statements.extend(then_block.statements.clone());
                                i += 1;
                                continue;
                            }
                            crate::parser::ast::Literal::Boolean(false) => {
                                // Always false - replace with else block
                                if let Some(else_block) = else_block {
                                    optimized_statements.extend(else_block.statements.clone());
                                }
                                i += 1;
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
                TypedStatementKind::Expression(expr) => {
                    self.optimize_expression_control_flow(expr)?;
                }
                TypedStatementKind::Let(_, _, Some(expr)) => {
                    self.optimize_expression_control_flow(expr)?;
                }
                TypedStatementKind::Return(Some(expr)) => {
                    self.optimize_expression_control_flow(expr)?;
                    // After a return, all subsequent statements are unreachable
                    optimized_statements.push(stmt.clone());
                    break;
                }
                TypedStatementKind::Return(None) => {
                    // After a return, all subsequent statements are unreachable
                    optimized_statements.push(stmt.clone());
                    break;
                }
                _ => {}
            }
            
            optimized_statements.push(stmt.clone());
            i += 1;
        }
        
        block.statements = optimized_statements;
        Ok(())
    }
    
    /// Optimize control flow in an expression
    fn optimize_expression_control_flow(&self, expr: &mut TypedExpression) -> Result<(), CodeGenError> {
        match &mut expr.kind {
            TypedExpressionKind::If(cond, then_block, else_block) => {
                self.optimize_expression_control_flow(cond)?;
                self.optimize_block_control_flow(then_block)?;
                if let Some(else_block) = else_block {
                    self.optimize_block_control_flow(else_block)?;
                }
            }
            TypedExpressionKind::Binary(left, _, right) => {
                self.optimize_expression_control_flow(left)?;
                self.optimize_expression_control_flow(right)?;
            }
            TypedExpressionKind::Unary(_, operand) => {
                self.optimize_expression_control_flow(operand)?;
            }
            TypedExpressionKind::Call(func_expr, args) => {
                self.optimize_expression_control_flow(func_expr)?;
                for arg in args {
                    self.optimize_expression_control_flow(arg)?;
                }
            }
            TypedExpressionKind::Block(block) => {
                self.optimize_block_control_flow(block)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Default for WasmOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// WebAssembly-specific memory layout optimizer
pub struct WasmMemoryOptimizer {
    /// Memory layout information
    layout: HashMap<String, MemoryLayout>,
}

/// Memory layout information for a type
#[derive(Debug, Clone)]
pub struct MemoryLayout {
    pub size: u32,
    pub alignment: u32,
    pub fields: Vec<FieldLayout>,
}

/// Field layout information
#[derive(Debug, Clone)]
pub struct FieldLayout {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

impl WasmMemoryOptimizer {
    /// Create a new memory optimizer
    pub fn new() -> Self {
        Self {
            layout: HashMap::new(),
        }
    }
    
    /// Optimize memory layout for WebAssembly
    pub fn optimize_layout(&mut self, program: &TypedProgram) -> Result<(), CodeGenError> {
        // Analyze struct layouts and optimize for WebAssembly memory model
        for item in &program.items {
            if let TypedItem::Struct(struct_def) = item {
                let layout = self.calculate_optimal_layout(struct_def)?;
                self.layout.insert(struct_def.name.clone(), layout);
            }
        }
        
        Ok(())
    }
    
    /// Calculate optimal memory layout for a struct
    fn calculate_optimal_layout(&self, struct_def: &TypedStruct) -> Result<MemoryLayout, CodeGenError> {
        let mut fields = Vec::new();
        let mut offset = 0u32;
        let mut max_alignment = 1u32;
        
        for field in &struct_def.fields {
            let field_size = self.get_type_size(&field.type_)?;
            let field_alignment = self.get_type_alignment(&field.type_)?;
            
            // Align the offset
            offset = self.align_offset(offset, field_alignment);
            
            fields.push(FieldLayout {
                name: field.name.clone(),
                offset,
                size: field_size,
            });
            
            offset += field_size;
            max_alignment = max_alignment.max(field_alignment);
        }
        
        // Align the total size
        let total_size = self.align_offset(offset, max_alignment);
        
        Ok(MemoryLayout {
            size: total_size,
            alignment: max_alignment,
            fields,
        })
    }
    
    /// Get the size of a type in bytes
    fn get_type_size(&self, type_: &crate::parser::ast::Type) -> Result<u32, CodeGenError> {
        match type_ {
            crate::parser::ast::Type::Int => Ok(8),
            crate::parser::ast::Type::Float => Ok(8),
            crate::parser::ast::Type::Bool => Ok(1),
            crate::parser::ast::Type::Char => Ok(1),
            crate::parser::ast::Type::Byte => Ok(1),
            crate::parser::ast::Type::String => Ok(4), // Pointer size
            crate::parser::ast::Type::Array(_) => Ok(4), // Pointer size
            crate::parser::ast::Type::Nullable(_) => Ok(4), // Pointer size
            _ => Err(CodeGenError {
                span: None,
                kind: CodeGenErrorKind::UnsupportedFeature {
                    feature: format!("Size calculation for type: {:?}", type_),
                },
            }),
        }
    }
    
    /// Get the alignment requirement of a type
    fn get_type_alignment(&self, type_: &crate::parser::ast::Type) -> Result<u32, CodeGenError> {
        match type_ {
            crate::parser::ast::Type::Int => Ok(8),
            crate::parser::ast::Type::Float => Ok(8),
            crate::parser::ast::Type::Bool => Ok(1),
            crate::parser::ast::Type::Char => Ok(1),
            crate::parser::ast::Type::Byte => Ok(1),
            crate::parser::ast::Type::String => Ok(4), // Pointer alignment
            crate::parser::ast::Type::Array(_) => Ok(4), // Pointer alignment
            crate::parser::ast::Type::Nullable(_) => Ok(4), // Pointer alignment
            _ => Ok(1), // Default alignment
        }
    }
    
    /// Align an offset to the specified alignment
    fn align_offset(&self, offset: u32, alignment: u32) -> u32 {
        (offset + alignment - 1) & !(alignment - 1)
    }
    
    /// Get the memory layout for a type
    pub fn get_layout(&self, type_name: &str) -> Option<&MemoryLayout> {
        self.layout.get(type_name)
    }
}

impl Default for WasmMemoryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}