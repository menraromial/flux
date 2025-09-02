//! Foreign Function Interface (FFI) support for Flux
//! 
//! This module provides support for calling C functions from Flux code,
//! including type marshaling, safety checks, and error handling.

pub mod c_types;
pub mod marshaling;
pub mod safety;
pub mod error;
pub mod call;

use std::collections::HashMap;

/// Represents an extern "C" function declaration
#[derive(Debug, Clone)]
pub struct ExternFunction {
    pub name: String,
    pub parameters: Vec<CParameter>,
    pub return_type: CType,
    pub library: Option<String>,
    pub is_variadic: bool,
}

/// Parameter for C function
#[derive(Debug, Clone)]
pub struct CParameter {
    pub name: String,
    pub c_type: CType,
}

/// C type representation
#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    // Primitive types
    Void,
    Char,
    UChar,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    LongLong,
    ULongLong,
    Float,
    Double,
    
    // Pointer types
    Pointer(Box<CType>),
    
    // Array types
    Array(Box<CType>, Option<usize>),
    
    // Function pointer
    Function(Vec<CType>, Box<CType>),
    
    // Struct/Union (opaque for now)
    Struct(String),
    Union(String),
}

/// FFI registry for managing extern function declarations
pub struct FFIRegistry {
    functions: HashMap<String, ExternFunction>,
    libraries: HashMap<String, String>, // library name -> path
}

impl FFIRegistry {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            libraries: HashMap::new(),
        }
    }
    
    /// Register an extern "C" function
    pub fn register_function(&mut self, func: ExternFunction) -> FFIResult<()> {
        // Validate function signature
        self.validate_function(&func)?;
        
        self.functions.insert(func.name.clone(), func);
        Ok(())
    }
    
    /// Get extern function by name
    pub fn get_function(&self, name: &str) -> Option<&ExternFunction> {
        self.functions.get(name)
    }
    
    /// Register a library path
    pub fn register_library(&mut self, name: String, path: String) {
        self.libraries.insert(name, path);
    }
    
    /// Validate function signature for safety
    fn validate_function(&self, func: &ExternFunction) -> FFIResult<()> {
        // Check for unsafe patterns
        for param in &func.parameters {
            self.validate_c_type(&param.c_type)?;
        }
        
        self.validate_c_type(&func.return_type)?;
        Ok(())
    }
    
    /// Validate C type for safety
    fn validate_c_type(&self, c_type: &CType) -> FFIResult<()> {
        match c_type {
            CType::Pointer(inner) => {
                // Warn about raw pointers
                self.validate_c_type(inner)?;
            }
            CType::Array(inner, _) => {
                self.validate_c_type(inner)?;
            }
            CType::Function(params, ret) => {
                for param in params {
                    self.validate_c_type(param)?;
                }
                self.validate_c_type(ret)?;
            }
            _ => {} // Primitive types are safe
        }
        Ok(())
    }
}

impl Default for FFIRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export key types from submodules
pub use c_types::*;
pub use marshaling::{FluxValue, MarshalingContext, MarshaledValue};
pub use safety::{SafetyChecker, SafetyLevel, SafetyReport, TrustLevel};
pub use error::{FFIError, FFIResult};
pub use call::FFICaller;