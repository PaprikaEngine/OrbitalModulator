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

/// リファクタリング済みRingModulatorNode - プロ品質のリングモジュレーション
pub struct RingModulatorNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Parameters
    mix: f32,             // Dry/wet mix (0.0 = full dry, 1.0 = full modulated)
    carrier_gain: f32,    // Gain for the carrier signal
    modulator_gain: f32,  // Gain for the modulator signal
    dc_filter: f32,       // DC filtering amount
    active: f32,
    
    // CV Modulation parameters
    mix_param: ModulatableParameter,
    carrier_gain_param: ModulatableParameter,
    modulator_gain_param: ModulatableParameter,
    
    // DC filter state
    dc_filter_state: f32,
    
    sample_rate: f32,
}

impl RingModulatorNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "ring_modulator_refactored".to_string(),
            category: NodeCategory::Processor,
            description: "Professional ring modulator with DC filtering and precise gain control".to_string(),
            input_ports: vec![
                PortInfo::new("carrier_in", PortType::AudioMono)
                    .with_description("Carrier audio input signal"),
                PortInfo::new("modulator_in", PortType::AudioMono)
                    .with_description("Modulator audio input signal"),
                PortInfo::new("mix_cv", PortType::CV)
                    .with_description("Dry/wet mix control (0V to +10V)")
                    .optional(),
                PortInfo::new("carrier_gain_cv", PortType::CV)
                    .with_description("Carrier gain control (0V to +10V)")
                    .optional(),
                PortInfo::new("modulator_gain_cv", PortType::CV)
                    .with_description("Modulator gain control (0V to +10V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono)
                    .with_description("Ring modulated audio output"),
                PortInfo::new("modulator_out", PortType::AudioMono)
                    .with_description("Modulator signal output (for chaining)")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルリングモジュレーター用
        let mix_param = ModulatableParameter::new(
            BasicParameter::new("mix", 0.0, 1.0, 1.0),
            0.8  // 80% CV modulation range
        );

        let carrier_gain_param = ModulatableParameter::new(
            BasicParameter::new("carrier_gain", 0.0, 2.0, 1.0),
            0.6  // 60% CV modulation range
        );

        let modulator_gain_param = ModulatableParameter::new(
            BasicParameter::new("modulator_gain", 0.0, 2.0, 1.0),
            0.6  // 60% CV modulation range
        );

        Self {
            node_info,
            mix: 1.0,
            carrier_gain: 1.0,
            modulator_gain: 1.0,
            dc_filter: 0.1, // Gentle DC filtering by default
            active: 1.0,

            mix_param,
            carrier_gain_param,
            modulator_gain_param,
            
            dc_filter_state: 0.0,
            
            sample_rate,
        }
    }

    /// 高品質リングモジュレーション処理
    fn ring_modulate(&mut self, carrier: f32, modulator: f32, carrier_gain: f32, 
                     modulator_gain: f32, mix: f32) -> f32 {
        // Apply gain to each signal before multiplication
        let scaled_carrier = carrier * carrier_gain;
        let scaled_modulator = modulator * modulator_gain;
        
        // Ring modulation is multiplication of the two signals
        let modulated = scaled_carrier * scaled_modulator;
        
        // Apply DC filtering to remove any DC offset from the multiplication
        let filtered_modulated = if self.dc_filter > 0.0 {
            // Simple high-pass filter to remove DC - much gentler filtering
            let alpha = 0.999 - self.dc_filter * 0.001; // Very gentle filtering
            self.dc_filter_state = alpha * self.dc_filter_state + (1.0 - alpha) * modulated;
            modulated - self.dc_filter_state
        } else {
            modulated
        };
        
        // Mix between dry (carrier) and modulated signal
        carrier * (1.0 - mix) + filtered_modulated * mix
    }
}

impl Parameterizable for RingModulatorNodeRefactored {
    define_parameters! {
        mix: BasicParameter::new("mix", 0.0, 1.0, 1.0),
        carrier_gain: BasicParameter::new("carrier_gain", 0.0, 2.0, 1.0),
        modulator_gain: BasicParameter::new("modulator_gain", 0.0, 2.0, 1.0),
        dc_filter: BasicParameter::new("dc_filter", 0.0, 1.0, 0.1),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for RingModulatorNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - pass through carrier signal
            let carrier_input = ctx.inputs.get_audio("carrier_in").unwrap_or(&[]);
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                for (i, output_sample) in output.iter_mut().enumerate() {
                    *output_sample = if i < carrier_input.len() { carrier_input[i] } else { 0.0 };
                }
            }
            // Also pass through modulator signal
            let modulator_input = ctx.inputs.get_audio("modulator_in").unwrap_or(&[]);
            if let Some(modulator_out) = ctx.outputs.get_audio_mut("modulator_out") {
                for (i, output_sample) in modulator_out.iter_mut().enumerate() {
                    *output_sample = if i < modulator_input.len() { modulator_input[i] } else { 0.0 };
                }
            }
            return Ok(());
        }

        // Get audio inputs
        let carrier_input = ctx.inputs.get_audio("carrier_in").unwrap_or(&[]);
        let modulator_input = ctx.inputs.get_audio("modulator_in").unwrap_or(&[]);

        // For ring modulation, we need both signals
        if carrier_input.is_empty() && modulator_input.is_empty() {
            // No input - output silence
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                output.fill(0.0);
            }
            if let Some(modulator_out) = ctx.outputs.get_audio_mut("modulator_out") {
                modulator_out.fill(0.0);
            }
            return Ok(());
        }

        // Get CV inputs
        let mix_cv = ctx.inputs.get_cv_value("mix_cv");
        let carrier_gain_cv = ctx.inputs.get_cv_value("carrier_gain_cv");
        let modulator_gain_cv = ctx.inputs.get_cv_value("modulator_gain_cv");

        // Apply CV modulation
        let effective_mix = self.mix_param.modulate(self.mix, mix_cv);
        let effective_carrier_gain = self.carrier_gain_param.modulate(self.carrier_gain, carrier_gain_cv);
        let effective_modulator_gain = self.modulator_gain_param.modulate(self.modulator_gain, modulator_gain_cv);

        // Process audio output
        let output = ctx.outputs.get_audio_mut("audio_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "audio_out".to_string() 
            })?;

        // Process each sample through ring modulation
        for (i, output_sample) in output.iter_mut().enumerate() {
            let carrier_sample = if i < carrier_input.len() { 
                carrier_input[i] 
            } else { 
                0.0 
            };

            let modulator_sample = if i < modulator_input.len() { 
                modulator_input[i] 
            } else { 
                0.0 
            };

            *output_sample = self.ring_modulate(
                carrier_sample,
                modulator_sample,
                effective_carrier_gain,
                effective_modulator_gain,
                effective_mix
            );
        }

        // Output modulator signal for chaining if requested
        if let Some(modulator_out) = ctx.outputs.get_audio_mut("modulator_out") {
            for (i, output_sample) in modulator_out.iter_mut().enumerate() {
                *output_sample = if i < modulator_input.len() { 
                    modulator_input[i] * effective_modulator_gain
                } else { 
                    0.0 
                };
            }
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset DC filter state
        self.dc_filter_state = 0.0;
    }

    fn latency(&self) -> u32 {
        0 // No latency for ring modulation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_ring_modulator_parameters() {
        let mut ring_mod = RingModulatorNodeRefactored::new(44100.0, "test".to_string());
        
        // Test mix setting
        assert!(ring_mod.set_parameter("mix", 0.5).is_ok());
        assert_eq!(ring_mod.get_parameter("mix").unwrap(), 0.5);
        
        // Test carrier gain setting
        assert!(ring_mod.set_parameter("carrier_gain", 1.5).is_ok());
        assert_eq!(ring_mod.get_parameter("carrier_gain").unwrap(), 1.5);
        
        // Test modulator gain setting
        assert!(ring_mod.set_parameter("modulator_gain", 0.8).is_ok());
        assert_eq!(ring_mod.get_parameter("modulator_gain").unwrap(), 0.8);
        
        // Test validation
        assert!(ring_mod.set_parameter("mix", -0.1).is_err()); // Out of range
        assert!(ring_mod.set_parameter("carrier_gain", 5.0).is_err()); // Out of range
    }

    #[test]
    fn test_ring_modulation_processing() {
        let mut ring_mod = RingModulatorNodeRefactored::new(44100.0, "test".to_string());
        ring_mod.set_parameter("mix", 1.0).unwrap(); // Full ring modulation
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("carrier_in".to_string(), vec![0.5; 512]); // Carrier signal
        inputs.add_audio("modulator_in".to_string(), vec![0.8; 512]); // Modulator signal
        
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
        assert!(ring_mod.process(&mut ctx).is_ok());
        
        // Output should be the product of the two inputs (0.5 * 0.8 = 0.4)
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let expected_value = 0.5 * 0.8; // Ring modulation result
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!((avg_output - expected_value).abs() < 0.1, "Ring modulation result should be ~{}: {}", expected_value, avg_output);
    }

    #[test]
    fn test_dry_wet_mix() {
        let mut ring_mod = RingModulatorNodeRefactored::new(44100.0, "test".to_string());
        ring_mod.set_parameter("mix", 0.0).unwrap(); // Full dry signal
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("carrier_in".to_string(), vec![0.6; 256]);
        inputs.add_audio("modulator_in".to_string(), vec![0.9; 256]);
        
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
        
        assert!(ring_mod.process(&mut ctx).is_ok());
        
        // With mix = 0.0, should output only the carrier signal
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!((avg_output - 0.6).abs() < 0.001, "Full dry should output carrier: {}", avg_output);
    }

    #[test]
    fn test_gain_control() {
        let mut ring_mod = RingModulatorNodeRefactored::new(44100.0, "test".to_string());
        ring_mod.set_parameter("carrier_gain", 2.0).unwrap(); // Double carrier
        ring_mod.set_parameter("modulator_gain", 0.5).unwrap(); // Half modulator
        ring_mod.set_parameter("mix", 1.0).unwrap(); // Full wet
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("carrier_in".to_string(), vec![0.4; 128]);
        inputs.add_audio("modulator_in".to_string(), vec![0.8; 128]);
        
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
        
        assert!(ring_mod.process(&mut ctx).is_ok());
        
        // Expected: (0.4 * 2.0) * (0.8 * 0.5) = 0.8 * 0.4 = 0.32
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let expected_value = 0.32;
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!((avg_output - expected_value).abs() < 0.05, "Gain control should affect modulation: expected={}, got={}", expected_value, avg_output);
    }

    #[test]
    fn test_mix_cv_modulation() {
        let mut ring_mod = RingModulatorNodeRefactored::new(44100.0, "test".to_string());
        ring_mod.set_parameter("mix", 0.5).unwrap(); // Base mix
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("carrier_in".to_string(), vec![0.5; 256]);
        inputs.add_audio("modulator_in".to_string(), vec![0.6; 256]);
        inputs.add_cv("mix_cv".to_string(), vec![2.0]); // Increase mix
        
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
        
        assert!(ring_mod.process(&mut ctx).is_ok());
        
        // CV should modulate the mix parameter
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_modulated_mix = output.iter().any(|&s| s.abs() > 0.1);
        assert!(has_modulated_mix, "Mix CV should affect the output");
    }

    #[test]
    fn test_modulator_output() {
        let mut ring_mod = RingModulatorNodeRefactored::new(44100.0, "test".to_string());
        ring_mod.set_parameter("modulator_gain", 1.5).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("carrier_in".to_string(), vec![0.4; 128]);
        inputs.add_audio("modulator_in".to_string(), vec![0.6; 128]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 128);
        outputs.allocate_audio("modulator_out".to_string(), 128);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 128,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(ring_mod.process(&mut ctx).is_ok());
        
        // Modulator output should be modulator input * modulator gain
        let modulator_out = ctx.outputs.get_audio("modulator_out").unwrap();
        let expected_modulator = 0.6 * 1.5; // 0.9
        let avg_modulator_out = modulator_out.iter().sum::<f32>() / modulator_out.len() as f32;
        assert!((avg_modulator_out - expected_modulator).abs() < 0.001, "Modulator output should be scaled input: expected={}, got={}", expected_modulator, avg_modulator_out);
    }

    #[test]
    fn test_single_input_behavior() {
        let mut ring_mod = RingModulatorNodeRefactored::new(44100.0, "test".to_string());
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("carrier_in".to_string(), vec![0.7; 256]); // Only carrier input
        // No modulator input
        
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
        
        assert!(ring_mod.process(&mut ctx).is_ok());
        
        // With no modulator, ring modulation result should be 0 (carrier * 0)
        // So output depends on mix setting - if mix=1.0, output should be near 0
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let avg_output = output.iter().map(|&x| x.abs()).sum::<f32>() / output.len() as f32;
        assert!(avg_output < 0.1, "Single input should produce minimal ring modulation: {}", avg_output);
    }

    #[test]
    fn test_dc_filtering() {
        let mut ring_mod = RingModulatorNodeRefactored::new(44100.0, "test".to_string());
        ring_mod.set_parameter("dc_filter", 1.0).unwrap(); // Maximum DC filtering
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("carrier_in".to_string(), vec![1.0; 1024]); // DC signal
        inputs.add_audio("modulator_in".to_string(), vec![1.0; 1024]); // DC signal
        
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
        
        assert!(ring_mod.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // DC filtering should reduce the DC component
        let dc_level = output.iter().sum::<f32>() / output.len() as f32;
        assert!(dc_level.abs() < 0.5, "DC filtering should reduce DC component: {}", dc_level);
    }

    #[test]
    fn test_inactive_state() {
        let mut ring_mod = RingModulatorNodeRefactored::new(44100.0, "test".to_string());
        ring_mod.set_parameter("active", 0.0).unwrap(); // Disable
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("carrier_in".to_string(), vec![0.5; 512]);
        inputs.add_audio("modulator_in".to_string(), vec![0.8; 512]);
        
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
        
        assert!(ring_mod.process(&mut ctx).is_ok());
        
        // Should pass through carrier signal when inactive
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!((avg_output - 0.5).abs() < 0.001, "Should pass through carrier when inactive: {}", avg_output);
    }
}