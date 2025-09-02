//! Semantic analysis module
//! 
//! Provides type checking, name resolution, and semantic validation for Flux programs.

use crate::error::{SemanticError, SemanticErrorKind};
use crate::parser::ast::{
    Program, Item, Function, Struct, Class, Const, Import, Visibility,
    Type, Pattern, Literal, BinaryOp, UnaryOp, Block, Expression, Statement, ResultPattern
};
use crate::semantic::symbol_table::ScopeType;
use crate::position::Span;
use std::collections::HashMap;

pub mod symbol_table;
pub mod type_checker;

pub use symbol_table::*;
pub use type_checker::*;

/// Core semantic analyzer trait
pub trait SemanticAnalyzer {
    /// Analyze a program and return a typed AST
    fn analyze(&mut self, program: Program) -> Result<TypedProgram, SemanticError>;
    
    /// Resolve names and build symbol table
    fn resolve_names(&mut self, program: &mut Program) -> Result<(), SemanticError>;
    
    /// Check types throughout the program
    fn check_types(&mut self, program: &Program) -> Result<TypedProgram, SemanticError>;
    
    /// Validate semantic constraints
    fn validate_semantics(&mut self, program: &TypedProgram) -> Result<(), SemanticError>;
}

/// Default implementation of semantic analysis
pub struct FluxSemanticAnalyzer {
    symbol_table: SymbolTable,
    type_checker: TypeChecker,
}

impl FluxSemanticAnalyzer {
    /// Create a new semantic analyzer
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            type_checker: TypeChecker::new(),
        }
    }
}

impl Default for FluxSemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer for FluxSemanticAnalyzer {
    fn analyze(&mut self, mut program: Program) -> Result<TypedProgram, SemanticError> {
        // Phase 1: Name resolution
        self.resolve_names(&mut program)?;
        
        // Phase 2: Type checking
        let typed_program = self.check_types(&program)?;
        
        // Phase 3: Semantic validation
        self.validate_semantics(&typed_program)?;
        
        Ok(typed_program)
    }
    
    fn resolve_names(&mut self, program: &mut Program) -> Result<(), SemanticError> {
        // First pass: Define all top-level items in global scope
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    self.symbol_table.define_function(func.name.clone(), func.clone())?;
                }
                Item::Struct(struct_def) => {
                    self.symbol_table.define_struct(struct_def.name.clone(), struct_def.clone())?;
                }
                Item::Class(class_def) => {
                    self.symbol_table.define_class(class_def.name.clone(), class_def.clone())?;
                }
                Item::Const(const_def) => {
                    self.symbol_table.define_const(const_def.name.clone(), const_def.clone())?;
                }
                Item::ExternFunction(extern_func) => {
                    // Register extern function in symbol table
                    self.symbol_table.define_extern_function(extern_func.name.clone(), extern_func.clone())?;
                }
            }
        }
        
        // Second pass: Resolve names within each item
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    self.resolve_function_names(func)?;
                }
                Item::Struct(struct_def) => {
                    self.resolve_struct_names(struct_def)?;
                }
                Item::Class(class_def) => {
                    self.resolve_class_names(class_def)?;
                }
                Item::Const(const_def) => {
                    self.resolve_const_names(const_def)?;
                }
                Item::ExternFunction(_extern_func) => {
                    // Extern functions don't have bodies to resolve
                }
            }
        }
        
        Ok(())
    }
    
    fn check_types(&mut self, program: &Program) -> Result<TypedProgram, SemanticError> {
        let mut typed_items = Vec::new();
        
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    let typed_func = self.type_checker.check_function(func)?;
                    typed_items.push(TypedItem::Function(typed_func));
                }
                Item::Struct(struct_def) => {
                    let typed_struct = self.type_checker.check_struct(struct_def)?;
                    typed_items.push(TypedItem::Struct(typed_struct));
                }
                Item::Class(class_def) => {
                    let typed_class = self.type_checker.check_class(class_def)?;
                    typed_items.push(TypedItem::Class(typed_class));
                }
                Item::Const(const_def) => {
                    let typed_const = self.type_checker.check_const(const_def)?;
                    typed_items.push(TypedItem::Const(typed_const));
                }
                Item::ExternFunction(extern_func) => {
                    // Extern functions don't need type checking, just validation
                    let typed_extern = self.type_checker.check_extern_function(extern_func)?;
                    typed_items.push(TypedItem::ExternFunction(typed_extern));
                }
            }
        }
        
        Ok(TypedProgram {
            package: program.package.clone(),
            imports: program.imports.clone(),
            items: typed_items,
        })
    }
    
    fn validate_semantics(&mut self, _program: &TypedProgram) -> Result<(), SemanticError> {
        // Placeholder for semantic validation
        // This would check things like:
        // - All code paths return a value
        // - No unreachable code
        // - Proper error handling
        // - etc.
        Ok(())
    }
}

impl FluxSemanticAnalyzer {
    fn resolve_function_names(&mut self, func: &Function) -> Result<(), SemanticError> {
        // Enter function scope
        self.symbol_table.enter_function_scope(func.name.clone());
        
        // Define parameters
        for (index, param) in func.parameters.iter().enumerate() {
            self.symbol_table.define_parameter(
                param.name.clone(),
                param.type_.clone(),
                param.is_mutable,
                index
            )?;
        }
        
        // Resolve names in function body
        self.resolve_block_names(&func.body)?;
        
        // Exit function scope
        self.symbol_table.exit_scope();
        
        Ok(())
    }
    
    fn resolve_struct_names(&mut self, _struct_def: &Struct) -> Result<(), SemanticError> {
        // Struct field types should already be resolved
        // In a more complete implementation, we'd validate field types here
        Ok(())
    }
    
    fn resolve_class_names(&mut self, class_def: &Class) -> Result<(), SemanticError> {
        // Resolve method names
        for method in &class_def.methods {
            self.resolve_method_names(method)?;
        }
        Ok(())
    }
    
    fn resolve_method_names(&mut self, method: &crate::parser::ast::Method) -> Result<(), SemanticError> {
        // Enter function scope for method
        self.symbol_table.enter_function_scope(method.name.clone());
        
        // Define parameters (including implicit 'self' if not static)
        let mut param_index = 0;
        if !method.is_static {
            // Add implicit 'self' parameter
            // Note: In a real implementation, we'd need the class type here
            param_index += 1;
        }
        
        for param in &method.parameters {
            self.symbol_table.define_parameter(
                param.name.clone(),
                param.type_.clone(),
                param.is_mutable,
                param_index
            )?;
            param_index += 1;
        }
        
        // Resolve names in method body
        self.resolve_block_names(&method.body)?;
        
        // Exit function scope
        self.symbol_table.exit_scope();
        
        Ok(())
    }
    
    fn resolve_const_names(&mut self, const_def: &Const) -> Result<(), SemanticError> {
        // Resolve names in constant expression
        self.resolve_expression_names(&const_def.value)?;
        Ok(())
    }
    
    fn resolve_block_names(&mut self, block: &Block) -> Result<(), SemanticError> {
        // Enter block scope
        self.symbol_table.enter_scope(ScopeType::Block);
        
        // Resolve each statement
        for stmt in &block.statements {
            self.resolve_statement_names(stmt)?;
        }
        
        // Exit block scope
        self.symbol_table.exit_scope();
        
        Ok(())
    }
    
    fn resolve_statement_names(&mut self, stmt: &Statement) -> Result<(), SemanticError> {
        match stmt {
            Statement::Expression(expr) => {
                self.resolve_expression_names(expr)?;
            }
            Statement::Let(name, type_annotation, init) => {
                // Resolve initializer first (if present)
                if let Some(init_expr) = init {
                    self.resolve_expression_names(init_expr)?;
                }
                
                // Define the variable
                let var_type = type_annotation.clone().unwrap_or(Type::Unit); // Placeholder
                self.symbol_table.define_variable(name.clone(), var_type, true)?;
                
                // Mark as initialized if there's an initializer
                if init.is_some() {
                    self.symbol_table.mark_initialized(name)?;
                }
            }
            Statement::Const(name, type_, value) => {
                // Resolve the value expression
                self.resolve_expression_names(value)?;
                
                // Define the constant
                self.symbol_table.define_variable(name.clone(), type_.clone(), false)?;
                self.symbol_table.mark_initialized(name)?;
            }
            Statement::Assignment(target, value) => {
                // Resolve both sides
                self.resolve_expression_names(target)?;
                self.resolve_expression_names(value)?;
                
                // Check if target is assignable (if it's an identifier)
                if let Expression::Identifier(name) = target {
                    if !self.symbol_table.can_assign(name)? {
                        return Err(SemanticError {
                            span: Span::single(crate::position::Position::start()),
                            kind: SemanticErrorKind::InvalidOperation {
                                message: format!("Cannot assign to immutable variable '{}'", name),
                            },
                        });
                    }
                    // Mark as initialized after assignment
                    self.symbol_table.mark_initialized(name)?;
                }
            }
            Statement::Return(expr) => {
                if let Some(e) = expr {
                    self.resolve_expression_names(e)?;
                }
                
                // Check if we're in a function
                if !self.symbol_table.in_function() {
                    return Err(SemanticError {
                        span: Span::single(crate::position::Position::start()),
                        kind: SemanticErrorKind::InvalidOperation {
                            message: "Return statement outside function".to_string(),
                        },
                    });
                }
            }
            Statement::Break(expr) => {
                if let Some(e) = expr {
                    self.resolve_expression_names(e)?;
                }
                
                // Check if we're in a loop
                if !self.symbol_table.in_loop() {
                    return Err(SemanticError {
                        span: Span::single(crate::position::Position::start()),
                        kind: SemanticErrorKind::InvalidOperation {
                            message: "Break statement outside loop".to_string(),
                        },
                    });
                }
            }
            Statement::Continue => {
                // Check if we're in a loop
                if !self.symbol_table.in_loop() {
                    return Err(SemanticError {
                        span: Span::single(crate::position::Position::start()),
                        kind: SemanticErrorKind::InvalidOperation {
                            message: "Continue statement outside loop".to_string(),
                        },
                    });
                }
            }
            Statement::Go(expr) => {
                self.resolve_expression_names(expr)?;
            }
            Statement::If(cond, then_block, else_block) => {
                self.resolve_expression_names(cond)?;
                self.resolve_block_names(then_block)?;
                if let Some(else_b) = else_block {
                    self.resolve_block_names(else_b)?;
                }
            }
            Statement::While(cond, body) => {
                self.resolve_expression_names(cond)?;
                
                // Enter loop scope
                self.symbol_table.enter_scope(ScopeType::Loop);
                self.resolve_block_names(body)?;
                self.symbol_table.exit_scope();
            }
            Statement::For(var, iter, body) => {
                // Resolve iterator expression
                self.resolve_expression_names(iter)?;
                
                // Enter loop scope and define loop variable
                self.symbol_table.enter_scope(ScopeType::Loop);
                
                // Define loop variable (type would be inferred from iterator)
                self.symbol_table.define_variable(var.clone(), Type::Unit, false)?; // Placeholder type
                self.symbol_table.mark_initialized(var)?;
                
                self.resolve_block_names(body)?;
                self.symbol_table.exit_scope();
            }
            Statement::Match(expr, arms) => {
                self.resolve_expression_names(expr)?;
                
                for arm in arms {
                    // Enter match scope for pattern bindings
                    self.symbol_table.enter_scope(ScopeType::Match);
                    
                    // Define pattern variables (simplified)
                    self.resolve_pattern_names(&arm.pattern)?;
                    
                    if let Some(guard) = &arm.guard {
                        self.resolve_expression_names(guard)?;
                    }
                    
                    self.resolve_block_names(&arm.body)?;
                    
                    self.symbol_table.exit_scope();
                }
            }
        }
        
        Ok(())
    }
    
    fn resolve_expression_names(&mut self, expr: &Expression) -> Result<(), SemanticError> {
        match expr {
            Expression::Literal(_) => {
                // Literals don't have names to resolve
                Ok(())
            }
            Expression::Identifier(name) => {
                // Resolve the identifier
                self.symbol_table.resolve_name(name)?;
                Ok(())
            }
            Expression::Binary(left, _op, right) => {
                self.resolve_expression_names(left)?;
                self.resolve_expression_names(right)?;
                Ok(())
            }
            Expression::Unary(_op, expr) => {
                self.resolve_expression_names(expr)?;
                Ok(())
            }
            Expression::Call(func, args) => {
                self.resolve_expression_names(func)?;
                for arg in args {
                    self.resolve_expression_names(arg)?;
                }
                Ok(())
            }
            Expression::Index(array, index) => {
                self.resolve_expression_names(array)?;
                self.resolve_expression_names(index)?;
                Ok(())
            }
            Expression::Field(obj, _field) => {
                self.resolve_expression_names(obj)?;
                // Field resolution would require type information
                Ok(())
            }
            Expression::Match(expr, arms) => {
                self.resolve_expression_names(expr)?;
                
                for arm in arms {
                    self.symbol_table.enter_scope(ScopeType::Match);
                    self.resolve_pattern_names(&arm.pattern)?;
                    
                    if let Some(guard) = &arm.guard {
                        self.resolve_expression_names(guard)?;
                    }
                    
                    self.resolve_block_names(&arm.body)?;
                    self.symbol_table.exit_scope();
                }
                Ok(())
            }
            Expression::If(cond, then_block, else_block) => {
                self.resolve_expression_names(cond)?;
                self.resolve_block_names(then_block)?;
                if let Some(else_b) = else_block {
                    self.resolve_block_names(else_b)?;
                }
                Ok(())
            }
            Expression::Block(block) => {
                self.resolve_block_names(block)?;
                Ok(())
            }
            Expression::Array(elements) => {
                for elem in elements {
                    self.resolve_expression_names(elem)?;
                }
                Ok(())
            }
            Expression::Map(pairs) => {
                for (key, value) in pairs {
                    self.resolve_expression_names(key)?;
                    self.resolve_expression_names(value)?;
                }
                Ok(())
            }
            Expression::Tuple(elements) => {
                for elem in elements {
                    self.resolve_expression_names(elem)?;
                }
                Ok(())
            }
        }
    }
    
    fn resolve_pattern_names(&mut self, pattern: &Pattern) -> Result<(), SemanticError> {
        match pattern {
            Pattern::Literal(_) => Ok(()),
            Pattern::Identifier(name) => {
                // Pattern identifiers bind new variables
                self.symbol_table.define_variable(name.clone(), Type::Unit, false)?; // Placeholder type
                self.symbol_table.mark_initialized(name)?;
                Ok(())
            }
            Pattern::Wildcard => Ok(()),
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    self.resolve_pattern_names(p)?;
                }
                Ok(())
            }
            Pattern::Struct(_name, fields) => {
                for (_field_name, pattern) in fields {
                    self.resolve_pattern_names(pattern)?;
                }
                Ok(())
            }
            Pattern::Result(result_pattern) => {
                match result_pattern {
                    ResultPattern::Ok(pattern) => self.resolve_pattern_names(pattern),
                    ResultPattern::Err(pattern) => self.resolve_pattern_names(pattern),
                }
            }
        }
    }
}

/// Typed version of the AST after semantic analysis
#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    pub package: String,
    pub imports: Vec<Import>,
    pub items: Vec<TypedItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedItem {
    Function(TypedFunction),
    Struct(TypedStruct),
    Class(TypedClass),
    Const(TypedConst),
    ExternFunction(TypedExternFunction),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedFunction {
    pub name: String,
    pub parameters: Vec<TypedParameter>,
    pub return_type: Type,
    pub body: TypedBlock,
    pub is_async: bool,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedParameter {
    pub name: String,
    pub type_: Type,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedStruct {
    pub name: String,
    pub fields: Vec<TypedField>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedClass {
    pub name: String,
    pub fields: Vec<TypedField>,
    pub methods: Vec<TypedMethod>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedField {
    pub name: String,
    pub type_: Type,
    pub visibility: Visibility,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedMethod {
    pub name: String,
    pub parameters: Vec<TypedParameter>,
    pub return_type: Type,
    pub body: TypedBlock,
    pub visibility: Visibility,
    pub is_static: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedConst {
    pub name: String,
    pub type_: Type,
    pub value: TypedExpression,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedExternFunction {
    pub name: String,
    pub parameters: Vec<TypedParameter>,
    pub return_type: Option<Type>,
    pub library: Option<String>,
    pub is_variadic: bool,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedBlock {
    pub statements: Vec<TypedStatement>,
    pub type_: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedStatement {
    pub kind: TypedStatementKind,
    pub span: Option<crate::position::Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedStatementKind {
    Expression(TypedExpression),
    Let(String, Type, Option<TypedExpression>),
    Const(String, Type, TypedExpression),
    Assignment(TypedExpression, TypedExpression),
    Return(Option<TypedExpression>),
    Break(Option<TypedExpression>),
    Continue,
    Go(TypedExpression),
    If(TypedExpression, TypedBlock, Option<TypedBlock>),
    While(TypedExpression, TypedBlock),
    For(String, TypedExpression, TypedBlock),
    Match(TypedExpression, Vec<TypedMatchArm>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedMatchArm {
    pub pattern: Pattern,
    pub guard: Option<TypedExpression>,
    pub body: TypedBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpression {
    pub kind: TypedExpressionKind,
    pub type_: Type,
    pub span: Option<crate::position::Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedExpressionKind {
    Literal(Literal),
    Identifier(String),
    Binary(Box<TypedExpression>, BinaryOp, Box<TypedExpression>),
    Unary(UnaryOp, Box<TypedExpression>),
    Call(Box<TypedExpression>, Vec<TypedExpression>),
    Index(Box<TypedExpression>, Box<TypedExpression>),
    Field(Box<TypedExpression>, String),
    Match(Box<TypedExpression>, Vec<TypedMatchArm>),
    If(Box<TypedExpression>, TypedBlock, Option<TypedBlock>),
    Block(TypedBlock),
    Array(Vec<TypedExpression>),
    Map(Vec<(TypedExpression, TypedExpression)>),
    Tuple(Vec<TypedExpression>),
}