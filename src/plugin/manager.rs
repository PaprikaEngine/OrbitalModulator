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

//! Plugin Manager - High-level plugin management and coordination
//! 
//! This module provides the main interface for managing plugins in OrbitalModulator,
//! coordinating between loading, sandboxing, and node creation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, Mutex};

use crate::processing::AudioNode;
use crate::plugin::{
    PluginError, PluginResult, PluginConfig, PluginStats,
    api::{PluginNodeFactory, PluginMetadata, PluginCategory},
    loader::{PluginLoader, LoadedPlugin},
    manifest::PluginManifest,
    sandbox::{PluginSandbox, SecurityViolation},
};

/// Central plugin management system
pub struct PluginManager {
    loader: PluginLoader,
    sandboxes: Arc<RwLock<HashMap<String, Arc<PluginSandbox>>>>,
    configs: Arc<RwLock<HashMap<String, PluginConfig>>>,
    host_version: String,
    default_config: PluginConfig,
}

/// Plugin instance information
#[derive(Debug, Clone)]
pub struct PluginInstanceInfo {
    pub plugin_id: String,
    pub node_type: String,
    pub instance_name: String,
    pub created_at: std::time::SystemTime,
    pub stats: PluginStats,
}

/// Plugin search criteria
#[derive(Debug, Clone)]
pub struct PluginSearchCriteria {
    pub category: Option<PluginCategory>,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub node_types: Vec<String>,
    pub min_version: Option<String>,
    pub max_version: Option<String>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(host_version: String) -> Self {
        let loader = PluginLoader::new(host_version.clone());
        
        Self {
            loader,
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
            host_version,
            default_config: PluginConfig::default(),
        }
    }

    /// Add a plugin directory to search path
    pub fn add_plugin_directory<P: AsRef<Path>>(&mut self, path: P) {
        self.loader.add_plugin_directory(path);
    }

    /// Scan for available plugins
    pub fn scan_available_plugins(&self) -> PluginResult<Vec<PluginMetadata>> {
        let manifests = self.loader.scan_plugins()?;
        Ok(manifests.into_iter().map(|m| m.plugin).collect())
    }

    /// Search plugins by criteria
    pub fn search_plugins(&self, criteria: PluginSearchCriteria) -> PluginResult<Vec<PluginMetadata>> {
        let available = self.scan_available_plugins()?;
        
        let filtered = available.into_iter().filter(|plugin| {
            // Filter by category
            if let Some(ref category) = criteria.category {
                if &plugin.category != category {
                    return false;
                }
            }
            
            // Filter by author
            if let Some(ref author) = criteria.author {
                if !plugin.author.to_lowercase().contains(&author.to_lowercase()) {
                    return false;
                }
            }
            
            // Filter by tags
            if !criteria.tags.is_empty() {
                let plugin_tags: Vec<String> = plugin.tags.iter()
                    .map(|t| t.to_lowercase())
                    .collect();
                let search_tags: Vec<String> = criteria.tags.iter()
                    .map(|t| t.to_lowercase())
                    .collect();
                
                if !search_tags.iter().any(|tag| plugin_tags.contains(tag)) {
                    return false;
                }
            }
            
            // Filter by node types
            if !criteria.node_types.is_empty() {
                if !criteria.node_types.iter().any(|nt| plugin.node_types.contains(nt)) {
                    return false;
                }
            }
            
            // TODO: Implement version filtering with proper semver comparison
            
            true
        }).collect();
        
        Ok(filtered)
    }

    /// Load a plugin
    pub fn load_plugin(&self, plugin_id: &str) -> PluginResult<()> {
        // Find plugin manifest
        let manifests = self.loader.scan_plugins()?;
        let manifest = manifests.into_iter()
            .find(|m| m.plugin.id == plugin_id)
            .ok_or_else(|| PluginError::NotFound {
                plugin_id: plugin_id.to_string(),
            })?;

        // Load the plugin
        let loaded_id = self.loader.load_plugin(manifest.clone())?;
        
        // Create sandbox if enabled
        if self.default_config.enable_sandbox {
            let sandbox = Arc::new(PluginSandbox::new(
                plugin_id.to_string(),
                self.default_config.clone(),
                manifest.requirements.clone(),
                manifest.requirements.permissions.clone(),
            ));
            
            sandbox.start_monitoring()?;
            
            let mut sandboxes = self.sandboxes.write().unwrap();
            sandboxes.insert(plugin_id.to_string(), sandbox);
        }
        
        // Store plugin configuration
        {
            let mut configs = self.configs.write().unwrap();
            configs.insert(plugin_id.to_string(), self.default_config.clone());
        }
        
        println!("Plugin loaded successfully: {}", plugin_id);
        Ok(())
    }

    /// Unload a plugin
    pub fn unload_plugin(&self, plugin_id: &str) -> PluginResult<()> {
        // Stop sandbox monitoring
        {
            let mut sandboxes = self.sandboxes.write().unwrap();
            if let Some(sandbox) = sandboxes.remove(plugin_id) {
                sandbox.stop_monitoring()?;
            }
        }
        
        // Remove configuration
        {
            let mut configs = self.configs.write().unwrap();
            configs.remove(plugin_id);
        }
        
        // Unload from loader
        self.loader.unload_plugin(plugin_id)?;
        
        println!("Plugin unloaded successfully: {}", plugin_id);
        Ok(())
    }

    /// Create a node from a loaded plugin
    pub fn create_node(
        &self, 
        plugin_id: &str, 
        node_type: &str, 
        instance_name: String,
        sample_rate: f32
    ) -> PluginResult<Box<dyn AudioNode>> {
        // Get loaded plugin
        let loaded_plugin = self.loader.get_plugin(plugin_id)
            .ok_or_else(|| PluginError::NotFound {
                plugin_id: plugin_id.to_string(),
            })?;
        
        // Check if node type is supported
        if !loaded_plugin.factory.supported_node_types().contains(&node_type.to_string()) {
            return Err(PluginError::ValidationError {
                plugin_id: plugin_id.to_string(),
                reason: format!("Node type '{}' not supported", node_type),
            });
        }
        
        // Create the node
        let node = loaded_plugin.factory.create_node(node_type, instance_name, sample_rate)?;
        
        Ok(node)
    }

    /// Get list of loaded plugins
    pub fn list_loaded_plugins(&self) -> Vec<String> {
        self.loader.list_loaded_plugins()
    }

    /// Get plugin information
    pub fn get_plugin_info(&self, plugin_id: &str) -> Option<PluginMetadata> {
        self.loader.get_plugin(plugin_id)
            .map(|plugin| plugin.manifest.plugin.clone())
    }

    /// Get plugin statistics
    pub fn get_plugin_stats(&self, plugin_id: &str) -> Option<PluginStats> {
        // Try to get stats from sandbox first (more accurate)
        {
            let sandboxes = self.sandboxes.read().unwrap();
            if let Some(sandbox) = sandboxes.get(plugin_id) {
                return Some(sandbox.get_stats());
            }
        }
        
        // Fallback to loader stats
        self.loader.get_plugin_stats(plugin_id)
    }

    /// Get security violations for a plugin
    pub fn get_security_violations(&self, plugin_id: &str) -> Vec<SecurityViolation> {
        let sandboxes = self.sandboxes.read().unwrap();
        sandboxes.get(plugin_id)
            .map(|sandbox| sandbox.get_violations())
            .unwrap_or_default()
    }

    /// Check if plugin should be disabled
    pub fn should_disable_plugin(&self, plugin_id: &str) -> bool {
        let sandboxes = self.sandboxes.read().unwrap();
        sandboxes.get(plugin_id)
            .map(|sandbox| sandbox.should_disable())
            .unwrap_or(false)
    }

    /// Configure a plugin
    pub fn configure_plugin(&self, plugin_id: &str, config: PluginConfig) -> PluginResult<()> {
        // Update stored configuration
        {
            let mut configs = self.configs.write().unwrap();
            configs.insert(plugin_id.to_string(), config.clone());
        }
        
        // Apply to loader
        self.loader.configure_plugin(plugin_id, config)?;
        
        Ok(())
    }

    /// Get plugin configuration
    pub fn get_plugin_config(&self, plugin_id: &str) -> Option<PluginConfig> {
        let configs = self.configs.read().unwrap();
        configs.get(plugin_id).cloned()
    }

    /// Set default plugin configuration
    pub fn set_default_config(&mut self, config: PluginConfig) {
        self.default_config = config;
    }

    /// Get supported node types from all loaded plugins
    pub fn get_all_supported_node_types(&self) -> HashMap<String, Vec<String>> {
        let mut result = HashMap::new();
        
        for plugin_id in self.list_loaded_plugins() {
            if let Some(plugin) = self.loader.get_plugin(&plugin_id) {
                let node_types = plugin.factory.supported_node_types();
                result.insert(plugin_id, node_types);
            }
        }
        
        result
    }

    /// Get plugins by category
    pub fn get_plugins_by_category(&self, category: PluginCategory) -> PluginResult<Vec<PluginMetadata>> {
        let criteria = PluginSearchCriteria {
            category: Some(category),
            author: None,
            tags: Vec::new(),
            node_types: Vec::new(),
            min_version: None,
            max_version: None,
        };
        
        self.search_plugins(criteria)
    }

    /// Auto-disable misbehaving plugins
    pub fn auto_disable_check(&self) -> Vec<String> {
        let mut disabled = Vec::new();
        
        for plugin_id in self.list_loaded_plugins() {
            if self.should_disable_plugin(&plugin_id) {
                if let Err(e) = self.unload_plugin(&plugin_id) {
                    eprintln!("Failed to auto-disable plugin {}: {}", plugin_id, e);
                } else {
                    println!("Auto-disabled misbehaving plugin: {}", plugin_id);
                    disabled.push(plugin_id);
                }
            }
        }
        
        disabled
    }

    /// Generate system-wide plugin report
    pub fn generate_report(&self) -> PluginSystemReport {
        let loaded_plugins = self.list_loaded_plugins();
        let mut plugin_reports = Vec::new();
        
        for plugin_id in &loaded_plugins {
            let info = self.get_plugin_info(plugin_id);
            let stats = self.get_plugin_stats(plugin_id);
            let violations = self.get_security_violations(plugin_id);
            let config = self.get_plugin_config(plugin_id);
            
            plugin_reports.push(PluginReport {
                plugin_id: plugin_id.clone(),
                metadata: info,
                stats,
                violations,
                config,
            });
        }
        
        PluginSystemReport {
            host_version: self.host_version.clone(),
            total_plugins_loaded: loaded_plugins.len(),
            plugin_reports,
            generated_at: std::time::SystemTime::now(),
        }
    }
}

/// Individual plugin report
#[derive(Debug, Clone)]
pub struct PluginReport {
    pub plugin_id: String,
    pub metadata: Option<PluginMetadata>,
    pub stats: Option<PluginStats>,
    pub violations: Vec<SecurityViolation>,
    pub config: Option<PluginConfig>,
}

/// System-wide plugin report
#[derive(Debug, Clone)]
pub struct PluginSystemReport {
    pub host_version: String,
    pub total_plugins_loaded: usize,
    pub plugin_reports: Vec<PluginReport>,
    pub generated_at: std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new("1.0.0".to_string());
        assert_eq!(manager.host_version, "1.0.0");
    }

    #[test]
    fn test_plugin_search() {
        let manager = PluginManager::new("1.0.0".to_string());
        
        let criteria = PluginSearchCriteria {
            category: Some(PluginCategory::Generator),
            author: None,
            tags: vec!["oscillator".to_string()],
            node_types: Vec::new(),
            min_version: None,
            max_version: None,
        };
        
        // This will return empty since no plugins are loaded in test
        let results = manager.search_plugins(criteria).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_default_config() {
        let mut manager = PluginManager::new("1.0.0".to_string());
        
        let custom_config = PluginConfig {
            max_cpu_usage: 0.2,
            max_memory_usage: 128 * 1024 * 1024,
            enable_sandbox: false,
            ..Default::default()
        };
        
        manager.set_default_config(custom_config.clone());
        assert_eq!(manager.default_config.max_cpu_usage, 0.2);
        assert!(!manager.default_config.enable_sandbox);
    }

    #[test]
    fn test_plugin_directory_management() {
        let mut manager = PluginManager::new("1.0.0".to_string());
        let temp_dir = tempdir().unwrap();
        
        manager.add_plugin_directory(temp_dir.path());
        // Directory is added to the loader (no direct way to verify in this test)
    }
}