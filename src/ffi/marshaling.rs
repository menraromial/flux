//! Data marshaling between Flux and C types
//! 
//! This module handles the safe conversion of data between Flux and C representations,
//! including memory management and safety checks.

use crate::ffi::{CType, CParameter};
use crate::parser::ast::{Type as FluxType, Expression};
use crate::ffi::error::{FFIError, FFIResult};
use std::ffi::{CString, CStr};
use std::os::raw::{c_char, c_void, c_int, c_double};

/// Represents a marshaled value ready for C function call
#[derive(Debug)]
pub enum MarshaledValue {
    Int(c_int),
    Double(c_double),
    Char(c_char),
    Pointer(*const c_void),
    String(CString),
    Null,
}

/// Marshaling context for managing temporary allocations
pub struct MarshalingContext {
    temp_strings: Vec<CString>,
    temp_allocations: Vec<*mut c_void>,
}

impl MarshalingContext {
    pub fn new() -> Self {
        Self {
            temp_strings: Vec::new(),
            temp_allocations: Vec::new(),
        }
    }
    
    /// Marshal a Flux value to C representation
    pub fn marshal_value(
        &mut self, 
        value: &FluxValue, 
        target_type: &CType
    ) -> FFIResult<MarshaledValue> {
        match (value, target_type) {
            // Integer types
            (FluxValue::Int(i), CType::Int) => Ok(MarshaledValue::Int(*i as c_int)),
            (FluxValue::Int(i), CType::Long) => Ok(MarshaledValue::Int(*i as c_int)),
            (FluxValue::Int(i), CType::UInt) => Ok(MarshaledValue::Int(*i as c_int)),
            
            // Float types
            (FluxValue::Float(f), CType::Float) => Ok(MarshaledValue::Double(*f as c_double)),
            (FluxValue::Float(f), CType::Double) => Ok(MarshaledValue::Double(*f)),
            
            // Character types
            (FluxValue::Char(c), CType::Char) => Ok(MarshaledValue::Char(*c as c_char)),
            (FluxValue::Byte(b), CType::UChar) => Ok(MarshaledValue::Char(*b as c_char)),
            
            // String to char*
            (FluxValue::String(s), CType::Pointer(inner)) if **inner == CType::Char => {
                let c_string = CString::new(s.as_str())
                    .map_err(|e| FFIError::marshaling("string conversion", &format!("Invalid string for C: {}", e)))?;
                let ptr = c_string.as_ptr() as *const c_void;
                self.temp_strings.push(c_string);
                Ok(MarshaledValue::Pointer(ptr))
            }
            
            // Boolean to unsigned char
            (FluxValue::Bool(b), CType::UChar) => {
                Ok(MarshaledValue::Char(if *b { 1 } else { 0 }))
            }
            
            // Null values
            (FluxValue::Null, CType::Pointer(_)) => Ok(MarshaledValue::Null),
            
            // Array to pointer (unsafe - requires length tracking)
            (FluxValue::Array(arr), CType::Pointer(inner)) => {
                self.marshal_array(arr, inner)
            }
            
            // Type mismatch
            _ => Err(FFIError::marshaling(
                "type conversion",
                &format!("Cannot marshal {:?} to C type {:?}", value, target_type)
            ))
        }
    }
    
    /// Marshal an array to a C pointer
    fn marshal_array(
        &mut self, 
        array: &[FluxValue], 
        element_type: &CType
    ) -> FFIResult<MarshaledValue> {
        match element_type {
            CType::Int => {
                let mut c_array = Vec::with_capacity(array.len());
                for value in array {
                    if let FluxValue::Int(i) = value {
                        c_array.push(*i as c_int);
                    } else {
                        return Err(FFIError::marshaling(
                            "array element conversion",
                            &format!("Array element type mismatch: expected int, got {:?}", value)
                        ));
                    }
                }
                
                let ptr = c_array.as_ptr() as *const c_void;
                // Leak the vector to keep data alive for C call
                std::mem::forget(c_array);
                Ok(MarshaledValue::Pointer(ptr))
            }
            
            CType::Double => {
                let mut c_array = Vec::with_capacity(array.len());
                for value in array {
                    if let FluxValue::Float(f) = value {
                        c_array.push(*f);
                    } else {
                        return Err(FFIError::marshaling(
                            "array element conversion",
                            &format!("Array element type mismatch: expected float, got {:?}", value)
                        ));
                    }
                }
                
                let ptr = c_array.as_ptr() as *const c_void;
                std::mem::forget(c_array);
                Ok(MarshaledValue::Pointer(ptr))
            }
            
            _ => Err(FFIError::marshaling(
                "array marshaling",
                &format!("Unsupported array element type for marshaling: {:?}", element_type)
            ))
        }
    }
    
    /// Unmarshal a C return value back to Flux
    pub fn unmarshal_return(
        &self, 
        c_value: *const c_void, 
        return_type: &CType
    ) -> FFIResult<FluxValue> {
        match return_type {
            CType::Void => Ok(FluxValue::Unit),
            
            CType::Int => {
                let int_val = unsafe { *(c_value as *const c_int) };
                Ok(FluxValue::Int(int_val as i64))
            }
            
            CType::Double => {
                let double_val = unsafe { *(c_value as *const c_double) };
                Ok(FluxValue::Float(double_val))
            }
            
            CType::Char => {
                let char_val = unsafe { *(c_value as *const c_char) };
                Ok(FluxValue::Char(char_val as u8 as char))
            }
            
            CType::Pointer(inner) if **inner == CType::Char => {
                if c_value.is_null() {
                    Ok(FluxValue::Null)
                } else {
                    let c_str = unsafe { CStr::from_ptr(c_value as *const c_char) };
                    let rust_str = c_str.to_str()
                        .map_err(|e| FFIError::marshaling("string unmarshaling", &format!("Invalid UTF-8 from C: {}", e)))?;
                    Ok(FluxValue::String(rust_str.to_string()))
                }
            }
            
            CType::Pointer(_) => {
                if c_value.is_null() {
                    Ok(FluxValue::Null)
                } else {
                    // Return as opaque pointer for now
                    Ok(FluxValue::Pointer(c_value as usize))
                }
            }
            
            _ => Err(FFIError::marshaling(
                "return value unmarshaling",
                &format!("Unsupported return type for unmarshaling: {:?}", return_type)
            ))
        }
    }
    
    /// Validate that parameters can be safely marshaled
    pub fn validate_parameters(
        &self, 
        args: &[FluxValue], 
        params: &[CParameter]
    ) -> FFIResult<()> {
        if args.len() != params.len() {
            return Err(FFIError::marshaling(
                "parameter validation",
                &format!("Parameter count mismatch: expected {}, got {}", params.len(), args.len())
            ));
        }
        
        for (arg, param) in args.iter().zip(params.iter()) {
            self.validate_parameter_compatibility(arg, &param.c_type)?;
        }
        
        Ok(())
    }
    
    /// Check if a Flux value can be marshaled to a C type
    fn validate_parameter_compatibility(
        &self, 
        value: &FluxValue, 
        c_type: &CType
    ) -> FFIResult<()> {
        match (value, c_type) {
            (FluxValue::Int(_), CType::Int | CType::Long | CType::UInt) => Ok(()),
            (FluxValue::Float(_), CType::Float | CType::Double) => Ok(()),
            (FluxValue::Char(_), CType::Char) => Ok(()),
            (FluxValue::Byte(_), CType::UChar) => Ok(()),
            (FluxValue::Bool(_), CType::UChar) => Ok(()),
            (FluxValue::String(_), CType::Pointer(inner)) if **inner == CType::Char => Ok(()),
            (FluxValue::Null, CType::Pointer(_)) => Ok(()),
            (FluxValue::Array(_), CType::Pointer(_)) => Ok(()),
            
            _ => Err(FFIError::type_conversion(
                &format!("{:?}", value),
                &format!("{:?}", c_type),
                "incompatible types"
            ))
        }
    }
}

impl Drop for MarshalingContext {
    fn drop(&mut self) {
        // Clean up temporary allocations
        for ptr in &self.temp_allocations {
            unsafe {
                libc::free(*ptr);
            }
        }
    }
}

/// Represents a Flux runtime value
#[derive(Debug, Clone)]
pub enum FluxValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Byte(u8),
    String(String),
    Array(Vec<FluxValue>),
    Null,
    Unit,
    Pointer(usize), // Opaque pointer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marshal_primitives() {
        let mut ctx = MarshalingContext::new();
        
        // Test integer marshaling
        let int_val = FluxValue::Int(42);
        let marshaled = ctx.marshal_value(&int_val, &CType::Int).unwrap();
        match marshaled {
            MarshaledValue::Int(i) => assert_eq!(i, 42),
            _ => panic!("Expected marshaled int"),
        }
        
        // Test float marshaling
        let float_val = FluxValue::Float(3.14);
        let marshaled = ctx.marshal_value(&float_val, &CType::Double).unwrap();
        match marshaled {
            MarshaledValue::Double(f) => assert_eq!(f, 3.14),
            _ => panic!("Expected marshaled double"),
        }
    }

    #[test]
    fn test_marshal_string() {
        let mut ctx = MarshalingContext::new();
        let string_val = FluxValue::String("hello".to_string());
        let char_ptr_type = CType::Pointer(Box::new(CType::Char));
        
        let marshaled = ctx.marshal_value(&string_val, &char_ptr_type).unwrap();
        match marshaled {
            MarshaledValue::Pointer(ptr) => assert!(!ptr.is_null()),
            _ => panic!("Expected marshaled pointer"),
        }
    }

    #[test]
    fn test_parameter_validation() {
        let ctx = MarshalingContext::new();
        let args = vec![FluxValue::Int(42), FluxValue::String("test".to_string())];
        let params = vec![
            CParameter { name: "x".to_string(), c_type: CType::Int },
            CParameter { name: "s".to_string(), c_type: CType::Pointer(Box::new(CType::Char)) },
        ];
        
        assert!(ctx.validate_parameters(&args, &params).is_ok());
    }

    #[test]
    fn test_parameter_count_mismatch() {
        let ctx = MarshalingContext::new();
        let args = vec![FluxValue::Int(42)];
        let params = vec![
            CParameter { name: "x".to_string(), c_type: CType::Int },
            CParameter { name: "y".to_string(), c_type: CType::Int },
        ];
        
        assert!(ctx.validate_parameters(&args, &params).is_err());
    }
}