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

//! Plugin Loader - Dynamic loading and management of plugin libraries
//! 
//! This module handles the runtime loading of plugin shared libraries,
//! symbol resolution, and lifecycle management.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use libloading::{Library, Symbol};

use crate::plugin::{
    PluginError, PluginResult, PluginConfig, PluginStats,
    api::{
        PluginNodeFactory, PluginMetadata, PLUGIN_API_VERSION,
        CreatePluginFactoryFn, DestroyPluginFactoryFn, 
        GetPluginInfoFn, GetApiVersionFn
    },
    manifest::PluginManifest,
};

/// Loaded plugin information
#[derive(Debug)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub factory: Box<dyn PluginNodeFactory>,
    pub library: Library,
    pub stats: Arc<RwLock<PluginStats>>,
    pub config: PluginConfig,
    pub loaded_at: std::time::SystemTime,
}

/// Plugin loader responsible for dynamic loading and unloading
pub struct PluginLoader {
    loaded_plugins: Arc<RwLock<HashMap<String, Arc<LoadedPlugin>>>>,
    plugin_directories: Vec<PathBuf>,
    host_version: String,
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new(host_version: String) -> Self {
        let mut plugin_directories = Vec::new();
        
        // Add default plugin directories
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                plugin_directories.push(exe_dir.join("plugins"));
            }
        }
        
        // Add user plugins directory
        if let Some(home_dir) = dirs::home_dir() {
            plugin_directories.push(home_dir.join(".orbital-modulator").join("plugins"));
        }
        
        // Add system plugins directory
        #[cfg(unix)]
        plugin_directories.push(PathBuf::from("/usr/local/share/orbital-modulator/plugins"));
        
        #[cfg(windows)]
        if let Ok(program_files) = std::env::var("PROGRAMFILES") {
            plugin_directories.push(
                PathBuf::from(program_files)
                    .join("OrbitalModulator")
                    .join("plugins")
            );
        }
        
        Self {
            loaded_plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_directories,
            host_version,
        }
    }
    
    /// Add a plugin search directory
    pub fn add_plugin_directory<P: AsRef<Path>>(&mut self, path: P) {
        self.plugin_directories.push(path.as_ref().to_path_buf());
    }
    
    /// Scan all plugin directories for available plugins
    pub fn scan_plugins(&self) -> PluginResult<Vec<PluginManifest>> {
        let mut manifests = Vec::new();
        
        for dir in &self.plugin_directories {
            if dir.exists() && dir.is_dir() {
                match self.scan_directory(dir) {
                    Ok(mut dir_manifests) => manifests.append(&mut dir_manifests),
                    Err(e) => {
                        eprintln!("Warning: Failed to scan plugin directory {}: {}", dir.display(), e);
                    }
                }
            }
        }
        
        Ok(manifests)
    }
    
    /// Scan a specific directory for plugins
    fn scan_directory(&self, dir: &Path) -> PluginResult<Vec<PluginManifest>> {
        let mut manifests = Vec::new();
        
        let entries = std::fs::read_dir(dir).map_err(|e| {
            PluginError::LoadError {
                plugin_id: "unknown".to_string(),
                reason: format!("Failed to read directory {}: {}", dir.display(), e),
            }
        })?;
        
        for entry in entries {
            let entry = entry.map_err(|e| {
                PluginError::LoadError {
                    plugin_id: "unknown".to_string(),
                    reason: format!("Failed to read directory entry: {}", e),
                }
            })?;
            
            let path = entry.path();
            
            // Look for plugin manifest files
            if path.is_file() && path.file_name() == Some(std::ffi::OsStr::new("orbital-plugin.toml")) {
                match PluginManifest::load_from_file(&path) {
                    Ok(manifest) => {
                        // Validate manifest
                        if let Err(e) = manifest.validate(&self.host_version) {
                            eprintln!("Warning: Invalid plugin manifest {}: {}", path.display(), e);
                            continue;
                        }
                        manifests.push(manifest);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load plugin manifest {}: {}", path.display(), e);
                    }
                }
            }
            
            // Recursively scan subdirectories
            if path.is_dir() {
                match self.scan_directory(&path) {
                    Ok(mut subdir_manifests) => manifests.append(&mut subdir_manifests),
                    Err(e) => {
                        eprintln!("Warning: Failed to scan subdirectory {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        Ok(manifests)
    }
    
    /// Load a plugin from a manifest
    pub fn load_plugin(&self, manifest: PluginManifest) -> PluginResult<String> {
        let plugin_id = manifest.plugin.id.clone();
        
        // Check if already loaded
        {
            let loaded = self.loaded_plugins.read().unwrap();
            if loaded.contains_key(&plugin_id) {
                return Err(PluginError::LoadError {
                    plugin_id,
                    reason: "Plugin already loaded".to_string(),
                });
            }
        }
        
        // Find the plugin library file
        let library_path = self.find_library_path(&manifest)?;
        
        // Load the dynamic library
        let library = unsafe {
            Library::new(&library_path).map_err(|e| {
                PluginError::LoadError {
                    plugin_id: plugin_id.clone(),
                    reason: format!("Failed to load library: {}", e),
                }
            })?
        };
        
        // Verify API version compatibility
        self.verify_api_version(&library, &plugin_id)?;
        
        // Get plugin info and validate
        let plugin_info = self.get_plugin_info(&library, &plugin_id)?;
        
        // Create plugin factory
        let factory = self.create_plugin_factory(&library, &plugin_id)?;
        
        // Initialize plugin
        let config = PluginConfig::default();
        let mut factory_mut = factory;
        factory_mut.initialize(&config)?;
        
        // Create loaded plugin entry
        let loaded_plugin = Arc::new(LoadedPlugin {
            manifest,
            factory: factory_mut,
            library,
            stats: Arc::new(RwLock::new(PluginStats::default())),
            config,
            loaded_at: std::time::SystemTime::now(),
        });
        
        // Add to loaded plugins
        {
            let mut loaded = self.loaded_plugins.write().unwrap();
            loaded.insert(plugin_id.clone(), loaded_plugin);
        }
        
        println!("Successfully loaded plugin: {}", plugin_id);
        Ok(plugin_id)
    }
    
    /// Unload a plugin
    pub fn unload_plugin(&self, plugin_id: &str) -> PluginResult<()> {
        let loaded_plugin = {
            let mut loaded = self.loaded_plugins.write().unwrap();
            loaded.remove(plugin_id).ok_or_else(|| {
                PluginError::NotFound {
                    plugin_id: plugin_id.to_string(),
                }
            })?
        };
        
        // The plugin will be automatically cleaned up when the Arc is dropped
        // The Library destructor will unload the shared library
        
        println!("Successfully unloaded plugin: {}", plugin_id);
        Ok(())
    }
    
    /// Get a loaded plugin
    pub fn get_plugin(&self, plugin_id: &str) -> Option<Arc<LoadedPlugin>> {
        let loaded = self.loaded_plugins.read().unwrap();
        loaded.get(plugin_id).cloned()
    }
    
    /// List all loaded plugins
    pub fn list_loaded_plugins(&self) -> Vec<String> {
        let loaded = self.loaded_plugins.read().unwrap();
        loaded.keys().cloned().collect()
    }
    
    /// Get plugin statistics
    pub fn get_plugin_stats(&self, plugin_id: &str) -> Option<PluginStats> {
        let loaded = self.loaded_plugins.read().unwrap();
        loaded.get(plugin_id).map(|plugin| {
            let stats = plugin.stats.read().unwrap();
            stats.clone()
        })
    }
    
    /// Update plugin configuration
    pub fn configure_plugin(&self, plugin_id: &str, config: PluginConfig) -> PluginResult<()> {
        let loaded = self.loaded_plugins.read().unwrap();
        let plugin = loaded.get(plugin_id).ok_or_else(|| {
            PluginError::NotFound {
                plugin_id: plugin_id.to_string(),
            }
        })?;
        
        // Note: This is a simplified implementation
        // In practice, we'd need interior mutability for the factory
        // or a different architecture to allow configuration updates
        
        Ok(())
    }
    
    /// Find the library file for a plugin
    fn find_library_path(&self, manifest: &PluginManifest) -> PluginResult<PathBuf> {
        // Look for the main library file in the plugin's files
        let lib_extension = if cfg!(windows) { ".dll" } else if cfg!(target_os = "macos") { ".dylib" } else { ".so" };
        
        for (file_path, file_info) in &manifest.files {
            if file_path.ends_with(lib_extension) && file_info.executable {
                let path = Path::new(file_path);
                if path.exists() {
                    return Ok(path.to_path_buf());
                }
                
                // Try relative to plugin directories
                for plugin_dir in &self.plugin_directories {
                    let full_path = plugin_dir.join(file_path);
                    if full_path.exists() {
                        return Ok(full_path);
                    }
                }
            }
        }
        
        Err(PluginError::LoadError {
            plugin_id: manifest.plugin.id.clone(),
            reason: "Plugin library file not found".to_string(),
        })
    }
    
    /// Verify API version compatibility
    fn verify_api_version(&self, library: &Library, plugin_id: &str) -> PluginResult<()> {
        let get_version: Symbol<GetApiVersionFn> = unsafe {
            library.get(b"get_api_version").map_err(|e| {
                PluginError::LoadError {
                    plugin_id: plugin_id.to_string(),
                    reason: format!("Missing get_api_version function: {}", e),
                }
            })?
        };
        
        let plugin_version = get_version();
        if plugin_version != PLUGIN_API_VERSION {
            return Err(PluginError::VersionMismatch {
                plugin_id: plugin_id.to_string(),
                required: PLUGIN_API_VERSION.to_string(),
                found: plugin_version.to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Get plugin information from library
    fn get_plugin_info(&self, library: &Library, plugin_id: &str) -> PluginResult<PluginMetadata> {
        let get_info: Symbol<GetPluginInfoFn> = unsafe {
            library.get(b"get_plugin_info").map_err(|e| {
                PluginError::LoadError {
                    plugin_id: plugin_id.to_string(),
                    reason: format!("Missing get_plugin_info function: {}", e),
                }
            })?
        };
        
        let info_ptr = get_info();
        if info_ptr.is_null() {
            return Err(PluginError::LoadError {
                plugin_id: plugin_id.to_string(),
                reason: "get_plugin_info returned null".to_string(),
            });
        }
        
        let info_cstr = unsafe { CStr::from_ptr(info_ptr) };
        let info_str = info_cstr.to_str().map_err(|e| {
            PluginError::LoadError {
                plugin_id: plugin_id.to_string(),
                reason: format!("Invalid plugin info string: {}", e),
            }
        })?;
        
        serde_json::from_str(info_str).map_err(|e| {
            PluginError::LoadError {
                plugin_id: plugin_id.to_string(),
                reason: format!("Invalid plugin info JSON: {}", e),
            }
        })
    }
    
    /// Create plugin factory instance
    fn create_plugin_factory(&self, library: &Library, plugin_id: &str) -> PluginResult<Box<dyn PluginNodeFactory>> {
        // Get the required C ABI functions
        let create_factory: Symbol<CreatePluginFactoryFn> = unsafe {
            library.get(b"create_plugin_factory").map_err(|e| {
                PluginError::LoadError {
                    plugin_id: plugin_id.to_string(),
                    reason: format!("Missing create_plugin_factory function: {}", e),
                }
            })?
        };
        
        let get_info: Symbol<GetPluginInfoFn> = unsafe {
            library.get(b"get_plugin_info").map_err(|e| {
                PluginError::LoadError {
                    plugin_id: plugin_id.to_string(),
                    reason: format!("Missing get_plugin_info function: {}", e),
                }
            })?
        };
        
        let get_version: Symbol<GetApiVersionFn> = unsafe {
            library.get(b"get_api_version").map_err(|e| {
                PluginError::LoadError {
                    plugin_id: plugin_id.to_string(),
                    reason: format!("Missing get_api_version function: {}", e),
                }
            })?
        };
        
        // Create factory using C ABI bridge
        let factory = unsafe {
            crate::plugin::c_abi::create_factory_from_c_symbols(
                *create_factory,
                *get_info,
                *get_version,
            )?
        };
        
        Ok(factory)
    }
}

/// Plugin loader builder for configuration
pub struct PluginLoaderBuilder {
    host_version: String,
    plugin_directories: Vec<PathBuf>,
}

impl PluginLoaderBuilder {
    pub fn new(host_version: String) -> Self {
        Self {
            host_version,
            plugin_directories: Vec::new(),
        }
    }
    
    pub fn add_directory<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.plugin_directories.push(path.as_ref().to_path_buf());
        self
    }
    
    pub fn build(self) -> PluginLoader {
        let mut loader = PluginLoader::new(self.host_version);
        for dir in self.plugin_directories {
            loader.add_plugin_directory(dir);
        }
        loader
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_plugin_loader_creation() {
        let loader = PluginLoader::new("1.0.0".to_string());
        assert!(!loader.plugin_directories.is_empty());
    }
    
    #[test]
    fn test_plugin_loader_builder() {
        let temp_dir = tempdir().unwrap();
        let loader = PluginLoaderBuilder::new("1.0.0".to_string())
            .add_directory(temp_dir.path())
            .build();
        
        assert!(loader.plugin_directories.contains(&temp_dir.path().to_path_buf()));
    }
    
    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let loader = PluginLoader::new("1.0.0".to_string());
        
        let manifests = loader.scan_directory(temp_dir.path()).unwrap();
        assert!(manifests.is_empty());
    }
}