//! Integration tests for the Flux CLI
//! 
//! Tests the command-line interface functionality including build, run, test, format, and lint commands.

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use flux_compiler::cli::{Cli, Commands, BuildMode, CliContext, CompilerDriver};
use flux_compiler::tools::{Formatter, Linter, TestRunner};
use flux_compiler::package::{Project, ProjectInstance};

/// Create a temporary Flux project for testing
fn create_test_project() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let project_path = temp_dir.path().to_path_buf();
    
    // Create project structure
    Project::create_structure(&project_path).expect("Failed to create project structure");
    
    // Create flux.toml
    let flux_toml = r#"[package]
name = "test-project"
version = "0.1.0"
authors = ["Test Author <test@example.com>"]
description = "A test Flux project"

[dependencies]

[build]
target = "native"
optimization_level = "debug"
debug_info = true
incremental = true
parallel = true
"#;
    fs::write(project_path.join("flux.toml"), flux_toml).expect("Failed to write flux.toml");
    
    (temp_dir, project_path)
}

#[test]
fn test_cli_context() {
    let context = CliContext::new(true, false);
    
    // Test verbose output
    context.verbose("This is a verbose message");
    context.info("This is an info message");
    context.success("This is a success message");
    context.warn("This is a warning message");
    
    // Test elapsed time
    let elapsed = context.elapsed();
    assert!(elapsed.as_millis() >= 0);
}

#[test]
fn test_project_loading() {
    let (_temp_dir, project_path) = create_test_project();
    
    // Test loading project
    let project = ProjectInstance::load(&project_path).expect("Failed to load project");
    assert_eq!(project.name(), "test-project");
}

#[test]
fn test_build_command() {
    let (_temp_dir, project_path) = create_test_project();
    let context = CliContext::new(false, true); // quiet mode for tests
    let driver = CompilerDriver::new(context);
    
    let build_command = Commands::Build {
        mode: BuildMode::Debug,
        target: None,
        output: None,
        optimize: false,
        check: false,
        progress: false,
        path: project_path,
    };
    
    // Test build command (should succeed even with mock implementation)
    let result = driver.build(&build_command);
    assert!(result.is_ok(), "Build command failed: {:?}", result);
}

#[test]
fn test_check_command() {
    let (_temp_dir, project_path) = create_test_project();
    let context = CliContext::new(false, true);
    let driver = CompilerDriver::new(context);
    
    let check_command = Commands::Build {
        mode: BuildMode::Check,
        target: None,
        output: None,
        optimize: false,
        check: true,
        progress: false,
        path: project_path,
    };
    
    let result = driver.build(&check_command);
    assert!(result.is_ok(), "Check command failed: {:?}", result);
}

#[test]
fn test_formatter() {
    let (_temp_dir, project_path) = create_test_project();
    let context = CliContext::new(false, true);
    let formatter = Formatter::new(context);
    
    // Test formatting a simple Flux source
    let source = r#"func main() {
    println("Hello, World!");
}"#;
    let formatted = formatter.format_source(source);
    
    // Should succeed even if formatting is basic
    assert!(formatted.is_ok(), "Formatting failed: {:?}", formatted);
}

#[test]
fn test_linter() {
    let (_temp_dir, project_path) = create_test_project();
    let context = CliContext::new(false, true);
    let linter = Linter::new(context);
    
    // Test linting a simple Flux source
    let source = r#"func main() {
    println("Hello, World!");
}"#;
    
    let issues = linter.lint_source(Path::new("test.flux"), source);
    assert!(issues.is_ok(), "Linting failed: {:?}", issues);
    
    let issues = issues.unwrap();
    // Should have some issues (like missing documentation)
    assert!(!issues.is_empty(), "Expected some lint issues");
}

#[test]
fn test_test_runner() {
    let (_temp_dir, project_path) = create_test_project();
    let context = CliContext::new(false, true);
    let test_runner = TestRunner::new(context);
    
    // Test running tests
    let results = test_runner.run_tests(&project_path, None);
    assert!(results.is_ok(), "Test runner failed: {:?}", results);
    
    let results = results.unwrap();
    assert!(results.passed > 0 || results.failed > 0, "Expected some test results");
}

#[test]
fn test_build_modes() {
    let (_temp_dir, project_path) = create_test_project();
    let context = CliContext::new(false, true);
    let driver = CompilerDriver::new(context);
    
    // Test debug build
    let debug_command = Commands::Build {
        mode: BuildMode::Debug,
        target: None,
        output: None,
        optimize: false,
        check: false,
        progress: false,
        path: project_path.clone(),
    };
    assert!(driver.build(&debug_command).is_ok());
    
    // Test release build
    let release_command = Commands::Build {
        mode: BuildMode::Release,
        target: None,
        output: None,
        optimize: true,
        check: false,
        progress: false,
        path: project_path.clone(),
    };
    assert!(driver.build(&release_command).is_ok());
}

#[test]
fn test_progress_reporting() {
    let context = CliContext::new(true, false); // verbose mode
    
    // Test progress bar creation
    let progress = context.progress_bar(10, "Testing");
    assert!(progress.is_some(), "Expected progress bar in verbose mode");
    
    if let Some(pb) = progress {
        pb.inc(1);
        pb.set_message("Step 1");
        pb.finish_with_message("Complete");
    }
}

#[test]
fn test_error_handling() {
    let context = CliContext::new(false, false);
    
    // Test error reporting
    context.error("This is an error message");
    context.warn("This is a warning message");
}

#[test]
fn test_file_discovery() {
    let (_temp_dir, project_path) = create_test_project();
    
    // Create additional source files
    let src_dir = project_path.join("src");
    fs::write(src_dir.join("module1.flux"), "// Module 1").expect("Failed to write module1.flux");
    fs::write(src_dir.join("module2.flux"), "// Module 2").expect("Failed to write module2.flux");
    
    // Test file discovery
    let source_files = Project::list_source_files(&project_path).expect("Failed to list source files");
    assert!(source_files.len() >= 3, "Expected at least 3 source files"); // main.flux, lib.flux, module1.flux, module2.flux
    
    let test_files = Project::list_test_files(&project_path).expect("Failed to list test files");
    assert!(test_files.len() >= 1, "Expected at least 1 test file");
}

#[test]
fn test_project_validation() {
    let (_temp_dir, project_path) = create_test_project();
    
    // Test project validation
    assert!(Project::is_flux_project(&project_path), "Should be a valid Flux project");
    
    // Test finding project root
    let nested_dir = project_path.join("src").join("nested");
    fs::create_dir_all(&nested_dir).expect("Failed to create nested directory");
    
    let found_root = Project::find_project_root(&nested_dir);
    assert!(found_root.is_some(), "Should find project root from nested directory");
    assert_eq!(found_root.unwrap(), project_path, "Should find correct project root");
}

#[test]
fn test_compilation_phases() {
    let (_temp_dir, project_path) = create_test_project();
    let context = CliContext::new(true, false); // verbose to see phases
    let driver = CompilerDriver::new(context);
    
    let build_command = Commands::Build {
        mode: BuildMode::Debug,
        target: None,
        output: None,
        optimize: false,
        check: false,
        progress: true, // Enable progress to test phases
        path: project_path,
    };
    
    // This should go through all compilation phases
    let result = driver.build(&build_command);
    assert!(result.is_ok(), "Compilation phases failed: {:?}", result);
}