//! Project directory structure creation and management

use crate::error::{FluxError, PackageError};
use std::fs;
use std::path::Path;

/// Project structure manager
pub struct Project {
    metadata: ProjectMetadata,
}

/// Project instance for build operations
#[derive(Debug, Clone)]
pub struct ProjectInstance {
    pub metadata: ProjectMetadata,
    pub config: super::ProjectConfig,
}

impl ProjectInstance {
    /// Load a project from a directory
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, FluxError> {
        let project_root = if Project::is_flux_project(&path) {
            path.as_ref().to_path_buf()
        } else if let Some(root) = Project::find_project_root(&path) {
            root
        } else {
            return Err(FluxError::Package(PackageError::ConfigNotFound(
                path.as_ref().join("flux.toml")
            )));
        };

        let metadata = ProjectMetadata::load(&project_root)?;
        let config = super::ProjectConfig::from_file(&project_root.join("flux.toml"))?;

        Ok(ProjectInstance { metadata, config })
    }

    /// Get the project name
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Build the project
    pub fn build(&self, build_config: &super::BuildConfig) -> Result<(), FluxError> {
        use super::{BuildSystem, PackageMetadata};

        // Create package metadata from project metadata
        let package_metadata = PackageMetadata {
            config: self.config.clone(),
            root_path: self.metadata.root_path.clone(),
            resolved_dependencies: vec![],
            build_metadata: super::BuildMetadata::new(),
            lock_file: None,
        };

        // Create and run build system
        let mut build_system = BuildSystem::new(package_metadata)?;
        *build_system.build_config_mut() = build_config.clone();
        
        let result = build_system.build()?;
        
        if !result.success {
            return Err(FluxError::Package(PackageError::ProjectCreationFailed(
                format!("Build failed with {} errors", result.errors.len())
            )));
        }

        Ok(())
    }

    /// Run the project executable
    pub fn run(&self, args: &[String]) -> Result<(), FluxError> {
        use std::process::Command;

        // Find the executable
        let target_dir = self.metadata.root_path.join("target").join("debug");
        let executable_name = if cfg!(windows) {
            format!("{}.exe", self.metadata.name)
        } else {
            self.metadata.name.clone()
        };
        let executable_path = target_dir.join(&executable_name);

        if !executable_path.exists() {
            return Err(FluxError::Package(PackageError::ProjectCreationFailed(
                format!("Executable not found: {}", executable_path.display())
            )));
        }

        // Run the executable
        let mut cmd = Command::new(&executable_path);
        cmd.args(args);

        let status = cmd.status()
            .map_err(|e| FluxError::Package(PackageError::IoError(
                format!("Failed to run executable: {}", e)
            )))?;

        if !status.success() {
            return Err(FluxError::Package(PackageError::ProjectCreationFailed(
                format!("Executable exited with code: {:?}", status.code())
            )));
        }

        Ok(())
    }

    /// Run tests for the project
    pub fn test(&self, build_config: &super::BuildConfig, filter: Option<&str>) -> Result<TestResults, FluxError> {
        // For now, return mock test results
        // In a real implementation, this would compile and run test files
        
        let mut results = TestResults {
            passed: 0,
            failed: 0,
        };

        // Mock some test results
        if filter.is_none() || filter == Some("basic") {
            results.passed += 2;
        }
        if filter.is_none() || filter == Some("advanced") {
            results.failed += 1;
        }

        Ok(results)
    }
}

#[derive(Debug)]
pub struct TestResults {
    pub passed: usize,
    pub failed: usize,
}

impl Project {
    /// Create a standard Flux project directory structure
    pub fn create_structure<P: AsRef<Path>>(project_root: P) -> Result<(), FluxError> {
        let root = project_root.as_ref();
        
        // Create main directories
        Self::create_dir(root)?;
        Self::create_dir(&root.join("src"))?;
        Self::create_dir(&root.join("tests"))?;
        Self::create_dir(&root.join("examples"))?;
        Self::create_dir(&root.join("docs"))?;
        Self::create_dir(&root.join("target"))?;
        Self::create_dir(&root.join(".flux"))?;
        
        // Create main.flux file
        let main_content = r#"// Main entry point for the Flux application
func main() {
    println("Hello, Flux!");
}
"#;
        Self::create_file(&root.join("src").join("main.flux"), main_content)?;
        
        // Create lib.flux file
        let lib_content = r#"// Library module for the Flux project
// Export public functions and types here

pub func hello() -> string {
    return "Hello from lib!";
}
"#;
        Self::create_file(&root.join("src").join("lib.flux"), lib_content)?;
        
        // Create a basic test file
        let test_content = r#"// Tests for the main library
import std/test;
import lib;

#[test]
func test_hello() {
    let result = lib.hello();
    test.assert_eq(result, "Hello from lib!");
}
"#;
        Self::create_file(&root.join("tests").join("lib_test.flux"), test_content)?;
        
        // Create an example file
        let example_content = r#"// Example usage of the library
import lib;

func main() {
    let message = lib.hello();
    println(message);
}
"#;
        Self::create_file(&root.join("examples").join("basic.flux"), example_content)?;
        
        // Create README.md
        let readme_content = r#"# Flux Project

This is a Flux programming language project.

## Building

```bash
flux build
```

## Running

```bash
flux run
```

## Testing

```bash
flux test
```

## Examples

```bash
flux run --example basic
```
"#;
        Self::create_file(&root.join("README.md"), readme_content)?;
        
        // Create .gitignore
        let gitignore_content = r#"# Flux build artifacts
/target/
*.flux-build
*.flux-cache

# IDE files
.vscode/
.idea/
*.swp
*.swo

# OS files
.DS_Store
Thumbs.db

# Temporary files
*.tmp
*.temp
"#;
        Self::create_file(&root.join(".gitignore"), gitignore_content)?;
        
        Ok(())
    }
    
    /// Create a directory if it doesn't exist
    fn create_dir<P: AsRef<Path>>(path: P) -> Result<(), FluxError> {
        if !path.as_ref().exists() {
            fs::create_dir_all(path.as_ref())
                .map_err(|e| FluxError::Package(PackageError::ProjectCreationFailed(
                    format!("Failed to create directory {}: {}", path.as_ref().display(), e)
                )))?;
        }
        Ok(())
    }
    
    /// Create a file with content
    fn create_file<P: AsRef<Path>>(path: P, content: &str) -> Result<(), FluxError> {
        fs::write(path.as_ref(), content)
            .map_err(|e| FluxError::Package(PackageError::ProjectCreationFailed(
                format!("Failed to create file {}: {}", path.as_ref().display(), e)
            )))?;
        Ok(())
    }
    
    /// Check if a directory is a valid Flux project
    pub fn is_flux_project<P: AsRef<Path>>(path: P) -> bool {
        let flux_toml = path.as_ref().join("flux.toml");
        flux_toml.exists()
    }
    
    /// Find the project root by walking up the directory tree
    pub fn find_project_root<P: AsRef<Path>>(start_path: P) -> Option<std::path::PathBuf> {
        let mut current = start_path.as_ref().to_path_buf();
        
        loop {
            if Self::is_flux_project(&current) {
                return Some(current);
            }
            
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        
        None
    }
    
    /// Get the source directory for a project
    pub fn src_dir<P: AsRef<Path>>(project_root: P) -> std::path::PathBuf {
        project_root.as_ref().join("src")
    }
    
    /// Get the tests directory for a project
    pub fn tests_dir<P: AsRef<Path>>(project_root: P) -> std::path::PathBuf {
        project_root.as_ref().join("tests")
    }
    
    /// Get the examples directory for a project
    pub fn examples_dir<P: AsRef<Path>>(project_root: P) -> std::path::PathBuf {
        project_root.as_ref().join("examples")
    }
    
    /// Get the target directory for a project
    pub fn target_dir<P: AsRef<Path>>(project_root: P) -> std::path::PathBuf {
        project_root.as_ref().join("target")
    }
    
    /// Get the documentation directory for a project
    pub fn docs_dir<P: AsRef<Path>>(project_root: P) -> std::path::PathBuf {
        project_root.as_ref().join("docs")
    }
    
    /// List all Flux source files in the project
    pub fn list_source_files<P: AsRef<Path>>(project_root: P) -> Result<Vec<std::path::PathBuf>, FluxError> {
        let src_dir = Self::src_dir(&project_root);
        Self::list_flux_files(&src_dir)
    }
    
    /// List all test files in the project
    pub fn list_test_files<P: AsRef<Path>>(project_root: P) -> Result<Vec<std::path::PathBuf>, FluxError> {
        let tests_dir = Self::tests_dir(&project_root);
        Self::list_flux_files(&tests_dir)
    }
    
    /// List all example files in the project
    pub fn list_example_files<P: AsRef<Path>>(project_root: P) -> Result<Vec<std::path::PathBuf>, FluxError> {
        let examples_dir = Self::examples_dir(&project_root);
        Self::list_flux_files(&examples_dir)
    }
    
    /// Recursively list all .flux files in a directory
    fn list_flux_files<P: AsRef<Path>>(dir: P) -> Result<Vec<std::path::PathBuf>, FluxError> {
        let mut files = Vec::new();
        
        if !dir.as_ref().exists() {
            return Ok(files);
        }
        
        Self::collect_flux_files(dir.as_ref(), &mut files)?;
        Ok(files)
    }
    
    /// Recursively collect .flux files
    fn collect_flux_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<(), FluxError> {
        let entries = fs::read_dir(dir)
            .map_err(|e| FluxError::Package(PackageError::IoError(
                format!("Failed to read directory {}: {}", dir.display(), e)
            )))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
            let path = entry.path();
            
            if path.is_dir() {
                Self::collect_flux_files(&path, files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("flux") {
                files.push(path);
            }
        }
        
        Ok(())
    }
}

/// Project metadata
#[derive(Debug, Clone)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
    pub root_path: std::path::PathBuf,
    pub source_files: Vec<std::path::PathBuf>,
    pub test_files: Vec<std::path::PathBuf>,
    pub example_files: Vec<std::path::PathBuf>,
}

impl ProjectMetadata {
    /// Load project metadata from a project root
    pub fn load<P: AsRef<Path>>(project_root: P) -> Result<Self, FluxError> {
        let project_root = project_root.as_ref().to_path_buf();
        
        // Load configuration to get name and version
        let config_path = project_root.join("flux.toml");
        let config = super::ProjectConfig::from_file(&config_path)?;
        
        // Collect file lists
        let source_files = Project::list_source_files(&project_root)?;
        let test_files = Project::list_test_files(&project_root)?;
        let example_files = Project::list_example_files(&project_root)?;
        
        Ok(ProjectMetadata {
            name: config.package.name,
            version: config.package.version,
            root_path: project_root,
            source_files,
            test_files,
            example_files,
        })
    }
    
    /// Get the main source file (src/main.flux)
    pub fn main_file(&self) -> std::path::PathBuf {
        self.root_path.join("src").join("main.flux")
    }
    
    /// Get the library file (src/lib.flux)
    pub fn lib_file(&self) -> std::path::PathBuf {
        self.root_path.join("src").join("lib.flux")
    }
    
    /// Check if this is a binary project (has main.flux)
    pub fn is_binary(&self) -> bool {
        self.main_file().exists()
    }
    
    /// Check if this is a library project (has lib.flux)
    pub fn is_library(&self) -> bool {
        self.lib_file().exists()
    }
}