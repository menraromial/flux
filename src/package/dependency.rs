//! Dependency specification and resolution system

use crate::error::{FluxError, PackageError};
use super::DependencySpec;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Resolved dependency information
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: String,
    pub source: DependencySource,
    pub path: PathBuf,
    pub features: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Source of a dependency
#[derive(Debug, Clone, PartialEq)]
pub enum DependencySource {
    Registry { version: String },
    Path { path: PathBuf },
    Git { url: String, rev: String },
}

/// Dependency resolver
pub struct DependencyResolver {
    registry_cache: HashMap<String, Vec<RegistryPackage>>,
    resolved_cache: HashMap<String, ResolvedDependency>,
}

/// Package information from registry
#[derive(Debug, Clone)]
pub struct RegistryPackage {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub dependencies: HashMap<String, DependencySpec>,
    pub features: HashMap<String, Vec<String>>,
    pub download_url: String,
    pub checksum: String,
}

/// Version requirement specification
#[derive(Debug, Clone, PartialEq)]
pub enum VersionReq {
    Exact(String),
    Range { min: String, max: Option<String> },
    Caret(String),  // ^1.2.3 - compatible version
    Tilde(String),  // ~1.2.3 - reasonably close version
    Wildcard(String), // 1.2.* - wildcard version
    Any,
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new() -> Self {
        DependencyResolver {
            registry_cache: HashMap::new(),
            resolved_cache: HashMap::new(),
        }
    }
    
    /// Resolve all dependencies for a project
    pub fn resolve(&self, dependencies: &HashMap<String, DependencySpec>) -> Result<Vec<ResolvedDependency>, FluxError> {
        let mut resolved = Vec::new();
        let mut visited = HashSet::new();
        
        for (name, spec) in dependencies {
            self.resolve_dependency(name, spec, &mut resolved, &mut visited)?;
        }
        
        // Sort by dependency order (dependencies first, then dependents)
        self.topological_sort(&mut resolved)?;
        
        Ok(resolved)
    }
    
    /// Resolve a single dependency recursively
    fn resolve_dependency(
        &self,
        name: &str,
        spec: &DependencySpec,
        resolved: &mut Vec<ResolvedDependency>,
        visited: &mut HashSet<String>,
    ) -> Result<(), FluxError> {
        if visited.contains(name) {
            return Ok(()); // Already processed
        }
        
        visited.insert(name.to_string());
        
        let dependency = match self.resolve_single_dependency(name, spec)? {
            Some(dep) => dep,
            None => return Err(FluxError::Package(PackageError::DependencyResolutionFailed(
                format!("Could not resolve dependency: {}", name)
            ))),
        };
        
        // Recursively resolve dependencies of this dependency
        let dep_config = self.load_dependency_config(&dependency)?;
        for (dep_name, dep_spec) in &dep_config.dependencies {
            self.resolve_dependency(dep_name, dep_spec, resolved, visited)?;
        }
        
        resolved.push(dependency);
        Ok(())
    }
    
    /// Resolve a single dependency without recursion
    fn resolve_single_dependency(&self, name: &str, spec: &DependencySpec) -> Result<Option<ResolvedDependency>, FluxError> {
        if let Some(cached) = self.resolved_cache.get(name) {
            return Ok(Some(cached.clone()));
        }
        
        let resolved = if let Some(ref path) = spec.path {
            // Local path dependency
            self.resolve_path_dependency(name, path, spec)?
        } else if let Some(ref git_url) = spec.git {
            // Git dependency
            self.resolve_git_dependency(name, git_url, spec)?
        } else if let Some(ref version) = spec.version {
            // Registry dependency
            self.resolve_registry_dependency(name, version, spec)?
        } else {
            return Err(FluxError::Package(PackageError::DependencyResolutionFailed(
                format!("Invalid dependency specification for {}", name)
            )));
        };
        
        Ok(Some(resolved))
    }
    
    /// Resolve a local path dependency
    fn resolve_path_dependency(&self, name: &str, path: &str, spec: &DependencySpec) -> Result<ResolvedDependency, FluxError> {
        let path_buf = PathBuf::from(path);
        
        if !path_buf.exists() {
            return Err(FluxError::Package(PackageError::DependencyResolutionFailed(
                format!("Path dependency not found: {}", path)
            )));
        }
        
        // Load the dependency's flux.toml to get version and other info
        let config_path = path_buf.join("flux.toml");
        let config = super::ProjectConfig::from_file(&config_path)?;
        
        Ok(ResolvedDependency {
            name: name.to_string(),
            version: config.package.version,
            source: DependencySource::Path { path: path_buf.clone() },
            path: path_buf,
            features: spec.features.clone(),
            dependencies: config.dependencies.keys().cloned().collect(),
        })
    }
    
    /// Resolve a git dependency
    fn resolve_git_dependency(&self, name: &str, git_url: &str, spec: &DependencySpec) -> Result<ResolvedDependency, FluxError> {
        // In a real implementation, this would clone the git repository
        // and resolve the specific revision/branch/tag
        let cache_dir = self.get_git_cache_dir(git_url, spec)?;
        
        // For now, simulate git resolution
        let version = spec.tag.as_ref()
            .or(spec.rev.as_ref())
            .or(spec.branch.as_ref())
            .unwrap_or(&"main".to_string())
            .clone();
        
        Ok(ResolvedDependency {
            name: name.to_string(),
            version: version.clone(),
            source: DependencySource::Git { 
                url: git_url.to_string(), 
                rev: version.clone() 
            },
            path: cache_dir,
            features: spec.features.clone(),
            dependencies: vec![], // Would be loaded from the git repo's flux.toml
        })
    }
    
    /// Resolve a registry dependency
    fn resolve_registry_dependency(&self, name: &str, version: &str, spec: &DependencySpec) -> Result<ResolvedDependency, FluxError> {
        // In a real implementation, this would query a package registry
        let version_req = VersionReq::parse(version)?;
        let package = self.find_registry_package(name, &version_req)?;
        
        let cache_dir = self.get_registry_cache_dir(name, &package.version)?;
        
        Ok(ResolvedDependency {
            name: name.to_string(),
            version: package.version.clone(),
            source: DependencySource::Registry { version: package.version },
            path: cache_dir,
            features: spec.features.clone(),
            dependencies: package.dependencies.keys().cloned().collect(),
        })
    }
    
    /// Load dependency configuration from resolved dependency
    fn load_dependency_config(&self, dependency: &ResolvedDependency) -> Result<super::ProjectConfig, FluxError> {
        let config_path = dependency.path.join("flux.toml");
        super::ProjectConfig::from_file(&config_path)
    }
    
    /// Get cache directory for git dependency
    fn get_git_cache_dir(&self, git_url: &str, spec: &DependencySpec) -> Result<PathBuf, FluxError> {
        // Create a cache directory based on git URL and revision
        let mut cache_dir = PathBuf::from(".flux/cache/git");
        
        // Sanitize git URL for directory name
        let hash_bytes = md5::compute(git_url.as_bytes());
        let url_hash = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        cache_dir.push(&url_hash);
        
        if let Some(ref rev) = spec.rev {
            cache_dir.push(rev);
        } else if let Some(ref tag) = spec.tag {
            cache_dir.push(tag);
        } else if let Some(ref branch) = spec.branch {
            cache_dir.push(branch);
        } else {
            cache_dir.push("main");
        }
        
        Ok(cache_dir)
    }
    
    /// Get cache directory for registry dependency
    fn get_registry_cache_dir(&self, name: &str, version: &str) -> Result<PathBuf, FluxError> {
        let mut cache_dir = PathBuf::from(".flux/cache/registry");
        cache_dir.push(name);
        cache_dir.push(version);
        Ok(cache_dir)
    }
    
    /// Find a package in the registry that matches the version requirement
    fn find_registry_package(&self, name: &str, _version_req: &VersionReq) -> Result<RegistryPackage, FluxError> {
        // In a real implementation, this would query a package registry
        // For now, simulate with a mock package
        Ok(RegistryPackage {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: Some(format!("Mock package {}", name)),
            dependencies: HashMap::new(),
            features: HashMap::new(),
            download_url: format!("https://registry.flux-lang.org/{}/1.0.0/download", name),
            checksum: "mock-checksum".to_string(),
        })
    }
    
    /// Perform topological sort on resolved dependencies
    fn topological_sort(&self, resolved: &mut Vec<ResolvedDependency>) -> Result<(), FluxError> {
        // Simple topological sort based on dependency relationships
        // In a real implementation, this would be more sophisticated
        resolved.sort_by(|a, b| {
            if a.dependencies.contains(&b.name) {
                std::cmp::Ordering::Greater
            } else if b.dependencies.contains(&a.name) {
                std::cmp::Ordering::Less
            } else {
                a.name.cmp(&b.name)
            }
        });
        
        Ok(())
    }
}

impl VersionReq {
    /// Parse a version requirement string
    pub fn parse(version: &str) -> Result<Self, FluxError> {
        let version = version.trim();
        
        if version == "*" {
            return Ok(VersionReq::Any);
        }
        
        if version.starts_with('^') {
            return Ok(VersionReq::Caret(version[1..].to_string()));
        }
        
        if version.starts_with('~') {
            return Ok(VersionReq::Tilde(version[1..].to_string()));
        }
        
        if version.contains('*') {
            return Ok(VersionReq::Wildcard(version.to_string()));
        }
        
        if version.contains("..") {
            let parts: Vec<&str> = version.split("..").collect();
            if parts.len() == 2 {
                return Ok(VersionReq::Range {
                    min: parts[0].to_string(),
                    max: if parts[1].is_empty() { None } else { Some(parts[1].to_string()) },
                });
            }
        }
        
        Ok(VersionReq::Exact(version.to_string()))
    }
    
    /// Check if a version satisfies this requirement
    pub fn matches(&self, version: &str) -> bool {
        match self {
            VersionReq::Any => true,
            VersionReq::Exact(req) => version == req,
            VersionReq::Caret(req) => self.matches_caret(version, req),
            VersionReq::Tilde(req) => self.matches_tilde(version, req),
            VersionReq::Wildcard(req) => self.matches_wildcard(version, req),
            VersionReq::Range { min, max } => {
                self.version_gte(version, min) && 
                max.as_ref().map_or(true, |m| self.version_lt(version, m))
            }
        }
    }
    
    /// Check caret version matching (^1.2.3 matches >=1.2.3 <2.0.0)
    fn matches_caret(&self, version: &str, req: &str) -> bool {
        // Simplified caret matching
        let version_parts = self.parse_version(version);
        let req_parts = self.parse_version(req);
        
        if version_parts.len() < 3 || req_parts.len() < 3 {
            return false;
        }
        
        // Major version must match
        if version_parts[0] != req_parts[0] {
            return false;
        }
        
        // Version must be >= requirement
        self.version_gte(version, req)
    }
    
    /// Check tilde version matching (~1.2.3 matches >=1.2.3 <1.3.0)
    fn matches_tilde(&self, version: &str, req: &str) -> bool {
        // Simplified tilde matching
        let version_parts = self.parse_version(version);
        let req_parts = self.parse_version(req);
        
        if version_parts.len() < 2 || req_parts.len() < 2 {
            return false;
        }
        
        // Major and minor versions must match
        if version_parts[0] != req_parts[0] || version_parts[1] != req_parts[1] {
            return false;
        }
        
        // Version must be >= requirement
        self.version_gte(version, req)
    }
    
    /// Check wildcard version matching (1.2.* matches 1.2.x for any x)
    fn matches_wildcard(&self, version: &str, req: &str) -> bool {
        let req_pattern = req.replace('*', "");
        version.starts_with(&req_pattern)
    }
    
    /// Check if version1 >= version2
    fn version_gte(&self, version1: &str, version2: &str) -> bool {
        let v1_parts = self.parse_version(version1);
        let v2_parts = self.parse_version(version2);
        
        for i in 0..std::cmp::max(v1_parts.len(), v2_parts.len()) {
            let v1_part = v1_parts.get(i).unwrap_or(&0);
            let v2_part = v2_parts.get(i).unwrap_or(&0);
            
            if v1_part > v2_part {
                return true;
            } else if v1_part < v2_part {
                return false;
            }
        }
        
        true // Equal
    }
    
    /// Check if version1 < version2
    fn version_lt(&self, version1: &str, version2: &str) -> bool {
        !self.version_gte(version1, version2) || version1 == version2
    }
    
    /// Parse version string into numeric parts
    fn parse_version(&self, version: &str) -> Vec<u32> {
        version.split('.')
            .filter_map(|part| part.parse().ok())
            .collect()
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

// Simple MD5 implementation for hashing (in a real implementation, use a proper crypto crate)
mod md5 {
    pub fn compute(data: &[u8]) -> [u8; 16] {
        // Simplified hash - in real implementation use proper MD5
        let mut hash = [0u8; 16];
        for (i, &byte) in data.iter().enumerate() {
            hash[i % 16] ^= byte;
        }
        hash
    }
}