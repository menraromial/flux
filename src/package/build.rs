//! Build system for Flux projects
//! 
//! This module provides functionality for:
//! - Multi-file compilation with dependency tracking
//! - Incremental compilation support
//! - Build artifact management
//! - Cross-compilation support

use crate::error::{FluxError, PackageError};
use super::{PackageMetadata, ProjectConfig, BuildArtifact, ArtifactType};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;

use std::time::SystemTime;

/// Build system for Flux projects
pub struct BuildSystem {
    metadata: PackageMetadata,
    build_config: BuildConfig,
    dependency_graph: DependencyGraph,
}

/// Build configuration options
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub target: Option<String>,
    pub optimization_level: OptimizationLevel,
    pub debug_info: bool,
    pub incremental: bool,
    pub parallel: bool,
    pub output_dir: PathBuf,
    pub verbose: bool,
}

/// Optimization levels for compilation
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationLevel {
    None,    // -O0
    Speed,   // -O3
    Size,    // -Os
    Debug,   // -O1
}

/// Dependency graph for tracking compilation dependencies
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    nodes: HashMap<PathBuf, DependencyNode>,
    edges: HashMap<PathBuf, Vec<PathBuf>>,
}

/// Node in the dependency graph
#[derive(Debug, Clone)]
pub struct DependencyNode {
    pub path: PathBuf,
    pub last_modified: SystemTime,
    pub dependencies: Vec<PathBuf>,
    pub dependents: Vec<PathBuf>,
}

/// Build result information
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub success: bool,
    pub artifacts: Vec<BuildArtifact>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub build_time: std::time::Duration,
}

/// Compilation target information
#[derive(Debug, Clone)]
pub struct CompilationTarget {
    pub name: String,
    pub triple: String,
    pub arch: String,
    pub os: String,
    pub env: String,
}

impl BuildSystem {
    /// Create a new build system for a project
    pub fn new(metadata: PackageMetadata) -> Result<Self, FluxError> {
        let build_config = BuildConfig::from_project_config(&metadata.config)?;
        let dependency_graph = DependencyGraph::build_from_sources(&metadata.source_files()?)?;
        
        Ok(BuildSystem {
            metadata,
            build_config,
            dependency_graph,
        })
    }
    
    /// Build the entire project
    pub fn build(&mut self) -> Result<BuildResult, FluxError> {
        let start_time = std::time::Instant::now();
        let mut result = BuildResult {
            success: true,
            artifacts: vec![],
            errors: vec![],
            warnings: vec![],
            build_time: std::time::Duration::new(0, 0),
        };
        
        // Check if incremental build is possible
        let changed_files = if self.build_config.incremental {
            self.find_changed_files()?
        } else {
            self.metadata.source_files()?
        };
        
        if changed_files.is_empty() && self.build_config.incremental {
            if self.build_config.verbose {
                println!("No changes detected, skipping build");
            }
            result.build_time = start_time.elapsed();
            return Ok(result);
        }
        
        // Create output directory
        fs::create_dir_all(&self.build_config.output_dir)
            .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
        
        // Determine compilation order
        let compilation_order = self.dependency_graph.topological_sort()?;
        
        // Compile files in dependency order
        if self.build_config.parallel && compilation_order.len() > 1 {
            self.build_parallel(&compilation_order, &mut result)?;
        } else {
            self.build_sequential(&compilation_order, &mut result)?;
        }
        
        // Link if necessary
        if result.success && self.should_link() {
            self.link_artifacts(&mut result)?;
        }
        
        result.build_time = start_time.elapsed();
        
        if self.build_config.verbose {
            println!("Build completed in {:?}", result.build_time);
            println!("Generated {} artifacts", result.artifacts.len());
        }
        
        Ok(result)
    }
    
    /// Clean build artifacts
    pub fn clean(&mut self) -> Result<(), FluxError> {
        // Clean individual files first
        if self.build_config.output_dir.exists() {
            let entries = fs::read_dir(&self.build_config.output_dir)
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
            
            for entry in entries {
                let entry = entry.map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
                let path = entry.path();
                
                if path.is_file() {
                    fs::remove_file(&path)
                        .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
                } else if path.is_dir() {
                    fs::remove_dir_all(&path)
                        .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
                }
            }
            
            // Try to remove the directory itself
            if let Err(_) = fs::remove_dir(&self.build_config.output_dir) {
                // If we can't remove it, that's okay - it might not be empty due to other files
            }
        }
        
        self.metadata.build_metadata.clean()?;
        
        if self.build_config.verbose {
            println!("Cleaned build artifacts");
        }
        
        Ok(())
    }
    
    /// Check if the project needs rebuilding
    pub fn needs_rebuild(&self) -> Result<bool, FluxError> {
        if !self.build_config.incremental {
            return Ok(true);
        }
        
        // Check if any source files are newer than artifacts
        let source_files = self.metadata.source_files()?;
        let latest_source_time = self.get_latest_modification_time(&source_files)?;
        
        // Check if any artifacts exist
        if self.metadata.build_metadata.build_artifacts.is_empty() {
            return Ok(true);
        }
        
        // Check if any artifact is older than source files
        for artifact in &self.metadata.build_metadata.build_artifacts {
            if artifact.build_time < latest_source_time {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Find files that have changed since last build
    fn find_changed_files(&self) -> Result<Vec<PathBuf>, FluxError> {
        let mut changed_files = Vec::new();
        let source_files = self.metadata.source_files()?;
        
        for file in source_files {
            let metadata = fs::metadata(&file)
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
            let modified = metadata.modified()
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
            
            // Check if file is newer than last build
            if let Some(last_build_time) = self.metadata.build_metadata.last_build_time {
                if modified > last_build_time {
                    changed_files.push(file);
                }
            } else {
                // No previous build, all files are "changed"
                changed_files.push(file);
            }
        }
        
        Ok(changed_files)
    }
    
    /// Build files sequentially
    fn build_sequential(&mut self, files: &[PathBuf], result: &mut BuildResult) -> Result<(), FluxError> {
        for file in files {
            if let Err(e) = self.compile_file(file, result) {
                result.success = false;
                result.errors.push(format!("Failed to compile {}: {}", file.display(), e));
                
                if !self.build_config.verbose {
                    break; // Stop on first error unless verbose
                }
            }
        }
        
        Ok(())
    }
    
    /// Build files in parallel
    fn build_parallel(&mut self, files: &[PathBuf], result: &mut BuildResult) -> Result<(), FluxError> {
        // For now, implement a simple parallel build using threads
        // In a real implementation, this would use a more sophisticated work-stealing approach
        
        use std::thread;
        let mut handles = vec![];
        
        // Split files into chunks for parallel processing
        let chunk_size = std::cmp::max(1, files.len() / num_cpus::get());
        
        for chunk in files.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            let build_config = self.build_config.clone();
            
            let handle = thread::spawn(move || {
                let mut local_result = BuildResult {
                    success: true,
                    artifacts: vec![],
                    errors: vec![],
                    warnings: vec![],
                    build_time: std::time::Duration::new(0, 0),
                };
                
                for file in chunk {
                    if let Err(e) = Self::compile_file_static(&file, &build_config, &mut local_result) {
                        local_result.success = false;
                        local_result.errors.push(format!("Failed to compile {}: {}", file.display(), e));
                    }
                }
                
                local_result
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads to complete and collect results
        for handle in handles {
            let local_result = handle.join().map_err(|_| FluxError::Package(PackageError::ProjectCreationFailed(
                "Thread join failed".to_string()
            )))?;
            
            result.success &= local_result.success;
            result.artifacts.extend(local_result.artifacts);
            result.errors.extend(local_result.errors);
            result.warnings.extend(local_result.warnings);
        }
        
        Ok(())
    }
    
    /// Compile a single file
    fn compile_file(&self, file: &Path, result: &mut BuildResult) -> Result<(), FluxError> {
        Self::compile_file_static(file, &self.build_config, result)
    }
    
    /// Static version of compile_file for use in parallel builds
    fn compile_file_static(file: &Path, build_config: &BuildConfig, result: &mut BuildResult) -> Result<(), FluxError> {
        if build_config.verbose {
            println!("Compiling {}", file.display());
        }
        
        // For now, simulate compilation by creating a mock object file
        let object_file = build_config.output_dir.join(
            file.file_stem().unwrap().to_string_lossy().to_string() + ".o"
        );
        
        // In a real implementation, this would invoke the Flux compiler
        // For now, just create an empty file to simulate compilation
        fs::write(&object_file, b"mock object file")
            .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
        
        let artifact = BuildArtifact {
            name: file.file_stem().unwrap().to_string_lossy().to_string(),
            artifact_type: ArtifactType::Object,
            path: object_file,
            dependencies: vec![], // Would be populated from actual compilation
            build_time: SystemTime::now(),
            checksum: "mock-checksum".to_string(),
        };
        
        result.artifacts.push(artifact);
        
        Ok(())
    }
    
    /// Check if linking is required
    fn should_link(&self) -> bool {
        // Link if this is a binary project (has main.flux)
        self.metadata.root_path.join("src").join("main.flux").exists()
    }
    
    /// Link compiled objects into final executable
    fn link_artifacts(&self, result: &mut BuildResult) -> Result<(), FluxError> {
        if self.build_config.verbose {
            println!("Linking artifacts");
        }
        
        let executable_name = if cfg!(windows) {
            format!("{}.exe", self.metadata.config.package.name)
        } else {
            self.metadata.config.package.name.clone()
        };
        
        let executable_path = self.build_config.output_dir.join(&executable_name);
        
        // In a real implementation, this would invoke the linker
        // For now, just create a mock executable
        fs::write(&executable_path, b"mock executable")
            .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
        
        // Make executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&executable_path)
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&executable_path, perms)
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
        }
        
        let artifact = BuildArtifact {
            name: executable_name,
            artifact_type: ArtifactType::Executable,
            path: executable_path,
            dependencies: result.artifacts.iter().map(|a| a.name.clone()).collect(),
            build_time: SystemTime::now(),
            checksum: "mock-checksum".to_string(),
        };
        
        result.artifacts.push(artifact);
        
        Ok(())
    }
    
    /// Get the latest modification time from a list of files
    fn get_latest_modification_time(&self, files: &[PathBuf]) -> Result<SystemTime, FluxError> {
        let mut latest = std::time::UNIX_EPOCH;
        
        for file in files {
            let metadata = fs::metadata(file)
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
            let modified = metadata.modified()
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
            
            if modified > latest {
                latest = modified;
            }
        }
        
        Ok(latest)
    }
    
    /// Get the build configuration
    pub fn build_config(&self) -> &BuildConfig {
        &self.build_config
    }
    
    /// Get a mutable reference to the build configuration
    pub fn build_config_mut(&mut self) -> &mut BuildConfig {
        &mut self.build_config
    }
    
    /// Get the dependency graph
    pub fn dependency_graph(&self) -> &DependencyGraph {
        &self.dependency_graph
    }
    
    /// Get available compilation targets
    pub fn available_targets() -> Vec<CompilationTarget> {
        vec![
            CompilationTarget {
                name: "native".to_string(),
                triple: "native".to_string(),
                arch: std::env::consts::ARCH.to_string(),
                os: std::env::consts::OS.to_string(),
                env: "gnu".to_string(),
            },
            CompilationTarget {
                name: "x86_64-linux-gnu".to_string(),
                triple: "x86_64-unknown-linux-gnu".to_string(),
                arch: "x86_64".to_string(),
                os: "linux".to_string(),
                env: "gnu".to_string(),
            },
            CompilationTarget {
                name: "x86_64-windows-msvc".to_string(),
                triple: "x86_64-pc-windows-msvc".to_string(),
                arch: "x86_64".to_string(),
                os: "windows".to_string(),
                env: "msvc".to_string(),
            },
            CompilationTarget {
                name: "x86_64-macos".to_string(),
                triple: "x86_64-apple-darwin".to_string(),
                arch: "x86_64".to_string(),
                os: "macos".to_string(),
                env: "".to_string(),
            },
            CompilationTarget {
                name: "wasm32-unknown-unknown".to_string(),
                triple: "wasm32-unknown-unknown".to_string(),
                arch: "wasm32".to_string(),
                os: "unknown".to_string(),
                env: "".to_string(),
            },
        ]
    }
}

impl BuildConfig {
    /// Create build config from project config
    pub fn from_project_config(config: &ProjectConfig) -> Result<Self, FluxError> {
        Ok(BuildConfig {
            target: config.build.target.clone(),
            optimization_level: match config.build.optimization_level {
                crate::package::config::OptimizationLevel::None => OptimizationLevel::None,
                crate::package::config::OptimizationLevel::Speed => OptimizationLevel::Speed,
                crate::package::config::OptimizationLevel::Size => OptimizationLevel::Size,
                crate::package::config::OptimizationLevel::Debug => OptimizationLevel::Debug,
            },
            debug_info: config.build.debug_info,
            incremental: config.build.incremental,
            parallel: config.build.parallel,
            output_dir: PathBuf::from("target"),
            verbose: false,
        })
    }
    
    /// Create a debug build config
    pub fn debug() -> Self {
        BuildConfig {
            target: None,
            optimization_level: OptimizationLevel::Debug,
            debug_info: true,
            incremental: true,
            parallel: true,
            output_dir: PathBuf::from("target/debug"),
            verbose: false,
        }
    }
    
    /// Create a release build config
    pub fn release() -> Self {
        BuildConfig {
            target: None,
            optimization_level: OptimizationLevel::Speed,
            debug_info: false,
            incremental: true,
            parallel: true,
            output_dir: PathBuf::from("target/release"),
            verbose: false,
        }
    }
}

impl DependencyGraph {
    /// Build dependency graph from source files
    pub fn build_from_sources(source_files: &[PathBuf]) -> Result<Self, FluxError> {
        let mut graph = DependencyGraph {
            nodes: HashMap::new(),
            edges: HashMap::new(),
        };
        
        // Create nodes for all source files
        for file in source_files {
            let metadata = fs::metadata(file)
                .map_err(|e| FluxError::Package(PackageError::IoError(
                    format!("Failed to read metadata for {}: {}", file.display(), e)
                )))?;
            let modified = metadata.modified()
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
            
            // Extract dependencies, but don't fail if we can't resolve them all
            let dependencies = graph.extract_dependencies(file).unwrap_or_else(|_| vec![]);
            
            let node = DependencyNode {
                path: file.clone(),
                last_modified: modified,
                dependencies: dependencies.clone(),
                dependents: vec![],
            };
            
            graph.nodes.insert(file.clone(), node);
            graph.edges.insert(file.clone(), dependencies);
        }
        
        // Build reverse edges (dependents)
        for (file, deps) in &graph.edges {
            for dep in deps {
                if let Some(dep_node) = graph.nodes.get_mut(dep) {
                    dep_node.dependents.push(file.clone());
                }
            }
        }
        
        Ok(graph)
    }
    
    /// Extract dependencies from a source file
    fn extract_dependencies(&self, file: &Path) -> Result<Vec<PathBuf>, FluxError> {
        let content = fs::read_to_string(file)
            .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
        
        let mut dependencies = Vec::new();
        
        // Simple dependency extraction - look for import statements
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("import ") {
                // Extract module name from import statement
                if let Some(module_name) = line.strip_prefix("import ").and_then(|s| s.split_whitespace().next()) {
                    // Remove semicolon if present
                    let module_name = module_name.trim_end_matches(';');
                    
                    // Convert module name to file path
                    let module_path = self.module_name_to_path(module_name, file)?;
                    if module_path.exists() {
                        dependencies.push(module_path);
                    }
                }
            }
        }
        
        Ok(dependencies)
    }
    
    /// Convert module name to file path
    fn module_name_to_path(&self, module_name: &str, current_file: &Path) -> Result<PathBuf, FluxError> {
        let current_dir = current_file.parent().unwrap_or(Path::new("."));
        
        if module_name.starts_with("std/") {
            // Standard library module - would be resolved differently in real implementation
            Ok(PathBuf::from(format!("std/{}.flux", &module_name[4..])))
        } else if module_name.contains('/') {
            // Relative path
            Ok(current_dir.join(format!("{}.flux", module_name)))
        } else {
            // Local module
            Ok(current_dir.join(format!("{}.flux", module_name)))
        }
    }
    
    /// Perform topological sort to determine compilation order
    pub fn topological_sort(&self) -> Result<Vec<PathBuf>, FluxError> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();
        
        for file in self.nodes.keys() {
            if !visited.contains(file) {
                self.topological_sort_visit(file, &mut visited, &mut temp_visited, &mut result)?;
            }
        }
        
        result.reverse(); // Reverse to get correct dependency order
        Ok(result)
    }
    
    /// Recursive helper for topological sort
    fn topological_sort_visit(
        &self,
        file: &PathBuf,
        visited: &mut HashSet<PathBuf>,
        temp_visited: &mut HashSet<PathBuf>,
        result: &mut Vec<PathBuf>,
    ) -> Result<(), FluxError> {
        if temp_visited.contains(file) {
            return Err(FluxError::Package(PackageError::DependencyResolutionFailed(
                format!("Circular dependency detected involving {}", file.display())
            )));
        }
        
        if visited.contains(file) {
            return Ok(());
        }
        
        temp_visited.insert(file.clone());
        
        if let Some(dependencies) = self.edges.get(file) {
            for dep in dependencies {
                self.topological_sort_visit(dep, visited, temp_visited, result)?;
            }
        }
        
        temp_visited.remove(file);
        visited.insert(file.clone());
        result.push(file.clone());
        
        Ok(())
    }
}