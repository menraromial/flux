//! Development tools for the Flux language
//! 
//! This module provides code formatting, linting, testing, and benchmarking tools.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use colored::*;

use crate::error::{FluxResult, FluxError};
use crate::lexer::{FluxLexer, Token};
use crate::parser::{FluxParser, Parser, ast};
use crate::cli::CliContext;

/// Code formatter for Flux source files
pub struct Formatter {
    context: CliContext,
    config: FormatterConfig,
}

#[derive(Debug, Clone)]
pub struct FormatterConfig {
    pub indent_size: usize,
    pub max_line_length: usize,
    pub use_tabs: bool,
    pub trailing_comma: bool,
    pub space_before_paren: bool,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            max_line_length: 100,
            use_tabs: false,
            trailing_comma: true,
            space_before_paren: false,
        }
    }
}

impl Formatter {
    pub fn new(context: CliContext) -> Self {
        Self {
            context,
            config: FormatterConfig::default(),
        }
    }

    pub fn with_config(context: CliContext, config: FormatterConfig) -> Self {
        Self { context, config }
    }

    /// Format a single file
    pub fn format_file(&self, path: &Path, check_only: bool) -> FluxResult<bool> {
        self.context.verbose(&format!("Formatting file: {:?}", path));

        let source = fs::read_to_string(path)
            .map_err(|e| FluxError::Io(format!("Failed to read file {:?}: {}", path, e)))?;

        let formatted = self.format_source(&source)?;

        if check_only {
            let needs_formatting = source != formatted;
            if needs_formatting {
                self.context.info(&format!("{} needs formatting", path.display()));
            }
            Ok(needs_formatting)
        } else {
            if source != formatted {
                fs::write(path, formatted)
                    .map_err(|e| FluxError::Io(format!("Failed to write file {:?}: {}", path, e)))?;
                self.context.info(&format!("Formatted {}", path.display()));
                Ok(true)
            } else {
                self.context.verbose(&format!("File {} already formatted", path.display()));
                Ok(false)
            }
        }
    }

    /// Format source code string
    pub fn format_source(&self, source: &str) -> FluxResult<String> {
        // Parse the source code
        let lexer = FluxLexer::new(source.to_string());
        let mut parser = FluxParser::new(lexer)?;
        let program = parser.parse_program()?;

        // Format the AST back to source code
        let mut formatter = SourceFormatter::new(&self.config);
        Ok(formatter.format_program(&program))
    }

    /// Format all Flux files in a directory
    pub fn format_directory(&self, dir: &Path, check_only: bool) -> FluxResult<FormatResults> {
        let mut results = FormatResults::default();
        
        for entry in fs::read_dir(dir)
            .map_err(|e| FluxError::Io(format!("Failed to read directory {:?}: {}", dir, e)))?
        {
            let entry = entry
                .map_err(|e| FluxError::Io(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                let sub_results = self.format_directory(&path, check_only)?;
                results.merge(sub_results);
            } else if path.extension().and_then(|s| s.to_str()) == Some("flux") {
                match self.format_file(&path, check_only) {
                    Ok(changed) => {
                        results.total += 1;
                        if changed {
                            results.changed += 1;
                        }
                    }
                    Err(e) => {
                        results.errors.push((path, e));
                    }
                }
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Default)]
pub struct FormatResults {
    pub total: usize,
    pub changed: usize,
    pub errors: Vec<(PathBuf, FluxError)>,
}

impl FormatResults {
    pub fn merge(&mut self, other: FormatResults) {
        self.total += other.total;
        self.changed += other.changed;
        self.errors.extend(other.errors);
    }
}

/// AST-based source code formatter
struct SourceFormatter<'a> {
    config: &'a FormatterConfig,
    indent_level: usize,
    output: String,
}

impl<'a> SourceFormatter<'a> {
    fn new(config: &'a FormatterConfig) -> Self {
        Self {
            config,
            indent_level: 0,
            output: String::new(),
        }
    }

    fn format_program(&mut self, program: &ast::Program) -> String {
        // Format package declaration
        if !program.package.is_empty() {
            self.write_line(&format!("package {};", program.package));
            self.write_line("");
        }

        // Format imports
        for import in &program.imports {
            self.write_line(&format!("import {};", import.path));
        }
        if !program.imports.is_empty() {
            self.write_line("");
        }

        // Format items
        for (i, item) in program.items.iter().enumerate() {
            if i > 0 {
                self.write_line("");
            }
            self.format_item(item);
        }

        self.output.clone()
    }

    fn format_item(&mut self, item: &ast::Item) {
        match item {
            ast::Item::Function(func) => self.format_function(func),
            ast::Item::Struct(struct_def) => self.format_struct(struct_def),
            ast::Item::Class(class_def) => self.format_class(class_def),
            ast::Item::Const(const_def) => self.format_const(const_def),
            ast::Item::ExternFunction(extern_func) => self.format_extern_function(extern_func),
        }
    }

    fn format_function(&mut self, func: &ast::Function) {
        let mut line = String::new();
        
        if func.is_async {
            line.push_str("async ");
        }
        
        line.push_str("func ");
        line.push_str(&func.name);
        line.push('(');

        for (i, param) in func.parameters.iter().enumerate() {
            if i > 0 {
                line.push_str(", ");
            }
            line.push_str(&param.name);
            line.push_str(": ");
            line.push_str(&format!("{:?}", param.type_));
        }

        line.push(')');

        if let Some(return_type) = &func.return_type {
            line.push_str(" -> ");
            line.push_str(&format!("{:?}", return_type));
        }

        line.push_str(" {");
        self.write_line(&line);

        self.indent();
        self.format_block(&func.body);
        self.dedent();

        self.write_line("}");
    }

    fn format_extern_function(&mut self, extern_func: &ast::ExternFunction) {
        let mut line = String::new();
        
        line.push_str("extern");
        
        if let Some(lib) = &extern_func.library {
            line.push_str(" \"");
            line.push_str(lib);
            line.push('"');
        }
        
        line.push_str(" func ");
        line.push_str(&extern_func.name);
        line.push('(');

        for (i, param) in extern_func.parameters.iter().enumerate() {
            if i > 0 {
                line.push_str(", ");
            }
            line.push_str(&param.name);
            line.push_str(": ");
            line.push_str(&format!("{:?}", param.type_));
        }
        
        if extern_func.is_variadic {
            if !extern_func.parameters.is_empty() {
                line.push_str(", ");
            }
            line.push_str("...");
        }

        line.push(')');

        if let Some(return_type) = &extern_func.return_type {
            line.push_str(" -> ");
            line.push_str(&format!("{:?}", return_type));
        }

        line.push(';');
        self.write_line(&line);
    }

    fn format_struct(&mut self, struct_def: &ast::Struct) {
        let line = format!("struct {} {{", struct_def.name);
        self.write_line(&line);

        self.indent();
        for field in &struct_def.fields {
            let field_line = format!("{}: {},", field.name, format!("{:?}", field.type_));
            self.write_line(&field_line);
        }
        self.dedent();

        self.write_line("}");
    }

    fn format_class(&mut self, class_def: &ast::Class) {
        let mut line = format!("class {}", class_def.name);
        
        line.push_str(" {");
        self.write_line(&line);

        self.indent();
        
        // Format fields
        for field in &class_def.fields {
            let field_line = format!("{}: {},", field.name, format!("{:?}", field.type_));
            self.write_line(&field_line);
        }

        if !class_def.fields.is_empty() && !class_def.methods.is_empty() {
            self.write_line("");
        }

        // Format methods
        for (i, method) in class_def.methods.iter().enumerate() {
            if i > 0 {
                self.write_line("");
            }
            self.format_method(method);
        }
        
        self.dedent();
        self.write_line("}");
    }

    fn format_const(&mut self, const_def: &ast::Const) {
        let line = format!("const {}: {} = {};", 
            const_def.name, 
            format!("{:?}", const_def.type_), 
            format!("{:?}", const_def.value)
        );
        self.write_line(&line);
    }

    fn format_block(&mut self, block: &ast::Block) {
        for stmt in &block.statements {
            self.format_statement(stmt);
        }
    }

    fn format_statement(&mut self, stmt: &ast::Statement) {
        match stmt {
            ast::Statement::Expression(expr) => {
                let line = format!("{};", format!("{:?}", expr));
                self.write_line(&line);
            }
            ast::Statement::Let(name, type_, value) => {
                let mut line = format!("let {}", name);
                if let Some(t) = type_ {
                    line.push_str(&format!(": {}", format!("{:?}", t)));
                }
                if let Some(v) = value {
                    line.push_str(&format!(" = {}", format!("{:?}", v)));
                }
                line.push(';');
                self.write_line(&line);
            }
            ast::Statement::Const(name, type_, value) => {
                let line = format!("const {}: {} = {};", 
                    name, 
                    format!("{:?}", type_), 
                    format!("{:?}", value)
                );
                self.write_line(&line);
            }
            ast::Statement::Assignment(target, value) => {
                let line = format!("{} = {};", 
                    format!("{:?}", target), 
                    format!("{:?}", value)
                );
                self.write_line(&line);
            }
            ast::Statement::Return(value) => {
                let line = if let Some(v) = value {
                    format!("return {};", format!("{:?}", v))
                } else {
                    "return;".to_string()
                };
                self.write_line(&line);
            }
            ast::Statement::Break(value) => {
                let line = if let Some(v) = value {
                    format!("break {};", format!("{:?}", v))
                } else {
                    "break;".to_string()
                };
                self.write_line(&line);
            }
            ast::Statement::Continue => {
                self.write_line("continue;");
            }
            ast::Statement::Go(expr) => {
                let line = format!("go {};", format!("{:?}", expr));
                self.write_line(&line);
            }
            ast::Statement::If(cond, then_block, else_block) => {
                let line = format!("if {} {{", format!("{:?}", cond));
                self.write_line(&line);
                self.indent();
                self.format_block(then_block);
                self.dedent();
                if let Some(else_block) = else_block {
                    self.write_line("} else {");
                    self.indent();
                    self.format_block(else_block);
                    self.dedent();
                }
                self.write_line("}");
            }
            ast::Statement::While(cond, body) => {
                let line = format!("while {} {{", format!("{:?}", cond));
                self.write_line(&line);
                self.indent();
                self.format_block(body);
                self.dedent();
                self.write_line("}");
            }
            ast::Statement::For(var, iter, body) => {
                let line = format!("for {} in {} {{", var, format!("{:?}", iter));
                self.write_line(&line);
                self.indent();
                self.format_block(body);
                self.dedent();
                self.write_line("}");
            }
            ast::Statement::Match(expr, arms) => {
                let line = format!("match {} {{", format!("{:?}", expr));
                self.write_line(&line);
                self.indent();
                for arm in arms {
                    let arm_line = format!("{} => {},", format!("{:?}", arm.pattern), format!("{:?}", arm.body));
                    self.write_line(&arm_line);
                }
                self.dedent();
                self.write_line("}");
            }
        }
    }

    fn format_method(&mut self, method: &ast::Method) {
        let mut line = String::new();
        
        line.push_str("func ");
        line.push_str(&method.name);
        line.push('(');

        for (i, param) in method.parameters.iter().enumerate() {
            if i > 0 {
                line.push_str(", ");
            }
            line.push_str(&param.name);
            line.push_str(": ");
            line.push_str(&format!("{:?}", param.type_));
        }

        line.push(')');

        line.push_str(" -> ");
        line.push_str(&format!("{:?}", method.return_type));

        line.push_str(" {");
        self.write_line(&line);

        self.indent();
        self.format_block(&method.body);
        self.dedent();

        self.write_line("}");
    }

    fn write_line(&mut self, line: &str) {
        if !line.is_empty() {
            self.write_indent();
            self.output.push_str(line);
        }
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        if self.config.use_tabs {
            for _ in 0..self.indent_level {
                self.output.push('\t');
            }
        } else {
            for _ in 0..(self.indent_level * self.config.indent_size) {
                self.output.push(' ');
            }
        }
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }
}

/// Code linter for Flux source files
pub struct Linter {
    context: CliContext,
    config: LinterConfig,
}

#[derive(Debug, Clone)]
pub struct LinterConfig {
    pub max_line_length: usize,
    pub max_function_length: usize,
    pub max_complexity: usize,
    pub enforce_naming_convention: bool,
    pub require_documentation: bool,
}

impl Default for LinterConfig {
    fn default() -> Self {
        Self {
            max_line_length: 100,
            max_function_length: 50,
            max_complexity: 10,
            enforce_naming_convention: true,
            require_documentation: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LintIssue {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub severity: LintSeverity,
    pub rule: String,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

impl Linter {
    pub fn new(context: CliContext) -> Self {
        Self {
            context,
            config: LinterConfig::default(),
        }
    }

    pub fn with_config(context: CliContext, config: LinterConfig) -> Self {
        Self { context, config }
    }

    /// Lint a single file
    pub fn lint_file(&self, path: &Path) -> FluxResult<Vec<LintIssue>> {
        self.context.verbose(&format!("Linting file: {:?}", path));

        let source = fs::read_to_string(path)
            .map_err(|e| FluxError::Io(format!("Failed to read file {:?}: {}", path, e)))?;

        self.lint_source(path, &source)
    }

    /// Lint source code string
    pub fn lint_source(&self, file_path: &Path, source: &str) -> FluxResult<Vec<LintIssue>> {
        let mut issues = Vec::new();

        // Parse the source code
        let lexer = FluxLexer::new(source.to_string());
        let mut parser = match FluxParser::new(lexer) {
            Ok(p) => p,
            Err(e) => {
                issues.push(LintIssue {
                    file: file_path.to_path_buf(),
                    line: 1,
                    column: 1,
                    severity: LintSeverity::Error,
                    rule: "parser-error".to_string(),
                    message: format!("Parser initialization failed: {}", e),
                    suggestion: None,
                });
                return Ok(issues);
            }
        };
        
        match parser.parse_program() {
            Ok(program) => {
                // Run AST-based linting rules
                issues.extend(self.check_naming_conventions(&program, file_path));
                issues.extend(self.check_function_complexity(&program, file_path));
                issues.extend(self.check_documentation(&program, file_path));
            }
            Err(e) => {
                issues.push(LintIssue {
                    file: file_path.to_path_buf(),
                    line: 1,
                    column: 1,
                    severity: LintSeverity::Error,
                    rule: "syntax-error".to_string(),
                    message: format!("Syntax error: {}", e),
                    suggestion: None,
                });
            }
        }

        // Run text-based linting rules
        issues.extend(self.check_line_length(source, file_path));
        issues.extend(self.check_whitespace(source, file_path));

        Ok(issues)
    }

    fn check_naming_conventions(&self, program: &ast::Program, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        if !self.config.enforce_naming_convention {
            return issues;
        }

        for item in &program.items {
            match item {
                ast::Item::Function(func) => {
                    if !is_snake_case(&func.name) {
                        issues.push(LintIssue {
                            file: file_path.to_path_buf(),
                            line: 1, // TODO: Add position tracking
                            column: 1,
                            severity: LintSeverity::Warning,
                            rule: "naming-convention".to_string(),
                            message: format!("Function '{}' should use snake_case", func.name),
                            suggestion: Some(format!("Consider renaming to '{}'", to_snake_case(&func.name))),
                        });
                    }
                }
                ast::Item::Struct(struct_def) => {
                    if !is_pascal_case(&struct_def.name) {
                        issues.push(LintIssue {
                            file: file_path.to_path_buf(),
                            line: 1,
                            column: 1,
                            severity: LintSeverity::Warning,
                            rule: "naming-convention".to_string(),
                            message: format!("Struct '{}' should use PascalCase", struct_def.name),
                            suggestion: Some(format!("Consider renaming to '{}'", to_pascal_case(&struct_def.name))),
                        });
                    }
                }
                ast::Item::Class(class_def) => {
                    if !is_pascal_case(&class_def.name) {
                        issues.push(LintIssue {
                            file: file_path.to_path_buf(),
                            line: 1,
                            column: 1,
                            severity: LintSeverity::Warning,
                            rule: "naming-convention".to_string(),
                            message: format!("Class '{}' should use PascalCase", class_def.name),
                            suggestion: Some(format!("Consider renaming to '{}'", to_pascal_case(&class_def.name))),
                        });
                    }
                }
                ast::Item::Const(const_def) => {
                    if !is_screaming_snake_case(&const_def.name) {
                        issues.push(LintIssue {
                            file: file_path.to_path_buf(),
                            line: 1,
                            column: 1,
                            severity: LintSeverity::Warning,
                            rule: "naming-convention".to_string(),
                            message: format!("Constant '{}' should use SCREAMING_SNAKE_CASE", const_def.name),
                            suggestion: Some(format!("Consider renaming to '{}'", to_screaming_snake_case(&const_def.name))),
                        });
                    }
                }
                ast::Item::ExternFunction(extern_func) => {
                    if !is_snake_case(&extern_func.name) {
                        issues.push(LintIssue {
                            file: file_path.to_path_buf(),
                            line: 1,
                            column: 1,
                            severity: LintSeverity::Warning,
                            rule: "naming-convention".to_string(),
                            message: format!("Extern function '{}' should use snake_case", extern_func.name),
                            suggestion: Some(format!("Consider renaming to '{}'", to_snake_case(&extern_func.name))),
                        });
                    }
                }
            }
        }

        issues
    }

    fn check_function_complexity(&self, program: &ast::Program, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for item in &program.items {
            if let ast::Item::Function(func) = item {
                let line_count = func.body.statements.len();
                if line_count > self.config.max_function_length {
                    issues.push(LintIssue {
                        file: file_path.to_path_buf(),
                        line: 1,
                        column: 1,
                        severity: LintSeverity::Warning,
                        rule: "function-length".to_string(),
                        message: format!("Function '{}' is too long ({} lines, max {})", 
                            func.name, line_count, self.config.max_function_length),
                        suggestion: Some("Consider breaking this function into smaller functions".to_string()),
                    });
                }
            }
        }

        issues
    }

    fn check_documentation(&self, program: &ast::Program, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        if !self.config.require_documentation {
            return issues;
        }

        for item in &program.items {
            match item {
                ast::Item::Function(func) => {
                    // TODO: Check if function has documentation comment
                    if func.name != "main" { // Skip main function
                        issues.push(LintIssue {
                            file: file_path.to_path_buf(),
                            line: 1,
                            column: 1,
                            severity: LintSeverity::Info,
                            rule: "missing-documentation".to_string(),
                            message: format!("Function '{}' is missing documentation", func.name),
                            suggestion: Some("Add a documentation comment above the function".to_string()),
                        });
                    }
                }
                _ => {}
            }
        }

        issues
    }

    fn check_line_length(&self, source: &str, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            if line.len() > self.config.max_line_length {
                issues.push(LintIssue {
                    file: file_path.to_path_buf(),
                    line: line_num + 1,
                    column: self.config.max_line_length + 1,
                    severity: LintSeverity::Warning,
                    rule: "line-length".to_string(),
                    message: format!("Line too long ({} characters, max {})", 
                        line.len(), self.config.max_line_length),
                    suggestion: Some("Consider breaking this line".to_string()),
                });
            }
        }

        issues
    }

    fn check_whitespace(&self, source: &str, file_path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                issues.push(LintIssue {
                    file: file_path.to_path_buf(),
                    line: line_num + 1,
                    column: line.len(),
                    severity: LintSeverity::Info,
                    rule: "trailing-whitespace".to_string(),
                    message: "Line has trailing whitespace".to_string(),
                    suggestion: Some("Remove trailing whitespace".to_string()),
                });
            }
        }

        issues
    }
}

/// Test runner for Flux test functions
pub struct TestRunner {
    context: CliContext,
}

#[derive(Debug)]
pub struct TestResults {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub test_cases: Vec<TestCase>,
}

#[derive(Debug)]
pub struct TestCase {
    pub name: String,
    pub result: TestResult,
    pub duration: std::time::Duration,
    pub output: String,
}

#[derive(Debug, Clone)]
pub enum TestResult {
    Passed,
    Failed(String),
    Ignored,
}

impl TestRunner {
    pub fn new(context: CliContext) -> Self {
        Self { context }
    }

    /// Run tests in a project
    pub fn run_tests(&self, project_path: &Path, filter: Option<&str>) -> FluxResult<TestResults> {
        self.context.info("Running tests...");

        // TODO: Implement actual test discovery and execution
        // For now, return mock results
        let mut results = TestResults {
            passed: 0,
            failed: 0,
            ignored: 0,
            test_cases: Vec::new(),
        };

        // Mock test cases
        let test_cases = vec![
            ("test_basic_arithmetic", TestResult::Passed),
            ("test_string_operations", TestResult::Passed),
            ("test_error_handling", TestResult::Failed("assertion failed".to_string())),
        ];

        for (name, result) in test_cases {
            if let Some(filter_str) = filter {
                if !name.contains(filter_str) {
                    continue;
                }
            }

            let test_case = TestCase {
                name: name.to_string(),
                result: result.clone(),
                duration: std::time::Duration::from_millis(10),
                output: String::new(),
            };

            match result {
                TestResult::Passed => results.passed += 1,
                TestResult::Failed(_) => results.failed += 1,
                TestResult::Ignored => results.ignored += 1,
            }

            results.test_cases.push(test_case);
        }

        Ok(results)
    }
}

// Helper functions for naming conventions
fn is_snake_case(s: &str) -> bool {
    s.chars().all(|c| c.is_lowercase() || c.is_numeric() || c == '_')
        && !s.starts_with('_')
        && !s.ends_with('_')
        && !s.contains("__")
}

fn is_pascal_case(s: &str) -> bool {
    !s.is_empty() 
        && s.chars().next().unwrap().is_uppercase()
        && s.chars().all(|c| c.is_alphanumeric())
}

fn is_screaming_snake_case(s: &str) -> bool {
    s.chars().all(|c| c.is_uppercase() || c.is_numeric() || c == '_')
        && !s.starts_with('_')
        && !s.ends_with('_')
        && !s.contains("__")
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_upper = false;

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !prev_was_upper {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
            prev_was_upper = true;
        } else {
            result.push(c);
            prev_was_upper = false;
        }
    }

    result
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

fn to_screaming_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}