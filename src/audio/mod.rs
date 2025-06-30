/*
 * OrbitalModulator - Professional Modular Synthesizer
 * Copyright (c) 2025 MACHIKO LAB
 *
 * Modern Audio Engine - Integrated with new ProcessContext architecture
 */

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cpal::{Device, Stream, StreamConfig, StreamError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::graph::{AudioGraph, Node, PortType, ProcessingGraph};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, InputPorts, OutputPorts};
use crate::parameters::{Parameterizable, ParameterError};
use crate::plugin::{PluginManager, PluginError};

/// Modern Audio Engine with plugin support
pub struct AudioEngine {
    graph: Arc<Mutex<ProcessingGraph>>,
    plugin_manager: Arc<Mutex<PluginManager>>,
    sample_rate: f32,
    buffer_size: usize,
    device: Device,
    stream: Option<Stream>,
    is_playing: bool,
}

impl AudioEngine {
    /// Create a new audio engine
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or("No output device available")?;

        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate().0 as f32;
        let buffer_size = 512; // Default buffer size

        println!("Audio Engine initialized:");
        println!("  Sample Rate: {} Hz", sample_rate);
        println!("  Buffer Size: {} samples", buffer_size);
        println!("  Device: {}", device.name().unwrap_or("Unknown".to_string()));

        let plugin_manager = PluginManager::new("1.0.0".to_string());

        Ok(Self {
            graph: Arc::new(Mutex::new(ProcessingGraph::new())),
            plugin_manager: Arc::new(Mutex::new(plugin_manager)),
            sample_rate,
            buffer_size,
            device,
            stream: None,
            is_playing: false,
        })
    }

    /// Add a plugin directory
    pub fn add_plugin_directory<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<(), String> {
        let mut manager = self.plugin_manager.lock()
            .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
        
        manager.add_plugin_directory(path);
        Ok(())
    }

    /// Load a plugin
    pub fn load_plugin(&self, plugin_id: &str) -> Result<(), String> {
        let manager = self.plugin_manager.lock()
            .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
        
        manager.load_plugin(plugin_id)
            .map_err(|e| format!("Failed to load plugin: {}", e))
    }

    /// Create a node (supporting both built-in and plugin nodes)
    pub fn create_node(&self, node_type: &str, name: String) -> Result<String, String> {
        // First try built-in nodes
        if let Ok(node_id) = self.create_builtin_node(node_type, name.clone()) {
            return Ok(node_id);
        }

        // Then try plugin nodes
        self.create_plugin_node(node_type, name)
    }

    /// Create a built-in node
    fn create_builtin_node(&self, node_type: &str, name: String) -> Result<String, String> {
        let mut graph = self.graph.lock()
            .map_err(|e| format!("Failed to lock graph: {}", e))?;

        let node: Box<dyn AudioNode> = match node_type {
            // Generator Nodes
            "sine_oscillator" => Box::new(crate::nodes::SineOscillatorNode::new(name.clone(), self.sample_rate)),
            "oscillator" => Box::new(crate::nodes::OscillatorNode::new(name.clone(), self.sample_rate)),
            "noise" => Box::new(crate::nodes::NoiseNode::new(name.clone(), self.sample_rate)),

            // Processor Nodes
            "vcf" => Box::new(crate::nodes::VCFNode::new(name.clone(), self.sample_rate)),
            "vca" => Box::new(crate::nodes::VCANode::new(name.clone(), self.sample_rate)),
            "delay" => Box::new(crate::nodes::DelayNode::new(name.clone(), self.sample_rate)),
            "compressor" => Box::new(crate::nodes::CompressorNode::new(name.clone(), self.sample_rate)),
            "waveshaper" => Box::new(crate::nodes::WaveshaperNode::new(name.clone(), self.sample_rate)),
            "ring_modulator" => Box::new(crate::nodes::RingModulatorNode::new(name.clone(), self.sample_rate)),

            // Controller Nodes
            "adsr" => Box::new(crate::nodes::ADSRNode::new(name.clone(), self.sample_rate)),
            "lfo" => Box::new(crate::nodes::LFONode::new(name.clone(), self.sample_rate)),
            "sequencer" => Box::new(crate::nodes::SequencerNode::new(name.clone(), self.sample_rate)),

            // Utility Nodes
            "sample_hold" => Box::new(crate::nodes::SampleHoldNode::new(name.clone(), self.sample_rate)),
            "quantizer" => Box::new(crate::nodes::QuantizerNode::new(name.clone(), self.sample_rate)),
            "attenuverter" => Box::new(crate::nodes::AttenuverterNode::new(name.clone(), self.sample_rate)),
            "multiple" => Box::new(crate::nodes::MultipleNode::new(name.clone(), self.sample_rate)),
            "clock_divider" => Box::new(crate::nodes::ClockDividerNode::new(name.clone(), self.sample_rate)),

            // Mixing/Routing Nodes
            "mixer" => Box::new(crate::nodes::MixerNode::new(name.clone(), self.sample_rate)),
            "output" => Box::new(crate::nodes::OutputNode::new(name.clone(), self.sample_rate)),

            // Analysis Nodes
            "oscilloscope" => Box::new(crate::nodes::OscilloscopeNode::new(name.clone(), self.sample_rate)),
            "spectrum_analyzer" => Box::new(crate::nodes::SpectrumAnalyzerNode::new(name.clone(), self.sample_rate)),

            _ => return Err(format!("Unknown built-in node type: {}", node_type)),
        };

        let node_id = node.node_info().id.to_string();
        graph.add_node_instance(node)?;
        
        println!("Created built-in node: {} ({})", name, node_type);
        Ok(node_id)
    }

    /// Create a plugin node
    fn create_plugin_node(&self, node_type: &str, name: String) -> Result<String, String> {
        let manager = self.plugin_manager.lock()
            .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;

        // Find which plugin provides this node type
        let supported_types = manager.get_all_supported_node_types();
        let plugin_id = supported_types.iter()
            .find(|(_, types)| types.contains(&node_type.to_string()))
            .map(|(id, _)| id.clone())
            .ok_or_else(|| format!("No plugin found for node type: {}", node_type))?;

        // Create the plugin node
        let plugin_node = manager.create_node(&plugin_id, node_type, name.clone(), self.sample_rate)
            .map_err(|e| format!("Failed to create plugin node: {}", e))?;

        // Add to graph
        let mut graph = self.graph.lock()
            .map_err(|e| format!("Failed to lock graph: {}", e))?;

        let node_id = plugin_node.node_info().id.to_string();
        graph.add_node_instance(plugin_node)?;

        println!("Created plugin node: {} ({}) from plugin: {}", name, node_type, plugin_id);
        Ok(node_id)
    }

    /// Set node parameter using the modern parameter system
    pub fn set_node_parameter(&self, node_id: &str, param_name: &str, value: f32) -> Result<(), String> {
        let mut graph = self.graph.lock()
            .map_err(|e| format!("Failed to lock graph: {}", e))?;

        if let Some(node) = graph.get_node_mut(node_id) {
            node.set_parameter(param_name, value)
                .map_err(|e| format!("Failed to set parameter: {}", e))?;
            Ok(())
        } else {
            Err(format!("Node not found: {}", node_id))
        }
    }

    /// Get node parameter
    pub fn get_node_parameter(&self, node_id: &str, param_name: &str) -> Result<f32, String> {
        let graph = self.graph.lock()
            .map_err(|e| format!("Failed to lock graph: {}", e))?;

        if let Some(node) = graph.get_node(node_id) {
            node.get_parameter(param_name)
                .map_err(|e| format!("Failed to get parameter: {}", e))
        } else {
            Err(format!("Node not found: {}", node_id))
        }
    }

    /// Connect two nodes
    pub fn connect_nodes(&self, source_id: &str, source_port: &str, target_id: &str, target_port: &str) -> Result<(), String> {
        let mut graph = self.graph.lock()
            .map_err(|e| format!("Failed to lock graph: {}", e))?;

        graph.connect_by_id(source_id, source_port, target_id, target_port)
    }

    /// Disconnect two nodes
    pub fn disconnect_nodes(&self, source_id: &str, source_port: &str, target_id: &str, target_port: &str) -> Result<(), String> {
        let mut graph = self.graph.lock()
            .map_err(|e| format!("Failed to lock graph: {}", e))?;

        graph.disconnect_by_id(source_id, source_port, target_id, target_port)
    }

    /// Start audio processing
    pub fn start(&mut self) -> Result<(), String> {
        if self.is_playing {
            return Ok(());
        }

        let config = StreamConfig {
            channels: 2, // Stereo output
            sample_rate: cpal::SampleRate(self.sample_rate as u32),
            buffer_size: cpal::BufferSize::Fixed(self.buffer_size as u32),
        };

        let graph = Arc::clone(&self.graph);
        let sample_rate = self.sample_rate;

        let stream = self.device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                Self::audio_callback(data, &graph, sample_rate);
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        ).map_err(|e| format!("Failed to create audio stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to start audio stream: {}", e))?;

        self.stream = Some(stream);
        self.is_playing = true;

        println!("Audio engine started");
        Ok(())
    }

    /// Stop audio processing
    pub fn stop(&mut self) -> Result<(), String> {
        if !self.is_playing {
            return Ok(());
        }

        if let Some(stream) = self.stream.take() {
            stream.pause().map_err(|e| format!("Failed to stop audio stream: {}", e))?;
        }

        self.is_playing = false;
        println!("Audio engine stopped");
        Ok(())
    }

    /// Audio callback function
    fn audio_callback(output: &mut [f32], graph: &Arc<Mutex<ProcessingGraph>>, sample_rate: f32) {
        // Clear output buffer
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        let mut graph = match graph.lock() {
            Ok(g) => g,
            Err(_) => return, // Skip this buffer if we can't lock
        };

        let buffer_size = output.len() / 2; // Stereo output

        // Create process context
        let mut inputs = InputPorts::new();
        let mut outputs = OutputPorts::new();

        // Process the audio graph
        if let Err(e) = graph.process_audio(&mut inputs, &mut outputs, sample_rate, buffer_size) {
            eprintln!("Audio processing error: {}", e);
            return;
        }

        // Get final output from the graph
        if let Some(left_output) = outputs.get_audio("main_left") {
            if let Some(right_output) = outputs.get_audio("main_right") {
                // Interleave stereo output
                for (i, (left, right)) in left_output.iter().zip(right_output.iter()).enumerate() {
                    if i * 2 + 1 < output.len() {
                        output[i * 2] = *left;
                        output[i * 2 + 1] = *right;
                    }
                }
            }
        }
    }

    /// List all available node types (built-in + plugins)
    pub fn list_node_types(&self) -> Vec<String> {
        let mut types = vec![
            // Built-in types
            "sine_oscillator".to_string(),
            "oscillator".to_string(),
            "noise".to_string(),
            "vcf".to_string(),
            "vca".to_string(),
            "delay".to_string(),
            "compressor".to_string(),
            "waveshaper".to_string(),
            "ring_modulator".to_string(),
            "adsr".to_string(),
            "lfo".to_string(),
            "sequencer".to_string(),
            "sample_hold".to_string(),
            "quantizer".to_string(),
            "attenuverter".to_string(),
            "multiple".to_string(),
            "clock_divider".to_string(),
            "mixer".to_string(),
            "output".to_string(),
            "oscilloscope".to_string(),
            "spectrum_analyzer".to_string(),
        ];

        // Add plugin types
        if let Ok(manager) = self.plugin_manager.lock() {
            let plugin_types = manager.get_all_supported_node_types();
            for (_, node_types) in plugin_types {
                types.extend(node_types);
            }
        }

        types.sort();
        types.dedup();
        types
    }

    /// Get plugin manager statistics
    pub fn get_plugin_stats(&self) -> HashMap<String, crate::plugin::PluginStats> {
        let mut stats = HashMap::new();

        if let Ok(manager) = self.plugin_manager.lock() {
            for plugin_id in manager.list_loaded_plugins() {
                if let Some(plugin_stats) = manager.get_plugin_stats(&plugin_id) {
                    stats.insert(plugin_id, plugin_stats);
                }
            }
        }

        stats
    }

    /// Check for misbehaving plugins and auto-disable them
    pub fn auto_disable_check(&self) -> Vec<String> {
        if let Ok(manager) = self.plugin_manager.lock() {
            manager.auto_disable_check()
        } else {
            Vec::new()
        }
    }

    /// Get audio engine info
    pub fn get_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        
        info.insert("sample_rate".to_string(), self.sample_rate.to_string());
        info.insert("buffer_size".to_string(), self.buffer_size.to_string());
        info.insert("is_playing".to_string(), self.is_playing.to_string());
        info.insert("device_name".to_string(), 
                   self.device.name().unwrap_or("Unknown".to_string()));

        // Add plugin info
        if let Ok(manager) = self.plugin_manager.lock() {
            let loaded_plugins = manager.list_loaded_plugins();
            info.insert("loaded_plugins".to_string(), loaded_plugins.len().to_string());
            info.insert("plugin_list".to_string(), loaded_plugins.join(", "));
        }

        info
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}