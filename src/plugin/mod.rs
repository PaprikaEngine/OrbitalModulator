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

//! OrbitalModulator Plugin System
//! 
//! This module provides a comprehensive plugin system that allows third-party developers
//! to create custom nodes for OrbitalModulator. The system supports:
//! 
//! - **Dynamic Loading**: Load plugins at runtime from shared libraries
//! - **Type Safety**: Full Rust type safety with version compatibility checks
//! - **Audio Processing**: Real-time audio processing with ProcessContext
//! - **Parameter Management**: Professional CV modulation and parameter control
//! - **UI Integration**: Tauri-compatible UI components for plugin controls
//! - **Security**: Plugin sandboxing and verification system
//! - **Performance**: Zero-allocation audio paths and resource monitoring

pub mod api;
pub mod c_abi;
pub mod loader;
pub mod manager;
pub mod manifest;
pub mod sandbox;
pub mod sdk;

pub use api::*;
pub use c_abi::*;
pub use loader::*;
pub use manager::*;
pub use manifest::*;
pub use sandbox::*;
pub use sdk::*;

/// Convenient prelude for plugin development
pub mod prelude {
    pub use crate::plugin::api::*;
    pub use crate::plugin::sdk::*;
    pub use crate::plugin::{PluginError, PluginResult, PluginConfig, PluginStats};
    pub use crate::processing::{AudioNode, ProcessContext, ProcessingError};
    pub use crate::parameters::{Parameterizable, ParameterDescriptor, ParameterError};
    pub use std::collections::HashMap;
}

use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Plugin system error types
#[derive(Debug, Clone)]
pub enum PluginError {
    /// Plugin not found
    NotFound { plugin_id: String },
    /// Plugin loading failed
    LoadError { plugin_id: String, reason: String },
    /// Plugin version incompatible
    VersionMismatch { plugin_id: String, required: String, found: String },
    /// Plugin validation failed
    ValidationError { plugin_id: String, reason: String },
    /// Plugin execution error
    RuntimeError { plugin_id: String, error: String },
    /// Plugin security violation
    SecurityViolation { plugin_id: String, violation: String },
    /// Plugin resource limit exceeded
    ResourceLimit { plugin_id: String, resource: String, limit: String },
    /// Plugin dependency missing
    DependencyMissing { plugin_id: String, dependency: String },
    /// Plugin license incompatible
    LicenseIncompatible { plugin_id: String, license: String },
    /// Internal plugin system error
    Internal { message: String },
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::NotFound { plugin_id } => {
                write!(f, "Plugin not found: {}", plugin_id)
            }
            PluginError::LoadError { plugin_id, reason } => {
                write!(f, "Failed to load plugin {}: {}", plugin_id, reason)
            }
            PluginError::VersionMismatch { plugin_id, required, found } => {
                write!(f, "Plugin {} version mismatch: required {}, found {}", plugin_id, required, found)
            }
            PluginError::ValidationError { plugin_id, reason } => {
                write!(f, "Plugin {} validation failed: {}", plugin_id, reason)
            }
            PluginError::RuntimeError { plugin_id, error } => {
                write!(f, "Plugin {} runtime error: {}", plugin_id, error)
            }
            PluginError::SecurityViolation { plugin_id, violation } => {
                write!(f, "Plugin {} security violation: {}", plugin_id, violation)
            }
            PluginError::ResourceLimit { plugin_id, resource, limit } => {
                write!(f, "Plugin {} exceeded {} limit: {}", plugin_id, resource, limit)
            }
            PluginError::DependencyMissing { plugin_id, dependency } => {
                write!(f, "Plugin {} missing dependency: {}", plugin_id, dependency)
            }
            PluginError::LicenseIncompatible { plugin_id, license } => {
                write!(f, "Plugin {} has incompatible license: {}", plugin_id, license)
            }
            PluginError::Internal { message } => {
                write!(f, "Internal plugin error: {}", message)
            }
        }
    }
}

impl std::error::Error for PluginError {}

/// Plugin system result type
pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin runtime statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStats {
    pub cpu_usage: f32,           // CPU usage percentage
    pub memory_usage: usize,      // Memory usage in bytes
    pub audio_dropouts: u64,      // Number of audio dropouts
    pub processing_time: f64,     // Average processing time in microseconds
    pub calls_per_second: f64,    // Processing calls per second
    pub last_error: Option<String>, // Last error message
}

impl Default for PluginStats {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0,
            audio_dropouts: 0,
            processing_time: 0.0,
            calls_per_second: 0.0,
            last_error: None,
        }
    }
}

/// Plugin configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub max_cpu_usage: f32,       // Maximum CPU usage (0.0-1.0)
    pub max_memory_usage: usize,  // Maximum memory usage in bytes
    pub enable_sandbox: bool,     // Enable sandboxing
    pub allow_network: bool,      // Allow network access
    pub allow_file_system: bool,  // Allow file system access
    pub timeout_ms: u64,          // Processing timeout in milliseconds
    pub auto_disable_on_error: bool, // Auto-disable on repeated errors
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            max_cpu_usage: 0.1,      // 10% CPU max
            max_memory_usage: 64 * 1024 * 1024, // 64MB max
            enable_sandbox: true,
            allow_network: false,
            allow_file_system: false,
            timeout_ms: 10,          // 10ms timeout
            auto_disable_on_error: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_error_display() {
        let error = PluginError::NotFound {
            plugin_id: "test_plugin".to_string(),
        };
        assert_eq!(format!("{}", error), "Plugin not found: test_plugin");
    }

    #[test]
    fn test_plugin_stats_default() {
        let stats = PluginStats::default();
        assert_eq!(stats.cpu_usage, 0.0);
        assert_eq!(stats.memory_usage, 0);
        assert_eq!(stats.audio_dropouts, 0);
    }

    #[test]
    fn test_plugin_config_default() {
        let config = PluginConfig::default();
        assert_eq!(config.max_cpu_usage, 0.1);
        assert_eq!(config.max_memory_usage, 64 * 1024 * 1024);
        assert!(config.enable_sandbox);
        assert!(!config.allow_network);
    }
}