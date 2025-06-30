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

//! Plugin SDK - Software Development Kit for creating OrbitalModulator plugins
//! 
//! This module provides convenience traits, macros, and utilities to make
//! plugin development easier and more productive.

use std::collections::HashMap;
use uuid::Uuid;

pub use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
pub use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor, ParameterError};
pub use crate::graph::PortType;
pub use crate::plugin::{
    PluginError, PluginResult, PluginConfig, PluginStats,
    api::{PluginNodeFactory, PluginMetadata, PluginCategory, PluginLicense},
    manifest::{PluginManifest, ManifestBuilder},
};

/// Convenience re-exports for plugin developers
pub mod prelude {
    pub use super::*;
    pub use crate::plugin_main;
    pub use crate::define_parameters;
}

/// Base trait for plugin nodes with common functionality
pub trait PluginNode: AudioNode + Parameterizable + Send + Sync {
    /// Get the plugin node's unique type identifier
    fn plugin_node_type(&self) -> &'static str;
    
    /// Get plugin-specific metadata
    fn plugin_metadata(&self) -> HashMap<String, String> {
        HashMap::new()
    }
    
    /// Called when the plugin is being initialized
    fn on_initialize(&mut self, sample_rate: f32) -> PluginResult<()> {
        Ok(())
    }
    
    /// Called when the plugin is being cleaned up
    fn on_cleanup(&mut self) -> PluginResult<()> {
        Ok(())
    }
    
    /// Called when parameters are updated from the UI
    fn on_parameter_changed(&mut self, _name: &str, _value: f32) -> PluginResult<()> {
        Ok(())
    }
}

/// Trait for creating custom oscillator plugins
pub trait OscillatorPlugin: PluginNode {
    /// Generate a single sample at the given frequency and phase
    fn generate_sample(&mut self, frequency: f32, phase: f32) -> f32;
    
    /// Get the oscillator's current frequency
    fn get_frequency(&self) -> f32;
    
    /// Set the oscillator's frequency
    fn set_frequency(&mut self, frequency: f32);
}

/// Trait for creating custom filter plugins
pub trait FilterPlugin: PluginNode {
    /// Process a single audio sample through the filter
    fn process_sample(&mut self, input: f32) -> f32;
    
    /// Set the filter's cutoff frequency
    fn set_cutoff(&mut self, cutoff: f32);
    
    /// Set the filter's resonance
    fn set_resonance(&mut self, resonance: f32);
    
    /// Reset the filter's internal state
    fn reset_filter(&mut self);
}

/// Trait for creating custom effect plugins
pub trait EffectPlugin: PluginNode {
    /// Process an audio buffer
    fn process_buffer(&mut self, input: &[f32], output: &mut [f32]);
    
    /// Get the effect's wet/dry mix
    fn get_mix(&self) -> f32;
    
    /// Set the effect's wet/dry mix (0.0 = dry, 1.0 = wet)
    fn set_mix(&mut self, mix: f32);
}

/// Base oscillator node implementation
pub struct BaseOscillator {
    node_info: NodeInfo,
    frequency: f32,
    phase: f32,
    sample_rate: f32,
    frequency_param: ModulatableParameter,
}

impl BaseOscillator {
    pub fn new(name: String, sample_rate: f32, node_type: &str) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: node_type.to_string(),
            category: NodeCategory::Generator,
            
            input_ports: vec![
                PortInfo::new("frequency_cv", PortType::CV),
            ],
            
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono),
            ],
            
            description: "Base oscillator plugin node".to_string(),
            latency_samples: 0,
            supports_bypass: true,
        };
        
        let frequency_param = ModulatableParameter::new(
            BasicParameter::new("frequency", 20.0, 20000.0, 440.0),
            1.0
        );
        
        Self {
            node_info,
            frequency: 440.0,
            phase: 0.0,
            sample_rate,
            frequency_param,
        }
    }
    
    /// Update the oscillator's phase for the next sample
    pub fn update_phase(&mut self) {
        let phase_increment = self.frequency / self.sample_rate;
        self.phase += phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
    }
}

impl AudioNode for BaseOscillator {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            return Ok(());
        }
        
        // Get CV input for frequency modulation
        let frequency_cv = ctx.inputs.get_cv_value("frequency_cv");
        let effective_frequency = self.frequency_param.modulate(self.frequency, frequency_cv);
        
        // Get output buffer
        let output = ctx.outputs.get_audio_mut("audio_out")
            .ok_or_else(|| ProcessingError::OutputBufferError {
                port_name: "audio_out".to_string()
            })?;
        
        // Generate samples (override in derived implementations)
        for sample in output.iter_mut() {
            self.update_phase();
            *sample = (self.phase * 2.0 * std::f32::consts::PI).sin();
        }
        
        Ok(())
    }
    
    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }
    
    fn reset(&mut self) {
        self.phase = 0.0;
    }
}

impl Parameterizable for BaseOscillator {
    fn get_parameter(&self, name: &str) -> Result<f32, ParameterError> {
        match name {
            "frequency" => Ok(self.frequency),
            "active" => Ok(1.0),
            _ => Err(ParameterError::NotFound { name: name.to_string() }),
        }
    }
    
    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), ParameterError> {
        match name {
            "frequency" => {
                if value >= 20.0 && value <= 20000.0 {
                    self.frequency = value;
                    Ok(())
                } else {
                    Err(ParameterError::OutOfRange { 
                        name: name.to_string(), 
                        value, 
                        min: 20.0, 
                        max: 20000.0 
                    })
                }
            }
            "active" => Ok(()), // Handle active state
            _ => Err(ParameterError::NotFound { name: name.to_string() }),
        }
    }
    
    fn get_parameter_descriptors(&self) -> Vec<Box<dyn ParameterDescriptor>> {
        vec![
            Box::new(BasicParameter::new("frequency", 20.0, 20000.0, 440.0).with_unit("Hz")),
            Box::new(BasicParameter::new("active", 0.0, 1.0, 1.0)),
        ]
    }
    
    fn get_all_parameters(&self) -> HashMap<String, f32> {
        let mut params = HashMap::new();
        params.insert("frequency".to_string(), self.frequency);
        params.insert("active".to_string(), 1.0);
        params
    }
}

/// Helper macro for creating simple oscillator plugins
#[macro_export]
macro_rules! create_oscillator_plugin {
    ($name:ident, $generate_fn:expr) => {
        pub struct $name {
            base: BaseOscillator,
        }
        
        impl $name {
            pub fn new(name: String, sample_rate: f32) -> Self {
                Self {
                    base: BaseOscillator::new(name, sample_rate, stringify!($name)),
                }
            }
        }
        
        impl AudioNode for $name {
            fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
                if !self.is_active() {
                    return Ok(());
                }
                
                let frequency_cv = ctx.inputs.get_cv_value("frequency_cv");
                let effective_frequency = self.base.frequency_param.modulate(self.base.frequency, frequency_cv);
                
                let output = ctx.outputs.get_audio_mut("audio_out")
                    .ok_or_else(|| ProcessingError::OutputBufferError {
                        port_name: "audio_out".to_string()
                    })?;
                
                for sample in output.iter_mut() {
                    self.base.update_phase();
                    *sample = $generate_fn(self.base.phase);
                }
                
                Ok(())
            }
            
            fn node_info(&self) -> &NodeInfo {
                self.base.node_info()
            }
            
            fn reset(&mut self) {
                self.base.reset();
            }
        }
        
        impl Parameterizable for $name {
            fn get_parameter(&self, name: &str) -> Result<f32, ParameterError> {
                self.base.get_parameter(name)
            }
            
            fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), ParameterError> {
                self.base.set_parameter(name, value)
            }
            
            fn get_parameter_descriptors(&self) -> Vec<Box<dyn ParameterDescriptor>> {
                self.base.get_parameter_descriptors()
            }
            
            fn get_all_parameters(&self) -> HashMap<String, f32> {
                self.base.get_all_parameters()
            }
        }
        
        impl PluginNode for $name {
            fn plugin_node_type(&self) -> &'static str {
                stringify!($name)
            }
            
            fn on_initialize(&mut self, sample_rate: f32) -> PluginResult<()> {
                self.base.sample_rate = sample_rate;
                Ok(())
            }
        }
        
        impl OscillatorPlugin for $name {
            fn generate_sample(&mut self, frequency: f32, phase: f32) -> f32 {
                $generate_fn(phase)
            }
            
            fn get_frequency(&self) -> f32 {
                self.base.frequency
            }
            
            fn set_frequency(&mut self, frequency: f32) {
                self.base.frequency = frequency;
            }
        }
    };
}

/// Factory implementation helper
pub struct SimplePluginFactory {
    metadata: PluginMetadata,
    stats: PluginStats,
}

impl SimplePluginFactory {
    pub fn new(metadata: PluginMetadata) -> Self {
        Self {
            metadata,
            stats: PluginStats::default(),
        }
    }
}

impl PluginNodeFactory for SimplePluginFactory {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn create_node(&self, node_type: &str, name: String, sample_rate: f32) -> PluginResult<Box<dyn AudioNode>> {
        Err(PluginError::NotFound {
            plugin_id: format!("Unknown node type: {}", node_type),
        })
    }
    
    fn supported_node_types(&self) -> Vec<String> {
        self.metadata.node_types.clone()
    }
    
    fn validate_compatibility(&self, host_version: &str) -> PluginResult<()> {
        // Simple version check
        if host_version >= &self.metadata.min_orbital_version {
            Ok(())
        } else {
            Err(PluginError::VersionMismatch {
                plugin_id: self.metadata.id.clone(),
                required: self.metadata.min_orbital_version.clone(),
                found: host_version.to_string(),
            })
        }
    }
    
    fn initialize(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        Ok(())
    }
    
    fn cleanup(&mut self) -> PluginResult<()> {
        Ok(())
    }
    
    fn get_stats(&self) -> PluginStats {
        self.stats.clone()
    }
    
    fn configure(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Example oscillator using the macro
    create_oscillator_plugin!(TestSawOscillator, |phase: f32| {
        // Sawtooth wave: -1 to 1 linear ramp
        2.0 * phase - 1.0
    });

    #[test]
    fn test_oscillator_macro() {
        let mut osc = TestSawOscillator::new("test".to_string(), 44100.0);
        
        assert_eq!(osc.get_frequency(), 440.0);
        
        osc.set_frequency(880.0);
        assert_eq!(osc.get_frequency(), 880.0);
        
        assert_eq!(osc.plugin_node_type(), "TestSawOscillator");
    }
    
    #[test]
    fn test_base_oscillator() {
        let mut osc = BaseOscillator::new("test".to_string(), 44100.0, "test_osc");
        
        assert_eq!(osc.get_parameter("frequency").unwrap(), 440.0);
        assert!(osc.set_parameter("frequency", 880.0).is_ok());
        assert_eq!(osc.get_parameter("frequency").unwrap(), 880.0);
        
        // Test out of range
        assert!(osc.set_parameter("frequency", 50000.0).is_err());
    }
    
    #[test]
    fn test_simple_plugin_factory() {
        let metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            website: None,
            category: PluginCategory::Generator,
            license: PluginLicense::MIT,
            api_version: crate::plugin::PLUGIN_API_VERSION,
            node_types: vec!["test_node".to_string()],
            dependencies: vec![],
            tags: vec![],
            min_orbital_version: "1.0.0".to_string(),
        };
        
        let factory = SimplePluginFactory::new(metadata);
        assert_eq!(factory.metadata().id, "test_plugin");
        assert!(factory.validate_compatibility("1.0.0").is_ok());
        assert!(factory.validate_compatibility("0.9.0").is_err());
    }
}