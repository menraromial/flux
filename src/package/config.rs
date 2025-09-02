//! Project configuration parsing for flux.toml files

use crate::error::{FluxError, PackageError};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Project configuration loaded from flux.toml
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfig {
    pub package: PackageInfo,
    pub dependencies: HashMap<String, DependencySpec>,
    pub dev_dependencies: HashMap<String, DependencySpec>,
    pub build: BuildConfig,
    pub features: HashMap<String, Vec<String>>,
}

/// Package information section
#[derive(Debug, Clone, PartialEq)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub edition: String,
}

/// Dependency specification
#[derive(Debug, Clone, PartialEq)]
pub struct DependencySpec {
    pub version: Option<String>,
    pub path: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
}

/// Build configuration
#[derive(Debug, Clone, PartialEq)]
pub struct BuildConfig {
    pub target: Option<String>,
    pub optimization_level: OptimizationLevel,
    pub debug_info: bool,
    pub incremental: bool,
    pub parallel: bool,
}

/// Optimization levels
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationLevel {
    None,
    Speed,
    Size,
    Debug,
}

impl ProjectConfig {
    /// Create a new default project configuration
    pub fn new(name: String, version: String) -> Self {
        ProjectConfig {
            package: PackageInfo {
                name,
                version,
                description: None,
                authors: vec![],
                license: None,
                repository: None,
                homepage: None,
                keywords: vec![],
                categories: vec![],
                edition: "2024".to_string(),
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build: BuildConfig {
                target: None,
                optimization_level: OptimizationLevel::Debug,
                debug_info: true,
                incremental: true,
                parallel: true,
            },
            features: HashMap::new(),
        }
    }

    /// Load configuration from a flux.toml file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, FluxError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))?;
        
        Self::from_toml(&content)
    }

    /// Parse configuration from TOML string
    pub fn from_toml(content: &str) -> Result<Self, FluxError> {
        // Simple TOML parser implementation
        let mut config = ProjectConfig::new("".to_string(), "0.1.0".to_string());
        let mut current_section = "";
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if line.starts_with('[') && line.ends_with(']') {
                current_section = &line[1..line.len()-1];
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                
                match current_section {
                    "package" => {
                        match key {
                            "name" => config.package.name = value.to_string(),
                            "version" => config.package.version = value.to_string(),
                            "description" => config.package.description = Some(value.to_string()),
                            "license" => config.package.license = Some(value.to_string()),
                            "repository" => config.package.repository = Some(value.to_string()),
                            "homepage" => config.package.homepage = Some(value.to_string()),
                            "edition" => config.package.edition = value.to_string(),
                            "authors" => {
                                // Handle array format: ["author1", "author2"] or simple comma-separated
                                if value.starts_with('[') && value.ends_with(']') {
                                    let inner = &value[1..value.len()-1];
                                    config.package.authors = inner.split(',')
                                        .map(|s| s.trim().trim_matches('"').to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                } else {
                                    config.package.authors = value.split(',')
                                        .map(|s| s.trim().to_string())
                                        .collect();
                                }
                            }
                            "keywords" => {
                                // Handle array format: ["keyword1", "keyword2"] or simple comma-separated
                                if value.starts_with('[') && value.ends_with(']') {
                                    let inner = &value[1..value.len()-1];
                                    config.package.keywords = inner.split(',')
                                        .map(|s| s.trim().trim_matches('"').to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                } else {
                                    config.package.keywords = value.split(',')
                                        .map(|s| s.trim().to_string())
                                        .collect();
                                }
                            }
                            "categories" => {
                                // Handle array format: ["category1", "category2"] or simple comma-separated
                                if value.starts_with('[') && value.ends_with(']') {
                                    let inner = &value[1..value.len()-1];
                                    config.package.categories = inner.split(',')
                                        .map(|s| s.trim().trim_matches('"').to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                } else {
                                    config.package.categories = value.split(',')
                                        .map(|s| s.trim().to_string())
                                        .collect();
                                }
                            }
                            _ => {}
                        }
                    }
                    "dependencies" => {
                        config.dependencies.insert(
                            key.to_string(),
                            DependencySpec::from_version_string(value)
                        );
                    }
                    "dev-dependencies" => {
                        config.dev_dependencies.insert(
                            key.to_string(),
                            DependencySpec::from_version_string(value)
                        );
                    }
                    "build" => {
                        match key {
                            "target" => config.build.target = Some(value.to_string()),
                            "optimization" => {
                                config.build.optimization_level = match value {
                                    "none" | "0" => OptimizationLevel::None,
                                    "speed" | "3" => OptimizationLevel::Speed,
                                    "size" | "s" => OptimizationLevel::Size,
                                    "debug" | "1" => OptimizationLevel::Debug,
                                    _ => OptimizationLevel::Debug,
                                };
                            }
                            "debug" => config.build.debug_info = value.parse().unwrap_or(true),
                            "incremental" => config.build.incremental = value.parse().unwrap_or(true),
                            "parallel" => config.build.parallel = value.parse().unwrap_or(true),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        
        if config.package.name.is_empty() {
            return Err(FluxError::Package(PackageError::InvalidConfig(
                "Package name is required".to_string()
            )));
        }
        
        Ok(config)
    }

    /// Write configuration to a flux.toml file
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), FluxError> {
        let toml_content = self.to_toml();
        fs::write(path.as_ref(), toml_content)
            .map_err(|e| FluxError::Package(PackageError::IoError(e.to_string())))
    }

    /// Convert configuration to TOML string
    pub fn to_toml(&self) -> String {
        let mut toml = String::new();
        
        // Package section
        toml.push_str("[package]\n");
        toml.push_str(&format!("name = \"{}\"\n", self.package.name));
        toml.push_str(&format!("version = \"{}\"\n", self.package.version));
        toml.push_str(&format!("edition = \"{}\"\n", self.package.edition));
        
        if let Some(ref description) = self.package.description {
            toml.push_str(&format!("description = \"{}\"\n", description));
        }
        
        if !self.package.authors.is_empty() {
            toml.push_str(&format!("authors = [{}]\n", 
                self.package.authors.iter()
                    .map(|a| format!("\"{}\"", a))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        
        if let Some(ref license) = self.package.license {
            toml.push_str(&format!("license = \"{}\"\n", license));
        }
        
        if let Some(ref repository) = self.package.repository {
            toml.push_str(&format!("repository = \"{}\"\n", repository));
        }
        
        if let Some(ref homepage) = self.package.homepage {
            toml.push_str(&format!("homepage = \"{}\"\n", homepage));
        }
        
        if !self.package.keywords.is_empty() {
            toml.push_str(&format!("keywords = [{}]\n",
                self.package.keywords.iter()
                    .map(|k| format!("\"{}\"", k))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        
        if !self.package.categories.is_empty() {
            toml.push_str(&format!("categories = [{}]\n",
                self.package.categories.iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        
        toml.push('\n');
        
        // Dependencies section
        if !self.dependencies.is_empty() {
            toml.push_str("[dependencies]\n");
            for (name, spec) in &self.dependencies {
                toml.push_str(&format!("{} = \"{}\"\n", name, spec.to_string()));
            }
            toml.push('\n');
        }
        
        // Dev dependencies section
        if !self.dev_dependencies.is_empty() {
            toml.push_str("[dev-dependencies]\n");
            for (name, spec) in &self.dev_dependencies {
                toml.push_str(&format!("{} = \"{}\"\n", name, spec.to_string()));
            }
            toml.push('\n');
        }
        
        // Build section
        toml.push_str("[build]\n");
        if let Some(ref target) = self.build.target {
            toml.push_str(&format!("target = \"{}\"\n", target));
        }
        
        let opt_str = match self.build.optimization_level {
            OptimizationLevel::None => "none",
            OptimizationLevel::Speed => "speed",
            OptimizationLevel::Size => "size",
            OptimizationLevel::Debug => "debug",
        };
        toml.push_str(&format!("optimization = \"{}\"\n", opt_str));
        toml.push_str(&format!("debug = {}\n", self.build.debug_info));
        toml.push_str(&format!("incremental = {}\n", self.build.incremental));
        toml.push_str(&format!("parallel = {}\n", self.build.parallel));
        
        toml
    }
}

impl DependencySpec {
    /// Create a dependency spec from a version string
    pub fn from_version_string(version: &str) -> Self {
        DependencySpec {
            version: Some(version.to_string()),
            path: None,
            git: None,
            branch: None,
            tag: None,
            rev: None,
            features: vec![],
            optional: false,
            default_features: true,
        }
    }
    
    /// Create a dependency spec from a local path
    pub fn from_path(path: &str) -> Self {
        DependencySpec {
            version: None,
            path: Some(path.to_string()),
            git: None,
            branch: None,
            tag: None,
            rev: None,
            features: vec![],
            optional: false,
            default_features: true,
        }
    }
    
    /// Create a dependency spec from a git repository
    pub fn from_git(git: &str, branch: Option<&str>, tag: Option<&str>, rev: Option<&str>) -> Self {
        DependencySpec {
            version: None,
            path: None,
            git: Some(git.to_string()),
            branch: branch.map(|s| s.to_string()),
            tag: tag.map(|s| s.to_string()),
            rev: rev.map(|s| s.to_string()),
            features: vec![],
            optional: false,
            default_features: true,
        }
    }
}

impl std::fmt::Display for DependencySpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref version) = self.version {
            write!(f, "{}", version)
        } else if let Some(ref path) = self.path {
            write!(f, "{{ path = \"{}\" }}", path)
        } else if let Some(ref git) = self.git {
            write!(f, "{{ git = \"{}\" }}", git)
        } else {
            write!(f, "*")
        }
    }
}