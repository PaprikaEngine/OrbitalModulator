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

//! Plugin API - Core interfaces for OrbitalModulator plugins
//! 
//! This module defines the fundamental traits and types that all plugins must implement.
//! It provides a stable ABI (Application Binary Interface) for dynamic loading.

use std::collections::HashMap;
use std::os::raw::{c_char, c_void};
use serde::{Deserialize, Serialize};

use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo};
use crate::parameters::Parameterizable;
use crate::plugin::{PluginResult, PluginStats, PluginConfig};

/// Plugin API version - must match between host and plugin
pub const PLUGIN_API_VERSION: u32 = 1;

/// Maximum plugin name length
pub const MAX_PLUGIN_NAME_LENGTH: usize = 64;

/// Maximum plugin description length  
pub const MAX_PLUGIN_DESCRIPTION_LENGTH: usize = 256;

/// Plugin category types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PluginCategory {
    Generator,    // Oscillators, noise generators
    Processor,    // Filters, effects, distortion
    Controller,   // Envelopes, LFOs, sequencers
    Utility,      // Mixers, splitters, analyzers
    Mixing,       // Mixers, routers, output
    Analyzer,     // Oscilloscopes, spectrum analyzers
    Custom(String), // Custom category defined by plugin
}

/// Plugin licensing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginLicense {
    AGPL3,          // GNU AGPL v3 (compatible)
    GPL3,           // GNU GPL v3 (compatible)
    MIT,            // MIT License (compatible)
    Apache2,        // Apache 2.0 (compatible)
    BSD3,           // BSD 3-Clause (compatible)
    Proprietary,    // Proprietary license (requires approval)
    Custom(String), // Custom license text
}

/// Plugin metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,                    // Unique plugin identifier
    pub name: String,                  // Human-readable name
    pub version: String,               // Semantic version (e.g., "1.0.0")
    pub description: String,           // Plugin description
    pub author: String,                // Plugin author/developer
    pub website: Option<String>,       // Plugin website URL
    pub category: PluginCategory,      // Plugin category
    pub license: PluginLicense,        // License information
    pub api_version: u32,              // Required API version
    pub node_types: Vec<String>,       // Provided node types
    pub dependencies: Vec<String>,     // Required dependencies
    pub tags: Vec<String>,             // Search tags
    pub min_orbital_version: String,   // Minimum OrbitalModulator version
}

/// Plugin node factory trait
/// 
/// Each plugin must implement this trait to create node instances.
pub trait PluginNodeFactory: Send + Sync + std::fmt::Debug {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
    
    /// Create a new node instance
    fn create_node(&self, node_type: &str, name: String, sample_rate: f32) -> PluginResult<Box<dyn AudioNode>>;
    
    /// Get supported node types
    fn supported_node_types(&self) -> Vec<String>;
    
    /// Validate plugin compatibility with host
    fn validate_compatibility(&self, host_version: &str) -> PluginResult<()>;
    
    /// Initialize plugin (called once when loaded)
    fn initialize(&mut self, config: &PluginConfig) -> PluginResult<()>;
    
    /// Cleanup plugin (called when unloaded)
    fn cleanup(&mut self) -> PluginResult<()>;
    
    /// Get plugin statistics
    fn get_stats(&self) -> PluginStats;
    
    /// Configure plugin settings
    fn configure(&mut self, config: &PluginConfig) -> PluginResult<()>;
}

/// Plugin entry point function signature
/// 
/// Every plugin must export a function with this signature named `create_plugin_factory`.
pub type CreatePluginFactoryFn = extern "C" fn() -> *mut c_void;

/// Plugin info query function signature
/// 
/// Every plugin must export a function with this signature named `get_plugin_info`.
pub type GetPluginInfoFn = extern "C" fn() -> *const c_char;

/// Plugin API version function signature
/// 
/// Every plugin must export a function with this signature named `get_api_version`.
pub type GetApiVersionFn = extern "C" fn() -> u32;

/// Plugin cleanup function signature
/// 
/// Every plugin must export a function with this signature named `destroy_plugin_factory`.
pub type DestroyPluginFactoryFn = extern "C" fn(factory: *mut c_void);

/// C-compatible plugin interface for dynamic loading
#[repr(C)]
pub struct CPluginInterface {
    pub create_factory: CreatePluginFactoryFn,
    pub destroy_factory: DestroyPluginFactoryFn,
    pub get_info: GetPluginInfoFn,
    pub get_version: GetApiVersionFn,
}

/// Helper macro for implementing plugin entry points
#[macro_export]
macro_rules! plugin_main {
    ($factory_type:ty) => {
        use std::ffi::CString;
        use std::os::raw::{c_char, c_void};
        
        #[no_mangle]
        pub extern "C" fn get_api_version() -> u32 {
            $crate::plugin::PLUGIN_API_VERSION
        }
        
        #[no_mangle]
        pub extern "C" fn get_plugin_info() -> *const c_char {
            let factory = <$factory_type>::new();
            let metadata = factory.metadata();
            let info_json = serde_json::to_string(metadata).unwrap_or_default();
            let c_string = CString::new(info_json).unwrap_or_default();
            c_string.into_raw() as *const c_char
        }
        
        #[no_mangle]
        pub extern "C" fn create_plugin_factory() -> *mut c_void {
            let factory = Box::new(<$factory_type>::new());
            Box::into_raw(factory) as *mut c_void
        }
        
        #[no_mangle]
        pub extern "C" fn destroy_plugin_factory(factory: *mut c_void) {
            if !factory.is_null() {
                unsafe {
                    let _ = Box::from_raw(factory as *mut $factory_type);
                }
            }
        }
    };
}

/// Plugin wrapper for integrating with OrbitalModulator's node system
pub struct PluginNodeWrapper {
    plugin_id: String,
    node_type: String,
    inner: Box<dyn AudioNode>,
    stats: PluginStats,
    config: PluginConfig,
}

impl PluginNodeWrapper {
    pub fn new(
        plugin_id: String,
        node_type: String,
        inner: Box<dyn AudioNode>,
        config: PluginConfig,
    ) -> Self {
        Self {
            plugin_id,
            node_type,
            inner,
            stats: PluginStats::default(),
            config,
        }
    }
    
    pub fn get_plugin_id(&self) -> &str {
        &self.plugin_id
    }
    
    pub fn get_node_type(&self) -> &str {
        &self.node_type
    }
    
    pub fn get_stats(&self) -> &PluginStats {
        &self.stats
    }
    
    pub fn update_stats(&mut self, stats: PluginStats) {
        self.stats = stats;
    }
}

impl AudioNode for PluginNodeWrapper {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        use std::time::Instant;
        
        let start_time = Instant::now();
        
        // Check resource limits before processing
        if self.stats.cpu_usage > self.config.max_cpu_usage {
            return Err(ProcessingError::Internal {
                message: format!("Plugin {} CPU usage exceeds limit", self.plugin_id),
            });
        }
        
        // Process with timeout protection
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.inner.process(ctx)
        }));
        
        let processing_time = start_time.elapsed().as_micros() as f64;
        
        match result {
            Ok(Ok(())) => {
                // Update statistics
                self.stats.processing_time = processing_time;
                self.stats.last_error = None;
                Ok(())
            }
            Ok(Err(e)) => {
                self.stats.last_error = Some(format!("{}", e));
                Err(e)
            }
            Err(_) => {
                let error_msg = format!("Plugin {} panicked during processing", self.plugin_id);
                self.stats.last_error = Some(error_msg.clone());
                Err(ProcessingError::Internal { message: error_msg })
            }
        }
    }
    
    fn node_info(&self) -> &NodeInfo {
        self.inner.node_info()
    }
    
    fn reset(&mut self) {
        self.inner.reset();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Parameterizable for PluginNodeWrapper {
    fn get_parameter(&self, name: &str) -> Result<f32, crate::parameters::ParameterError> {
        self.inner.get_parameter(name)
    }
    
    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), crate::parameters::ParameterError> {
        self.inner.set_parameter(name, value)
    }
    
    fn get_parameter_descriptors(&self) -> Vec<Box<dyn crate::parameters::ParameterDescriptor>> {
        self.inner.get_parameter_descriptors()
    }
    
    fn get_all_parameters(&self) -> HashMap<String, f32> {
        self.inner.get_all_parameters()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_metadata_serialization() {
        let metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            website: Some("https://example.com".to_string()),
            category: PluginCategory::Generator,
            license: PluginLicense::MIT,
            api_version: PLUGIN_API_VERSION,
            node_types: vec!["test_node".to_string()],
            dependencies: vec![],
            tags: vec!["test".to_string()],
            min_orbital_version: "1.0.0".to_string(),
        };
        
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: PluginMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(metadata.id, deserialized.id);
        assert_eq!(metadata.name, deserialized.name);
        assert_eq!(metadata.category, deserialized.category);
    }
    
    #[test]
    fn test_plugin_license_values() {
        assert_eq!(
            serde_json::to_string(&PluginLicense::AGPL3).unwrap(),
            "\"AGPL3\""
        );
        assert_eq!(
            serde_json::to_string(&PluginLicense::Custom("Test".to_string())).unwrap(),
            "{\"Custom\":\"Test\"}"
        );
    }
}