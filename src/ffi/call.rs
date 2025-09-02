//! FFI function calling implementation
//! 
//! This module provides the actual mechanism for calling C functions from Flux,
//! including dynamic library loading, symbol resolution, and safe function invocation.

use crate::ffi::{ExternFunction, CType, CParameter};
use crate::ffi::marshaling::{FluxValue, MarshalingContext, MarshaledValue};
use crate::ffi::safety::{SafetyChecker, SafetyLevel};
use crate::ffi::error::{FFIError, FFIResult};
use std::collections::HashMap;
use std::ffi::{CString, c_void};
use std::os::raw::{c_int, c_double, c_char};

/// FFI function caller that manages dynamic library loading and function invocation
pub struct FFICaller {
    libraries: HashMap<String, LibraryHandle>,
    functions: HashMap<String, FunctionHandle>,
    safety_checker: SafetyChecker,
}

/// Handle to a loaded dynamic library
struct LibraryHandle {
    #[cfg(unix)]
    handle: *mut c_void,
    #[cfg(windows)]
    handle: *mut c_void,
    path: String,
}

/// Handle to a resolved function symbol
struct FunctionHandle {
    symbol: *mut c_void,
    signature: ExternFunction,
}

impl FFICaller {
    pub fn new(safety_level: SafetyLevel) -> Self {
        Self {
            libraries: HashMap::new(),
            functions: HashMap::new(),
            safety_checker: SafetyChecker::new(safety_level),
        }
    }
    
    /// Load a dynamic library
    pub fn load_library(&mut self, name: &str, path: &str) -> FFIResult<()> {
        if self.libraries.contains_key(name) {
            return Ok(()); // Already loaded
        }
        
        let c_path = CString::new(path)
            .map_err(|e| FFIError::library_load(name, &format!("Invalid path: {}", e)))?;
        
        #[cfg(unix)]
        let handle = unsafe {
            libc::dlopen(c_path.as_ptr(), libc::RTLD_LAZY)
        };
        
        #[cfg(windows)]
        let handle = unsafe {
            use std::os::windows::ffi::OsStrExt;
            use std::ffi::OsStr;
            let wide_path: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
            winapi::um::libloaderapi::LoadLibraryW(wide_path.as_ptr()) as *mut c_void
        };
        
        if handle.is_null() {
            return Err(FFIError::library_load(name, "Failed to load library"));
        }
        
        let lib_handle = LibraryHandle {
            handle,
            path: path.to_string(),
        };
        
        self.libraries.insert(name.to_string(), lib_handle);
        Ok(())
    }
    
    /// Resolve a function symbol from a loaded library
    pub fn resolve_function(&mut self, func: ExternFunction) -> FFIResult<()> {
        // Check function safety first
        let safety_report = self.safety_checker.check_function_safety(&func)
            .map_err(|e| FFIError::safety_violation(&func.name, &e.to_string()))?;
        
        if !safety_report.is_safe() {
            return Err(FFIError::safety_violation(
                &func.name, 
                "Function failed safety checks"
            ));
        }
        
        // Determine which library to use
        let library_name = func.library.as_deref().unwrap_or("C");
        
        // For standard C library, try to resolve without explicit loading
        let symbol = if library_name == "C" {
            self.resolve_c_function(&func.name)?
        } else {
            // Load library if not already loaded
            if !self.libraries.contains_key(library_name) {
                // Try common library paths
                let lib_path = self.find_library_path(library_name)?;
                self.load_library(library_name, &lib_path)?;
            }
            
            self.resolve_library_function(library_name, &func.name)?
        };
        
        let func_handle = FunctionHandle {
            symbol,
            signature: func.clone(),
        };
        
        self.functions.insert(func.name.clone(), func_handle);
        Ok(())
    }
    
    /// Call an FFI function with the given arguments
    pub fn call_function(&self, name: &str, args: Vec<FluxValue>) -> FFIResult<FluxValue> {
        let func_handle = self.functions.get(name)
            .ok_or_else(|| FFIError::symbol_not_found(name, None))?;
        
        // Safety check for the call
        self.safety_checker.check_call_safety(&func_handle.signature, &args)
            .map_err(|e| FFIError::safety_violation(name, &e.to_string()))?;
        
        // Marshal arguments
        let mut marshaling_ctx = MarshalingContext::new();
        marshaling_ctx.validate_parameters(&args, &func_handle.signature.parameters)
            .map_err(|e| FFIError::marshaling("parameter validation", &e.to_string()))?;
        
        let mut marshaled_args = Vec::new();
        for (arg, param) in args.iter().zip(func_handle.signature.parameters.iter()) {
            let marshaled = marshaling_ctx.marshal_value(arg, &param.c_type)
                .map_err(|e| FFIError::marshaling("argument marshaling", &e.to_string()))?;
            marshaled_args.push(marshaled);
        }
        
        // Perform the actual function call
        let result = unsafe {
            self.call_function_unsafe(func_handle, &marshaled_args)?
        };
        
        // Unmarshal return value
        let return_value = marshaling_ctx.unmarshal_return(result, &func_handle.signature.return_type)
            .map_err(|e| FFIError::marshaling("return value unmarshaling", &e.to_string()))?;
        
        Ok(return_value)
    }
    
    /// Unsafe function call implementation
    unsafe fn call_function_unsafe(
        &self, 
        func_handle: &FunctionHandle, 
        args: &[MarshaledValue]
    ) -> FFIResult<*const c_void> {
        let func_ptr = func_handle.symbol;
        
        match func_handle.signature.return_type {
            CType::Void => {
                match args.len() {
                    0 => {
                        let f: extern "C" fn() = std::mem::transmute(func_ptr);
                        f();
                        Ok(std::ptr::null())
                    }
                    1 => {
                        let f: extern "C" fn(*const c_void) = std::mem::transmute(func_ptr);
                        let arg0 = self.get_arg_ptr(&args[0]);
                        f(arg0);
                        Ok(std::ptr::null())
                    }
                    2 => {
                        let f: extern "C" fn(*const c_void, *const c_void) = std::mem::transmute(func_ptr);
                        let arg0 = self.get_arg_ptr(&args[0]);
                        let arg1 = self.get_arg_ptr(&args[1]);
                        f(arg0, arg1);
                        Ok(std::ptr::null())
                    }
                    _ => return Err(FFIError::runtime(&func_handle.signature.name, "Too many arguments for void function"))
                }
            }
            
            CType::Int => {
                match args.len() {
                    0 => {
                        let f: extern "C" fn() -> c_int = std::mem::transmute(func_ptr);
                        let result = f();
                        Ok(Box::into_raw(Box::new(result)) as *const c_void)
                    }
                    1 => {
                        let f: extern "C" fn(*const c_void) -> c_int = std::mem::transmute(func_ptr);
                        let arg0 = self.get_arg_ptr(&args[0]);
                        let result = f(arg0);
                        Ok(Box::into_raw(Box::new(result)) as *const c_void)
                    }
                    2 => {
                        let f: extern "C" fn(*const c_void, *const c_void) -> c_int = std::mem::transmute(func_ptr);
                        let arg0 = self.get_arg_ptr(&args[0]);
                        let arg1 = self.get_arg_ptr(&args[1]);
                        let result = f(arg0, arg1);
                        Ok(Box::into_raw(Box::new(result)) as *const c_void)
                    }
                    _ => return Err(FFIError::runtime(&func_handle.signature.name, "Too many arguments"))
                }
            }
            
            CType::Double => {
                match args.len() {
                    0 => {
                        let f: extern "C" fn() -> c_double = std::mem::transmute(func_ptr);
                        let result = f();
                        Ok(Box::into_raw(Box::new(result)) as *const c_void)
                    }
                    1 => {
                        let f: extern "C" fn(*const c_void) -> c_double = std::mem::transmute(func_ptr);
                        let arg0 = self.get_arg_ptr(&args[0]);
                        let result = f(arg0);
                        Ok(Box::into_raw(Box::new(result)) as *const c_void)
                    }
                    _ => return Err(FFIError::runtime(&func_handle.signature.name, "Too many arguments for double function"))
                }
            }
            
            CType::Pointer(_) => {
                match args.len() {
                    0 => {
                        let f: extern "C" fn() -> *const c_void = std::mem::transmute(func_ptr);
                        Ok(f())
                    }
                    1 => {
                        let f: extern "C" fn(*const c_void) -> *const c_void = std::mem::transmute(func_ptr);
                        let arg0 = self.get_arg_ptr(&args[0]);
                        Ok(f(arg0))
                    }
                    2 => {
                        let f: extern "C" fn(*const c_void, *const c_void) -> *const c_void = std::mem::transmute(func_ptr);
                        let arg0 = self.get_arg_ptr(&args[0]);
                        let arg1 = self.get_arg_ptr(&args[1]);
                        Ok(f(arg0, arg1))
                    }
                    _ => return Err(FFIError::runtime(&func_handle.signature.name, "Too many arguments for pointer function"))
                }
            }
            
            _ => return Err(FFIError::runtime(&func_handle.signature.name, "Unsupported return type"))
        }
    }
    
    /// Extract pointer from marshaled value
    unsafe fn get_arg_ptr(&self, arg: &MarshaledValue) -> *const c_void {
        match arg {
            MarshaledValue::Int(i) => i as *const c_int as *const c_void,
            MarshaledValue::Double(d) => d as *const c_double as *const c_void,
            MarshaledValue::Char(c) => c as *const c_char as *const c_void,
            MarshaledValue::Pointer(ptr) => *ptr,
            MarshaledValue::String(s) => s.as_ptr() as *const c_void,
            MarshaledValue::Null => std::ptr::null(),
        }
    }
    
    /// Resolve a standard C library function
    fn resolve_c_function(&self, name: &str) -> FFIResult<*mut c_void> {
        let c_name = CString::new(name)
            .map_err(|e| FFIError::symbol_not_found(name, Some("C")))?;
        
        #[cfg(unix)]
        let symbol = unsafe {
            libc::dlsym(libc::RTLD_DEFAULT, c_name.as_ptr())
        };
        
        #[cfg(windows)]
        let symbol = unsafe {
            // On Windows, we need to get the handle to the C runtime
            let kernel32 = winapi::um::libloaderapi::GetModuleHandleA(std::ptr::null());
            if kernel32.is_null() {
                return Err(FFIError::symbol_not_found(name, Some("C")));
            }
            winapi::um::libloaderapi::GetProcAddress(kernel32, c_name.as_ptr()) as *mut c_void
        };
        
        if symbol.is_null() {
            return Err(FFIError::symbol_not_found(name, Some("C")));
        }
        
        Ok(symbol)
    }
    
    /// Resolve a function from a specific library
    fn resolve_library_function(&self, library: &str, name: &str) -> FFIResult<*mut c_void> {
        let lib_handle = self.libraries.get(library)
            .ok_or_else(|| FFIError::library_load(library, "Library not loaded"))?;
        
        let c_name = CString::new(name)
            .map_err(|e| FFIError::symbol_not_found(name, Some(library)))?;
        
        #[cfg(unix)]
        let symbol = unsafe {
            libc::dlsym(lib_handle.handle, c_name.as_ptr())
        };
        
        #[cfg(windows)]
        let symbol = unsafe {
            winapi::um::libloaderapi::GetProcAddress(
                lib_handle.handle as winapi::shared::minwindef::HMODULE, 
                c_name.as_ptr()
            ) as *mut c_void
        };
        
        if symbol.is_null() {
            return Err(FFIError::symbol_not_found(name, Some(library)));
        }
        
        Ok(symbol)
    }
    
    /// Find library path using common search patterns
    fn find_library_path(&self, name: &str) -> FFIResult<String> {
        let common_paths = vec![
            format!("lib{}.so", name),      // Linux
            format!("lib{}.dylib", name),   // macOS
            format!("{}.dll", name),        // Windows
            format!("/usr/lib/lib{}.so", name),
            format!("/usr/local/lib/lib{}.so", name),
        ];
        
        for path in common_paths {
            if std::path::Path::new(&path).exists() {
                return Ok(path);
            }
        }
        
        Err(FFIError::library_load(name, "Library not found in common paths"))
    }
}

impl Drop for FFICaller {
    fn drop(&mut self) {
        // Clean up loaded libraries
        for (_, lib_handle) in &self.libraries {
            unsafe {
                #[cfg(unix)]
                libc::dlclose(lib_handle.handle);
                
                #[cfg(windows)]
                winapi::um::libloaderapi::FreeLibrary(
                    lib_handle.handle as winapi::shared::minwindef::HMODULE
                );
            }
        }
    }
}

// Platform-specific imports
#[cfg(unix)]
extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}

#[cfg(windows)]
extern crate winapi;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::CType;

    #[test]
    fn test_ffi_caller_creation() {
        let caller = FFICaller::new(SafetyLevel::Safe);
        assert_eq!(caller.libraries.len(), 0);
        assert_eq!(caller.functions.len(), 0);
    }

    #[test]
    fn test_resolve_c_function() {
        let mut caller = FFICaller::new(SafetyLevel::Safe);
        
        // Try to resolve a common C function
        let strlen_func = ExternFunction {
            name: "strlen".to_string(),
            parameters: vec![CParameter {
                name: "str".to_string(),
                c_type: CType::Pointer(Box::new(CType::Char)),
            }],
            return_type: CType::Int,
            library: Some("C".to_string()),
            is_variadic: false,
        };
        
        // This might fail on some systems, so we just test that it doesn't panic
        let _ = caller.resolve_function(strlen_func);
    }

    #[test]
    fn test_library_path_finding() {
        let caller = FFICaller::new(SafetyLevel::Safe);
        
        // Test with a non-existent library
        let result = caller.find_library_path("nonexistent_lib_12345");
        assert!(result.is_err());
    }
}