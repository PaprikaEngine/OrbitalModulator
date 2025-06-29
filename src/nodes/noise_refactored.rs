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
pub enum NoiseType {
    White = 0,
    Pink = 1,
    Brown = 2,
    Blue = 3,
}

impl NoiseType {
    pub fn from_f32(value: f32) -> Self {
        match value as i32 {
            0 => NoiseType::White,
            1 => NoiseType::Pink,
            2 => NoiseType::Brown,
            3 => NoiseType::Blue,
            _ => NoiseType::White,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            NoiseType::White => "White",
            NoiseType::Pink => "Pink",
            NoiseType::Brown => "Brown",
            NoiseType::Blue => "Blue",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            NoiseType::White => "Full spectrum white noise",
            NoiseType::Pink => "1/f pink noise (equal energy per octave)",
            NoiseType::Brown => "Brownian noise (low frequency emphasis)",
            NoiseType::Blue => "Blue noise (high frequency emphasis)",
        }
    }
}

/// リファクタリング済みNoiseNode - プロ品質の多色ノイズジェネレーター
pub struct NoiseNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Parameters
    noise_type: f32,
    amplitude: f32,
    active: f32,
    
    // CV Modulation parameters
    amplitude_param: ModulatableParameter,
    
    // Noise generation state
    rng_state: u32,
    pink_state: [f32; 7],
    brown_state: f32,
    blue_state: f32,
    
    sample_rate: f32,
}

impl NoiseNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "noise_refactored".to_string(),
            category: NodeCategory::Generator,
            description: "Professional multi-color noise generator with 4 noise types".to_string(),
            input_ports: vec![
                PortInfo::new("amplitude_cv", PortType::CV)
                    .with_description("Amplitude modulation (0V to +10V)")
                    .optional(),
                PortInfo::new("type_cv", PortType::CV)
                    .with_description("Noise type selection (0-3V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono)
                    .with_description("Noise audio output"),
            ],
            latency_samples: 0,
            supports_bypass: false,
        };

        let amplitude_param = ModulatableParameter::new(
            BasicParameter::new("amplitude", 0.0, 1.0, 0.5),
            0.8  // 80% CV modulation range
        );

        Self {
            node_info,
            noise_type: 0.0, // White noise default
            amplitude: 0.5,
            active: 1.0,
            amplitude_param,
            rng_state: 1, // Non-zero seed
            pink_state: [0.0; 7],
            brown_state: 0.0,
            blue_state: 0.0,
            sample_rate,
        }
    }

    /// Simple linear congruential generator for deterministic noise
    fn next_random(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.rng_state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Generate white noise sample
    fn generate_white_noise(&mut self) -> f32 {
        self.next_random()
    }

    /// Generate pink noise using Paul Kellet's algorithm
    fn generate_pink_noise(&mut self) -> f32 {
        let white = self.next_random();
        
        // Paul Kellet's pink noise algorithm - industry standard
        self.pink_state[0] = 0.99886 * self.pink_state[0] + white * 0.0555179;
        self.pink_state[1] = 0.99332 * self.pink_state[1] + white * 0.0750759;
        self.pink_state[2] = 0.96900 * self.pink_state[2] + white * 0.1538520;
        self.pink_state[3] = 0.86650 * self.pink_state[3] + white * 0.3104856;
        self.pink_state[4] = 0.55000 * self.pink_state[4] + white * 0.5329522;
        self.pink_state[5] = -0.7616 * self.pink_state[5] - white * 0.0168980;
        
        let pink = self.pink_state[0] + self.pink_state[1] + self.pink_state[2] + 
                   self.pink_state[3] + self.pink_state[4] + self.pink_state[5] + 
                   self.pink_state[6] + white * 0.5362;
        
        self.pink_state[6] = white * 0.115926;
        
        pink * 0.5 // Normalize to reasonable amplitude
    }

    /// Generate brown noise (Brownian motion)
    fn generate_brown_noise(&mut self) -> f32 {
        let white = self.next_random();
        self.brown_state = (self.brown_state + white * 0.1).clamp(-1.0, 1.0);
        self.brown_state
    }

    /// Generate blue noise (high frequency emphasis)
    fn generate_blue_noise(&mut self) -> f32 {
        let white = self.next_random();
        let blue = white - self.blue_state * 0.8;
        self.blue_state = white;
        blue * 0.8 // Reasonable amplitude
    }

    /// Generate noise sample based on type
    fn generate_noise_sample(&mut self, noise_type: NoiseType) -> f32 {
        match noise_type {
            NoiseType::White => self.generate_white_noise(),
            NoiseType::Pink => self.generate_pink_noise(),
            NoiseType::Brown => self.generate_brown_noise(),
            NoiseType::Blue => self.generate_blue_noise(),
        }
    }
}

impl Parameterizable for NoiseNodeRefactored {
    define_parameters! {
        noise_type: BasicParameter::new("noise_type", 0.0, 3.0, 0.0),
        amplitude: BasicParameter::new("amplitude", 0.0, 1.0, 0.5),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for NoiseNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - output silence
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                output.fill(0.0);
            }
            return Ok(());
        }

        // Get CV inputs
        let amplitude_cv = ctx.inputs.get_cv_value("amplitude_cv");
        let type_cv = ctx.inputs.get_cv_value("type_cv");

        // Apply CV modulation
        let effective_amplitude = self.amplitude_param.modulate(self.amplitude, amplitude_cv);
        
        // Update noise type from CV if provided
        let current_noise_type = if type_cv != 0.0 {
            NoiseType::from_f32(type_cv.clamp(0.0, 3.0))
        } else {
            NoiseType::from_f32(self.noise_type)
        };

        // Process audio output
        let output = ctx.outputs.get_audio_mut("audio_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "audio_out".to_string() 
            })?;

        // Generate noise samples
        for sample in output.iter_mut() {
            let noise_sample = self.generate_noise_sample(current_noise_type);
            *sample = noise_sample * effective_amplitude;
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset noise generation state
        self.rng_state = 1;
        self.pink_state = [0.0; 7];
        self.brown_state = 0.0;
        self.blue_state = 0.0;
    }

    fn latency(&self) -> u32 {
        0 // No latency for noise generator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_noise_parameters() {
        let mut noise = NoiseNodeRefactored::new(44100.0, "test".to_string());
        
        // Test noise type setting
        assert!(noise.set_parameter("noise_type", 1.0).is_ok()); // Pink
        assert_eq!(noise.get_parameter("noise_type").unwrap(), 1.0);
        
        // Test amplitude setting
        assert!(noise.set_parameter("amplitude", 0.75).is_ok());
        assert_eq!(noise.get_parameter("amplitude").unwrap(), 0.75);
        
        // Test validation
        assert!(noise.set_parameter("noise_type", 5.0).is_err()); // Out of range
        assert!(noise.set_parameter("amplitude", -0.1).is_err()); // Out of range
    }

    #[test]
    fn test_noise_generation() {
        let mut noise = NoiseNodeRefactored::new(44100.0, "test".to_string());
        
        let inputs = InputBuffers::new();
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
        
        assert!(noise.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // Should generate noise (not silent)
        let has_signal = output.iter().any(|&s| s.abs() > 0.001);
        assert!(has_signal);
        
        // Should have reasonable amplitude distribution
        let max_val = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        assert!(max_val > 0.1);
        assert!(min_val < -0.1);
    }

    #[test]
    fn test_different_noise_types() {
        let mut noise = NoiseNodeRefactored::new(44100.0, "test".to_string());
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 1024);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 1024,
            timestamp: 0,
            bpm: 120.0,
        };
        
        // Test each noise type
        for noise_type in 0..4 {
            noise.set_parameter("noise_type", noise_type as f32).unwrap();
            ctx.outputs.clear_audio("audio_out");
            
            assert!(noise.process(&mut ctx).is_ok());
            
            let output = ctx.outputs.get_audio("audio_out").unwrap();
            let has_signal = output.iter().any(|&s| s.abs() > 0.001);
            assert!(has_signal, "Noise type {} should generate signal", noise_type);
            
            // Different noise types should have different characteristics
            let variance: f32 = output.iter()
                .map(|&x| x * x)
                .sum::<f32>() / output.len() as f32;
            assert!(variance > 0.01, "Noise type {} should have reasonable variance", noise_type);
        }
    }

    #[test]
    fn test_amplitude_cv_modulation() {
        let mut noise = NoiseNodeRefactored::new(44100.0, "test".to_string());
        
        let mut inputs = InputBuffers::new();
        inputs.add_cv("amplitude_cv".to_string(), vec![0.5]); // Boost amplitude
        
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
        
        assert!(noise.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let rms = (output.iter().map(|&x| x * x).sum::<f32>() / output.len() as f32).sqrt();
        
        // Should have modulated amplitude
        assert!(rms > 0.2, "RMS amplitude should be increased by CV: {}", rms);
    }

    #[test]
    fn test_type_cv_modulation() {
        let mut noise = NoiseNodeRefactored::new(44100.0, "test".to_string());
        
        let mut inputs = InputBuffers::new();
        inputs.add_cv("type_cv".to_string(), vec![2.0]); // Brown noise
        
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
        
        assert!(noise.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_signal = output.iter().any(|&s| s.abs() > 0.001);
        assert!(has_signal, "Type CV should control noise generation");
    }

    #[test]
    fn test_deterministic_generation() {
        let mut noise1 = NoiseNodeRefactored::new(44100.0, "test1".to_string());
        let mut noise2 = NoiseNodeRefactored::new(44100.0, "test2".to_string());
        
        // Reset both to same state
        noise1.reset();
        noise2.reset();
        
        let inputs = InputBuffers::new();
        let mut outputs1 = OutputBuffers::new();
        let mut outputs2 = OutputBuffers::new();
        outputs1.allocate_audio("audio_out".to_string(), 64);
        outputs2.allocate_audio("audio_out".to_string(), 64);
        
        let mut ctx1 = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs1,
            sample_rate: 44100.0,
            buffer_size: 64,
            timestamp: 0,
            bpm: 120.0,
        };
        
        let mut ctx2 = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs2,
            sample_rate: 44100.0,
            buffer_size: 64,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(noise1.process(&mut ctx1).is_ok());
        assert!(noise2.process(&mut ctx2).is_ok());
        
        let output1 = ctx1.outputs.get_audio("audio_out").unwrap();
        let output2 = ctx2.outputs.get_audio("audio_out").unwrap();
        
        // Should generate identical sequences (deterministic)
        for (i, (&s1, &s2)) in output1.iter().zip(output2.iter()).enumerate() {
            assert!((s1 - s2).abs() < 0.0001, "Sample {} should be identical: {} vs {}", i, s1, s2);
        }
    }

    #[test]
    fn test_pink_noise_characteristics() {
        let mut noise = NoiseNodeRefactored::new(44100.0, "test".to_string());
        noise.set_parameter("noise_type", 1.0).unwrap(); // Pink noise
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 4096); // Larger buffer for analysis
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4096,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(noise.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // Pink noise should have specific spectral characteristics
        // Basic test: should have energy across frequency spectrum
        let energy: f32 = output.iter().map(|&x| x * x).sum();
        assert!(energy > 1.0, "Pink noise should have substantial energy");
        
        // Test that pink noise filter is working (state should be modified)
        let has_low_freq_content = output.windows(2).any(|w| (w[0] - w[1]).abs() < 0.1);
        assert!(has_low_freq_content, "Pink noise should have low frequency content");
    }

    #[test]
    fn test_inactive_state() {
        let mut noise = NoiseNodeRefactored::new(44100.0, "test".to_string());
        noise.set_parameter("active", 0.0).unwrap(); // Disable
        
        let inputs = InputBuffers::new();
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
        
        assert!(noise.process(&mut ctx).is_ok());
        
        // Should output silence when inactive
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let is_silent = output.iter().all(|&s| s.abs() < 0.001);
        assert!(is_silent);
    }
}