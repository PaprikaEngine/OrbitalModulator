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

use uuid::Uuid;

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;

/// リファクタリング済みMultipleNode - プロ品質の信号分配器
pub struct MultipleNode {
    // Node identification
    node_info: NodeInfo,
    
    // Multiple parameters
    channel_count: f32,      // 2 ~ 8 (number of output channels)
    buffered: f32,           // 0.0 = passive, 1.0 = active/buffered
    invert_alternate: f32,   // 0.0 = normal, 1.0 = invert alternate outputs
    active: f32,
    
    // Per-channel parameters (stored as individual parameters)
    output_gains: Vec<f32>,  // Individual gain for each output (0.0 ~ 2.0)
    
    // CV Modulation parameters
    gain_params: Vec<ModulatableParameter>, // One per channel
    
    #[allow(dead_code)]
    sample_rate: f32,
}

impl MultipleNode {
    pub fn new(sample_rate: f32, name: String, channel_count: u8) -> Self {
        let channels = channel_count.clamp(2, 8) as usize;
        
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "multiple".to_string(),
            category: NodeCategory::Utility,
            description: format!("Professional {}-channel signal splitter with individual gain control", channels),
            input_ports: vec![
                PortInfo::new("signal_in", PortType::AudioMono)
                    .with_description("Input signal to be distributed"),
            ],
            output_ports: (1..=channels).map(|i| 
                PortInfo::new(&format!("out_{}", i), PortType::AudioMono)
                    .with_description(&format!("Output channel {}", i))
            ).collect(),
            latency_samples: 0,
            supports_bypass: true,
        };

        // Create modulation parameters for each channel
        let gain_params: Vec<ModulatableParameter> = (0..channels)
            .map(|_i| ModulatableParameter::new(
                BasicParameter::new("gain", 0.0, 2.0, 1.0),
                0.8  // 80% CV modulation range
            ))
            .collect();

        Self {
            node_info,
            channel_count: channels as f32,
            buffered: 0.0,           // Passive by default
            invert_alternate: 0.0,   // Normal by default
            active: 1.0,

            output_gains: vec![1.0; channels],
            gain_params,
            
            sample_rate,
        }
    }

    /// Get the current channel count
    pub fn get_channel_count(&self) -> usize {
        self.channel_count as usize
    }

    /// Process signal distribution with individual gains and options
    fn process_distribution(&self, input_sample: f32, channel: usize, effective_gain: f32) -> f32 {
        let mut output = input_sample;
        
        // Apply individual channel gain
        output *= effective_gain;
        
        // Apply alternate inversion if enabled
        if self.invert_alternate > 0.5 && (channel % 2 == 1) {
            output = -output;
        }
        
        // Apply buffering characteristics if enabled
        if self.buffered > 0.5 {
            // Active buffering - slight gain boost and impedance buffering
            output *= 1.01; // Tiny gain boost for active buffer characteristic
            
            // Simple high-frequency emphasis for buffer character
            // (In a real implementation, this would be more sophisticated)
            output = output * 0.99 + output.signum() * 0.01;
        }
        
        output
    }

    /// Soft clipping to prevent harsh distortion
    fn soft_clip(&self, input: f32) -> f32 {
        const CLIP_THRESHOLD: f32 = 8.0; // ±8V for headroom
        
        if input.abs() > CLIP_THRESHOLD {
            CLIP_THRESHOLD * (input / CLIP_THRESHOLD).tanh()
        } else {
            input
        }
    }
}

impl Parameterizable for MultipleNode {
    fn get_all_parameters(&self) -> std::collections::HashMap<String, f32> {
        let mut params = std::collections::HashMap::new();
        params.insert("channel_count".to_string(), self.channel_count);
        params.insert("buffered".to_string(), self.buffered);
        params.insert("invert_alternate".to_string(), self.invert_alternate);
        params.insert("active".to_string(), self.active);
        
        // Add per-channel gain parameters
        for (i, &gain) in self.output_gains.iter().enumerate() {
            params.insert(format!("gain_{}", i), gain);
        }
        
        params
    }
    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), crate::parameters::ParameterError> {
        // Handle per-channel gain parameters
        if name.starts_with("gain_") {
            if let Some(channel_str) = name.strip_prefix("gain_") {
                if let Ok(channel) = channel_str.parse::<usize>() {
                    if channel < self.output_gains.len() {
                        if value >= 0.0 && value <= 2.0 {
                            self.output_gains[channel] = value;
                            self.gain_params[channel].set_base_value(value)?;
                            return Ok(());
                        } else {
                            return Err(crate::parameters::ParameterError::OutOfRange { 
                                value, min: 0.0, max: 2.0 
                            });
                        }
                    } else {
                        return Err(crate::parameters::ParameterError::InvalidType {
                            expected: "valid channel index".to_string(),
                            found: format!("index {}", channel)
                        });
                    }
                }
            }
            return Err(crate::parameters::ParameterError::NotFound { name: name.to_string() });
        }

        // Handle standard parameters
        match name {
            "channel_count" => {
                let count = value.clamp(2.0, 8.0) as usize;
                if count != self.output_gains.len() {
                    // Resize vectors when channel count changes
                    self.output_gains.resize(count, 1.0);
                    self.gain_params.resize_with(count, || 
                        ModulatableParameter::new(
                            BasicParameter::new("gain", 0.0, 2.0, 1.0),
                            0.8
                        )
                    );
                }
                self.channel_count = count as f32;
                Ok(())
            },
            "buffered" => {
                if value >= 0.0 && value <= 1.0 {
                    self.buffered = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 1.0 
                    })
                }
            },
            "invert_alternate" => {
                if value >= 0.0 && value <= 1.0 {
                    self.invert_alternate = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 1.0 
                    })
                }
            },
            "active" => {
                if value >= 0.0 && value <= 1.0 {
                    self.active = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 1.0 
                    })
                }
            },
            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() }),
        }
    }

    fn get_parameter(&self, name: &str) -> Result<f32, crate::parameters::ParameterError> {
        // Handle per-channel gain parameters
        if name.starts_with("gain_") {
            if let Some(channel_str) = name.strip_prefix("gain_") {
                if let Ok(channel) = channel_str.parse::<usize>() {
                    if channel < self.output_gains.len() {
                        return Ok(self.output_gains[channel]);
                    } else {
                        return Err(crate::parameters::ParameterError::InvalidType {
                            expected: "valid channel index".to_string(),
                            found: format!("index {}", channel)
                        });
                    }
                }
            }
            return Err(crate::parameters::ParameterError::NotFound { name: name.to_string() });
        }

        // Handle standard parameters
        match name {
            "channel_count" => Ok(self.channel_count),
            "buffered" => Ok(self.buffered),
            "invert_alternate" => Ok(self.invert_alternate),
            "active" => Ok(self.active),
            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() }),
        }
    }

    fn get_parameter_descriptors(&self) -> Vec<Box<dyn ParameterDescriptor>> {
        let mut descriptors: Vec<Box<dyn ParameterDescriptor>> = vec![
            Box::new(BasicParameter::new("channel_count", 2.0, 8.0, 4.0)),
            Box::new(BasicParameter::new("buffered", 0.0, 1.0, 0.0)),
            Box::new(BasicParameter::new("invert_alternate", 0.0, 1.0, 0.0)),
            Box::new(BasicParameter::new("active", 0.0, 1.0, 1.0)),
        ];

        // Add per-channel gain parameters
        for _i in 0..self.output_gains.len() {
            descriptors.push(Box::new(BasicParameter::new(
                "gain",  // Use static string for now
                0.0, 2.0, 1.0
            )));
        }

        descriptors
    }
}

impl AudioNode for MultipleNode {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - zero all outputs
            for i in 1..=self.get_channel_count() {
                if let Some(output) = ctx.outputs.get_audio_mut(&format!("out_{}", i)) {
                    output.fill(0.0);
                }
            }
            return Ok(());
        }

        // Get input signal
        let signal_input = ctx.inputs.get_audio("signal_in").unwrap_or(&[]);

        // Get buffer size from first output
        let buffer_size = ctx.outputs.get_audio(&format!("out_1"))
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "out_1".to_string() 
            })?.len();

        // Process each channel
        for channel in 0..self.get_channel_count() {
            let output_name = format!("out_{}", channel + 1);
            
            if let Some(output) = ctx.outputs.get_audio_mut(&output_name) {
                // Get CV modulation for this channel if available
                let gain_cv_name = format!("gain_{}_cv", channel);
                let gain_cv = ctx.inputs.get_cv_value(&gain_cv_name);
                
                // Apply CV modulation to gain
                let effective_gain = self.gain_params[channel].modulate(
                    self.output_gains[channel], 
                    gain_cv
                );

                // Process each sample
                for i in 0..buffer_size.min(output.len()) {
                    let input_sample = if i < signal_input.len() { 
                        signal_input[i] 
                    } else { 
                        0.0 
                    };

                    // Process distribution
                    let distributed_sample = self.process_distribution(
                        input_sample, 
                        channel, 
                        effective_gain
                    );

                    // Apply soft clipping
                    output[i] = self.soft_clip(distributed_sample);
                }
            }
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // No internal state to reset for signal distribution
    }

    fn latency(&self) -> u32 {
        0 // No latency for signal distribution
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_multiple_parameters() {
        let mut mult = MultipleNode::new(44100.0, "test".to_string(), 4);
        
        // Test channel count
        assert_eq!(mult.get_parameter("channel_count").unwrap(), 4.0);
        
        // Test individual gain setting
        assert!(mult.set_parameter("gain_0", 0.5).is_ok());
        assert_eq!(mult.get_parameter("gain_0").unwrap(), 0.5);
        
        assert!(mult.set_parameter("gain_1", 1.5).is_ok());
        assert_eq!(mult.get_parameter("gain_1").unwrap(), 1.5);
        
        // Test buffered mode
        assert!(mult.set_parameter("buffered", 1.0).is_ok());
        assert_eq!(mult.get_parameter("buffered").unwrap(), 1.0);
        
        // Test invert alternate
        assert!(mult.set_parameter("invert_alternate", 1.0).is_ok());
        assert_eq!(mult.get_parameter("invert_alternate").unwrap(), 1.0);
        
        // Test validation
        assert!(mult.set_parameter("gain_0", 5.0).is_err()); // Out of range
        assert!(mult.set_parameter("gain_10", 1.0).is_err()); // Invalid channel
    }

    #[test]
    fn test_signal_distribution() {
        let mut mult = MultipleNode::new(44100.0, "test".to_string(), 4);
        
        let signal_data = vec![1.0, -1.0, 0.5, -0.5];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data.clone());
        
        let mut outputs = OutputBuffers::new();
        for i in 1..=4 {
            outputs.allocate_audio(format!("out_{}", i), 4);
        }
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(mult.process(&mut ctx).is_ok());
        
        // All outputs should match input (unity gain)
        for i in 1..=4 {
            let output = ctx.outputs.get_audio(&format!("out_{}", i)).unwrap();
            for (j, &expected) in signal_data.iter().enumerate() {
                assert!((output[j] - expected).abs() < 0.01, 
                        "Channel {}, sample {}: expected {}, got {}", i, j, expected, output[j]);
            }
        }
    }

    #[test]
    fn test_individual_gains() {
        let mut mult = MultipleNode::new(44100.0, "test".to_string(), 3);
        
        // Set different gains for each channel
        mult.set_parameter("gain_0", 0.5).unwrap(); // 50%
        mult.set_parameter("gain_1", 1.0).unwrap(); // 100%
        mult.set_parameter("gain_2", 2.0).unwrap(); // 200%
        
        let signal_data = vec![1.0];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        for i in 1..=3 {
            outputs.allocate_audio(format!("out_{}", i), 1);
        }
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 1,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(mult.process(&mut ctx).is_ok());
        
        // Check individual gains
        assert!((ctx.outputs.get_audio("out_1").unwrap()[0] - 0.5).abs() < 0.01);
        assert!((ctx.outputs.get_audio("out_2").unwrap()[0] - 1.0).abs() < 0.01);
        assert!((ctx.outputs.get_audio("out_3").unwrap()[0] - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_invert_alternate() {
        let mut mult = MultipleNode::new(44100.0, "test".to_string(), 4);
        mult.set_parameter("invert_alternate", 1.0).unwrap(); // Enable alternate inversion
        
        let signal_data = vec![1.0];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        for i in 1..=4 {
            outputs.allocate_audio(format!("out_{}", i), 1);
        }
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 1,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(mult.process(&mut ctx).is_ok());
        
        // Check alternating inversion (channels 2 and 4 should be inverted)
        assert!(ctx.outputs.get_audio("out_1").unwrap()[0] > 0.0); // Normal
        assert!(ctx.outputs.get_audio("out_2").unwrap()[0] < 0.0); // Inverted
        assert!(ctx.outputs.get_audio("out_3").unwrap()[0] > 0.0); // Normal
        assert!(ctx.outputs.get_audio("out_4").unwrap()[0] < 0.0); // Inverted
    }

    #[test]
    fn test_buffered_mode() {
        let mut mult = MultipleNode::new(44100.0, "test".to_string(), 2);
        mult.set_parameter("buffered", 1.0).unwrap(); // Enable buffered mode
        
        let signal_data = vec![1.0];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("out_1".to_string(), 1);
        outputs.allocate_audio("out_2".to_string(), 1);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 1,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(mult.process(&mut ctx).is_ok());
        
        // Buffered mode should produce slightly different output
        let output1 = ctx.outputs.get_audio("out_1").unwrap()[0];
        assert!(output1 > 1.0, "Buffered mode should have slight gain boost: {}", output1);
    }

    #[test]
    fn test_inactive_state() {
        let mut mult = MultipleNode::new(44100.0, "test".to_string(), 3);
        mult.set_parameter("active", 0.0).unwrap(); // Disable
        
        let signal_data = vec![1.0, -1.0, 0.5];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        for i in 1..=3 {
            outputs.allocate_audio(format!("out_{}", i), 3);
        }
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(mult.process(&mut ctx).is_ok());
        
        // All outputs should be zero when inactive
        for i in 1..=3 {
            let output = ctx.outputs.get_audio(&format!("out_{}", i)).unwrap();
            for (j, &sample) in output.iter().enumerate() {
                assert!((sample - 0.0).abs() < 0.001, 
                        "Channel {}, sample {}: should be zero but got {}", i, j, sample);
            }
        }
    }

    #[test]
    fn test_channel_count_change() {
        let mut mult = MultipleNode::new(44100.0, "test".to_string(), 4);
        
        // Check initial channel count
        assert_eq!(mult.get_channel_count(), 4);
        
        // Change channel count
        assert!(mult.set_parameter("channel_count", 6.0).is_ok());
        assert_eq!(mult.get_channel_count(), 6);
        assert_eq!(mult.output_gains.len(), 6);
        assert_eq!(mult.gain_params.len(), 6);
        
        // Test that new channels have default gain
        assert_eq!(mult.get_parameter("gain_4").unwrap(), 1.0);
        assert_eq!(mult.get_parameter("gain_5").unwrap(), 1.0);
    }
}