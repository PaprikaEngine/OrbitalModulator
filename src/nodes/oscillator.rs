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
pub enum WaveformType {
    Sine = 0,
    Triangle = 1,
    Sawtooth = 2,
    Pulse = 3,
}

impl WaveformType {
    pub fn from_f32(value: f32) -> Self {
        match value as i32 {
            0 => WaveformType::Sine,
            1 => WaveformType::Triangle,
            2 => WaveformType::Sawtooth,
            3 => WaveformType::Pulse,
            _ => WaveformType::Sine,
        }
    }
}

/// リファクタリング済みOscillatorNode
pub struct OscillatorNode {
    // Node identification
    node_info: NodeInfo,
    
    // Parameters - 新しいパラメーターシステム使用
    frequency: f32,
    amplitude: f32,
    waveform: f32,  // WaveformTypeのf32表現
    pulse_width: f32,
    active: f32,
    
    // CV Modulation parameters
    frequency_param: ModulatableParameter,
    amplitude_param: ModulatableParameter,
    pulse_width_param: ModulatableParameter,
    
    // Internal state
    phase: f32,
    sample_rate: f32,
}

impl OscillatorNode {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "oscillator".to_string(),
            category: NodeCategory::Generator,
            description: "Multi-waveform voltage controlled oscillator with CV modulation".to_string(),
            input_ports: vec![
                PortInfo::new("frequency_cv", PortType::CV)
                    .with_description("1V/Oct frequency control"),
                PortInfo::new("amplitude_cv", PortType::CV)
                    .with_description("Amplitude modulation"),
                PortInfo::new("waveform_cv", PortType::CV)
                    .with_description("Waveform selection")
                    .optional(),
                PortInfo::new("pulse_width_cv", PortType::CV)
                    .with_description("Pulse width modulation (for pulse wave)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono)
                    .with_description("Audio output signal"),
            ],
            latency_samples: 0,
            supports_bypass: false,
        };

        // パラメーター記述子の定義
        let frequency_param = ModulatableParameter::new(
            BasicParameter::new("frequency", 20.0, 20000.0, 440.0).with_unit("Hz"),
            1.0  // 100% CV modulation
        ).with_curve(ModulationCurve::Exponential); // 周波数は指数的変化

        let amplitude_param = ModulatableParameter::new(
            BasicParameter::new("amplitude", 0.0, 1.0, 0.5),
            0.5  // 50% CV modulation
        );

        let pulse_width_param = ModulatableParameter::new(
            BasicParameter::new("pulse_width", 0.1, 0.9, 0.5),
            0.4  // 40% CV modulation
        );

        Self {
            node_info,
            frequency: 440.0,
            amplitude: 0.5,
            waveform: 0.0, // Sine wave default
            pulse_width: 0.5,
            active: 1.0,
            frequency_param,
            amplitude_param,
            pulse_width_param,
            phase: 0.0,
            sample_rate,
        }
    }

    fn generate_sample(&self, phase: f32) -> f32 {
        let waveform_type = WaveformType::from_f32(self.waveform);
        
        match waveform_type {
            WaveformType::Sine => phase.sin(),
            WaveformType::Triangle => {
                let normalized_phase = phase / (2.0 * std::f32::consts::PI);
                if normalized_phase < 0.5 {
                    4.0 * normalized_phase - 1.0
                } else {
                    3.0 - 4.0 * normalized_phase
                }
            },
            WaveformType::Sawtooth => {
                let normalized_phase = phase / (2.0 * std::f32::consts::PI);
                2.0 * normalized_phase - 1.0
            },
            WaveformType::Pulse => {
                let normalized_phase = phase / (2.0 * std::f32::consts::PI);
                if normalized_phase < self.pulse_width {
                    1.0
                } else {
                    -1.0
                }
            },
        }
    }

    fn advance_phase(&mut self, frequency: f32, samples: usize) {
        let phase_increment = 2.0 * std::f32::consts::PI * frequency / self.sample_rate;
        self.phase += phase_increment * samples as f32;
        
        // Wrap phase to prevent accumulation errors
        while self.phase >= 2.0 * std::f32::consts::PI {
            self.phase -= 2.0 * std::f32::consts::PI;
        }
    }
}

impl Parameterizable for OscillatorNode {
    define_parameters! {
        frequency: BasicParameter::new("frequency", 20.0, 20000.0, 440.0).with_unit("Hz"),
        amplitude: BasicParameter::new("amplitude", 0.0, 1.0, 0.5),
        waveform: BasicParameter::new("waveform", 0.0, 3.0, 0.0),
        pulse_width: BasicParameter::new("pulse_width", 0.1, 0.9, 0.5),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for OscillatorNode {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - output silence
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                output.fill(0.0);
            }
            return Ok(());
        }

        // Get CV inputs
        let frequency_cv = ctx.inputs.get_cv_value("frequency_cv");
        let amplitude_cv = ctx.inputs.get_cv_value("amplitude_cv");
        let waveform_cv = ctx.inputs.get_cv_value("waveform_cv");
        let pulse_width_cv = ctx.inputs.get_cv_value("pulse_width_cv");

        // Apply CV modulation
        let effective_frequency = self.frequency_param.modulate(self.frequency, frequency_cv);
        let effective_amplitude = self.amplitude_param.modulate(self.amplitude, amplitude_cv);
        let effective_pulse_width = self.pulse_width_param.modulate(self.pulse_width, pulse_width_cv);

        // Update waveform from CV if provided
        if waveform_cv != 0.0 {
            self.waveform = (waveform_cv * 4.0).clamp(0.0, 3.0);
        }

        // Update pulse width for current processing
        let original_pulse_width = self.pulse_width;
        self.pulse_width = effective_pulse_width;

        // Process audio output
        let output = ctx.outputs.get_audio_mut("audio_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "audio_out".to_string() 
            })?;

        for (i, sample) in output.iter_mut().enumerate() {
            let phase_increment = 2.0 * std::f32::consts::PI * effective_frequency / ctx.sample_rate;
            let current_phase = self.phase + phase_increment * i as f32;
            *sample = self.generate_sample(current_phase) * effective_amplitude;
        }

        // Advance phase for next buffer
        self.advance_phase(effective_frequency, ctx.buffer_size);

        // Restore original pulse width
        self.pulse_width = original_pulse_width;

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn latency(&self) -> u32 {
        0 // No latency for oscillator
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
    fn test_oscillator_parameters() {
        let mut osc = OscillatorNode::new(44100.0, "test".to_string());
        
        // Test parameter setting
        assert!(osc.set_parameter("frequency", 880.0).is_ok());
        assert_eq!(osc.get_parameter("frequency").unwrap(), 880.0);
        
        // Test out of range (should fail with validation)
        assert!(osc.set_parameter("frequency", -100.0).is_err());
        assert!(osc.set_parameter("amplitude", 2.0).is_err());
    }

    #[test]
    fn test_oscillator_processing() {
        let mut osc = OscillatorNode::new(44100.0, "test".to_string());
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        // Should process without error
        assert!(osc.process(&mut ctx).is_ok());
        
        // Output should not be silent when active
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_signal = output.iter().any(|&s| s.abs() > 0.001);
        assert!(has_signal);
    }

    #[test]
    fn test_cv_modulation() {
        let mut osc = OscillatorNode::new(44100.0, "test".to_string());
        
        let mut inputs = InputBuffers::new();
        inputs.add_cv("frequency_cv".to_string(), vec![1.0]); // +1V should double frequency
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(osc.process(&mut ctx).is_ok());
        // With exponential CV curve, +1V should significantly increase frequency
    }

    #[test]
    fn test_waveform_generation() {
        let osc = OscillatorNode::new(44100.0, "test".to_string());
        
        // Test different waveforms
        let phase = std::f32::consts::PI / 2.0; // 90 degrees
        
        // Sine wave at 90° should be ~1.0
        assert!((osc.generate_sample(phase) - 1.0).abs() < 0.001);
        
        // Test that different waveforms produce different outputs
        let sine_val = osc.generate_sample(0.0);
        let mut osc_tri = osc;
        osc_tri.waveform = 1.0; // Triangle
        let tri_val = osc_tri.generate_sample(0.0);
        
        // At phase 0, sine should be 0, triangle should be -1
        assert!((sine_val - 0.0).abs() < 0.001);
        assert!((tri_val - (-1.0)).abs() < 0.001);
    }
}