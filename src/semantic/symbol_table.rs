//! Symbol table for name resolution and scope management
//! 
//! Provides hierarchical symbol tables for managing variable, function, and type bindings.

use crate::error::{SemanticError, SemanticErrorKind};
use crate::parser::ast::{Type, Function, Struct, Class, Const, ExternFunction};
use crate::position::Span;
use std::collections::HashMap;

/// Symbol information stored in the symbol table
#[derive(Debug, Clone)]
pub enum Symbol {
    Variable { 
        type_: Type, 
        is_mutable: bool,
        scope_level: usize,
        is_initialized: bool,
    },
    Function(Function),
    Struct(Struct),
    Class(Class),
    Const(Const),
    ExternFunction(ExternFunction),
    Parameter {
        type_: Type,
        is_mutable: bool,
        index: usize, // Parameter index for code generation
    },
}

/// Scope type for better error reporting
#[derive(Debug, Clone, PartialEq)]
pub enum ScopeType {
    Global,
    Function,
    Block,
    Loop,
    Match,
}

/// Enhanced scope information
#[derive(Debug)]
struct Scope {
    symbols: HashMap<String, Symbol>,
    scope_type: ScopeType,
    parent_function: Option<String>, // Name of containing function
}

/// Hierarchical symbol table with scope management
#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
    current_function: Option<String>,
}

impl SymbolTable {
    /// Create a new symbol table with global scope
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope {
                symbols: HashMap::new(),
                scope_type: ScopeType::Global,
                parent_function: None,
            }],
            current_function: None,
        }
    }
    
    /// Enter a new scope with specified type
    pub fn enter_scope(&mut self, scope_type: ScopeType) {
        let parent_function = if scope_type == ScopeType::Function {
            None // Will be set when function is defined
        } else {
            self.current_function.clone()
        };
        
        self.scopes.push(Scope {
            symbols: HashMap::new(),
            scope_type,
            parent_function,
        });
    }
    
    /// Enter a function scope
    pub fn enter_function_scope(&mut self, function_name: String) {
        self.current_function = Some(function_name.clone());
        self.scopes.push(Scope {
            symbols: HashMap::new(),
            scope_type: ScopeType::Function,
            parent_function: Some(function_name),
        });
    }
    
    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            let exited_scope = self.scopes.pop().unwrap();
            
            // If exiting a function scope, update current function
            if exited_scope.scope_type == ScopeType::Function {
                self.current_function = self.scopes.last()
                    .and_then(|scope| scope.parent_function.clone());
            }
        }
    }
    
    /// Get the current scope type
    pub fn current_scope_type(&self) -> &ScopeType {
        &self.scopes.last().unwrap().scope_type
    }
    
    /// Check if we're currently in a function
    pub fn in_function(&self) -> bool {
        self.current_function.is_some()
    }
    
    /// Check if we're currently in a loop
    pub fn in_loop(&self) -> bool {
        self.scopes.iter().any(|scope| scope.scope_type == ScopeType::Loop)
    }
    
    /// Get the current function name
    pub fn current_function_name(&self) -> Option<&String> {
        self.current_function.as_ref()
    }
    
    /// Define a variable in the current scope
    pub fn define_variable(&mut self, name: String, type_: Type, is_mutable: bool) -> Result<(), SemanticError> {
        let scope_level = self.scopes.len() - 1;
        let current_scope = self.scopes.last_mut().unwrap();
        
        if current_scope.symbols.contains_key(&name) {
            return Err(SemanticError {
                span: Span::single(crate::position::Position::start()), // Placeholder
                kind: SemanticErrorKind::DuplicateDefinition { name },
            });
        }
        
        current_scope.symbols.insert(name, Symbol::Variable { 
            type_, 
            is_mutable,
            scope_level,
            is_initialized: false,
        });
        Ok(())
    }
    
    /// Define a parameter in the current scope
    pub fn define_parameter(&mut self, name: String, type_: Type, is_mutable: bool, index: usize) -> Result<(), SemanticError> {
        let current_scope = self.scopes.last_mut().unwrap();
        
        if current_scope.symbols.contains_key(&name) {
            return Err(SemanticError {
                span: Span::single(crate::position::Position::start()),
                kind: SemanticErrorKind::DuplicateDefinition { name },
            });
        }
        
        current_scope.symbols.insert(name, Symbol::Parameter { 
            type_, 
            is_mutable,
            index,
        });
        Ok(())
    }
    
    /// Mark a variable as initialized
    pub fn mark_initialized(&mut self, name: &str) -> Result<(), SemanticError> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.symbols.get_mut(name) {
                match symbol {
                    Symbol::Variable { is_initialized, .. } => {
                        *is_initialized = true;
                        return Ok(());
                    }
                    _ => return Ok(()), // Parameters and other symbols are always "initialized"
                }
            }
        }
        
        Err(SemanticError {
            span: Span::single(crate::position::Position::start()),
            kind: SemanticErrorKind::UndefinedVariable { name: name.to_string() },
        })
    }
    
    /// Check if a variable is initialized
    pub fn is_initialized(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.symbols.get(name) {
                match symbol {
                    Symbol::Variable { is_initialized, .. } => return *is_initialized,
                    _ => return true, // Parameters and other symbols are always "initialized"
                }
            }
        }
        false
    }
    
    /// Define a function in the current scope
    pub fn define_function(&mut self, name: String, func: Function) -> Result<(), SemanticError> {
        let current_scope = self.scopes.last_mut().unwrap();
        
        if current_scope.symbols.contains_key(&name) {
            return Err(SemanticError {
                span: Span::single(crate::position::Position::start()), // Placeholder
                kind: SemanticErrorKind::DuplicateDefinition { name },
            });
        }
        
        current_scope.symbols.insert(name, Symbol::Function(func));
        Ok(())
    }
    
    /// Define an extern function in the current scope
    pub fn define_extern_function(&mut self, name: String, extern_func: ExternFunction) -> Result<(), SemanticError> {
        let current_scope = self.scopes.last_mut().unwrap();
        
        if current_scope.symbols.contains_key(&name) {
            return Err(SemanticError {
                span: Span::single(crate::position::Position::start()), // Placeholder
                kind: SemanticErrorKind::DuplicateDefinition { name },
            });
        }
        
        current_scope.symbols.insert(name, Symbol::ExternFunction(extern_func));
        Ok(())
    }
    
    /// Define a struct in the current scope
    pub fn define_struct(&mut self, name: String, struct_def: Struct) -> Result<(), SemanticError> {
        let current_scope = self.scopes.last_mut().unwrap();
        
        if current_scope.symbols.contains_key(&name) {
            return Err(SemanticError {
                span: Span::single(crate::position::Position::start()), // Placeholder
                kind: SemanticErrorKind::DuplicateDefinition { name },
            });
        }
        
        current_scope.symbols.insert(name, Symbol::Struct(struct_def));
        Ok(())
    }
    
    /// Define a class in the current scope
    pub fn define_class(&mut self, name: String, class_def: Class) -> Result<(), SemanticError> {
        let current_scope = self.scopes.last_mut().unwrap();
        
        if current_scope.symbols.contains_key(&name) {
            return Err(SemanticError {
                span: Span::single(crate::position::Position::start()), // Placeholder
                kind: SemanticErrorKind::DuplicateDefinition { name },
            });
        }
        
        current_scope.symbols.insert(name, Symbol::Class(class_def));
        Ok(())
    }
    
    /// Define a constant in the current scope
    pub fn define_const(&mut self, name: String, const_def: Const) -> Result<(), SemanticError> {
        let current_scope = self.scopes.last_mut().unwrap();
        
        if current_scope.symbols.contains_key(&name) {
            return Err(SemanticError {
                span: Span::single(crate::position::Position::start()), // Placeholder
                kind: SemanticErrorKind::DuplicateDefinition { name },
            });
        }
        
        current_scope.symbols.insert(name, Symbol::Const(const_def));
        Ok(())
    }
    
    /// Look up a symbol by name, searching from innermost to outermost scope
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.symbols.get(name) {
                return Some(symbol);
            }
        }
        None
    }
    
    /// Look up a symbol and return its scope level
    pub fn lookup_with_scope(&self, name: &str) -> Option<(&Symbol, usize)> {
        for (level, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(symbol) = scope.symbols.get(name) {
                return Some((symbol, level));
            }
        }
        None
    }
    
    /// Check if a symbol exists in the current scope only
    pub fn exists_in_current_scope(&self, name: &str) -> bool {
        self.scopes.last().unwrap().symbols.contains_key(name)
    }
    
    /// Get the current scope depth
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }
    
    /// Get all symbols in the current scope
    pub fn current_scope_symbols(&self) -> &HashMap<String, Symbol> {
        &self.scopes.last().unwrap().symbols
    }
    
    /// Resolve a name and check for common errors
    pub fn resolve_name(&self, name: &str) -> Result<&Symbol, SemanticError> {
        match self.lookup(name) {
            Some(symbol) => {
                // Check if variable is initialized before use
                match symbol {
                    Symbol::Variable { is_initialized, .. } if !is_initialized => {
                        Err(SemanticError {
                            span: Span::single(crate::position::Position::start()),
                            kind: SemanticErrorKind::InvalidOperation {
                                message: format!("Use of uninitialized variable '{}'", name),
                            },
                        })
                    }
                    _ => Ok(symbol),
                }
            }
            None => Err(SemanticError {
                span: Span::single(crate::position::Position::start()),
                kind: SemanticErrorKind::UndefinedVariable { name: name.to_string() },
            }),
        }
    }
    
    /// Check if a name can be assigned to (is mutable)
    pub fn can_assign(&self, name: &str) -> Result<bool, SemanticError> {
        match self.lookup(name) {
            Some(Symbol::Variable { is_mutable, .. }) => Ok(*is_mutable),
            Some(Symbol::Parameter { is_mutable, .. }) => Ok(*is_mutable),
            Some(_) => Ok(false), // Functions, structs, etc. are not assignable
            None => Err(SemanticError {
                span: Span::single(crate::position::Position::start()),
                kind: SemanticErrorKind::UndefinedVariable { name: name.to_string() },
            }),
        }
    }
    
    /// Get the type of a symbol
    pub fn get_type(&self, name: &str) -> Result<&Type, SemanticError> {
        match self.resolve_name(name)? {
            Symbol::Variable { type_, .. } => Ok(type_),
            Symbol::Parameter { type_, .. } => Ok(type_),
            Symbol::Function(func) => {
                // Return function type
                let _param_types: Vec<Type> = func.parameters.iter().map(|p| p.type_.clone()).collect();
                let _return_type = func.return_type.clone().unwrap_or(Type::Unit);
                // Note: This is a simplified approach. In a real implementation,
                // we'd want to cache function types or handle this differently.
                Err(SemanticError {
                    span: Span::single(crate::position::Position::start()),
                    kind: SemanticErrorKind::InvalidOperation {
                        message: "Function type lookup not fully implemented".to_string(),
                    },
                })
            }
            _ => Err(SemanticError {
                span: Span::single(crate::position::Position::start()),
                kind: SemanticErrorKind::InvalidOperation {
                    message: format!("Cannot get type of '{}'", name),
                },
            }),
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Visibility, Parameter};

    fn create_test_function(name: &str) -> Function {
        Function {
            name: name.to_string(),
            parameters: vec![],
            return_type: Some(Type::Unit),
            body: crate::parser::ast::Block { statements: vec![] },
            is_async: false,
            visibility: Visibility::Private,
        }
    }

    fn create_test_struct(name: &str) -> Struct {
        Struct {
            name: name.to_string(),
            fields: vec![],
            visibility: Visibility::Private,
        }
    }

    #[test]
    fn test_basic_symbol_definition() {
        let mut table = SymbolTable::new();
        
        // Define a variable
        assert!(table.define_variable("x".to_string(), Type::Int, false).is_ok());
        
        // Look it up
        let symbol = table.lookup("x").unwrap();
        match symbol {
            Symbol::Variable { type_, is_mutable, .. } => {
                assert_eq!(*type_, Type::Int);
                assert!(!is_mutable);
            }
            _ => panic!("Expected variable symbol"),
        }
    }

    #[test]
    fn test_duplicate_definition_error() {
        let mut table = SymbolTable::new();
        
        // Define a variable
        assert!(table.define_variable("x".to_string(), Type::Int, false).is_ok());
        
        // Try to define it again - should fail
        assert!(table.define_variable("x".to_string(), Type::String, false).is_err());
    }

    #[test]
    fn test_scope_management() {
        let mut table = SymbolTable::new();
        
        // Define in global scope
        assert!(table.define_variable("global".to_string(), Type::Int, false).is_ok());
        assert_eq!(table.scope_depth(), 1);
        
        // Enter new scope
        table.enter_scope(ScopeType::Block);
        assert_eq!(table.scope_depth(), 2);
        
        // Define in inner scope
        assert!(table.define_variable("local".to_string(), Type::String, false).is_ok());
        
        // Both should be visible
        assert!(table.lookup("global").is_some());
        assert!(table.lookup("local").is_some());
        
        // Exit scope
        table.exit_scope();
        assert_eq!(table.scope_depth(), 1);
        
        // Only global should be visible
        assert!(table.lookup("global").is_some());
        assert!(table.lookup("local").is_none());
    }

    #[test]
    fn test_shadowing() {
        let mut table = SymbolTable::new();
        
        // Define in global scope
        assert!(table.define_variable("x".to_string(), Type::Int, false).is_ok());
        
        // Enter new scope and shadow
        table.enter_scope(ScopeType::Block);
        assert!(table.define_variable("x".to_string(), Type::String, true).is_ok());
        
        // Should see the shadowed version
        let symbol = table.lookup("x").unwrap();
        match symbol {
            Symbol::Variable { type_, is_mutable, .. } => {
                assert_eq!(*type_, Type::String);
                assert!(*is_mutable);
            }
            _ => panic!("Expected variable symbol"),
        }
        
        // Exit scope
        table.exit_scope();
        
        // Should see original version
        let symbol = table.lookup("x").unwrap();
        match symbol {
            Symbol::Variable { type_, is_mutable, .. } => {
                assert_eq!(*type_, Type::Int);
                assert!(!is_mutable);
            }
            _ => panic!("Expected variable symbol"),
        }
    }

    #[test]
    fn test_function_scope_tracking() {
        let mut table = SymbolTable::new();
        
        assert!(!table.in_function());
        assert!(table.current_function_name().is_none());
        
        // Enter function scope
        table.enter_function_scope("test_func".to_string());
        
        assert!(table.in_function());
        assert_eq!(table.current_function_name(), Some(&"test_func".to_string()));
        assert_eq!(*table.current_scope_type(), ScopeType::Function);
        
        // Enter nested block scope
        table.enter_scope(ScopeType::Block);
        assert!(table.in_function());
        assert_eq!(table.current_function_name(), Some(&"test_func".to_string()));
        
        // Exit block scope
        table.exit_scope();
        assert!(table.in_function());
        
        // Exit function scope
        table.exit_scope();
        assert!(!table.in_function());
        assert!(table.current_function_name().is_none());
    }

    #[test]
    fn test_loop_scope_tracking() {
        let mut table = SymbolTable::new();
        
        assert!(!table.in_loop());
        
        // Enter loop scope
        table.enter_scope(ScopeType::Loop);
        assert!(table.in_loop());
        
        // Enter nested block scope
        table.enter_scope(ScopeType::Block);
        assert!(table.in_loop());
        
        // Exit block scope
        table.exit_scope();
        assert!(table.in_loop());
        
        // Exit loop scope
        table.exit_scope();
        assert!(!table.in_loop());
    }

    #[test]
    fn test_parameter_definition() {
        let mut table = SymbolTable::new();
        
        table.enter_function_scope("test".to_string());
        
        // Define parameters
        assert!(table.define_parameter("a".to_string(), Type::Int, false, 0).is_ok());
        assert!(table.define_parameter("b".to_string(), Type::String, true, 1).is_ok());
        
        // Look up parameters
        let param_a = table.lookup("a").unwrap();
        match param_a {
            Symbol::Parameter { type_, is_mutable, index } => {
                assert_eq!(*type_, Type::Int);
                assert!(!is_mutable);
                assert_eq!(*index, 0);
            }
            _ => panic!("Expected parameter symbol"),
        }
        
        let param_b = table.lookup("b").unwrap();
        match param_b {
            Symbol::Parameter { type_, is_mutable, index } => {
                assert_eq!(*type_, Type::String);
                assert!(*is_mutable);
                assert_eq!(*index, 1);
            }
            _ => panic!("Expected parameter symbol"),
        }
    }

    #[test]
    fn test_initialization_tracking() {
        let mut table = SymbolTable::new();
        
        // Define uninitialized variable
        assert!(table.define_variable("x".to_string(), Type::Int, true).is_ok());
        
        // Should not be initialized
        assert!(!table.is_initialized("x"));
        
        // Resolving should fail due to uninitialized use
        assert!(table.resolve_name("x").is_err());
        
        // Mark as initialized
        assert!(table.mark_initialized("x").is_ok());
        assert!(table.is_initialized("x"));
        
        // Now resolving should work
        assert!(table.resolve_name("x").is_ok());
    }

    #[test]
    fn test_mutability_checking() {
        let mut table = SymbolTable::new();
        
        // Define immutable variable
        assert!(table.define_variable("immutable".to_string(), Type::Int, false).is_ok());
        table.mark_initialized("immutable").unwrap();
        
        // Define mutable variable
        assert!(table.define_variable("mutable".to_string(), Type::Int, true).is_ok());
        table.mark_initialized("mutable").unwrap();
        
        // Check mutability
        assert!(!table.can_assign("immutable").unwrap());
        assert!(table.can_assign("mutable").unwrap());
        
        // Undefined variable should error
        assert!(table.can_assign("undefined").is_err());
    }

    #[test]
    fn test_function_definition() {
        let mut table = SymbolTable::new();
        
        let func = create_test_function("test_func");
        assert!(table.define_function("test_func".to_string(), func).is_ok());
        
        let symbol = table.lookup("test_func").unwrap();
        match symbol {
            Symbol::Function(f) => {
                assert_eq!(f.name, "test_func");
            }
            _ => panic!("Expected function symbol"),
        }
    }

    #[test]
    fn test_struct_definition() {
        let mut table = SymbolTable::new();
        
        let struct_def = create_test_struct("TestStruct");
        assert!(table.define_struct("TestStruct".to_string(), struct_def).is_ok());
        
        let symbol = table.lookup("TestStruct").unwrap();
        match symbol {
            Symbol::Struct(s) => {
                assert_eq!(s.name, "TestStruct");
            }
            _ => panic!("Expected struct symbol"),
        }
    }

    #[test]
    fn test_scope_level_tracking() {
        let mut table = SymbolTable::new();
        
        // Global scope (level 0)
        assert!(table.define_variable("global".to_string(), Type::Int, false).is_ok());
        
        // Level 1
        table.enter_scope(ScopeType::Block);
        assert!(table.define_variable("level1".to_string(), Type::String, false).is_ok());
        
        // Level 2
        table.enter_scope(ScopeType::Block);
        assert!(table.define_variable("level2".to_string(), Type::Bool, false).is_ok());
        
        // Check scope levels
        let (_, level) = table.lookup_with_scope("global").unwrap();
        assert_eq!(level, 0);
        
        let (_, level) = table.lookup_with_scope("level1").unwrap();
        assert_eq!(level, 1);
        
        let (_, level) = table.lookup_with_scope("level2").unwrap();
        assert_eq!(level, 2);
    }

    #[test]
    fn test_current_scope_symbols() {
        let mut table = SymbolTable::new();
        
        // Define in global scope
        assert!(table.define_variable("global".to_string(), Type::Int, false).is_ok());
        
        // Enter new scope
        table.enter_scope(ScopeType::Block);
        assert!(table.define_variable("local1".to_string(), Type::String, false).is_ok());
        assert!(table.define_variable("local2".to_string(), Type::Bool, false).is_ok());
        
        // Current scope should only have local symbols
        let current_symbols = table.current_scope_symbols();
        assert_eq!(current_symbols.len(), 2);
        assert!(current_symbols.contains_key("local1"));
        assert!(current_symbols.contains_key("local2"));
        assert!(!current_symbols.contains_key("global"));
    }

    #[test]
    fn test_exists_in_current_scope() {
        let mut table = SymbolTable::new();
        
        // Define in global scope
        assert!(table.define_variable("global".to_string(), Type::Int, false).is_ok());
        assert!(table.exists_in_current_scope("global"));
        
        // Enter new scope
        table.enter_scope(ScopeType::Block);
        assert!(!table.exists_in_current_scope("global")); // Not in current scope
        
        // Define in current scope
        assert!(table.define_variable("local".to_string(), Type::String, false).is_ok());
        assert!(table.exists_in_current_scope("local"));
    }
}