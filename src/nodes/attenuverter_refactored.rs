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

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor, ModulationCurve};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;
use crate::define_parameters;

/// リファクタリング済みAttenuverterNode - プロ品質のアッテニューバーター
pub struct AttenuverterNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Attenuverter parameters
    attenuation: f32,        // -2.0 to +2.0 (negative = invert, >1 = amplify)
    offset: f32,             // DC offset -10V to +10V
    scale: f32,              // 0.0 to 2.0 (additional scaling factor)
    response_curve: f32,     // 0.0 = linear, 1.0 = exponential
    active: f32,
    
    // CV Modulation parameters
    attenuation_param: ModulatableParameter,
    offset_param: ModulatableParameter,
    
    sample_rate: f32,
}

impl AttenuverterNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "attenuverter_refactored".to_string(),
            category: NodeCategory::Utility,
            description: "Professional attenuverter with CV modulation and response curves".to_string(),
            input_ports: vec![
                PortInfo::new("signal_in", PortType::AudioMono)
                    .with_description("Input signal to be processed"),
                PortInfo::new("attenuation_cv", PortType::CV)
                    .with_description("Attenuation amount modulation")
                    .optional(),
                PortInfo::new("offset_cv", PortType::CV)
                    .with_description("DC offset modulation")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("signal_out", PortType::AudioMono)
                    .with_description("Processed signal output"),
                PortInfo::new("inverted_out", PortType::AudioMono)
                    .with_description("Inverted signal output")
                    .optional(),
                PortInfo::new("scaled_out", PortType::AudioMono)
                    .with_description("Additional scaled output")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルアッテニューバーター用
        let attenuation_param = ModulatableParameter::new(
            BasicParameter::new("attenuation", -2.0, 2.0, 1.0),
            1.0  // 100% CV modulation range
        );

        let offset_param = ModulatableParameter::new(
            BasicParameter::new("offset", -10.0, 10.0, 0.0),
            1.0  // 100% CV modulation range
        );

        Self {
            node_info,
            attenuation: 1.0,        // Unity gain by default
            offset: 0.0,             // No offset by default
            scale: 1.0,              // Unity scale by default
            response_curve: 0.0,     // Linear response by default
            active: 1.0,

            attenuation_param,
            offset_param,
            
            sample_rate,
        }
    }

    /// Process signal with attenuation, offset, and response curve
    fn process_attenuversion(&self, input: f32, effective_attenuation: f32, effective_offset: f32) -> f32 {
        // Apply response curve to the input signal
        let curved_input = if self.response_curve > 0.0 {
            self.apply_response_curve(input)
        } else {
            input
        };
        
        // Apply attenuation/inversion first
        let attenuated = curved_input * effective_attenuation * self.scale;
        
        // Add DC offset
        let output = attenuated + effective_offset;
        
        // Soft clipping to prevent harsh digital clipping
        self.soft_clip(output)
    }

    /// Apply response curve to input signal
    fn apply_response_curve(&self, input: f32) -> f32 {
        if self.response_curve == 0.0 {
            return input; // Linear response
        }
        
        let abs_input = input.abs();
        let sign = input.signum();
        
        // Exponential response curve
        let curved = if abs_input > 0.001 {
            let curve_amount = self.response_curve;
            abs_input.powf(1.0 + curve_amount)
        } else {
            abs_input
        };
        
        curved * sign
    }

    /// Soft clipping using tanh function for musical saturation
    fn soft_clip(&self, input: f32) -> f32 {
        const CLIP_THRESHOLD: f32 = 10.0; // ±10V typical modular range
        
        if input.abs() > CLIP_THRESHOLD {
            CLIP_THRESHOLD * (input / CLIP_THRESHOLD).tanh()
        } else {
            // Gentle saturation even below threshold for warmth
            input * (1.0 + 0.1 * (input / CLIP_THRESHOLD).tanh())
        }
    }

    /// Generate inverted signal with independent processing
    fn process_inverted(&self, input: f32, effective_attenuation: f32) -> f32 {
        // Inverted version without offset, but with scale and curve
        let curved_input = if self.response_curve > 0.0 {
            self.apply_response_curve(input)
        } else {
            input
        };
        
        let inverted = -curved_input * effective_attenuation.abs() * self.scale;
        self.soft_clip(inverted)
    }

    /// Generate additional scaled output
    fn process_scaled(&self, input: f32, effective_attenuation: f32, effective_offset: f32) -> f32 {
        // Scaled version with different scaling factor
        let scaled_attenuation = effective_attenuation * self.scale * 0.5; // Half scale for variety
        let curved_input = if self.response_curve > 0.0 {
            self.apply_response_curve(input)
        } else {
            input
        };
        
        let scaled = curved_input * scaled_attenuation + (effective_offset * 0.5);
        self.soft_clip(scaled)
    }
}

impl Parameterizable for AttenuverterNodeRefactored {
    define_parameters! {
        attenuation: BasicParameter::new("attenuation", -2.0, 2.0, 1.0),
        offset: BasicParameter::new("offset", -10.0, 10.0, 0.0),
        scale: BasicParameter::new("scale", 0.0, 2.0, 1.0),
        response_curve: BasicParameter::new("response_curve", 0.0, 1.0, 0.0),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for AttenuverterNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - pass through input signal
            if let (Some(input), Some(output)) = 
                (ctx.inputs.get_audio("signal_in"), ctx.outputs.get_audio_mut("signal_out")) {
                output.copy_from_slice(&input[..output.len().min(input.len())]);
            }
            return Ok(());
        }

        // Get input signals
        let signal_input = ctx.inputs.get_audio("signal_in").unwrap_or(&[]);
        
        // Get CV inputs
        let attenuation_cv = ctx.inputs.get_cv_value("attenuation_cv");
        let offset_cv = ctx.inputs.get_cv_value("offset_cv");

        // Apply CV modulation
        let effective_attenuation = self.attenuation_param.modulate(self.attenuation, attenuation_cv);
        let effective_offset = self.offset_param.modulate(self.offset, offset_cv);

        // Get buffer size
        let buffer_size = ctx.outputs.get_audio("signal_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "signal_out".to_string() 
            })?.len();

        // Process each sample
        let mut output_samples = Vec::with_capacity(buffer_size);
        let mut inverted_samples = Vec::with_capacity(buffer_size);
        let mut scaled_samples = Vec::with_capacity(buffer_size);

        for i in 0..buffer_size {
            // Get input sample
            let input_sample = if i < signal_input.len() { 
                signal_input[i] 
            } else { 
                0.0 
            };

            // Process main output
            let main_output = self.process_attenuversion(
                input_sample, 
                effective_attenuation, 
                effective_offset
            );
            output_samples.push(main_output);

            // Process inverted output
            let inverted_output = self.process_inverted(
                input_sample, 
                effective_attenuation
            );
            inverted_samples.push(inverted_output);

            // Process scaled output
            let scaled_output = self.process_scaled(
                input_sample, 
                effective_attenuation, 
                effective_offset
            );
            scaled_samples.push(scaled_output);
        }

        // Write to output buffers
        if let Some(signal_output) = ctx.outputs.get_audio_mut("signal_out") {
            for (i, &sample) in output_samples.iter().enumerate() {
                if i < signal_output.len() {
                    signal_output[i] = sample;
                }
            }
        }

        if let Some(inverted_output) = ctx.outputs.get_audio_mut("inverted_out") {
            for (i, &sample) in inverted_samples.iter().enumerate() {
                if i < inverted_output.len() {
                    inverted_output[i] = sample;
                }
            }
        }

        if let Some(scaled_output) = ctx.outputs.get_audio_mut("scaled_out") {
            for (i, &sample) in scaled_samples.iter().enumerate() {
                if i < scaled_output.len() {
                    scaled_output[i] = sample;
                }
            }
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // No internal state to reset for attenuverter
    }

    fn latency(&self) -> u32 {
        0 // No latency for attenuation/offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_attenuverter_parameters() {
        let mut atten = AttenuverterNodeRefactored::new(44100.0, "test".to_string());
        
        // Test attenuation setting
        assert!(atten.set_parameter("attenuation", 0.5).is_ok());
        assert_eq!(atten.get_parameter("attenuation").unwrap(), 0.5);
        
        // Test offset setting
        assert!(atten.set_parameter("offset", 2.5).is_ok());
        assert_eq!(atten.get_parameter("offset").unwrap(), 2.5);
        
        // Test scale setting
        assert!(atten.set_parameter("scale", 1.5).is_ok());
        assert_eq!(atten.get_parameter("scale").unwrap(), 1.5);
        
        // Test response curve setting
        assert!(atten.set_parameter("response_curve", 0.3).is_ok());
        assert_eq!(atten.get_parameter("response_curve").unwrap(), 0.3);
        
        // Test validation
        assert!(atten.set_parameter("attenuation", 5.0).is_err()); // Out of range
        assert!(atten.set_parameter("offset", 20.0).is_err()); // Out of range
    }

    #[test]
    fn test_basic_attenuation() {
        let mut atten = AttenuverterNodeRefactored::new(44100.0, "test".to_string());
        atten.set_parameter("attenuation", 0.5).unwrap(); // 50% attenuation
        atten.set_parameter("offset", 0.0).unwrap(); // No offset
        
        let signal_data = vec![1.0, -1.0, 2.0, -2.0];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(atten.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should be attenuated by 50% (with tolerance for soft clipping effects)
        assert!((output[0] - 0.5).abs() < 0.01);
        assert!((output[1] - (-0.5)).abs() < 0.01);
        assert!((output[2] - 1.0).abs() < 0.01);
        assert!((output[3] - (-1.0)).abs() < 0.01);
    }

    #[test]
    fn test_inversion() {
        let mut atten = AttenuverterNodeRefactored::new(44100.0, "test".to_string());
        atten.set_parameter("attenuation", -1.0).unwrap(); // Full inversion
        atten.set_parameter("offset", 0.0).unwrap(); // No offset
        
        let signal_data = vec![1.0, -1.0, 0.5];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 3);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(atten.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should be inverted
        assert!((output[0] - (-1.0)).abs() < 0.01);
        assert!((output[1] - 1.0).abs() < 0.01);
        assert!((output[2] - (-0.5)).abs() < 0.01);
    }

    #[test]
    fn test_offset() {
        let mut atten = AttenuverterNodeRefactored::new(44100.0, "test".to_string());
        atten.set_parameter("attenuation", 1.0).unwrap(); // No attenuation
        atten.set_parameter("offset", 2.0).unwrap(); // +2V offset
        
        let signal_data = vec![0.0, 1.0, -1.0];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 3);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(atten.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should have +2V offset
        assert!((output[0] - 2.0).abs() < 0.01);
        assert!((output[1] - 3.0).abs() < 0.01);
        assert!((output[2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_amplification() {
        let mut atten = AttenuverterNodeRefactored::new(44100.0, "test".to_string());
        atten.set_parameter("attenuation", 2.0).unwrap(); // 2x amplification
        atten.set_parameter("offset", 0.0).unwrap(); // No offset
        
        let signal_data = vec![0.5, -0.5, 1.0];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 3);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(atten.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should be amplified by 2x
        assert!((output[0] - 1.0).abs() < 0.01);
        assert!((output[1] - (-1.0)).abs() < 0.01);
        assert!((output[2] - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_cv_modulation() {
        let mut atten = AttenuverterNodeRefactored::new(44100.0, "test".to_string());
        atten.set_parameter("attenuation", 1.0).unwrap(); // Base unity gain
        atten.set_parameter("offset", 0.0).unwrap(); // Base no offset
        
        let signal_data = vec![1.0];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        inputs.add_cv("attenuation_cv".to_string(), vec![-0.5]); // Reduce attenuation
        inputs.add_cv("offset_cv".to_string(), vec![1.0]); // Add offset
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 1);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 1,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(atten.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should be modulated by CV inputs
        assert!(output[0] != 1.0, "Output should be affected by CV modulation");
    }

    #[test]
    fn test_inverted_output() {
        let mut atten = AttenuverterNodeRefactored::new(44100.0, "test".to_string());
        atten.set_parameter("attenuation", 1.0).unwrap();
        
        let signal_data = vec![1.0, -1.0, 0.5];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 3);
        outputs.allocate_audio("inverted_out".to_string(), 3);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(atten.process(&mut ctx).is_ok());
        
        let main_output = ctx.outputs.get_audio("signal_out").unwrap();
        let inverted_output = ctx.outputs.get_audio("inverted_out").unwrap();
        
        // Inverted output should be negative of input (without offset)
        assert!((inverted_output[0] - (-1.0)).abs() < 0.1);
        assert!((inverted_output[1] - 1.0).abs() < 0.1);
        assert!((inverted_output[2] - (-0.5)).abs() < 0.1);
    }

    #[test]
    fn test_inactive_state() {
        let mut atten = AttenuverterNodeRefactored::new(44100.0, "test".to_string());
        atten.set_parameter("active", 0.0).unwrap(); // Disable
        atten.set_parameter("attenuation", 0.5).unwrap(); // Should be ignored
        atten.set_parameter("offset", 2.0).unwrap(); // Should be ignored
        
        let signal_data = vec![1.0, -1.0, 0.5];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data.clone());
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 3);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(atten.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should pass through input unchanged when inactive
        for (i, &expected) in signal_data.iter().enumerate() {
            assert!((output[i] - expected).abs() < 0.01, 
                    "Sample {}: expected {}, got {}", i, expected, output[i]);
        }
    }
}