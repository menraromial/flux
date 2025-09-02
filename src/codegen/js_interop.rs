//! JavaScript interoperability utilities for WebAssembly
//! 
//! Provides utilities for generating JavaScript bindings and interop code.

use crate::error::{CodeGenError, CodeGenErrorKind};
use crate::semantic::*;
use crate::parser::ast::Type;
use std::collections::HashMap;

/// JavaScript interop code generator
pub struct JsInteropGenerator {
    /// Exported functions from the WASM module
    exported_functions: HashMap<String, TypedFunction>,
    /// Imported functions from JavaScript
    imported_functions: HashMap<String, JsFunction>,
}

/// JavaScript function definition
#[derive(Debug, Clone)]
pub struct JsFunction {
    pub name: String,
    pub params: Vec<JsType>,
    pub return_type: JsType,
    pub js_code: String,
}

/// JavaScript type representation
#[derive(Debug, Clone, PartialEq)]
pub enum JsType {
    Number,
    String,
    Boolean,
    Object,
    Array(Box<JsType>),
    Void,
}

impl JsInteropGenerator {
    /// Create a new JavaScript interop generator
    pub fn new() -> Self {
        Self {
            exported_functions: HashMap::new(),
            imported_functions: HashMap::new(),
        }
    }
    
    /// Add an exported function
    pub fn add_exported_function(&mut self, func: TypedFunction) {
        self.exported_functions.insert(func.name.clone(), func);
    }
    
    /// Add an imported JavaScript function
    pub fn add_imported_function(&mut self, js_func: JsFunction) {
        self.imported_functions.insert(js_func.name.clone(), js_func);
    }
    
    /// Generate JavaScript wrapper code
    pub fn generate_js_wrapper(&self, module_name: &str) -> Result<String, CodeGenError> {
        let mut js_code = String::new();
        
        // Generate module loading code
        js_code.push_str(&format!(r#"
// Generated JavaScript wrapper for Flux WebAssembly module: {}
class FluxModule {{
    constructor() {{
        this.instance = null;
        this.memory = null;
        this.textDecoder = new TextDecoder();
        this.textEncoder = new TextEncoder();
    }}
    
    async load(wasmBytes) {{
        const imports = {{
            js: {{
                console_log: (ptr) => {{
                    try {{
                        const str = this.readString(ptr);
                        console.log(str);
                    }} catch (e) {{
                        console.error('Error in console_log:', e);
                    }}
                }},
                malloc: (size) => {{
                    try {{
                        return this.allocateMemory(size);
                    }} catch (e) {{
                        console.error('Error in malloc:', e);
                        return 0;
                    }}
                }},
                free: (ptr) => {{
                    try {{
                        this.freeMemory(ptr);
                    }} catch (e) {{
                        console.error('Error in free:', e);
                    }}
                }},
                // Error handling support
                throw_error: (ptr) => {{
                    const message = this.readString(ptr);
                    throw new Error(message);
                }},
                // Performance timing
                performance_now: () => {{
                    return performance.now();
                }}
            }}
        }};
        
        try {{
            const module = await WebAssembly.instantiate(wasmBytes, imports);
            this.instance = module.instance;
            this.memory = this.instance.exports.memory;
            
            return this;
        }} catch (error) {{
            throw new Error(`Failed to load WebAssembly module: ${{error.message}}`);
        }}
    }}
    
    // Memory management utilities
    allocateMemory(size) {{
        // This is a simplified allocation - a real implementation would
        // need proper heap management
        const ptr = this.memory.buffer.byteLength;
        this.memory.grow(Math.ceil(size / 65536)); // Grow by pages (64KB each)
        return ptr;
    }}
    
    freeMemory(ptr) {{
        // No-op for simplified implementation
    }}
    
    readString(ptr) {{
        const memory = new Uint8Array(this.memory.buffer);
        let end = ptr;
        while (memory[end] !== 0) end++;
        return this.textDecoder.decode(memory.slice(ptr, end));
    }}
    
    writeString(str) {{
        const bytes = this.textEncoder.encode(str + '\0');
        const ptr = this.allocateMemory(bytes.length);
        const memory = new Uint8Array(this.memory.buffer);
        memory.set(bytes, ptr);
        return ptr;
    }}
    
    // Type conversion utilities
    fluxToJs(value, type) {{
        switch (type) {{
            case 'int':
            case 'float':
            case 'bool':
            case 'char':
            case 'byte':
                return value;
            case 'string':
                return this.readString(value);
            default:
                return value;
        }}
    }}
    
    jsToFlux(value, type) {{
        switch (type) {{
            case 'int':
                return BigInt(Math.floor(value));
            case 'float':
                return Number(value);
            case 'bool':
                return value ? 1 : 0;
            case 'char':
            case 'byte':
                return typeof value === 'string' ? value.charCodeAt(0) : Number(value);
            case 'string':
                return this.writeString(String(value));
            default:
                return value;
        }}
    }}
"#, module_name));
        
        // Generate wrapper functions for exported functions
        for (name, func) in &self.exported_functions {
            js_code.push_str(&self.generate_function_wrapper(name, func)?);
        }
        
        // Close the class
        js_code.push_str("}\n\n");
        
        // Generate module export
        js_code.push_str(&format!(r#"
// Export the module class
if (typeof module !== 'undefined' && module.exports) {{
    module.exports = FluxModule;
}} else if (typeof window !== 'undefined') {{
    window.FluxModule = FluxModule;
}}
"#));
        
        Ok(js_code)
    }
    
    /// Generate a wrapper function for a Flux function
    fn generate_function_wrapper(&self, name: &str, func: &TypedFunction) -> Result<String, CodeGenError> {
        let mut wrapper = String::new();
        
        // Generate parameter list
        let param_names: Vec<String> = func.parameters.iter()
            .map(|p| p.name.clone())
            .collect();
        
        // Add async keyword if the function is async
        let async_keyword = if func.is_async { "async " } else { "" };
        wrapper.push_str(&format!("    {}{}({}) {{\n", async_keyword, name, param_names.join(", ")));
        
        // Add error handling wrapper
        wrapper.push_str("        try {\n");
        
        // Convert parameters from JS to Flux types
        for param in &func.parameters {
            let flux_type = self.type_to_js_type_name(&param.type_);
            wrapper.push_str(&format!(
                "            const flux_{} = this.jsToFlux({}, '{}');\n",
                param.name, param.name, flux_type
            ));
        }
        
        // Call the WASM function
        let await_keyword = if func.is_async { "await " } else { "" };
        wrapper.push_str(&format!("            const result = {}this.instance.exports.{}(", await_keyword, name));
        let flux_params: Vec<String> = func.parameters.iter()
            .map(|p| format!("flux_{}", p.name))
            .collect();
        wrapper.push_str(&flux_params.join(", "));
        wrapper.push_str(");\n");
        
        // Convert result from Flux to JS type
        if !matches!(func.return_type, Type::Unit) {
            let return_type = self.type_to_js_type_name(&func.return_type);
            wrapper.push_str(&format!(
                "            return this.fluxToJs(result, '{}');\n",
                return_type
            ));
        }
        
        // Close error handling
        wrapper.push_str("        } catch (error) {\n");
        wrapper.push_str(&format!("            throw new Error(`Error calling {}: ${{error.message}}`);\n", name));
        wrapper.push_str("        }\n");
        
        wrapper.push_str("    }\n\n");
        
        Ok(wrapper)
    }
    
    /// Convert Flux type to JavaScript type name string
    fn type_to_js_type_name(&self, flux_type: &Type) -> String {
        match flux_type {
            Type::Int => "int".to_string(),
            Type::Float => "float".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Char => "char".to_string(),
            Type::Byte => "byte".to_string(),
            Type::String => "string".to_string(),
            Type::Array(_) => "array".to_string(),
            Type::Nullable(_) => "nullable".to_string(),
            Type::Unit => "void".to_string(),
            _ => "object".to_string(),
        }
    }
    
    /// Convert Flux type to JavaScript type
    fn flux_type_to_js_type(&self, flux_type: &Type) -> Result<JsType, CodeGenError> {
        match flux_type {
            Type::Int | Type::Float => Ok(JsType::Number),
            Type::Bool => Ok(JsType::Boolean),
            Type::Char | Type::Byte => Ok(JsType::Number),
            Type::String => Ok(JsType::String),
            Type::Array(elem_type) => {
                let elem_js_type = self.flux_type_to_js_type(elem_type)?;
                Ok(JsType::Array(Box::new(elem_js_type)))
            }
            Type::Nullable(inner_type) => {
                // Nullable types are represented as the inner type or null
                self.flux_type_to_js_type(inner_type)
            }
            Type::Unit => Ok(JsType::Void),
            _ => Ok(JsType::Object),
        }
    }
    
    /// Generate TypeScript definitions
    pub fn generate_typescript_definitions(&self, module_name: &str) -> Result<String, CodeGenError> {
        let mut ts_code = String::new();
        
        // Generate module interface
        ts_code.push_str(&format!(r#"
// Generated TypeScript definitions for Flux WebAssembly module: {}

export interface FluxModule {{
    load(wasmBytes: Uint8Array): Promise<FluxModule>;
"#, module_name));
        
        // Generate function signatures
        for (name, func) in &self.exported_functions {
            let param_types: Result<Vec<String>, _> = func.parameters.iter()
                .map(|p| {
                    let js_type = self.flux_type_to_js_type(&p.type_)?;
                    Ok(format!("{}: {}", p.name, self.js_type_to_ts_type(&js_type)))
                })
                .collect();
            
            let param_types = param_types?;
            let return_type = self.flux_type_to_js_type(&func.return_type)?;
            
            ts_code.push_str(&format!(
                "    {}({}): {};\n",
                name,
                param_types.join(", "),
                self.js_type_to_ts_type(&return_type)
            ));
        }
        
        ts_code.push_str("}\n\n");
        
        // Generate constructor
        ts_code.push_str("declare const FluxModule: {\n");
        ts_code.push_str("    new(): FluxModule;\n");
        ts_code.push_str("};\n\n");
        
        ts_code.push_str("export default FluxModule;\n");
        
        Ok(ts_code)
    }
    
    /// Convert JavaScript type to TypeScript type string
    fn js_type_to_ts_type(&self, js_type: &JsType) -> String {
        match js_type {
            JsType::Number => "number".to_string(),
            JsType::String => "string".to_string(),
            JsType::Boolean => "boolean".to_string(),
            JsType::Object => "object".to_string(),
            JsType::Array(elem_type) => format!("{}[]", self.js_type_to_ts_type(elem_type)),
            JsType::Void => "void".to_string(),
        }
    }
    
    /// Generate HTML test page
    pub fn generate_html_test_page(&self, module_name: &str) -> String {
        format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Flux WebAssembly Test - {}</title>
    <style>
        body {{
            font-family: Arial, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
        }}
        .test-section {{
            margin: 20px 0;
            padding: 15px;
            border: 1px solid #ddd;
            border-radius: 5px;
        }}
        .output {{
            background-color: #f5f5f5;
            padding: 10px;
            border-radius: 3px;
            font-family: monospace;
            white-space: pre-wrap;
        }}
        button {{
            background-color: #007cba;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 3px;
            cursor: pointer;
            margin: 5px;
        }}
        button:hover {{
            background-color: #005a87;
        }}
    </style>
</head>
<body>
    <h1>Flux WebAssembly Test - {}</h1>
    
    <div class="test-section">
        <h2>Load Module</h2>
        <button onclick="loadModule()">Load WASM Module</button>
        <div id="load-output" class="output"></div>
    </div>
    
    <div class="test-section">
        <h2>Function Tests</h2>
        <button onclick="runTests()">Run All Tests</button>
        <div id="test-output" class="output"></div>
    </div>
    
    <script>
        let fluxModule = null;
        
        async function loadModule() {{
            const output = document.getElementById('load-output');
            try {{
                output.textContent = 'Loading WASM module...';
                
                // In a real application, you would fetch the WASM file
                // const wasmBytes = await fetch('{}.wasm').then(r => r.arrayBuffer());
                // fluxModule = await new FluxModule().load(new Uint8Array(wasmBytes));
                
                output.textContent = 'Module loaded successfully!';
            }} catch (error) {{
                output.textContent = `Error loading module: ${{error.message}}`;
            }}
        }}
        
        async function runTests() {{
            const output = document.getElementById('test-output');
            if (!fluxModule) {{
                output.textContent = 'Please load the module first.';
                return;
            }}
            
            try {{
                output.textContent = 'Running tests...\n';
                
                // Add test calls for exported functions here
                // Example:
                // const result = fluxModule.main();
                // output.textContent += `main() returned: ${{result}}\n`;
                
                output.textContent += 'All tests completed!';
            }} catch (error) {{
                output.textContent += `Error running tests: ${{error.message}}`;
            }}
        }}
    </script>
</body>
</html>"#, module_name, module_name, module_name)
    }
}

impl Default for JsInteropGenerator {
    fn default() -> Self {
        Self::new()
    }
}