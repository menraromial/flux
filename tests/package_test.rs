//! Tests for the package management system

use flux_compiler::package::*;
use flux_compiler::error::*;
use std::fs;
use tempfile::TempDir;

/// Test project configuration parsing
#[test]
fn test_project_config_parsing() {
    let toml_content = r#"
[package]
name = "test-project"
version = "1.0.0"
description = "A test project"
authors = ["Test Author <test@example.com>"]
license = "MIT"
edition = "2024"

[dependencies]
std-collections = "1.0.0"
http-client = "^2.1.0"

[dev-dependencies]
test-framework = "1.0.0"

[build]
optimization = "speed"
debug = true
incremental = true
parallel = true
"#;

    let config = ProjectConfig::from_toml(toml_content).unwrap();
    
    assert_eq!(config.package.name, "test-project");
    assert_eq!(config.package.version, "1.0.0");
    assert_eq!(config.package.description, Some("A test project".to_string()));
    assert_eq!(config.package.authors, vec!["Test Author <test@example.com>"]);
    assert_eq!(config.package.license, Some("MIT".to_string()));
    assert_eq!(config.package.edition, "2024");
    
    assert!(config.dependencies.contains_key("std-collections"));
    assert!(config.dependencies.contains_key("http-client"));
    assert!(config.dev_dependencies.contains_key("test-framework"));
    
    assert_eq!(config.build.optimization_level, ConfigOptimizationLevel::Speed);
    assert!(config.build.debug_info);
    assert!(config.build.incremental);
    assert!(config.build.parallel);
}

/// Test project configuration round-trip (parse -> serialize -> parse)
#[test]
fn test_project_config_roundtrip() {
    let original_config = ProjectConfig::new("test-project".to_string(), "1.0.0".to_string());
    
    let toml_content = original_config.to_toml();
    let parsed_config = ProjectConfig::from_toml(&toml_content).unwrap();
    
    assert_eq!(original_config.package.name, parsed_config.package.name);
    assert_eq!(original_config.package.version, parsed_config.package.version);
    assert_eq!(original_config.package.edition, parsed_config.package.edition);
}

/// Test project structure creation
#[test]
fn test_project_structure_creation() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    Project::create_structure(project_root).unwrap();
    
    // Check that all expected directories exist
    assert!(project_root.join("src").exists());
    assert!(project_root.join("tests").exists());
    assert!(project_root.join("examples").exists());
    assert!(project_root.join("docs").exists());
    assert!(project_root.join("target").exists());
    assert!(project_root.join(".flux").exists());
    
    // Check that expected files exist
    assert!(project_root.join("src").join("main.flux").exists());
    assert!(project_root.join("src").join("lib.flux").exists());
    assert!(project_root.join("tests").join("lib_test.flux").exists());
    assert!(project_root.join("examples").join("basic.flux").exists());
    assert!(project_root.join("README.md").exists());
    assert!(project_root.join(".gitignore").exists());
    
    // Check file contents
    let main_content = fs::read_to_string(project_root.join("src").join("main.flux")).unwrap();
    assert!(main_content.contains("func main()"));
    assert!(main_content.contains("println(\"Hello, Flux!\");"));
    
    let lib_content = fs::read_to_string(project_root.join("src").join("lib.flux")).unwrap();
    assert!(lib_content.contains("pub func hello()"));
}

/// Test project detection
#[test]
fn test_project_detection() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    // Initially not a Flux project
    assert!(!Project::is_flux_project(project_root));
    
    // Create flux.toml
    let config = ProjectConfig::new("test-project".to_string(), "1.0.0".to_string());
    config.write_to_file(&project_root.join("flux.toml")).unwrap();
    
    // Now it should be detected as a Flux project
    assert!(Project::is_flux_project(project_root));
}

/// Test project root finding
#[test]
fn test_project_root_finding() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    // Create project structure
    Project::create_structure(project_root).unwrap();
    let config = ProjectConfig::new("test-project".to_string(), "1.0.0".to_string());
    config.write_to_file(&project_root.join("flux.toml")).unwrap();
    
    // Create nested directory
    let nested_dir = project_root.join("src").join("nested");
    fs::create_dir_all(&nested_dir).unwrap();
    
    // Should find project root from nested directory
    let found_root = Project::find_project_root(&nested_dir).unwrap();
    assert_eq!(found_root, project_root);
    
    // Should return None for non-project directory
    let non_project_dir = TempDir::new().unwrap();
    assert!(Project::find_project_root(non_project_dir.path()).is_none());
}

/// Test source file listing
#[test]
fn test_source_file_listing() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    Project::create_structure(project_root).unwrap();
    
    // Create additional source files
    fs::write(project_root.join("src").join("module1.flux"), "// Module 1").unwrap();
    fs::write(project_root.join("src").join("module2.flux"), "// Module 2").unwrap();
    
    // Create subdirectory with source file
    let subdir = project_root.join("src").join("submodule");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("sub.flux"), "// Submodule").unwrap();
    
    let source_files = Project::list_source_files(project_root).unwrap();
    
    // Should find all .flux files in src directory
    assert!(source_files.len() >= 4); // main.flux, lib.flux, module1.flux, module2.flux, sub.flux
    
    let file_names: Vec<String> = source_files.iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    
    assert!(file_names.contains(&"main.flux".to_string()));
    assert!(file_names.contains(&"lib.flux".to_string()));
    assert!(file_names.contains(&"module1.flux".to_string()));
    assert!(file_names.contains(&"module2.flux".to_string()));
    assert!(file_names.contains(&"sub.flux".to_string()));
}

/// Test dependency specification parsing
#[test]
fn test_dependency_spec_parsing() {
    let version_spec = DependencySpec::from_version_string("1.2.3");
    assert_eq!(version_spec.version, Some("1.2.3".to_string()));
    assert!(version_spec.path.is_none());
    assert!(version_spec.git.is_none());
    
    let path_spec = DependencySpec::from_path("../local-lib");
    assert!(path_spec.version.is_none());
    assert_eq!(path_spec.path, Some("../local-lib".to_string()));
    assert!(path_spec.git.is_none());
    
    let git_spec = DependencySpec::from_git("https://github.com/user/repo.git", Some("main"), None, None);
    assert!(git_spec.version.is_none());
    assert!(git_spec.path.is_none());
    assert_eq!(git_spec.git, Some("https://github.com/user/repo.git".to_string()));
    assert_eq!(git_spec.branch, Some("main".to_string()));
}

/// Test version requirement parsing and matching
#[test]
fn test_version_requirements() {
    // Exact version
    let exact = VersionReq::parse("1.2.3").unwrap();
    assert!(exact.matches("1.2.3"));
    assert!(!exact.matches("1.2.4"));
    
    // Caret version (^1.2.3 matches >=1.2.3 <2.0.0)
    let caret = VersionReq::parse("^1.2.3").unwrap();
    assert!(caret.matches("1.2.3"));
    assert!(caret.matches("1.2.4"));
    assert!(caret.matches("1.9.9"));
    assert!(!caret.matches("2.0.0"));
    assert!(!caret.matches("0.9.9"));
    
    // Tilde version (~1.2.3 matches >=1.2.3 <1.3.0)
    let tilde = VersionReq::parse("~1.2.3").unwrap();
    assert!(tilde.matches("1.2.3"));
    assert!(tilde.matches("1.2.4"));
    assert!(!tilde.matches("1.3.0"));
    assert!(!tilde.matches("2.0.0"));
    
    // Wildcard version (1.2.* matches 1.2.x)
    let wildcard = VersionReq::parse("1.2.*").unwrap();
    assert!(wildcard.matches("1.2.0"));
    assert!(wildcard.matches("1.2.999"));
    assert!(!wildcard.matches("1.3.0"));
    assert!(!wildcard.matches("2.2.0"));
    
    // Any version
    let any = VersionReq::parse("*").unwrap();
    assert!(any.matches("0.0.1"));
    assert!(any.matches("999.999.999"));
}

/// Test package manager initialization
#[test]
fn test_package_manager_init() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    let package_manager = PackageManager::init_project(
        project_root,
        "test-project".to_string(),
        "1.0.0".to_string(),
    ).unwrap();
    
    assert_eq!(package_manager.config().package.name, "test-project");
    assert_eq!(package_manager.config().package.version, "1.0.0");
    assert_eq!(package_manager.project_root(), project_root);
    
    // Check that project structure was created
    assert!(project_root.join("flux.toml").exists());
    assert!(project_root.join("src").exists());
    assert!(project_root.join("src").join("main.flux").exists());
}

/// Test package manager loading existing project
#[test]
fn test_package_manager_load() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    // Create project first
    PackageManager::init_project(
        project_root,
        "test-project".to_string(),
        "1.0.0".to_string(),
    ).unwrap();
    
    // Load existing project
    let package_manager = PackageManager::new(project_root).unwrap();
    
    assert_eq!(package_manager.config().package.name, "test-project");
    assert_eq!(package_manager.config().package.version, "1.0.0");
}

/// Test package manager error handling
#[test]
fn test_package_manager_errors() {
    let temp_dir = TempDir::new().unwrap();
    let non_project_dir = temp_dir.path();
    
    // Should fail to load non-existent project
    let result = PackageManager::new(non_project_dir);
    assert!(result.is_err());
    
    match result.unwrap_err() {
        FluxError::Package(PackageError::ConfigNotFound(_)) => {
            // Expected error
        }
        _ => panic!("Expected ConfigNotFound error"),
    }
}

/// Test project metadata loading
#[test]
fn test_project_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    
    // Create project
    PackageManager::init_project(
        project_root,
        "test-project".to_string(),
        "1.0.0".to_string(),
    ).unwrap();
    
    // Load metadata
    let metadata = ProjectMetadata::load(project_root).unwrap();
    
    assert_eq!(metadata.name, "test-project");
    assert_eq!(metadata.version, "1.0.0");
    assert_eq!(metadata.root_path, project_root);
    
    // Should have source files
    assert!(!metadata.source_files.is_empty());
    assert!(metadata.is_binary()); // Has main.flux
    assert!(metadata.is_library()); // Has lib.flux
}

/// Test lock file creation and parsing
#[test]
fn test_lock_file() {
    let lock_content = r#"
[metadata]
version = "1"

[dependencies.test-dep]
version = "1.0.0"
source = "registry"
checksum = "abc123"
dependencies = ["sub-dep"]

[dependencies.sub-dep]
version = "0.5.0"
source = "registry"
checksum = "def456"
dependencies = []
"#;

    let lock_file = LockFile::from_toml(lock_content).unwrap();
    
    assert_eq!(lock_file.version, "1");
    assert!(lock_file.dependencies.contains_key("test-dep"));
    assert!(lock_file.dependencies.contains_key("sub-dep"));
    
    let test_dep = &lock_file.dependencies["test-dep"];
    assert_eq!(test_dep.version, "1.0.0");
    assert_eq!(test_dep.source, "registry");
    assert_eq!(test_dep.checksum, "abc123");
    assert_eq!(test_dep.dependencies, vec!["sub-dep"]);
    
    // Test round-trip
    let serialized = lock_file.to_toml();
    let reparsed = LockFile::from_toml(&serialized).unwrap();
    assert_eq!(lock_file.version, reparsed.version);
    assert_eq!(lock_file.dependencies.len(), reparsed.dependencies.len());
}

/// Test invalid configuration handling
#[test]
fn test_invalid_config() {
    let invalid_toml = r#"
[package]
# Missing required name field
version = "1.0.0"
"#;

    let result = ProjectConfig::from_toml(invalid_toml);
    assert!(result.is_err());
    
    match result.unwrap_err() {
        FluxError::Package(PackageError::InvalidConfig(_)) => {
            // Expected error
        }
        _ => panic!("Expected InvalidConfig error"),
    }
}