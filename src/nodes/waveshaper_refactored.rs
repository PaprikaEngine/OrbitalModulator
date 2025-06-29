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
use crate::define_parameters;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveshaperType {
    Tanh = 0,
    ArcTan = 1,
    Sine = 2,
    Cubic = 3,
    HardClip = 4,
    SoftClip = 5,
    Tube = 6,
    Asymmetric = 7,
}

impl WaveshaperType {
    pub fn from_f32(value: f32) -> Self {
        match value as i32 {
            0 => WaveshaperType::Tanh,
            1 => WaveshaperType::ArcTan,
            2 => WaveshaperType::Sine,
            3 => WaveshaperType::Cubic,
            4 => WaveshaperType::HardClip,
            5 => WaveshaperType::SoftClip,
            6 => WaveshaperType::Tube,
            7 => WaveshaperType::Asymmetric,
            _ => WaveshaperType::Tanh,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            WaveshaperType::Tanh => "Tanh",
            WaveshaperType::ArcTan => "ArcTan",
            WaveshaperType::Sine => "Sine",
            WaveshaperType::Cubic => "Cubic",
            WaveshaperType::HardClip => "Hard Clip",
            WaveshaperType::SoftClip => "Soft Clip",
            WaveshaperType::Tube => "Tube",
            WaveshaperType::Asymmetric => "Asymmetric",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            WaveshaperType::Tanh => "Hyperbolic tangent saturation",
            WaveshaperType::ArcTan => "Arctangent soft saturation",
            WaveshaperType::Sine => "Sine wave distortion",
            WaveshaperType::Cubic => "Cubic polynomial distortion",
            WaveshaperType::HardClip => "Hard clipping distortion",
            WaveshaperType::SoftClip => "Soft saturation clipping",
            WaveshaperType::Tube => "Tube-style harmonic distortion",
            WaveshaperType::Asymmetric => "Asymmetric bias distortion",
        }
    }
}

/// リファクタリング済みWaveshaperNode - プロ品質のウェーブシェイピング歪み
pub struct WaveshaperNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Parameters
    drive: f32,              // Input drive/gain
    shape_type: f32,         // Waveshaper type (0-7)
    shape_amount: f32,       // Shaping intensity
    bias: f32,              // DC bias for asymmetric distortion
    output_gain: f32,       // Output level compensation
    tone: f32,              // Tone control (pre/post filtering)
    active: f32,
    
    // CV Modulation parameters
    drive_param: ModulatableParameter,
    shape_amount_param: ModulatableParameter,
    bias_param: ModulatableParameter,
    output_gain_param: ModulatableParameter,
    
    // Internal filter state for tone control
    filter_state: f32,
    
    sample_rate: f32,
}

impl WaveshaperNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "waveshaper_refactored".to_string(),
            category: NodeCategory::Processor,
            description: "Professional waveshaping distortion with 8 distortion types".to_string(),
            input_ports: vec![
                PortInfo::new("audio_in", PortType::AudioMono)
                    .with_description("Audio input signal"),
                PortInfo::new("drive_cv", PortType::CV)
                    .with_description("Drive/gain control (0V to +10V)")
                    .optional(),
                PortInfo::new("shape_amount_cv", PortType::CV)
                    .with_description("Shape intensity control (0V to +10V)")
                    .optional(),
                PortInfo::new("bias_cv", PortType::CV)
                    .with_description("Asymmetric bias control (-10V to +10V)")
                    .optional(),
                PortInfo::new("output_gain_cv", PortType::CV)
                    .with_description("Output gain control (0V to +10V)")
                    .optional(),
                PortInfo::new("shape_type_cv", PortType::CV)
                    .with_description("Waveshaper type selection (0-7V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono)
                    .with_description("Waveshaped audio output"),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルウェーブシェイパー用
        let drive_param = ModulatableParameter::new(
            BasicParameter::new("drive", 0.1, 10.0, 1.0),
            0.8  // 80% CV modulation range
        );

        let shape_amount_param = ModulatableParameter::new(
            BasicParameter::new("shape_amount", 0.0, 1.0, 0.5),
            0.8  // 80% CV modulation range
        );

        let bias_param = ModulatableParameter::new(
            BasicParameter::new("bias", -1.0, 1.0, 0.0),
            0.8  // 80% CV modulation range
        );

        let output_gain_param = ModulatableParameter::new(
            BasicParameter::new("output_gain", 0.1, 2.0, 1.0),
            0.6  // 60% CV modulation range
        );

        Self {
            node_info,
            drive: 1.0,
            shape_type: 0.0, // Tanh default
            shape_amount: 0.5,
            bias: 0.0,
            output_gain: 1.0,
            tone: 0.5, // Neutral tone
            active: 1.0,

            drive_param,
            shape_amount_param,
            bias_param,
            output_gain_param,
            
            filter_state: 0.0,
            
            sample_rate,
        }
    }

    /// Apply waveshaping based on the selected type
    fn apply_waveshaping(&self, input: f32, shape_type: WaveshaperType, shape_amount: f32, bias: f32) -> f32 {
        // Apply bias for asymmetric distortion
        let biased_input = input + bias;
        
        // Apply waveshaping based on type
        let shaped = match shape_type {
            WaveshaperType::Tanh => {
                // Hyperbolic tangent - smooth saturation
                let drive_factor = 1.0 + shape_amount * 4.0;
                (biased_input * drive_factor).tanh()
            },
            WaveshaperType::ArcTan => {
                // Arctangent - gentle soft clipping
                let drive_factor = 1.0 + shape_amount * 3.0;
                (biased_input * drive_factor).atan() * (2.0 / std::f32::consts::PI)
            },
            WaveshaperType::Sine => {
                // Sine wave distortion - adds harmonics
                let drive_factor = 1.0 + shape_amount * 2.0;
                let driven = biased_input * drive_factor;
                if driven.abs() > 1.0 {
                    driven.signum()
                } else {
                    driven + shape_amount * (driven * std::f32::consts::PI).sin() * 0.3
                }
            },
            WaveshaperType::Cubic => {
                // Cubic polynomial - warm harmonic distortion
                let x = biased_input.clamp(-1.0, 1.0);
                let cubic = x - (x * x * x) / 3.0;
                biased_input + shape_amount * (cubic - biased_input)
            },
            WaveshaperType::HardClip => {
                // Hard clipping
                let threshold = 1.0 - shape_amount * 0.8;
                biased_input.clamp(-threshold, threshold)
            },
            WaveshaperType::SoftClip => {
                // Soft clipping with smooth transition
                let threshold = 1.0 - shape_amount * 0.6;
                if biased_input.abs() <= threshold {
                    biased_input
                } else {
                    let overshoot = biased_input.abs() - threshold;
                    let soft_factor = 1.0 / (1.0 + overshoot);
                    biased_input.signum() * (threshold + overshoot * soft_factor)
                }
            },
            WaveshaperType::Tube => {
                // Tube-style distortion with even harmonics
                let drive_factor = 1.0 + shape_amount * 2.0;
                let driven = biased_input * drive_factor;
                let tube_curve = driven / (1.0 + driven.abs().powf(0.7));
                tube_curve + shape_amount * driven * driven * 0.1 // Add even harmonics
            },
            WaveshaperType::Asymmetric => {
                // Asymmetric distortion - different curves for positive/negative
                if biased_input >= 0.0 {
                    let curve = 1.0 + shape_amount * 2.0;
                    biased_input.powf(curve)
                } else {
                    let curve = 1.0 + shape_amount * 0.5;
                    -(-biased_input).powf(curve)
                }
            },
        };
        
        // For asymmetric distortion, keep the bias effect
        if matches!(shape_type, WaveshaperType::Asymmetric) {
            shaped
        } else {
            // Remove bias offset for other types
            shaped - bias
        }
    }

    /// Simple tone control filter
    fn apply_tone_filter(&mut self, input: f32, tone: f32) -> f32 {
        // Simple low-pass filter for tone control
        // tone = 0.0 (dark), tone = 0.5 (neutral), tone = 1.0 (bright)
        let cutoff_factor = 0.1 + tone * 0.8;
        self.filter_state += (input - self.filter_state) * cutoff_factor;
        
        // Mix between filtered (dark) and original (bright)
        let mix = tone * 2.0;
        if mix <= 1.0 {
            self.filter_state * (1.0 - mix) + input * mix
        } else {
            // Brightness boost for mix > 1.0
            input + (input - self.filter_state) * (mix - 1.0) * 0.3
        }
    }
}

impl Parameterizable for WaveshaperNodeRefactored {
    define_parameters! {
        drive: BasicParameter::new("drive", 0.1, 10.0, 1.0),
        shape_type: BasicParameter::new("shape_type", 0.0, 7.0, 0.0),
        shape_amount: BasicParameter::new("shape_amount", 0.0, 1.0, 0.5),
        bias: BasicParameter::new("bias", -1.0, 1.0, 0.0),
        output_gain: BasicParameter::new("output_gain", 0.1, 2.0, 1.0),
        tone: BasicParameter::new("tone", 0.0, 1.0, 0.5),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for WaveshaperNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - pass through input signal
            let audio_input = ctx.inputs.get_audio("audio_in").unwrap_or(&[]);
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                for (i, output_sample) in output.iter_mut().enumerate() {
                    *output_sample = if i < audio_input.len() { audio_input[i] } else { 0.0 };
                }
            }
            return Ok(());
        }

        // Get audio input
        let audio_input = ctx.inputs.get_audio("audio_in").unwrap_or(&[]);
        if audio_input.is_empty() {
            // No input - output silence
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                output.fill(0.0);
            }
            return Ok(());
        }

        // Get CV inputs
        let drive_cv = ctx.inputs.get_cv_value("drive_cv");
        let shape_amount_cv = ctx.inputs.get_cv_value("shape_amount_cv");
        let bias_cv = ctx.inputs.get_cv_value("bias_cv");
        let output_gain_cv = ctx.inputs.get_cv_value("output_gain_cv");
        let shape_type_cv = ctx.inputs.get_cv_value("shape_type_cv");

        // Apply CV modulation
        let effective_drive = self.drive_param.modulate(self.drive, drive_cv);
        let effective_shape_amount = self.shape_amount_param.modulate(self.shape_amount, shape_amount_cv);
        let effective_bias = self.bias_param.modulate(self.bias, bias_cv);
        let effective_output_gain = self.output_gain_param.modulate(self.output_gain, output_gain_cv);

        // Update shape type from CV if provided
        let current_shape_type = if shape_type_cv != 0.0 {
            WaveshaperType::from_f32(shape_type_cv.clamp(0.0, 7.0))
        } else {
            WaveshaperType::from_f32(self.shape_type)
        };

        // Process audio output
        let output = ctx.outputs.get_audio_mut("audio_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "audio_out".to_string() 
            })?;

        // Process each sample through the waveshaper
        for (i, output_sample) in output.iter_mut().enumerate() {
            let input_sample = if i < audio_input.len() { 
                audio_input[i] 
            } else { 
                0.0 
            };

            // Apply drive/input gain
            let driven_sample = input_sample * effective_drive;

            // Apply waveshaping
            let shaped_sample = self.apply_waveshaping(
                driven_sample,
                current_shape_type,
                effective_shape_amount,
                effective_bias
            );

            // Apply tone filtering
            let filtered_sample = self.apply_tone_filter(shaped_sample, self.tone);

            // Apply output gain and ensure no clipping
            *output_sample = (filtered_sample * effective_output_gain).clamp(-1.0, 1.0);
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset filter state
        self.filter_state = 0.0;
    }

    fn latency(&self) -> u32 {
        0 // No latency for waveshaping
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_waveshaper_parameters() {
        let mut waveshaper = WaveshaperNodeRefactored::new(44100.0, "test".to_string());
        
        // Test drive setting
        assert!(waveshaper.set_parameter("drive", 2.0).is_ok());
        assert_eq!(waveshaper.get_parameter("drive").unwrap(), 2.0);
        
        // Test shape type setting
        assert!(waveshaper.set_parameter("shape_type", 3.0).is_ok());
        assert_eq!(waveshaper.get_parameter("shape_type").unwrap(), 3.0);
        
        // Test shape amount setting
        assert!(waveshaper.set_parameter("shape_amount", 0.8).is_ok());
        assert_eq!(waveshaper.get_parameter("shape_amount").unwrap(), 0.8);
        
        // Test validation
        assert!(waveshaper.set_parameter("drive", -1.0).is_err()); // Out of range
        assert!(waveshaper.set_parameter("shape_type", 10.0).is_err()); // Out of range
    }

    #[test]
    fn test_waveshaper_processing() {
        let mut waveshaper = WaveshaperNodeRefactored::new(44100.0, "test".to_string());
        waveshaper.set_parameter("drive", 2.0).unwrap();
        waveshaper.set_parameter("shape_amount", 0.7).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.5; 512]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        // Should process without error
        assert!(waveshaper.process(&mut ctx).is_ok());
        
        // Output should be different from input (distorted)
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_distortion = output.iter().any(|&s| (s - 0.5).abs() > 0.1);
        assert!(has_distortion, "Should apply waveshaping distortion");
    }

    #[test]
    fn test_different_waveshaper_types() {
        let mut waveshaper = WaveshaperNodeRefactored::new(44100.0, "test".to_string());
        
        // Test each waveshaper type
        for shape_type in 0..8 {
            waveshaper.set_parameter("shape_type", shape_type as f32).unwrap();
            waveshaper.set_parameter("shape_amount", 0.8).unwrap();
            
            let mut inputs = InputBuffers::new();
            inputs.add_audio("audio_in".to_string(), vec![0.7; 64]);
            
            let mut outputs = OutputBuffers::new();
            outputs.allocate_audio("audio_out".to_string(), 64);
            
            let mut ctx = ProcessContext {
                inputs: &inputs,
                outputs: &mut outputs,
                sample_rate: 44100.0,
                buffer_size: 64,
                timestamp: 0,
                bpm: 120.0,
            };
            
            assert!(waveshaper.process(&mut ctx).is_ok());
            
            let output = ctx.outputs.get_audio("audio_out").unwrap();
            let has_shaped_output = output.iter().any(|&s| s.abs() > 0.001);
            assert!(has_shaped_output, "Shape type {} should produce output", shape_type);
            
            // Output should be bounded
            let max_output = output.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
            assert!(max_output <= 1.0, "Output should be clipped to [-1,1]: {}", max_output);
        }
    }

    #[test]
    fn test_drive_cv_modulation() {
        let mut waveshaper = WaveshaperNodeRefactored::new(44100.0, "test".to_string());
        waveshaper.set_parameter("shape_amount", 0.5).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.3; 256]);
        inputs.add_cv("drive_cv".to_string(), vec![3.0]); // Increase drive
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 256);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 256,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(waveshaper.process(&mut ctx).is_ok());
        
        // CV should increase drive, making distortion more pronounced
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let avg_output = output.iter().map(|&x| x.abs()).sum::<f32>() / output.len() as f32;
        assert!(avg_output > 0.2, "Drive CV should increase distortion: {}", avg_output);
    }

    #[test]
    fn test_bias_distortion() {
        let mut waveshaper = WaveshaperNodeRefactored::new(44100.0, "test".to_string());
        waveshaper.set_parameter("shape_type", 7.0).unwrap(); // Asymmetric
        waveshaper.set_parameter("bias", 0.5).unwrap(); // Positive bias
        waveshaper.set_parameter("shape_amount", 0.8).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.5; 256]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 256);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 256,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(waveshaper.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // Asymmetric distortion with bias should produce different output from input
        let input_avg = 0.5;
        let output_avg = output.iter().sum::<f32>() / output.len() as f32;
        let has_asymmetric_output = (output_avg - input_avg).abs() > 0.05;
        assert!(has_asymmetric_output, "Bias should create asymmetric distortion: in={}, out={}", input_avg, output_avg);
    }

    #[test]
    fn test_shape_amount_cv() {
        let mut waveshaper = WaveshaperNodeRefactored::new(44100.0, "test".to_string());
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.6; 128]);
        inputs.add_cv("shape_amount_cv".to_string(), vec![5.0]); // High shaping
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 128);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 128,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(waveshaper.process(&mut ctx).is_ok());
        
        // Higher shape amount should produce more distortion
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let distortion_amount = output.iter().map(|&s| (s - 0.6).abs()).sum::<f32>() / output.len() as f32;
        assert!(distortion_amount > 0.05, "Shape amount CV should increase distortion: {}", distortion_amount);
    }

    #[test]
    fn test_output_clipping() {
        let mut waveshaper = WaveshaperNodeRefactored::new(44100.0, "test".to_string());
        waveshaper.set_parameter("drive", 10.0).unwrap(); // Very high drive
        waveshaper.set_parameter("shape_amount", 1.0).unwrap(); // Max shaping
        waveshaper.set_parameter("output_gain", 2.0).unwrap(); // High output gain
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.8; 512]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(waveshaper.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // Even with extreme settings, output should be clipped to [-1,1]
        let max_output = output.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        assert!(max_output <= 1.0, "Output should be clipped: {}", max_output);
    }

    #[test]
    fn test_inactive_state() {
        let mut waveshaper = WaveshaperNodeRefactored::new(44100.0, "test".to_string());
        waveshaper.set_parameter("active", 0.0).unwrap(); // Disable
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.5; 512]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(waveshaper.process(&mut ctx).is_ok());
        
        // Should pass through unchanged when inactive
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!((avg_output - 0.5).abs() < 0.001, "Should pass through when inactive: {}", avg_output);
    }
}