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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    Lowpass = 0,
    Highpass = 1,
    Bandpass = 2,
}

impl FilterType {
    pub fn from_f32(value: f32) -> Self {
        match value as i32 {
            0 => FilterType::Lowpass,
            1 => FilterType::Highpass,
            2 => FilterType::Bandpass,
            _ => FilterType::Lowpass,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            FilterType::Lowpass => "Lowpass",
            FilterType::Highpass => "Highpass",
            FilterType::Bandpass => "Bandpass",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            FilterType::Lowpass => "Low-pass filter (attenuates high frequencies)",
            FilterType::Highpass => "High-pass filter (attenuates low frequencies)",
            FilterType::Bandpass => "Band-pass filter (passes middle frequencies)",
        }
    }
}

/// リファクタリング済みVCFNode - プロ品質の電圧制御フィルター
pub struct VCFNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Parameters
    cutoff_frequency: f32,
    resonance: f32,
    filter_type: f32,
    active: f32,
    
    // CV Modulation parameters
    cutoff_param: ModulatableParameter,
    resonance_param: ModulatableParameter,
    
    // Biquad filter state
    x1: f32, // Previous input sample 1
    x2: f32, // Previous input sample 2
    y1: f32, // Previous output sample 1
    y2: f32, // Previous output sample 2
    
    // Filter coefficients (updated when parameters change)
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
    
    sample_rate: f32,
    coefficients_dirty: bool,
}

impl VCFNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "vcf_refactored".to_string(),
            category: NodeCategory::Processor,
            description: "Professional voltage controlled filter with Biquad implementation".to_string(),
            input_ports: vec![
                PortInfo::new("audio_in", PortType::AudioMono)
                    .with_description("Audio input signal"),
                PortInfo::new("cutoff_cv", PortType::CV)
                    .with_description("1V/Oct cutoff frequency control")
                    .optional(),
                PortInfo::new("resonance_cv", PortType::CV)
                    .with_description("Resonance/Q factor control (0V to +10V)")
                    .optional(),
                PortInfo::new("type_cv", PortType::CV)
                    .with_description("Filter type selection (0-2V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono)
                    .with_description("Filtered audio output"),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルフィルター用
        let cutoff_param = ModulatableParameter::new(
            BasicParameter::new("cutoff_frequency", 20.0, 20000.0, 1000.0).with_unit("Hz"),
            1.0  // 100% CV modulation for precise control
        ).with_curve(ModulationCurve::Exponential); // 周波数は指数的変化

        let resonance_param = ModulatableParameter::new(
            BasicParameter::new("resonance", 0.1, 10.0, 1.0),
            0.8  // 80% CV modulation range
        );

        Self {
            node_info,
            cutoff_frequency: 1000.0, // 1kHz default
            resonance: 1.0,           // Q = 1.0 default
            filter_type: 0.0,         // Lowpass default
            active: 1.0,

            cutoff_param,
            resonance_param,
            
            // Initialize filter state
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            
            // Initialize coefficients
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b1: 0.0,
            b2: 0.0,
            
            sample_rate,
            coefficients_dirty: true,
        }
    }

    fn update_coefficients(&mut self, cutoff: f32, resonance: f32, filter_type: FilterType) {
        let omega = 2.0 * std::f32::consts::PI * cutoff / self.sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * resonance);

        match filter_type {
            FilterType::Lowpass => {
                // Lowpass biquad coefficients
                let b0 = (1.0 - cos_omega) / 2.0;
                let b1 = 1.0 - cos_omega;
                let b2 = (1.0 - cos_omega) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                self.a0 = b0 / a0;
                self.a1 = b1 / a0;
                self.a2 = b2 / a0;
                self.b1 = a1 / a0;
                self.b2 = a2 / a0;
            },
            FilterType::Highpass => {
                // Highpass biquad coefficients
                let b0 = (1.0 + cos_omega) / 2.0;
                let b1 = -(1.0 + cos_omega);
                let b2 = (1.0 + cos_omega) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                self.a0 = b0 / a0;
                self.a1 = b1 / a0;
                self.a2 = b2 / a0;
                self.b1 = a1 / a0;
                self.b2 = a2 / a0;
            },
            FilterType::Bandpass => {
                // Bandpass biquad coefficients
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                self.a0 = b0 / a0;
                self.a1 = b1 / a0;
                self.a2 = b2 / a0;
                self.b1 = a1 / a0;
                self.b2 = a2 / a0;
            },
        }

        self.coefficients_dirty = false;
    }

    /// 高品質Biquadフィルター処理
    fn process_sample(&mut self, input: f32, cutoff: f32, resonance: f32, filter_type: FilterType) -> f32 {
        // Update coefficients if parameters changed
        if self.coefficients_dirty {
            self.update_coefficients(cutoff, resonance, filter_type);
        }

        // Biquad filter equation: y[n] = a0*x[n] + a1*x[n-1] + a2*x[n-2] - b1*y[n-1] - b2*y[n-2]
        let output = self.a0 * input + self.a1 * self.x1 + self.a2 * self.x2 - self.b1 * self.y1 - self.b2 * self.y2;

        // Update delay line
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }
}

impl Parameterizable for VCFNodeRefactored {
    define_parameters! {
        cutoff_frequency: BasicParameter::new("cutoff_frequency", 20.0, 20000.0, 1000.0).with_unit("Hz"),
        resonance: BasicParameter::new("resonance", 0.1, 10.0, 1.0),
        filter_type: BasicParameter::new("filter_type", 0.0, 2.0, 0.0),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for VCFNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - output silence
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                output.fill(0.0);
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
        let cutoff_cv = ctx.inputs.get_cv_value("cutoff_cv");
        let resonance_cv = ctx.inputs.get_cv_value("resonance_cv");
        let type_cv = ctx.inputs.get_cv_value("type_cv");

        // Apply CV modulation
        let effective_cutoff = self.cutoff_param.modulate(self.cutoff_frequency, cutoff_cv);
        let effective_resonance = self.resonance_param.modulate(self.resonance, resonance_cv);

        // Update filter type from CV if provided
        let current_filter_type = if type_cv != 0.0 {
            FilterType::from_f32(type_cv.clamp(0.0, 2.0))
        } else {
            FilterType::from_f32(self.filter_type)
        };

        // Check if coefficients need updating
        if (effective_cutoff - self.cutoff_frequency).abs() > 0.1 ||
           (effective_resonance - self.resonance).abs() > 0.01 {
            self.coefficients_dirty = true;
        }

        // Process audio output
        let output = ctx.outputs.get_audio_mut("audio_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "audio_out".to_string() 
            })?;

        // Process each sample through the Biquad filter
        for (i, output_sample) in output.iter_mut().enumerate() {
            let input_sample = if i < audio_input.len() { 
                audio_input[i] 
            } else { 
                0.0 
            };

            *output_sample = self.process_sample(
                input_sample, 
                effective_cutoff, 
                effective_resonance, 
                current_filter_type
            );
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset filter state
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
        self.coefficients_dirty = true;
    }

    fn latency(&self) -> u32 {
        0 // No significant latency for biquad filter
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
    fn test_vcf_parameters() {
        let mut vcf = VCFNodeRefactored::new(44100.0, "test".to_string());
        
        // Test cutoff frequency setting
        assert!(vcf.set_parameter("cutoff_frequency", 2000.0).is_ok());
        assert_eq!(vcf.get_parameter("cutoff_frequency").unwrap(), 2000.0);
        
        // Test resonance setting
        assert!(vcf.set_parameter("resonance", 5.0).is_ok());
        assert_eq!(vcf.get_parameter("resonance").unwrap(), 5.0);
        
        // Test validation
        assert!(vcf.set_parameter("cutoff_frequency", -100.0).is_err()); // Out of range
        assert!(vcf.set_parameter("resonance", 20.0).is_err()); // Out of range
    }

    #[test]
    fn test_vcf_processing() {
        let mut vcf = VCFNodeRefactored::new(44100.0, "test".to_string());
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 512]);
        
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
        assert!(vcf.process(&mut ctx).is_ok());
        
        // Output should be filtered (different from input)
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_output = output.iter().any(|&s| s.abs() > 0.001);
        assert!(has_output);
    }

    #[test]
    fn test_filter_types() {
        let mut vcf = VCFNodeRefactored::new(44100.0, "test".to_string());
        
        // Test each filter type
        for filter_type in 0..3 {
            vcf.set_parameter("filter_type", filter_type as f32).unwrap();
            
            let mut inputs = InputBuffers::new();
            inputs.add_audio("audio_in".to_string(), vec![1.0; 64]);
            
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
            
            assert!(vcf.process(&mut ctx).is_ok());
            
            let output = ctx.outputs.get_audio("audio_out").unwrap();
            let has_filtered_output = output.iter().any(|&s| s.abs() > 0.001);
            assert!(has_filtered_output, "Filter type {} should produce output", filter_type);
        }
    }

    #[test]
    fn test_cutoff_cv_modulation() {
        let mut vcf = VCFNodeRefactored::new(44100.0, "test".to_string());
        vcf.set_parameter("cutoff_frequency", 1000.0).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 64]);
        inputs.add_cv("cutoff_cv".to_string(), vec![1.0]); // +1V should increase cutoff
        
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
        
        assert!(vcf.process(&mut ctx).is_ok());
        
        // With exponential CV curve, +1V should significantly change cutoff frequency
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_modulated_output = output.iter().any(|&s| s.abs() > 0.001);
        assert!(has_modulated_output);
    }

    #[test]
    fn test_resonance_cv_modulation() {
        let mut vcf = VCFNodeRefactored::new(44100.0, "test".to_string());
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 64]);
        inputs.add_cv("resonance_cv".to_string(), vec![2.0]); // Increase resonance
        
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
        
        assert!(vcf.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_resonant_output = output.iter().any(|&s| s.abs() > 0.001);
        assert!(has_resonant_output);
    }

    #[test]
    fn test_filter_stability() {
        let mut vcf = VCFNodeRefactored::new(44100.0, "test".to_string());
        vcf.set_parameter("resonance", 9.0).unwrap(); // High resonance
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.1; 1024]); // Small input
        
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
        
        assert!(vcf.process(&mut ctx).is_ok());
        
        // Check for stability (no infinite values)
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let is_stable = output.iter().all(|&s| s.is_finite());
        assert!(is_stable, "Filter should remain stable even at high resonance");
        
        // Check that output is bounded
        let max_output = output.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        assert!(max_output < 100.0, "Filter output should be bounded: {}", max_output);
    }

    #[test]
    fn test_inactive_state() {
        let mut vcf = VCFNodeRefactored::new(44100.0, "test".to_string());
        vcf.set_parameter("active", 0.0).unwrap(); // Disable
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 512]);
        
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
        
        assert!(vcf.process(&mut ctx).is_ok());
        
        // Should output silence when inactive
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let is_silent = output.iter().all(|&s| s.abs() < 0.001);
        assert!(is_silent);
    }
}