//! FFI safety checks and validation
//! 
//! This module provides safety checks for FFI operations to prevent
//! common issues like buffer overflows, null pointer dereferences, and
//! memory safety violations.

use crate::ffi::{CType, ExternFunction, CParameter};
use crate::ffi::marshaling::FluxValue;
use crate::ffi::error::{FFIError, FFIResult};
use std::collections::HashSet;

/// Safety level for FFI operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafetyLevel {
    /// Unsafe operations allowed (like raw C)
    Unsafe,
    /// Basic safety checks (null pointer checks, bounds checking where possible)
    Safe,
    /// Strict safety (additional validation, no raw pointers)
    Strict,
}

/// FFI safety checker
pub struct SafetyChecker {
    safety_level: SafetyLevel,
    trusted_functions: HashSet<String>,
    dangerous_patterns: Vec<DangerousPattern>,
}

/// Represents a dangerous FFI pattern to check for
#[derive(Debug, Clone)]
pub struct DangerousPattern {
    pub name: String,
    pub description: String,
    pub check: fn(&ExternFunction) -> bool,
}

impl SafetyChecker {
    pub fn new(safety_level: SafetyLevel) -> Self {
        let mut checker = Self {
            safety_level,
            trusted_functions: HashSet::new(),
            dangerous_patterns: Vec::new(),
        };
        
        checker.init_dangerous_patterns();
        checker.init_trusted_functions();
        checker
    }
    
    /// Initialize known dangerous patterns
    fn init_dangerous_patterns(&mut self) {
        self.dangerous_patterns.push(DangerousPattern {
            name: "raw_pointer_return".to_string(),
            description: "Function returns raw pointer without size information".to_string(),
            check: |func| matches!(func.return_type, CType::Pointer(_)),
        });
        
        self.dangerous_patterns.push(DangerousPattern {
            name: "void_pointer_param".to_string(),
            description: "Function accepts void pointer parameter".to_string(),
            check: |func| {
                func.parameters.iter().any(|p| {
                    matches!(p.c_type, CType::Pointer(ref inner) if **inner == CType::Void)
                })
            },
        });
        
        self.dangerous_patterns.push(DangerousPattern {
            name: "variadic_function".to_string(),
            description: "Variadic functions are inherently unsafe".to_string(),
            check: |func| func.is_variadic,
        });
        
        self.dangerous_patterns.push(DangerousPattern {
            name: "function_pointer".to_string(),
            description: "Function pointer parameters can be dangerous".to_string(),
            check: |func| {
                func.parameters.iter().any(|p| {
                    matches!(p.c_type, CType::Function(_, _))
                })
            },
        });
    }
    
    /// Initialize trusted standard library functions
    fn init_trusted_functions(&mut self) {
        // Standard C library functions that are generally safe
        let trusted = vec![
            "strlen", "strcmp", "strcpy", "strcat", "memcpy", "memset",
            "malloc", "free", "calloc", "realloc",
            "printf", "sprintf", "fprintf",
            "fopen", "fclose", "fread", "fwrite",
            "sin", "cos", "tan", "sqrt", "pow", "log",
        ];
        
        for func in trusted {
            self.trusted_functions.insert(func.to_string());
        }
    }
    
    /// Check if a function is safe to call
    pub fn check_function_safety(&self, func: &ExternFunction) -> FFIResult<SafetyReport> {
        let mut report = SafetyReport::new(func.name.clone());
        
        // Check if function is in trusted list (but not if it's variadic)
        if self.trusted_functions.contains(&func.name) && !func.is_variadic {
            report.trust_level = TrustLevel::Trusted;
        }
        
        // Check for dangerous patterns
        for pattern in &self.dangerous_patterns {
            if (pattern.check)(func) {
                let warning = SafetyWarning {
                    pattern: pattern.name.clone(),
                    description: pattern.description.clone(),
                    severity: self.get_pattern_severity(&pattern.name),
                };
                report.warnings.push(warning);
            }
        }
        
        // Apply safety level restrictions
        match self.safety_level {
            SafetyLevel::Strict => {
                if !report.warnings.is_empty() && report.trust_level != TrustLevel::Trusted {
                    return Err(FFIError::safety_violation(
                        &func.name,
                        &format!("Function violates strict safety requirements: {:?}", report.warnings)
                    ));
                }
            }
            SafetyLevel::Safe => {
                let critical_warnings: Vec<_> = report.warnings.iter()
                    .filter(|w| w.severity == Severity::Critical)
                    .collect();
                
                if !critical_warnings.is_empty() && report.trust_level != TrustLevel::Trusted {
                    return Err(FFIError::safety_violation(
                        &func.name,
                        &format!("Function has critical safety issues: {:?}", critical_warnings)
                    ));
                }
            }
            SafetyLevel::Unsafe => {
                // Allow everything, just record warnings
            }
        }
        
        Ok(report)
    }
    
    /// Check safety of function call arguments
    pub fn check_call_safety(
        &self,
        func: &ExternFunction,
        args: &[FluxValue],
    ) -> FFIResult<()> {
        // Check argument count
        if args.len() != func.parameters.len() && !func.is_variadic {
            return Err(FFIError::safety_violation(
                &func.name,
                &format!("Argument count mismatch: expected {}, got {}", func.parameters.len(), args.len())
            ));
        }
        
        // Check each argument
        for (i, (arg, param)) in args.iter().zip(func.parameters.iter()).enumerate() {
            self.check_argument_safety(arg, param, i)?;
        }
        
        Ok(())
    }
    
    /// Check safety of individual argument
    fn check_argument_safety(
        &self,
        arg: &FluxValue,
        param: &CParameter,
        index: usize,
    ) -> FFIResult<()> {
        match (&param.c_type, arg) {
            // Null pointer checks
            (CType::Pointer(_), FluxValue::Null) => {
                if self.safety_level != SafetyLevel::Unsafe {
                    return Err(FFIError::safety_violation(
                        "function_call",
                        &format!("Null pointer passed to parameter {} ({})", index, param.name)
                    ));
                }
            }
            
            // String safety checks
            (CType::Pointer(inner), FluxValue::String(s)) if **inner == CType::Char => {
                if s.contains('\0') {
                    return Err(FFIError::safety_violation(
                        "function_call",
                        &format!("String contains null byte at parameter {} ({})", index, param.name)
                    ));
                }
            }
            
            // Array bounds checking (limited without size info)
            (CType::Pointer(_), FluxValue::Array(arr)) => {
                if arr.is_empty() && self.safety_level == SafetyLevel::Strict {
                    return Err(FFIError::safety_violation(
                        "function_call",
                        &format!("Empty array passed to parameter {} ({})", index, param.name)
                    ));
                }
            }
            
            _ => {} // Other combinations are checked during marshaling
        }
        
        Ok(())
    }
    
    /// Get severity level for a dangerous pattern
    fn get_pattern_severity(&self, pattern_name: &str) -> Severity {
        match pattern_name {
            "void_pointer_param" | "variadic_function" => Severity::Critical,
            "raw_pointer_return" | "function_pointer" => Severity::Warning,
            _ => Severity::Info,
        }
    }
    
    /// Add a function to the trusted list
    pub fn trust_function(&mut self, name: String) {
        self.trusted_functions.insert(name);
    }
    
    /// Remove a function from the trusted list
    pub fn untrust_function(&mut self, name: &str) {
        self.trusted_functions.remove(name);
    }
}

/// Safety report for an FFI function
#[derive(Debug)]
pub struct SafetyReport {
    pub function_name: String,
    pub trust_level: TrustLevel,
    pub warnings: Vec<SafetyWarning>,
}

impl SafetyReport {
    fn new(function_name: String) -> Self {
        Self {
            function_name,
            trust_level: TrustLevel::Unknown,
            warnings: Vec::new(),
        }
    }
    
    pub fn is_safe(&self) -> bool {
        self.trust_level == TrustLevel::Trusted || 
        self.warnings.iter().all(|w| w.severity != Severity::Critical)
    }
}

/// Trust level for FFI functions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrustLevel {
    Trusted,   // Known safe function
    Unknown,   // Unknown safety status
    Dangerous, // Known dangerous function
}

/// Safety warning for FFI operations
#[derive(Debug, Clone)]
pub struct SafetyWarning {
    pub pattern: String,
    pub description: String,
    pub severity: Severity,
}

/// Severity levels for safety warnings
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Info,     // Informational only
    Warning,  // Potentially dangerous
    Critical, // Definitely dangerous
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_function(name: &str, params: Vec<CParameter>, ret: CType) -> ExternFunction {
        ExternFunction {
            name: name.to_string(),
            parameters: params,
            return_type: ret,
            library: None,
            is_variadic: false,
        }
    }

    #[test]
    fn test_safe_function() {
        let checker = SafetyChecker::new(SafetyLevel::Safe);
        let func = create_test_function(
            "add",
            vec![
                CParameter { name: "a".to_string(), c_type: CType::Int },
                CParameter { name: "b".to_string(), c_type: CType::Int },
            ],
            CType::Int,
        );
        
        let report = checker.check_function_safety(&func).unwrap();
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_dangerous_void_pointer() {
        let checker = SafetyChecker::new(SafetyLevel::Safe);
        let func = create_test_function(
            "dangerous",
            vec![
                CParameter { 
                    name: "ptr".to_string(), 
                    c_type: CType::Pointer(Box::new(CType::Void)) 
                },
            ],
            CType::Void,
        );
        
        let result = checker.check_function_safety(&func);
        assert!(result.is_err());
    }

    #[test]
    fn test_trusted_function() {
        let checker = SafetyChecker::new(SafetyLevel::Strict);
        let func = create_test_function(
            "strlen",
            vec![
                CParameter { 
                    name: "str".to_string(), 
                    c_type: CType::Pointer(Box::new(CType::Char)) 
                },
            ],
            CType::Int,
        );
        
        let report = checker.check_function_safety(&func).unwrap();
        assert_eq!(report.trust_level, TrustLevel::Trusted);
    }

    #[test]
    fn test_null_pointer_check() {
        let checker = SafetyChecker::new(SafetyLevel::Safe);
        let func = create_test_function(
            "test",
            vec![
                CParameter { 
                    name: "ptr".to_string(), 
                    c_type: CType::Pointer(Box::new(CType::Int)) 
                },
            ],
            CType::Void,
        );
        
        let args = vec![FluxValue::Null];
        let result = checker.check_call_safety(&func, &args);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_with_null_byte() {
        let checker = SafetyChecker::new(SafetyLevel::Safe);
        let func = create_test_function(
            "test",
            vec![
                CParameter { 
                    name: "str".to_string(), 
                    c_type: CType::Pointer(Box::new(CType::Char)) 
                },
            ],
            CType::Void,
        );
        
        let args = vec![FluxValue::String("hello\0world".to_string())];
        let result = checker.check_call_safety(&func, &args);
        assert!(result.is_err());
    }
}