/*
 * OrbitalModulator - Professional Modular Synthesizer
 * Copyright (c) 2025 MACHIKO LAB
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! Plugin Manifest System
//! 
//! Handles plugin metadata, validation, and package management.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

use crate::plugin::{PluginError, PluginResult, PluginMetadata, PluginLicense};

/// Plugin package manifest structure
/// 
/// This is the main configuration file for a plugin package (orbital-plugin.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata
    pub plugin: PluginMetadata,
    
    /// Build information
    pub build: BuildInfo,
    
    /// Package files and checksums
    pub files: HashMap<String, FileInfo>,
    
    /// Plugin dependencies
    pub dependencies: HashMap<String, DependencyInfo>,
    
    /// Installation requirements
    pub requirements: Requirements,
    
    /// Digital signature (for verification)
    pub signature: Option<String>,
}

/// Build information for the plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    pub target: String,           // Target platform (e.g., "x86_64-unknown-linux-gnu")
    pub rust_version: String,     // Rust version used to build
    pub orbital_version: String,  // OrbitalModulator version used
    pub build_date: String,       // ISO 8601 build timestamp
    pub build_hash: String,       // Git commit hash if available
    pub optimization: String,     // Optimization level (debug/release)
    pub features: Vec<String>,    // Enabled features
}

/// File information with integrity checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,          // Relative path in package
    pub sha256: String,        // SHA256 checksum
    pub size: u64,             // File size in bytes
    pub executable: bool,      // Whether file is executable
    pub required: bool,        // Whether file is required for operation
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub version: String,       // Required version (semver)
    pub optional: bool,        // Whether dependency is optional
    pub features: Vec<String>, // Required features
    pub source: DependencySource, // Where to find the dependency
}

/// Source of a plugin dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencySource {
    OrbitalStore,              // Official OrbitalModulator plugin store
    Git { url: String, rev: Option<String> }, // Git repository
    Path { path: String },     // Local file path
    Url { url: String },       // Direct download URL
}

/// Installation and runtime requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirements {
    pub min_memory: u64,       // Minimum memory in bytes
    pub min_cpu_cores: u32,    // Minimum CPU cores
    pub max_cpu_usage: f32,    // Maximum CPU usage (0.0-1.0)
    pub network_access: bool,  // Requires network access
    pub file_access: Vec<String>, // Required file system paths
    pub platforms: Vec<String>, // Supported platforms
    pub permissions: Vec<Permission>, // Required permissions
}

/// Plugin permission types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    FileRead { path: String },    // Read access to specific path
    FileWrite { path: String },   // Write access to specific path
    Network { domains: Vec<String> }, // Network access to domains
    Audio,                        // Audio device access
    Midi,                        // MIDI device access
    System,                      // System information access
}

impl PluginManifest {
    /// Load manifest from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> PluginResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            PluginError::LoadError {
                plugin_id: "unknown".to_string(),
                reason: format!("Failed to read manifest: {}", e),
            }
        })?;
        
        Self::load_from_str(&content)
    }
    
    /// Load manifest from TOML string
    pub fn load_from_str(content: &str) -> PluginResult<Self> {
        toml::from_str(content).map_err(|e| {
            PluginError::ValidationError {
                plugin_id: "unknown".to_string(),
                reason: format!("Invalid manifest format: {}", e),
            }
        })
    }
    
    /// Save manifest to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> PluginResult<()> {
        let content = toml::to_string_pretty(self).map_err(|e| {
            PluginError::Internal {
                message: format!("Failed to serialize manifest: {}", e),
            }
        })?;
        
        std::fs::write(path, content).map_err(|e| {
            PluginError::Internal {
                message: format!("Failed to write manifest: {}", e),
            }
        })
    }
    
    /// Validate manifest integrity and compatibility
    pub fn validate(&self, host_version: &str) -> PluginResult<()> {
        // Validate plugin metadata
        self.validate_metadata()?;
        
        // Check version compatibility
        self.check_version_compatibility(host_version)?;
        
        // Validate file integrity
        self.validate_files()?;
        
        // Check license compatibility
        self.validate_license()?;
        
        // Validate requirements
        self.validate_requirements()?;
        
        Ok(())
    }
    
    /// Validate plugin metadata
    fn validate_metadata(&self) -> PluginResult<()> {
        if self.plugin.id.is_empty() {
            return Err(PluginError::ValidationError {
                plugin_id: self.plugin.id.clone(),
                reason: "Plugin ID cannot be empty".to_string(),
            });
        }
        
        if self.plugin.name.is_empty() {
            return Err(PluginError::ValidationError {
                plugin_id: self.plugin.id.clone(),
                reason: "Plugin name cannot be empty".to_string(),
            });
        }
        
        // Validate semantic version format
        if !self.is_valid_semver(&self.plugin.version) {
            return Err(PluginError::ValidationError {
                plugin_id: self.plugin.id.clone(),
                reason: format!("Invalid version format: {}", self.plugin.version),
            });
        }
        
        Ok(())
    }
    
    /// Check version compatibility
    fn check_version_compatibility(&self, host_version: &str) -> PluginResult<()> {
        if !self.is_version_compatible(&self.plugin.min_orbital_version, host_version) {
            return Err(PluginError::VersionMismatch {
                plugin_id: self.plugin.id.clone(),
                required: self.plugin.min_orbital_version.clone(),
                found: host_version.to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Validate file integrity using checksums
    fn validate_files(&self) -> PluginResult<()> {
        for (file_path, file_info) in &self.files {
            if file_info.required {
                let path = Path::new(file_path);
                if !path.exists() {
                    return Err(PluginError::ValidationError {
                        plugin_id: self.plugin.id.clone(),
                        reason: format!("Required file not found: {}", file_path),
                    });
                }
                
                // Verify checksum
                let actual_hash = self.calculate_file_hash(path)?;
                if actual_hash != file_info.sha256 {
                    return Err(PluginError::ValidationError {
                        plugin_id: self.plugin.id.clone(),
                        reason: format!("File checksum mismatch: {}", file_path),
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate license compatibility
    fn validate_license(&self) -> PluginResult<()> {
        match &self.plugin.license {
            PluginLicense::AGPL3 | PluginLicense::GPL3 | PluginLicense::MIT | 
            PluginLicense::Apache2 | PluginLicense::BSD3 => {
                // Compatible open source licenses
                Ok(())
            }
            PluginLicense::Proprietary => {
                // Proprietary plugins require special approval
                Err(PluginError::LicenseIncompatible {
                    plugin_id: self.plugin.id.clone(),
                    license: "Proprietary license requires approval".to_string(),
                })
            }
            PluginLicense::Custom(license) => {
                // Custom licenses need manual review
                Err(PluginError::LicenseIncompatible {
                    plugin_id: self.plugin.id.clone(),
                    license: format!("Custom license requires review: {}", license),
                })
            }
        }
    }
    
    /// Validate system requirements
    fn validate_requirements(&self) -> PluginResult<()> {
        // Check available memory
        if let Ok(available_memory) = self.get_available_memory() {
            if available_memory < self.requirements.min_memory {
                return Err(PluginError::ResourceLimit {
                    plugin_id: self.plugin.id.clone(),
                    resource: "memory".to_string(),
                    limit: format!("Required: {} bytes, Available: {} bytes", 
                                 self.requirements.min_memory, available_memory),
                });
            }
        }
        
        // Check CPU cores
        let cpu_cores = num_cpus::get() as u32;
        if cpu_cores < self.requirements.min_cpu_cores {
            return Err(PluginError::ResourceLimit {
                plugin_id: self.plugin.id.clone(),
                resource: "cpu_cores".to_string(),
                limit: format!("Required: {}, Available: {}", 
                             self.requirements.min_cpu_cores, cpu_cores),
            });
        }
        
        Ok(())
    }
    
    /// Calculate SHA256 hash of a file
    fn calculate_file_hash(&self, path: &Path) -> PluginResult<String> {
        let content = std::fs::read(path).map_err(|e| {
            PluginError::ValidationError {
                plugin_id: self.plugin.id.clone(),
                reason: format!("Failed to read file {}: {}", path.display(), e),
            }
        })?;
        
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        
        Ok(format!("{:x}", hash))
    }
    
    /// Check if version string is valid semantic version
    fn is_valid_semver(&self, version: &str) -> bool {
        // Simple semver validation (major.minor.patch)
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }
        
        parts.iter().all(|part| part.parse::<u32>().is_ok())
    }
    
    /// Check if plugin version is compatible with host version
    fn is_version_compatible(&self, required: &str, available: &str) -> bool {
        // Simple version comparison (in practice, use a semver library)
        let req_parts: Vec<u32> = required.split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect();
        let avail_parts: Vec<u32> = available.split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect();
        
        if req_parts.len() >= 1 && avail_parts.len() >= 1 {
            // Major version must match
            if req_parts[0] != avail_parts[0] {
                return false;
            }
            
            // Available minor version must be >= required
            if req_parts.len() >= 2 && avail_parts.len() >= 2 {
                if avail_parts[1] < req_parts[1] {
                    return false;
                }
            }
        }
        
        true
    }
    
    /// Get available system memory (stub implementation)
    fn get_available_memory(&self) -> Result<u64, ()> {
        // In a real implementation, this would query system memory
        // For now, assume we have enough memory
        Ok(8 * 1024 * 1024 * 1024) // 8GB
    }
}

/// Plugin manifest builder for creating new manifests
pub struct ManifestBuilder {
    manifest: PluginManifest,
}

impl ManifestBuilder {
    pub fn new(plugin_id: String, name: String, version: String) -> Self {
        let plugin = PluginMetadata {
            id: plugin_id,
            name,
            version,
            description: String::new(),
            author: String::new(),
            website: None,
            category: crate::plugin::PluginCategory::Custom("Unknown".to_string()),
            license: PluginLicense::MIT,
            api_version: crate::plugin::PLUGIN_API_VERSION,
            node_types: Vec::new(),
            dependencies: Vec::new(),
            tags: Vec::new(),
            min_orbital_version: "1.0.0".to_string(),
        };
        
        let build = BuildInfo {
            target: std::env::consts::ARCH.to_string(),
            rust_version: "1.70.0".to_string(), // Default
            orbital_version: "1.0.0".to_string(),
            build_date: chrono::Utc::now().to_rfc3339(),
            build_hash: String::new(),
            optimization: "release".to_string(),
            features: Vec::new(),
        };
        
        let requirements = Requirements {
            min_memory: 64 * 1024 * 1024, // 64MB
            min_cpu_cores: 1,
            max_cpu_usage: 0.1, // 10%
            network_access: false,
            file_access: Vec::new(),
            platforms: vec![std::env::consts::OS.to_string()],
            permissions: Vec::new(),
        };
        
        let manifest = PluginManifest {
            plugin,
            build,
            files: HashMap::new(),
            dependencies: HashMap::new(),
            requirements,
            signature: None,
        };
        
        Self { manifest }
    }
    
    pub fn description(mut self, description: String) -> Self {
        self.manifest.plugin.description = description;
        self
    }
    
    pub fn author(mut self, author: String) -> Self {
        self.manifest.plugin.author = author;
        self
    }
    
    pub fn category(mut self, category: crate::plugin::PluginCategory) -> Self {
        self.manifest.plugin.category = category;
        self
    }
    
    pub fn license(mut self, license: PluginLicense) -> Self {
        self.manifest.plugin.license = license;
        self
    }
    
    pub fn add_node_type(mut self, node_type: String) -> Self {
        self.manifest.plugin.node_types.push(node_type);
        self
    }
    
    pub fn add_file(mut self, path: String, required: bool) -> PluginResult<Self> {
        let file_path = Path::new(&path);
        if file_path.exists() {
            let metadata = std::fs::metadata(&path).map_err(|e| {
                PluginError::Internal {
                    message: format!("Failed to read file metadata: {}", e),
                }
            })?;
            
            let hash = self.manifest.calculate_file_hash(file_path)?;
            
            let file_info = FileInfo {
                path: path.clone(),
                sha256: hash,
                size: metadata.len(),
                executable: false, // Could be detected from file permissions
                required,
            };
            
            self.manifest.files.insert(path, file_info);
        }
        
        Ok(self)
    }
    
    pub fn build(self) -> PluginManifest {
        self.manifest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_manifest_builder() {
        let manifest = ManifestBuilder::new(
            "test_plugin".to_string(),
            "Test Plugin".to_string(),
            "1.0.0".to_string(),
        )
        .description("A test plugin".to_string())
        .author("Test Author".to_string())
        .category(crate::plugin::PluginCategory::Generator)
        .license(PluginLicense::MIT)
        .add_node_type("test_oscillator".to_string())
        .build();
        
        assert_eq!(manifest.plugin.id, "test_plugin");
        assert_eq!(manifest.plugin.name, "Test Plugin");
        assert_eq!(manifest.plugin.description, "A test plugin");
        assert_eq!(manifest.plugin.node_types, vec!["test_oscillator"]);
    }
    
    #[test]
    fn test_manifest_serialization() {
        let manifest = ManifestBuilder::new(
            "test".to_string(),
            "Test".to_string(),
            "1.0.0".to_string(),
        ).build();
        
        let toml = toml::to_string(&manifest).unwrap();
        let deserialized: PluginManifest = toml::from_str(&toml).unwrap();
        
        assert_eq!(manifest.plugin.id, deserialized.plugin.id);
    }
    
    #[test]
    fn test_version_compatibility() {
        let manifest = ManifestBuilder::new(
            "test".to_string(),
            "Test".to_string(),
            "1.0.0".to_string(),
        ).build();
        
        assert!(manifest.is_version_compatible("1.0.0", "1.0.0"));
        assert!(manifest.is_version_compatible("1.0.0", "1.1.0"));
        assert!(!manifest.is_version_compatible("1.1.0", "1.0.0"));
        assert!(!manifest.is_version_compatible("1.0.0", "2.0.0"));
    }
}