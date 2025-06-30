/*
 * Example Plugin: Simple Oscillator
 * 
 * This demonstrates how to create a basic oscillator plugin for OrbitalModulator.
 * This file would typically be compiled as a separate dynamic library (.so/.dll/.dylib).
 */

use std::collections::HashMap;
use orbital_modulator::plugin::prelude::*;

// Create a simple triangle oscillator using the SDK macro
create_oscillator_plugin!(SimpleTriangleOscillator, |phase: f32| {
    // Triangle wave: linear ramp from -1 to 1 and back
    if phase < 0.5 {
        4.0 * phase - 1.0
    } else {
        3.0 - 4.0 * phase
    }
});

// Plugin factory implementation
pub struct SimpleOscillatorFactory {
    metadata: PluginMetadata,
}

impl SimpleOscillatorFactory {
    pub fn new() -> Self {
        let metadata = PluginMetadata {
            id: "simple_oscillator".to_string(),
            name: "Simple Oscillator".to_string(),
            version: "1.0.0".to_string(),
            description: "A simple triangle wave oscillator example plugin".to_string(),
            author: "OrbitalModulator Example".to_string(),
            website: Some("https://github.com/orbital-modulator".to_string()),
            category: PluginCategory::Generator,
            license: PluginLicense::MIT,
            api_version: PLUGIN_API_VERSION,
            node_types: vec!["simple_triangle_osc".to_string()],
            dependencies: vec![],
            tags: vec!["oscillator".to_string(), "generator".to_string(), "triangle".to_string()],
            min_orbital_version: "1.0.0".to_string(),
        };

        Self { metadata }
    }
}

impl PluginNodeFactory for SimpleOscillatorFactory {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn create_node(&self, node_type: &str, name: String, sample_rate: f32) -> PluginResult<Box<dyn AudioNode>> {
        match node_type {
            "simple_triangle_osc" => {
                let node = SimpleTriangleOscillator::new(name, sample_rate);
                Ok(Box::new(node))
            }
            _ => Err(PluginError::NotFound {
                plugin_id: format!("Unknown node type: {}", node_type),
            }),
        }
    }

    fn supported_node_types(&self) -> Vec<String> {
        vec!["simple_triangle_osc".to_string()]
    }

    fn validate_compatibility(&self, host_version: &str) -> PluginResult<()> {
        // Simple version check - in practice you'd use a proper semver library
        if host_version >= "1.0.0" {
            Ok(())
        } else {
            Err(PluginError::VersionMismatch {
                plugin_id: self.metadata.id.clone(),
                required: "1.0.0".to_string(),
                found: host_version.to_string(),
            })
        }
    }

    fn initialize(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        println!("Simple Oscillator Plugin initialized");
        Ok(())
    }

    fn cleanup(&mut self) -> PluginResult<()> {
        println!("Simple Oscillator Plugin cleaned up");
        Ok(())
    }

    fn get_stats(&self) -> PluginStats {
        PluginStats::default()
    }

    fn configure(&mut self, _config: &PluginConfig) -> PluginResult<()> {
        Ok(())
    }
}

// This macro generates the required C-compatible entry points
// In a real plugin, this would be in a separate library crate
plugin_main!(SimpleOscillatorFactory);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_oscillator_creation() {
        let mut osc = SimpleTriangleOscillator::new("test".to_string(), 44100.0);
        
        assert_eq!(osc.get_frequency(), 440.0);
        assert_eq!(osc.plugin_node_type(), "SimpleTriangleOscillator");
        
        // Test frequency setting
        osc.set_frequency(880.0);
        assert_eq!(osc.get_frequency(), 880.0);
    }

    #[test]
    fn test_plugin_factory() {
        let factory = SimpleOscillatorFactory::new();
        
        assert_eq!(factory.metadata().id, "simple_oscillator");
        assert_eq!(factory.metadata().name, "Simple Oscillator");
        assert_eq!(factory.supported_node_types(), vec!["simple_triangle_osc"]);
        
        // Test node creation
        let result = factory.create_node("simple_triangle_osc", "test_osc".to_string(), 44100.0);
        assert!(result.is_ok());
        
        // Test invalid node type
        let result = factory.create_node("invalid_type", "test".to_string(), 44100.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_version_compatibility() {
        let factory = SimpleOscillatorFactory::new();
        
        assert!(factory.validate_compatibility("1.0.0").is_ok());
        assert!(factory.validate_compatibility("1.1.0").is_ok());
        assert!(factory.validate_compatibility("2.0.0").is_ok());
        assert!(factory.validate_compatibility("0.9.0").is_err());
    }
}