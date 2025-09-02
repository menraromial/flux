//! C type system and mapping to Flux types

use crate::parser::ast::Type as FluxType;
use crate::ffi::CType;
use crate::ffi::error::{FFIError, FFIResult};

/// Maps Flux types to C types for FFI
pub fn flux_to_c_type(flux_type: &FluxType) -> FFIResult<CType> {
    match flux_type {
        FluxType::Int => Ok(CType::Int),
        FluxType::Float => Ok(CType::Double),
        FluxType::Bool => Ok(CType::UChar), // C doesn't have native bool
        FluxType::Char => Ok(CType::Char),
        FluxType::Byte => Ok(CType::UChar),
        FluxType::String => Ok(CType::Pointer(Box::new(CType::Char))), // char*
        FluxType::Unit => Ok(CType::Void),
        
        // Pointer types
        FluxType::Nullable(inner) => {
            let c_inner = flux_to_c_type(inner)?;
            Ok(CType::Pointer(Box::new(c_inner)))
        }
        
        // Arrays become pointers in C
        FluxType::Array(inner) => {
            let c_inner = flux_to_c_type(inner)?;
            Ok(CType::Pointer(Box::new(c_inner)))
        }
        
        // Function types
        FluxType::Function(params, ret) => {
            let c_params: FFIResult<Vec<CType>> = params
                .iter()
                .map(flux_to_c_type)
                .collect();
            let c_ret = flux_to_c_type(ret)?;
            Ok(CType::Function(c_params?, Box::new(c_ret)))
        }
        
        // Complex types not directly supported
        FluxType::List(_) | 
        FluxType::Map(_, _) | 
        FluxType::Set(_) |
        FluxType::Named(_) => {
            Err(FFIError::type_conversion(
                &format!("{:?}", flux_type),
                "C type",
                "Type cannot be directly marshaled to C"
            ))
        }
        
        FluxType::Generic(_, _) => {
            Err(FFIError::type_conversion(
                "generic type",
                "C type",
                "Generic types must be resolved before FFI conversion"
            ))
        }
        
        FluxType::Result(_, _) => {
            Err(FFIError::type_conversion(
                "Result type",
                "C type",
                "Result types cannot be directly passed to C functions"
            ))
        }
        
        FluxType::Never => {
            Err(FFIError::type_conversion(
                "Never type",
                "C type",
                "Never type cannot be used in FFI"
            ))
        }
    }
}

/// Maps C types back to Flux types
pub fn c_to_flux_type(c_type: &CType) -> FFIResult<FluxType> {
    match c_type {
        CType::Void => Ok(FluxType::Unit),
        CType::Char => Ok(FluxType::Char),
        CType::UChar => Ok(FluxType::Byte),
        CType::Short | CType::UShort | 
        CType::Int | CType::UInt |
        CType::Long | CType::ULong |
        CType::LongLong | CType::ULongLong => Ok(FluxType::Int),
        CType::Float | CType::Double => Ok(FluxType::Float),
        
        CType::Pointer(inner) => {
            match inner.as_ref() {
                CType::Char => Ok(FluxType::String), // char* -> string
                CType::Void => Ok(FluxType::Nullable(Box::new(FluxType::Byte))), // void* -> byte?
                other => {
                    let flux_inner = c_to_flux_type(other)?;
                    Ok(FluxType::Nullable(Box::new(flux_inner)))
                }
            }
        }
        
        CType::Array(inner, _) => {
            let flux_inner = c_to_flux_type(inner)?;
            Ok(FluxType::Array(Box::new(flux_inner)))
        }
        
        CType::Function(params, ret) => {
            let flux_params: FFIResult<Vec<FluxType>> = params
                .iter()
                .map(c_to_flux_type)
                .collect();
            let flux_ret = c_to_flux_type(ret)?;
            Ok(FluxType::Function(flux_params?, Box::new(flux_ret)))
        }
        
        CType::Struct(name) | CType::Union(name) => {
            // For now, treat as opaque named types
            Ok(FluxType::Named(name.clone()))
        }
    }
}

/// Get the size of a C type in bytes
pub fn c_type_size(c_type: &CType) -> usize {
    match c_type {
        CType::Void => 0,
        CType::Char | CType::UChar => 1,
        CType::Short | CType::UShort => 2,
        CType::Int | CType::UInt | CType::Float => 4,
        CType::Long | CType::ULong | CType::Double => 8,
        CType::LongLong | CType::ULongLong => 8,
        CType::Pointer(_) => std::mem::size_of::<*const ()>(),
        CType::Array(inner, Some(len)) => c_type_size(inner) * len,
        CType::Array(_, None) => std::mem::size_of::<*const ()>(), // Decay to pointer
        CType::Function(_, _) => std::mem::size_of::<*const ()>(),
        CType::Struct(_) | CType::Union(_) => {
            // Would need struct layout information
            std::mem::size_of::<*const ()>() // Treat as pointer for now
        }
    }
}

/// Get the alignment of a C type
pub fn c_type_alignment(c_type: &CType) -> usize {
    match c_type {
        CType::Void => 1,
        CType::Char | CType::UChar => 1,
        CType::Short | CType::UShort => 2,
        CType::Int | CType::UInt | CType::Float => 4,
        CType::Long | CType::ULong | CType::Double => 8,
        CType::LongLong | CType::ULongLong => 8,
        CType::Pointer(_) => std::mem::align_of::<*const ()>(),
        CType::Array(inner, _) => c_type_alignment(inner),
        CType::Function(_, _) => std::mem::align_of::<*const ()>(),
        CType::Struct(_) | CType::Union(_) => {
            // Would need struct layout information
            std::mem::align_of::<*const ()>() // Conservative alignment
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flux_to_c_type_primitives() {
        assert_eq!(flux_to_c_type(&FluxType::Int).unwrap(), CType::Int);
        assert_eq!(flux_to_c_type(&FluxType::Float).unwrap(), CType::Double);
        assert_eq!(flux_to_c_type(&FluxType::Bool).unwrap(), CType::UChar);
        assert_eq!(flux_to_c_type(&FluxType::Char).unwrap(), CType::Char);
        assert_eq!(flux_to_c_type(&FluxType::Unit).unwrap(), CType::Void);
    }

    #[test]
    fn test_c_to_flux_type_primitives() {
        assert_eq!(c_to_flux_type(&CType::Int).unwrap(), FluxType::Int);
        assert_eq!(c_to_flux_type(&CType::Double).unwrap(), FluxType::Float);
        assert_eq!(c_to_flux_type(&CType::Char).unwrap(), FluxType::Char);
        assert_eq!(c_to_flux_type(&CType::Void).unwrap(), FluxType::Unit);
    }

    #[test]
    fn test_pointer_types() {
        let char_ptr = CType::Pointer(Box::new(CType::Char));
        assert_eq!(c_to_flux_type(&char_ptr).unwrap(), FluxType::String);
        
        let int_ptr = CType::Pointer(Box::new(CType::Int));
        assert_eq!(
            c_to_flux_type(&int_ptr).unwrap(), 
            FluxType::Nullable(Box::new(FluxType::Int))
        );
    }

    #[test]
    fn test_type_sizes() {
        assert_eq!(c_type_size(&CType::Char), 1);
        assert_eq!(c_type_size(&CType::Int), 4);
        assert_eq!(c_type_size(&CType::Double), 8);
        assert_eq!(c_type_size(&CType::Pointer(Box::new(CType::Int))), std::mem::size_of::<*const ()>());
    }
}