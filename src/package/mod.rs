//! Package management system for Flux
//! 
//! This module provides functionality for:
//! - Project configuration parsing (flux.toml)
//! - Project directory structure creation
//! - Dependency specification and resolution
//! - Package metadata handling

pub mod config;
pub mod project;
pub mod dependency;
pub mod metadata;
pub mod build;

pub use config::{ProjectConfig, PackageInfo, DependencySpec, BuildConfig as ConfigBuildConfig, OptimizationLevel as ConfigOptimizationLevel};
pub use project::{Project, ProjectInstance, ProjectMetadata, TestResults};
pub use dependency::{ResolvedDependency, DependencySource, DependencyResolver, VersionReq, RegistryPackage};
pub use metadata::{PackageMetadata, BuildMetadata, BuildArtifact, ArtifactType, CompilationUnit, LockFile, LockedDependency};
pub use build::{BuildSystem, BuildConfig, OptimizationLevel, DependencyGraph, DependencyNode, BuildResult, CompilationTarget};

use crate::error::{FluxError, PackageError};
use std::path::Path;

/// Package manager for Flux projects
#[derive(Debug)]
pub struct PackageManager {
    config: ProjectConfig,
    project_root: std::path::PathBuf,
}

impl PackageManager {
    /// Create a new package manager for the given project root
    pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self, FluxError> {
        let project_root = project_root.as_ref().to_path_buf();
        let config_path = project_root.join("flux.toml");
        
        let config = if config_path.exists() {
            ProjectConfig::from_file(&config_path)?
        } else {
            return Err(FluxError::Package(PackageError::ConfigNotFound(config_path)));
        };

        Ok(PackageManager {
            config,
            project_root,
        })
    }

    /// Initialize a new Flux project in the given directory
    pub fn init_project<P: AsRef<Path>>(
        project_root: P,
        name: String,
        version: String,
    ) -> Result<Self, FluxError> {
        let project_root = project_root.as_ref().to_path_buf();
        
        // Create project structure
        Project::create_structure(&project_root)?;
        
        // Create default configuration
        let config = ProjectConfig::new(name, version);
        config.write_to_file(&project_root.join("flux.toml"))?;
        
        Ok(PackageManager {
            config,
            project_root,
        })
    }

    /// Get the project configuration
    pub fn config(&self) -> &ProjectConfig {
        &self.config
    }

    /// Get the project root directory
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Resolve all dependencies for the project
    pub fn resolve_dependencies(&self) -> Result<Vec<ResolvedDependency>, FluxError> {
        let resolver = DependencyResolver::new();
        resolver.resolve(&self.config.dependencies)
    }
}

