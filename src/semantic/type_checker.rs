//! Type checking and inference for the Flux language
//! 
//! Provides type checking, inference, and unification algorithms.

use crate::error::{SemanticError, SemanticErrorKind};
use crate::parser::ast::{
    Type, Expression, Statement, Block, Literal, BinaryOp, UnaryOp, 
    Function, Struct, Class, Const, Method, Parameter, Field, Visibility, ExternFunction
};
use crate::position::Span;
use crate::semantic::*;
use std::collections::HashMap;

/// Type variable for generic type inference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar(pub u32);

/// Extended type representation for type inference
#[derive(Debug, Clone, PartialEq)]
pub enum InferType {
    /// Concrete type from AST
    Concrete(Type),
    /// Type variable for inference
    Variable(TypeVar),
    /// Function type with parameters and return type
    Function(Vec<InferType>, Box<InferType>),
}

impl InferType {
    /// Convert to concrete type if possible
    pub fn to_concrete(&self) -> Option<Type> {
        match self {
            InferType::Concrete(t) => Some(t.clone()),
            InferType::Variable(_) => None,
            InferType::Function(params, ret) => {
                let concrete_params: Option<Vec<Type>> = params.iter()
                    .map(|p| p.to_concrete())
                    .collect();
                let concrete_ret = ret.to_concrete()?;
                Some(Type::Function(concrete_params?, Box::new(concrete_ret)))
            }
        }
    }
    
    /// Check if this type contains the given type variable
    pub fn occurs_check(&self, var: &TypeVar) -> bool {
        match self {
            InferType::Variable(v) => v == var,
            InferType::Concrete(Type::Function(params, ret)) => {
                params.iter().any(|p| InferType::Concrete(p.clone()).occurs_check(var)) ||
                InferType::Concrete((**ret).clone()).occurs_check(var)
            }
            InferType::Function(params, ret) => {
                params.iter().any(|p| p.occurs_check(var)) || ret.occurs_check(var)
            }
            _ => false,
        }
    }
}

/// Type substitution for unification
#[derive(Debug, Clone)]
pub struct Substitution {
    bindings: HashMap<TypeVar, InferType>,
}

impl Substitution {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
    
    pub fn bind(&mut self, var: TypeVar, ty: InferType) {
        self.bindings.insert(var, ty);
    }
    
    pub fn lookup(&self, var: &TypeVar) -> Option<&InferType> {
        self.bindings.get(var)
    }
    
    pub fn apply(&self, ty: &InferType) -> InferType {
        match ty {
            InferType::Variable(var) => {
                if let Some(substituted) = self.lookup(var) {
                    self.apply(substituted)
                } else {
                    ty.clone()
                }
            }
            InferType::Function(params, ret) => {
                let new_params = params.iter().map(|p| self.apply(p)).collect();
                let new_ret = Box::new(self.apply(ret));
                InferType::Function(new_params, new_ret)
            }
            InferType::Concrete(Type::Function(params, ret)) => {
                let new_params = params.iter().map(|p| {
                    match self.apply(&InferType::Concrete(p.clone())) {
                        InferType::Concrete(t) => t,
                        _ => p.clone(), // Fallback if substitution doesn't yield concrete type
                    }
                }).collect();
                let new_ret = match self.apply(&InferType::Concrete((**ret).clone())) {
                    InferType::Concrete(t) => Box::new(t),
                    _ => ret.clone(), // Fallback
                };
                InferType::Concrete(Type::Function(new_params, new_ret))
            }
            _ => ty.clone(),
        }
    }
    
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = Substitution::new();
        
        // Apply this substitution to other's bindings
        for (var, ty) in &other.bindings {
            result.bind(var.clone(), self.apply(ty));
        }
        
        // Add this substitution's bindings
        for (var, ty) in &self.bindings {
            if !result.bindings.contains_key(var) {
                result.bind(var.clone(), ty.clone());
            }
        }
        
        result
    }
}

/// Type environment for managing type bindings with scope support
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    scopes: Vec<HashMap<String, InferType>>,
    next_var_id: u32,
}

impl TypeEnvironment {
    /// Create a new type environment with global scope
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![HashMap::new()],
            next_var_id: 0,
        };
        
        // Add built-in types and functions
        env.add_builtins();
        env
    }
    
    /// Add built-in types and functions to the environment
    fn add_builtins(&mut self) {
        // Built-in functions
        self.bind("print".to_string(), InferType::Function(
            vec![InferType::Concrete(Type::String)],
            Box::new(InferType::Concrete(Type::Unit))
        ));
        
        self.bind("println".to_string(), InferType::Function(
            vec![InferType::Concrete(Type::String)],
            Box::new(InferType::Concrete(Type::Unit))
        ));
        
        // Built-in type constructors
        let fresh_var = self.fresh_var();
        self.bind("Some".to_string(), InferType::Function(
            vec![InferType::Variable(fresh_var)],
            Box::new(InferType::Concrete(Type::Nullable(Box::new(Type::Generic("T".to_string(), vec![])))))
        ));
    }
    
    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    
    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
    
    /// Bind a name to a type in the current scope
    pub fn bind(&mut self, name: String, type_: InferType) {
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.insert(name, type_);
        }
    }
    
    /// Look up the type of a name, searching from innermost to outermost scope
    pub fn lookup(&self, name: &str) -> Option<&InferType> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }
    
    /// Generate a fresh type variable
    pub fn fresh_var(&mut self) -> TypeVar {
        let var = TypeVar(self.next_var_id);
        self.next_var_id += 1;
        var
    }
    
    /// Get the current scope depth
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Type checker for Flux programs with unification-based inference
#[derive(Debug)]
pub struct TypeChecker {
    type_env: TypeEnvironment,
    constraints: Vec<(InferType, InferType, Span)>,
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new() -> Self {
        Self {
            type_env: TypeEnvironment::new(),
            constraints: Vec::new(),
        }
    }
    
    /// Add a type constraint for later unification
    pub fn add_constraint(&mut self, t1: InferType, t2: InferType, span: Span) {
        self.constraints.push((t1, t2, span));
    }
    
    /// Solve all accumulated constraints using unification
    pub fn solve_constraints(&mut self) -> Result<Substitution, SemanticError> {
        let mut subst = Substitution::new();
        
        for (t1, t2, span) in &self.constraints {
            let unified_subst = self.unify(&subst.apply(t1), &subst.apply(t2), span.clone())?;
            subst = subst.compose(&unified_subst);
        }
        
        self.constraints.clear();
        Ok(subst)
    }
    
    /// Unify two types using Robinson's unification algorithm
    pub fn unify(&self, t1: &InferType, t2: &InferType, span: Span) -> Result<Substitution, SemanticError> {
        match (t1, t2) {
            // Same concrete types unify
            (InferType::Concrete(a), InferType::Concrete(b)) if a == b => {
                Ok(Substitution::new())
            }
            
            // Variable unifies with any type (occurs check)
            (InferType::Variable(var), ty) | (ty, InferType::Variable(var)) => {
                if ty.occurs_check(var) {
                    return Err(SemanticError {
                        span,
                        kind: SemanticErrorKind::InvalidOperation {
                            message: "Infinite type detected in unification".to_string(),
                        },
                    });
                }
                
                let mut subst = Substitution::new();
                subst.bind(var.clone(), ty.clone());
                Ok(subst)
            }
            
            // Function types unify if parameters and return types unify
            (InferType::Function(params1, ret1), InferType::Function(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return Err(SemanticError {
                        span,
                        kind: SemanticErrorKind::TypeMismatch {
                            expected: format!("function with {} parameters", params1.len()),
                            found: format!("function with {} parameters", params2.len()),
                        },
                    });
                }
                
                let mut subst = Substitution::new();
                
                // Unify parameters
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    let param_subst = self.unify(&subst.apply(p1), &subst.apply(p2), span.clone())?;
                    subst = subst.compose(&param_subst);
                }
                
                // Unify return types
                let ret_subst = self.unify(&subst.apply(ret1), &subst.apply(ret2), span)?;
                subst = subst.compose(&ret_subst);
                
                Ok(subst)
            }
            
            // Concrete function types
            (InferType::Concrete(Type::Function(params1, ret1)), InferType::Concrete(Type::Function(params2, ret2))) => {
                if params1.len() != params2.len() {
                    return Err(SemanticError {
                        span,
                        kind: SemanticErrorKind::TypeMismatch {
                            expected: format!("function with {} parameters", params1.len()),
                            found: format!("function with {} parameters", params2.len()),
                        },
                    });
                }
                
                let mut subst = Substitution::new();
                
                // Unify parameters
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    let param_subst = self.unify(
                        &InferType::Concrete(p1.clone()),
                        &InferType::Concrete(p2.clone()),
                        span.clone()
                    )?;
                    subst = subst.compose(&param_subst);
                }
                
                // Unify return types
                let ret_subst = self.unify(
                    &InferType::Concrete((**ret1).clone()),
                    &InferType::Concrete((**ret2).clone()),
                    span
                )?;
                subst = subst.compose(&ret_subst);
                
                Ok(subst)
            }
            
            // Mixed function types
            (InferType::Function(params1, ret1), InferType::Concrete(Type::Function(params2, ret2))) |
            (InferType::Concrete(Type::Function(params2, ret2)), InferType::Function(params1, ret1)) => {
                if params1.len() != params2.len() {
                    return Err(SemanticError {
                        span,
                        kind: SemanticErrorKind::TypeMismatch {
                            expected: format!("function with {} parameters", params2.len()),
                            found: format!("function with {} parameters", params1.len()),
                        },
                    });
                }
                
                let mut subst = Substitution::new();
                
                // Unify parameters
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    let param_subst = self.unify(p1, &InferType::Concrete(p2.clone()), span.clone())?;
                    subst = subst.compose(&param_subst);
                }
                
                // Unify return types
                let ret_subst = self.unify(ret1, &InferType::Concrete((**ret2).clone()), span)?;
                subst = subst.compose(&ret_subst);
                
                Ok(subst)
            }
            
            // Types don't unify
            _ => Err(SemanticError {
                span,
                kind: SemanticErrorKind::TypeMismatch {
                    expected: format!("{:?}", t1),
                    found: format!("{:?}", t2),
                },
            })
        }
    }
    
    /// Infer the type of an expression using constraint-based inference
    pub fn infer_expression(&mut self, expr: &Expression) -> Result<InferType, SemanticError> {
        let span = Span::single(crate::position::Position::start()); // Placeholder
        
        match expr {
            Expression::Literal(lit) => Ok(InferType::Concrete(self.literal_type(lit))),
            
            Expression::Identifier(name) => {
                self.type_env.lookup(name)
                    .cloned()
                    .ok_or_else(|| SemanticError {
                        span,
                        kind: SemanticErrorKind::UndefinedVariable { name: name.clone() },
                    })
            }
            
            Expression::Binary(left, op, right) => {
                let left_type = self.infer_expression(left)?;
                let right_type = self.infer_expression(right)?;
                self.infer_binary_op(&left_type, op, &right_type, span)
            }
            
            Expression::Unary(op, expr) => {
                let expr_type = self.infer_expression(expr)?;
                self.infer_unary_op(op, &expr_type, span)
            }
            
            Expression::Call(func, args) => {
                let func_type = self.infer_expression(func)?;
                let arg_types: Result<Vec<_>, _> = args.iter()
                    .map(|arg| self.infer_expression(arg))
                    .collect();
                let arg_types = arg_types?;
                
                // Create fresh type variable for return type
                let return_var = self.type_env.fresh_var();
                let expected_func_type = InferType::Function(
                    arg_types,
                    Box::new(InferType::Variable(return_var.clone()))
                );
                
                // Add constraint that function type matches expected
                self.add_constraint(func_type, expected_func_type, span);
                
                Ok(InferType::Variable(return_var))
            }
            
            Expression::If(cond, then_block, else_block) => {
                let cond_type = self.infer_expression(cond)?;
                
                // Condition must be boolean
                self.add_constraint(
                    cond_type,
                    InferType::Concrete(Type::Bool),
                    span.clone()
                );
                
                let then_type = self.infer_block(then_block)?;
                
                if let Some(else_b) = else_block {
                    let else_type = self.infer_block(else_b)?;
                    
                    // Both branches must have same type
                    self.add_constraint(then_type.clone(), else_type, span);
                    Ok(then_type)
                } else {
                    // If without else returns unit
                    self.add_constraint(
                        then_type,
                        InferType::Concrete(Type::Unit),
                        span
                    );
                    Ok(InferType::Concrete(Type::Unit))
                }
            }
            
            Expression::Block(block) => self.infer_block(block),
            
            Expression::Array(elements) => {
                if elements.is_empty() {
                    // Empty array gets fresh type variable
                    let _elem_var = self.type_env.fresh_var();
                    Ok(InferType::Concrete(Type::Array(Box::new(Type::Generic("T".to_string(), vec![])))))
                } else {
                    let first_type = self.infer_expression(&elements[0])?;
                    
                    // All elements must have same type
                    for elem in &elements[1..] {
                        let elem_type = self.infer_expression(elem)?;
                        self.add_constraint(first_type.clone(), elem_type, span.clone());
                    }
                    
                    // Solve constraints to get concrete type
                    let subst = self.solve_constraints()?;
                    let resolved_first = subst.apply(&first_type);
                    
                    if let Some(concrete) = resolved_first.to_concrete() {
                        Ok(InferType::Concrete(Type::Array(Box::new(concrete))))
                    } else {
                        Ok(InferType::Concrete(Type::Array(Box::new(Type::Generic("T".to_string(), vec![])))))
                    }
                }
            }
            
            Expression::Index(array, index) => {
                let array_type = self.infer_expression(array)?;
                let index_type = self.infer_expression(index)?;
                
                // Index must be integer
                self.add_constraint(
                    index_type,
                    InferType::Concrete(Type::Int),
                    span.clone()
                );
                
                // Create fresh type variable for element type
                let elem_var = self.type_env.fresh_var();
                let expected_array_type = InferType::Concrete(Type::Array(
                    Box::new(Type::Generic("T".to_string(), vec![]))
                ));
                
                self.add_constraint(array_type, expected_array_type, span);
                
                Ok(InferType::Variable(elem_var))
            }
            
            Expression::Field(obj, field) => {
                let _obj_type = self.infer_expression(obj)?;
                // Field access requires struct/class type information
                // For now, return a fresh type variable
                let field_var = self.type_env.fresh_var();
                Ok(InferType::Variable(field_var))
            }
            
            _ => {
                // For unimplemented expression types, return fresh type variable
                let var = self.type_env.fresh_var();
                Ok(InferType::Variable(var))
            }
        }
    }
    
    /// Infer the type of a block
    pub fn infer_block(&mut self, block: &Block) -> Result<InferType, SemanticError> {
        self.type_env.enter_scope();
        
        let mut block_type = InferType::Concrete(Type::Unit);
        
        for stmt in &block.statements {
            match stmt {
                Statement::Expression(expr) => {
                    block_type = self.infer_expression(expr)?;
                }
                Statement::Let(name, type_annotation, init) => {
                    let var_type = if let Some(init_expr) = init {
                        let inferred = self.infer_expression(init_expr)?;
                        
                        if let Some(annotation) = type_annotation {
                            let annotated = InferType::Concrete(annotation.clone());
                            self.add_constraint(
                                inferred.clone(),
                                annotated.clone(),
                                Span::single(crate::position::Position::start())
                            );
                            annotated
                        } else {
                            inferred
                        }
                    } else if let Some(annotation) = type_annotation {
                        InferType::Concrete(annotation.clone())
                    } else {
                        return Err(SemanticError {
                            span: Span::single(crate::position::Position::start()),
                            kind: SemanticErrorKind::CannotInferType,
                        });
                    };
                    
                    self.type_env.bind(name.clone(), var_type);
                    block_type = InferType::Concrete(Type::Unit);
                }
                Statement::Return(expr) => {
                    if let Some(e) = expr {
                        block_type = self.infer_expression(e)?;
                    } else {
                        block_type = InferType::Concrete(Type::Unit);
                    }
                }
                _ => {
                    // Other statements return unit
                    block_type = InferType::Concrete(Type::Unit);
                }
            }
        }
        
        self.type_env.exit_scope();
        Ok(block_type)
    }
    
    /// Infer type for binary operations
    fn infer_binary_op(&mut self, left: &InferType, op: &BinaryOp, right: &InferType, span: Span) -> Result<InferType, SemanticError> {
        match op {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                // Arithmetic operations require numeric types
                let numeric_constraint = |t: &InferType| -> bool {
                    matches!(t, InferType::Concrete(Type::Int) | InferType::Concrete(Type::Float))
                };
                
                // Both operands must be same numeric type
                self.add_constraint(left.clone(), right.clone(), span.clone());
                
                // Result type is same as operands
                Ok(left.clone())
            }
            
            BinaryOp::Equal | BinaryOp::NotEqual => {
                // Equality works on any types that are the same
                self.add_constraint(left.clone(), right.clone(), span);
                Ok(InferType::Concrete(Type::Bool))
            }
            
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                // Comparison requires same comparable types
                self.add_constraint(left.clone(), right.clone(), span);
                Ok(InferType::Concrete(Type::Bool))
            }
            
            BinaryOp::And | BinaryOp::Or => {
                // Logical operations require boolean operands
                self.add_constraint(left.clone(), InferType::Concrete(Type::Bool), span.clone());
                self.add_constraint(right.clone(), InferType::Concrete(Type::Bool), span);
                Ok(InferType::Concrete(Type::Bool))
            }
            
            _ => {
                // Other binary operations not yet implemented
                let result_var = self.type_env.fresh_var();
                Ok(InferType::Variable(result_var))
            }
        }
    }
    
    /// Infer type for unary operations
    fn infer_unary_op(&mut self, op: &UnaryOp, operand: &InferType, span: Span) -> Result<InferType, SemanticError> {
        match op {
            UnaryOp::Plus | UnaryOp::Minus => {
                // Unary arithmetic requires numeric type
                Ok(operand.clone())
            }
            
            UnaryOp::Not => {
                // Logical not requires boolean
                self.add_constraint(operand.clone(), InferType::Concrete(Type::Bool), span);
                Ok(InferType::Concrete(Type::Bool))
            }
            
            _ => {
                // Other unary operations not yet implemented
                let result_var = self.type_env.fresh_var();
                Ok(InferType::Variable(result_var))
            }
        }
    }
    
    /// Legacy method for backward compatibility
    pub fn infer_type(&mut self, expr: &Expression) -> Result<Type, SemanticError> {
        let inferred = self.infer_expression(expr)?;
        let subst = self.solve_constraints()?;
        let resolved = subst.apply(&inferred);
        
        resolved.to_concrete().ok_or_else(|| SemanticError {
            span: Span::single(crate::position::Position::start()),
            kind: SemanticErrorKind::CannotInferType,
        })
    }
    
    /// Check that an expression has the expected type
    pub fn check_type(&mut self, expr: &Expression, expected: &Type) -> Result<(), SemanticError> {
        let actual = self.infer_type(expr)?;
        if self.types_compatible(&actual, expected) {
            Ok(())
        } else {
            Err(SemanticError {
                span: Span::single(crate::position::Position::start()),
                kind: SemanticErrorKind::TypeMismatch {
                    expected: format!("{}", expected),
                    found: format!("{}", actual),
                },
            })
        }
    }
    
    /// Unify two concrete types using the unification algorithm
    pub fn unify_types(&mut self, t1: &Type, t2: &Type) -> Result<Type, SemanticError> {
        let infer_t1 = InferType::Concrete(t1.clone());
        let infer_t2 = InferType::Concrete(t2.clone());
        let span = Span::single(crate::position::Position::start());
        
        let subst = self.unify(&infer_t1, &infer_t2, span)?;
        let unified = subst.apply(&infer_t1);
        
        unified.to_concrete().ok_or_else(|| SemanticError {
            span: Span::single(crate::position::Position::start()),
            kind: SemanticErrorKind::TypeMismatch {
                expected: format!("{}", t1),
                found: format!("{}", t2),
            },
        })
    }
    
    /// Type check a function
    pub fn check_function(&mut self, func: &Function) -> Result<TypedFunction, SemanticError> {
        // Add parameters to type environment
        let mut typed_params = Vec::new();
        for param in &func.parameters {
            self.type_env.bind(param.name.clone(), InferType::Concrete(param.type_.clone()));
            typed_params.push(TypedParameter {
                name: param.name.clone(),
                type_: param.type_.clone(),
                is_mutable: param.is_mutable,
            });
        }
        
        // Type check function body
        let typed_body = self.check_block(&func.body)?;
        
        // Determine return type
        let return_type = func.return_type.clone().unwrap_or(Type::Unit);
        
        Ok(TypedFunction {
            name: func.name.clone(),
            parameters: typed_params,
            return_type,
            body: typed_body,
            is_async: func.is_async,
            visibility: func.visibility.clone(),
        })
    }
    
    /// Type check an extern function
    pub fn check_extern_function(&mut self, extern_func: &ExternFunction) -> Result<TypedExternFunction, SemanticError> {
        // Type check parameters
        let mut typed_params = Vec::new();
        for param in &extern_func.parameters {
            typed_params.push(TypedParameter {
                name: param.name.clone(),
                type_: param.type_.clone(),
                is_mutable: param.is_mutable,
            });
        }
        
        // Extern functions are already validated by the parser
        Ok(TypedExternFunction {
            name: extern_func.name.clone(),
            parameters: typed_params,
            return_type: extern_func.return_type.clone(),
            library: extern_func.library.clone(),
            is_variadic: extern_func.is_variadic,
            visibility: extern_func.visibility.clone(),
        })
    }
    
    /// Type check a struct
    pub fn check_struct(&mut self, struct_def: &Struct) -> Result<TypedStruct, SemanticError> {
        let mut typed_fields = Vec::new();
        
        for field in &struct_def.fields {
            typed_fields.push(TypedField {
                name: field.name.clone(),
                type_: field.type_.clone(),
                visibility: field.visibility.clone(),
                is_mutable: field.is_mutable,
            });
        }
        
        Ok(TypedStruct {
            name: struct_def.name.clone(),
            fields: typed_fields,
            visibility: struct_def.visibility.clone(),
        })
    }
    
    /// Type check a class
    pub fn check_class(&mut self, class_def: &Class) -> Result<TypedClass, SemanticError> {
        let mut typed_fields = Vec::new();
        let mut typed_methods = Vec::new();
        
        for field in &class_def.fields {
            typed_fields.push(TypedField {
                name: field.name.clone(),
                type_: field.type_.clone(),
                visibility: field.visibility.clone(),
                is_mutable: field.is_mutable,
            });
        }
        
        for method in &class_def.methods {
            let typed_method = self.check_method(method)?;
            typed_methods.push(typed_method);
        }
        
        Ok(TypedClass {
            name: class_def.name.clone(),
            fields: typed_fields,
            methods: typed_methods,
            visibility: class_def.visibility.clone(),
        })
    }
    
    /// Type check a method
    pub fn check_method(&mut self, method: &Method) -> Result<TypedMethod, SemanticError> {
        let mut typed_params = Vec::new();
        for param in &method.parameters {
            typed_params.push(TypedParameter {
                name: param.name.clone(),
                type_: param.type_.clone(),
                is_mutable: param.is_mutable,
            });
        }
        
        let typed_body = self.check_block(&method.body)?;
        let return_type = method.return_type.clone().unwrap_or(Type::Unit);
        
        Ok(TypedMethod {
            name: method.name.clone(),
            parameters: typed_params,
            return_type,
            body: typed_body,
            visibility: method.visibility.clone(),
            is_static: method.is_static,
        })
    }
    

    
    /// Check if a type is numeric
    fn is_numeric_type(&self, type_: &Type) -> bool {
        matches!(type_, Type::Int | Type::Float)
    }
    
    /// Get the type of a literal
    fn literal_type(&self, lit: &Literal) -> Type {
        match lit {
            Literal::Integer(_) => Type::Int,
            Literal::Float(_) => Type::Float,
            Literal::String(_) => Type::String,
            Literal::Boolean(_) => Type::Bool,
            Literal::Character(_) => Type::Char,
            Literal::Null => Type::Nullable(Box::new(Type::Unit)),
        }
    }
    
    /// Type check a constant
    pub fn check_const(&mut self, const_def: &Const) -> Result<TypedConst, SemanticError> {
        // First infer the type of the value expression
        let inferred_type = self.infer_type(&const_def.value)?;
        
        // Check that it matches the declared type
        if !self.types_compatible(&inferred_type, &const_def.type_) {
            return Err(SemanticError {
                span: Span::single(crate::position::Position::start()),
                kind: SemanticErrorKind::TypeMismatch {
                    expected: format!("{}", const_def.type_),
                    found: format!("{}", inferred_type),
                },
            });
        }
        
        // Create typed expression for the value
        let typed_value = TypedExpression {
            kind: match &const_def.value {
                Expression::Literal(lit) => TypedExpressionKind::Literal(lit.clone()),
                _ => return Err(SemanticError {
                    span: Span::single(crate::position::Position::start()),
                    kind: SemanticErrorKind::UnsupportedFeature {
                        feature: "Non-literal constant expressions".to_string(),
                    },
                }),
            },
            type_: inferred_type,
        };
        
        Ok(TypedConst {
            name: const_def.name.clone(),
            type_: const_def.type_.clone(),
            value: typed_value,
            visibility: const_def.visibility.clone(),
        })
    }
    
    /// Simple type check for expressions (without full inference)
    pub fn check_expression(&mut self, expr: &Expression) -> Result<TypedExpression, SemanticError> {
        match expr {
            Expression::Literal(lit) => {
                let type_ = self.literal_type(lit);
                Ok(TypedExpression {
                    kind: TypedExpressionKind::Literal(lit.clone()),
                    type_,
                })
            }
            Expression::Identifier(name) => {
                let type_ = self.type_env.lookup(name)
                    .ok_or_else(|| SemanticError {
                        span: Span::single(crate::position::Position::start()),
                        kind: SemanticErrorKind::UndefinedVariable { name: name.clone() },
                    })?
                    .to_concrete()
                    .ok_or_else(|| SemanticError {
                        span: Span::single(crate::position::Position::start()),
                        kind: SemanticErrorKind::CannotInferType,
                    })?;
                
                Ok(TypedExpression {
                    kind: TypedExpressionKind::Identifier(name.clone()),
                    type_,
                })
            }
            Expression::Binary(left, op, right) => {
                let typed_left = self.check_expression(left)?;
                let typed_right = self.check_expression(right)?;
                let result_type = self.binary_op_result_type(&typed_left.type_, op, &typed_right.type_)?;
                
                Ok(TypedExpression {
                    kind: TypedExpressionKind::Binary(
                        Box::new(typed_left),
                        op.clone(),
                        Box::new(typed_right)
                    ),
                    type_: result_type,
                })
            }
            _ => Err(SemanticError {
                span: Span::single(crate::position::Position::start()),
                kind: SemanticErrorKind::UnsupportedFeature {
                    feature: format!("Expression: {:?}", expr),
                },
            }),
        }
    }
    
    /// Get the result type of a binary operation
    fn binary_op_result_type(&self, left: &Type, op: &BinaryOp, right: &Type) -> Result<Type, SemanticError> {
        match op {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                if self.types_compatible(left, right) && self.is_numeric_type(left) {
                    Ok(left.clone())
                } else {
                    Err(SemanticError {
                        span: Span::single(crate::position::Position::start()),
                        kind: SemanticErrorKind::TypeMismatch {
                            expected: "numeric types".to_string(),
                            found: format!("{:?} and {:?}", left, right),
                        },
                    })
                }
            }
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::Greater => {
                if self.types_compatible(left, right) {
                    Ok(Type::Bool)
                } else {
                    Err(SemanticError {
                        span: Span::single(crate::position::Position::start()),
                        kind: SemanticErrorKind::TypeMismatch {
                            expected: "compatible types".to_string(),
                            found: format!("{:?} and {:?}", left, right),
                        },
                    })
                }
            }
            _ => Err(SemanticError {
                span: Span::single(crate::position::Position::start()),
                kind: SemanticErrorKind::UnsupportedFeature {
                    feature: format!("Binary operator: {:?}", op),
                },
            }),
        }
    }
    

    
    /// Type check a block
    pub fn check_block(&mut self, block: &Block) -> Result<TypedBlock, SemanticError> {
        let mut typed_statements = Vec::new();
        let mut block_type = Type::Unit;
        
        for stmt in &block.statements {
            let typed_stmt = self.check_statement(stmt)?;
            
            // If this is an expression statement, it might determine the block type
            if let TypedStatement::Expression(ref expr) = typed_stmt {
                block_type = expr.type_.clone();
            }
            
            typed_statements.push(typed_stmt);
        }
        
        Ok(TypedBlock {
            statements: typed_statements,
            type_: block_type,
        })
    }
    
    /// Type check a statement
    pub fn check_statement(&mut self, stmt: &Statement) -> Result<TypedStatement, SemanticError> {
        match stmt {
            Statement::Expression(expr) => {
                let typed_expr = self.check_expression(expr)?;
                Ok(TypedStatement::Expression(typed_expr))
            }
            Statement::Let(name, type_annotation, init) => {
                let var_type = if let Some(init_expr) = init {
                    let typed_init = self.check_expression(init_expr)?;
                    
                    if let Some(annotation) = type_annotation {
                        if !self.types_compatible(&typed_init.type_, annotation) {
                            return Err(SemanticError {
                                span: Span::single(crate::position::Position::start()),
                                kind: SemanticErrorKind::TypeMismatch {
                                    expected: format!("{}", annotation),
                                    found: format!("{}", typed_init.type_),
                                },
                            });
                        }
                        annotation.clone()
                    } else {
                        typed_init.type_.clone()
                    }
                } else if let Some(annotation) = type_annotation {
                    annotation.clone()
                } else {
                    return Err(SemanticError {
                        span: Span::single(crate::position::Position::start()),
                        kind: SemanticErrorKind::CannotInferType,
                    });
                };
                
                self.type_env.bind(name.clone(), InferType::Concrete(var_type.clone()));
                
                let typed_init = if let Some(init_expr) = init {
                    Some(self.check_expression(init_expr)?)
                } else {
                    None
                };
                
                Ok(TypedStatement::Let(name.clone(), var_type, typed_init))
            }
            Statement::Return(expr) => {
                let typed_expr = if let Some(e) = expr {
                    Some(self.check_expression(e)?)
                } else {
                    None
                };
                Ok(TypedStatement::Return(typed_expr))
            }
            _ => {
                // Placeholder for other statement types
                Err(SemanticError {
                    span: Span::single(crate::position::Position::start()),
                    kind: SemanticErrorKind::InvalidOperation {
                        message: "Statement type not yet implemented".to_string(),
                    },
                })
            }
        }
    }
    

    
    /// Check if two types are compatible
    fn types_compatible(&self, t1: &Type, t2: &Type) -> bool {
        // Simplified type compatibility check
        t1 == t2
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    fn create_span() -> Span {
        Span::single(Position::start())
    }

    #[test]
    fn test_type_variable_creation() {
        let mut env = TypeEnvironment::new();
        let var1 = env.fresh_var();
        let var2 = env.fresh_var();
        assert_ne!(var1, var2);
    }

    #[test]
    fn test_literal_types() {
        let checker = TypeChecker::new();
        
        assert_eq!(checker.literal_type(&Literal::Integer(42)), Type::Int);
        assert_eq!(checker.literal_type(&Literal::Float(3.14)), Type::Float);
        assert_eq!(checker.literal_type(&Literal::Boolean(true)), Type::Bool);
        assert_eq!(checker.literal_type(&Literal::String("hello".to_string())), Type::String);
        assert_eq!(checker.literal_type(&Literal::Character('a')), Type::Char);
    }

    #[test]
    fn test_type_compatibility() {
        let checker = TypeChecker::new();
        
        assert!(checker.types_compatible(&Type::Int, &Type::Int));
        assert!(!checker.types_compatible(&Type::Int, &Type::String));
        assert!(checker.types_compatible(&Type::Bool, &Type::Bool));
    }

    #[test]
    fn test_numeric_types() {
        let checker = TypeChecker::new();
        
        assert!(checker.is_numeric_type(&Type::Int));
        assert!(checker.is_numeric_type(&Type::Float));
        assert!(!checker.is_numeric_type(&Type::String));
        assert!(!checker.is_numeric_type(&Type::Bool));
    }
}