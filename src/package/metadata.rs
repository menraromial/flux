//! Package metadata handling and management

use crate::error::{FluxError, PackageError};
use super::{ProjectConfig, ResolvedDependency};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

/// Complete package metadata including resolved dependencies
#[derive(Debug, Clone)]
pub struct PackageMetadata {
    pub config: ProjectConfig,
    pub root_path: PathBuf,
    pub resolved_dependencies: Vec<ResolvedDependency>,
    pub build_metadata: BuildMetadata,
    pub lock_file: Option<LockFile>,
}

/// Build-specific metadata
#[derive(Debug, Clone)]
pub struct BuildMetadata {
    pub target_dir: PathBuf,
    pub build_artifacts: Vec<BuildArtifact>,
    pub compilation_units: Vec<CompilationUnit>,
    pub last_build_time: Option<std::time::SystemTime>,
}

/// Build artifact information
#[derive(Debug, Clone)]
pub struct BuildArtifact {
    pub name: String,
    pub artifact_type: ArtifactType,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub build_time: std::time::SystemTime,
    pub checksum: String,
}

/// Types of build artifacts
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactType {
    Executable,
    Library,
    Object,
    Archive,
}

/// Compilation unit for incremental compilation
#[derive(Debug, Clone)]
pub struct CompilationUnit {
    pub name: String,
    pub source_files: Vec<PathBuf>,
    pub dependencies: Vec<String>,
    pub last_modified: std::time::SystemTime,
    pub output_path: PathBuf,
}

/// Lock file for dependency resolution
#[derive(Debug, Clone)]
pub struct LockFile {
    pub version: String,
    pub dependencies: HashMap<String, LockedDependency>,
    pub metadata: HashMap<String, String>,
}

/// Locked dependency information
#[derive(Debug, Clone)]
pub struct LockedDependency {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: String,
    pub dependencies: Vec<String>,
}

impl PackageMetadata {
    /// Load package metadata from a project directory
    pub fn load<P: AsRef<Path>>(project_root: P) -> Result<Self, FluxError> {
        let project_root = project_root.as_ref().to_path_buf();
        
        // Load project configuration
        let config_path = project_root.join("flux.toml");
        let config = ProjectConfig::from_file(&config_path)?;
        
        // Load lock file if it exists
        let lock_file_path = project_root.join("flux.lock");
        let lock_file = if lock_file_path.exists() {
            Some(LockFile::load(&lock_file_path)?)
        } else {
            None
        };
        
        // Initialize build metadata
        let target_dir = project_root.join("target");
        let build_metadata = BuildMetadata::load(&target_dir)?;
        
        Ok(PackageMetadata {
            config,
            root_path: project_root,
            resolved_dependencies: vec![],
            build_metadata,
            lock_file,
        })
    }
    
    /// Save package metadata (primarily the lock file)
    pub fn save(&self) -> Result<(), FluxError> {
        if let Some(ref lock_file) = self.lock_file {
            let lock_file_path = self.root_path.join("flux.lock");
            lock_file.save(&lock_file_path)?;
        }
        
        self.build_metadata.save()?;
        
        Ok(())
    }
    
    /// Update resolved dependencies and create/update lock file
    pub fn update_dependencies(&mut self, resolved: Vec<ResolvedDependency>) -> Result<(), FluxError> {
        self.resolved_dependencies = resolved;
        
        // Create or update lock file
        let mut locked_deps = HashMap::new();
        for dep in &self.resolved_dependencies {
            locked_deps.insert(dep.name.clone(), LockedDependency {
                name: dep.name.clone(),
                version: dep.version.clone(),
                source: format!("{:?}", dep.source),
                checksum: self.calculate_dependency_checksum(dep)?,
                dependencies: dep.dependencies.clone(),
            });
        }
        
        self.lock_file = Some(LockFile {
            version: "1".to_string(),
            dependencies: locked_deps,
            metadata: HashMap::new(),
        });
        
        Ok(())
    }
    
    /// Calculate checksum for a dependency
    fn calculate_dependency_checksum(&self, dep: &ResolvedDependency) -> Result<String, FluxError> {
        // In a real implementation, this would calculate a proper checksum
        // For now, return a mock checksum based on name and version
        Ok(format!("{}:{}", dep.name, dep.version))
    }
    
    /// Get all source files for the package
    pub fn source_files(&self) -> Result<Vec<PathBuf>, FluxError> {
        super::Project::list_source_files(&self.root_path)
    }
    
    /// Get all test files for the package
    pub fn test_files(&self) -> Result<Vec<PathBuf>, FluxError> {
        super::Project::list_test_files(&self.root_path)
    }
    
    /// Get all example files for the package
    pub fn example_files(&self) -> Result<Vec<PathBuf>, FluxError> {
        super::Project::list_example_files(&self.root_path)
    }
    
    /// Check if the package needs rebuilding
    pub fn needs_rebuild(&self) -> Result<bool, FluxError> {
        // Check if any source files are newer than build artifacts
        let source_files = self.source_files()?;
        let latest_source_time = self.get_latest_modification_time(&source_files)?;
        
        if let Some(last_build_time) = self.build_metadata.last_build_time {
            Ok(latest_source_time > last_build_time)
        } else {
            Ok(true) // Never built before
        }
    }
    
    /// Get the latest modification time from a list of files
    fn get_latest_modification_time(&self, files: &[PathBuf]) -> Result<std::time::SystemTime, FluxError> {
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
}

impl BuildMetadata {
    /// Create new empty build metadata
    pub fn new() -> Self {
        BuildMetadata {
            target_dir: PathBuf::from("target"),
            build_artifacts: vec![],
            compilation_units: vec![],
            last_build_time: None,
        }
    }

    /// Load build metadata from target directory
    pub fn load<P: AsRef<Path>>(target_dir: P) -> Result<Self, FluxError> {
        let target_dir = target_dir.as_ref().to_path_buf();
        
        // Create target directory if it doesn't exist
        if !target_dir.exists() {
            fs::create_dir_all(&target_dir)
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
        }
        
        Ok(BuildMetadata {
            target_dir,
            build_artifacts: vec![],
            compilation_units: vec![],
            last_build_time: None,
        })
    }
    
    /// Save build metadata
    pub fn save(&self) -> Result<(), FluxError> {
        // In a real implementation, this would save build metadata to a file
        // For now, just ensure the target directory exists
        if !self.target_dir.exists() {
            fs::create_dir_all(&self.target_dir)
                .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
        }
        
        Ok(())
    }
    
    /// Add a build artifact
    pub fn add_artifact(&mut self, artifact: BuildArtifact) {
        self.build_artifacts.push(artifact);
        self.last_build_time = Some(std::time::SystemTime::now());
    }
    
    /// Get artifacts of a specific type
    pub fn artifacts_of_type(&self, artifact_type: ArtifactType) -> Vec<&BuildArtifact> {
        self.build_artifacts.iter()
            .filter(|a| a.artifact_type == artifact_type)
            .collect()
    }
    
    /// Clean build artifacts
    pub fn clean(&mut self) -> Result<(), FluxError> {
        for artifact in &self.build_artifacts {
            if artifact.path.exists() {
                if artifact.path.is_dir() {
                    fs::remove_dir_all(&artifact.path)
                        .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
                } else {
                    fs::remove_file(&artifact.path)
                        .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
                }
            }
        }
        
        self.build_artifacts.clear();
        self.last_build_time = None;
        
        Ok(())
    }
}

impl LockFile {
    /// Load lock file from disk
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, FluxError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
        
        Self::from_toml(&content)
    }
    
    /// Save lock file to disk
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), FluxError> {
        let content = self.to_toml();
        fs::write(path.as_ref(), content)
            .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))
    }
    
    /// Parse lock file from TOML content
    pub fn from_toml(content: &str) -> Result<Self, FluxError> {
        // Simple TOML parser for lock file
        let mut lock_file = LockFile {
            version: "1".to_string(),
            dependencies: HashMap::new(),
            metadata: HashMap::new(),
        };
        
        let mut current_section = "";
        let mut current_dependency: Option<String> = None;
        let mut current_locked_dep = LockedDependency {
            name: String::new(),
            version: String::new(),
            source: String::new(),
            checksum: String::new(),
            dependencies: vec![],
        };
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if line.starts_with('[') && line.ends_with(']') {
                // Save previous dependency if we were parsing one
                if let Some(dep_name) = current_dependency.take() {
                    current_locked_dep.name = dep_name.clone();
                    lock_file.dependencies.insert(dep_name, current_locked_dep.clone());
                }
                
                current_section = &line[1..line.len()-1];
                
                // Check if this is a dependency section
                if current_section.starts_with("dependencies.") {
                    current_dependency = Some(current_section[13..].to_string());
                    current_locked_dep = LockedDependency {
                        name: String::new(),
                        version: String::new(),
                        source: String::new(),
                        checksum: String::new(),
                        dependencies: vec![],
                    };
                }
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                
                match current_section {
                    "metadata" => {
                        match key {
                            "version" => lock_file.version = value.to_string(),
                            _ => {
                                lock_file.metadata.insert(key.to_string(), value.to_string());
                            }
                        }
                    }
                    section if section.starts_with("dependencies.") => {
                        match key {
                            "version" => current_locked_dep.version = value.to_string(),
                            "source" => current_locked_dep.source = value.to_string(),
                            "checksum" => current_locked_dep.checksum = value.to_string(),
                            "dependencies" => {
                                // Handle array format: ["dep1", "dep2"] or simple comma-separated
                                if value.starts_with('[') && value.ends_with(']') {
                                    let inner = &value[1..value.len()-1];
                                    current_locked_dep.dependencies = inner.split(',')
                                        .map(|s| s.trim().trim_matches('"').to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                } else {
                                    current_locked_dep.dependencies = value.split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Save the last dependency if we were parsing one
        if let Some(dep_name) = current_dependency {
            current_locked_dep.name = dep_name.clone();
            lock_file.dependencies.insert(dep_name, current_locked_dep);
        }
        
        Ok(lock_file)
    }
    
    /// Convert lock file to TOML content
    pub fn to_toml(&self) -> String {
        let mut toml = String::new();
        
        // Metadata section
        toml.push_str("[metadata]\n");
        toml.push_str(&format!("version = \"{}\"\n", self.version));
        
        for (key, value) in &self.metadata {
            toml.push_str(&format!("{} = \"{}\"\n", key, value));
        }
        
        toml.push('\n');
        
        // Dependencies sections
        for (name, dep) in &self.dependencies {
            toml.push_str(&format!("[dependencies.{}]\n", name));
            toml.push_str(&format!("version = \"{}\"\n", dep.version));
            toml.push_str(&format!("source = \"{}\"\n", dep.source));
            toml.push_str(&format!("checksum = \"{}\"\n", dep.checksum));
            
            if !dep.dependencies.is_empty() {
                toml.push_str(&format!("dependencies = [{}]\n",
                    dep.dependencies.iter()
                        .map(|d| format!("\"{}\"", d))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            
            toml.push('\n');
        }
        
        toml
    }
}