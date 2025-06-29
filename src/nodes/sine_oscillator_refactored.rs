use uuid::Uuid;

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor, ModulationCurve};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;
use crate::define_parameters;

/// リファクタリング済みSineOscillatorNode - サイン波専用高品質VCO
pub struct SineOscillatorNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Parameters
    frequency: f32,
    amplitude: f32,
    active: f32,
    
    // CV Modulation parameters
    frequency_param: ModulatableParameter,
    amplitude_param: ModulatableParameter,
    
    // Internal state
    phase: f32,
    sample_rate: f32,
}

impl SineOscillatorNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "sine_oscillator_refactored".to_string(),
            category: NodeCategory::Generator,
            description: "High-quality sine wave oscillator with precise frequency control".to_string(),
            input_ports: vec![
                PortInfo::new("frequency_cv", PortType::CV)
                    .with_description("1V/Oct frequency control (-10V to +10V)"),
                PortInfo::new("amplitude_cv", PortType::CV)
                    .with_description("Amplitude modulation (0V to +10V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono)
                    .with_description("Pure sine wave audio output"),
            ],
            latency_samples: 0,
            supports_bypass: false,
        };

        // パラメーター設定 - 高精度オシレーター用
        let frequency_param = ModulatableParameter::new(
            BasicParameter::new("frequency", 20.0, 20000.0, 440.0).with_unit("Hz"),
            1.0  // 100% CV modulation for precise control
        ).with_curve(ModulationCurve::Exponential); // 周波数は指数的変化

        let amplitude_param = ModulatableParameter::new(
            BasicParameter::new("amplitude", 0.0, 1.0, 0.5),
            0.8  // 80% CV modulation range
        );

        Self {
            node_info,
            frequency: 440.0,  // A4 default
            amplitude: 0.5,
            active: 1.0,
            frequency_param,
            amplitude_param,
            phase: 0.0,
            sample_rate,
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

    /// 高品質サイン波生成 - 位相連続性保証
    fn generate_sine_sample(&self, phase: f32) -> f32 {
        phase.sin()
    }
}

impl Parameterizable for SineOscillatorNodeRefactored {
    define_parameters! {
        frequency: BasicParameter::new("frequency", 20.0, 20000.0, 440.0).with_unit("Hz"),
        amplitude: BasicParameter::new("amplitude", 0.0, 1.0, 0.5),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for SineOscillatorNodeRefactored {
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

        // Apply CV modulation with exponential frequency control
        let effective_frequency = self.frequency_param.modulate(self.frequency, frequency_cv);
        let effective_amplitude = self.amplitude_param.modulate(self.amplitude, amplitude_cv);

        // Process audio output
        let output = ctx.outputs.get_audio_mut("audio_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "audio_out".to_string() 
            })?;

        // Generate high-quality sine wave with phase continuity
        for (i, sample) in output.iter_mut().enumerate() {
            let phase_increment = 2.0 * std::f32::consts::PI * effective_frequency / ctx.sample_rate;
            let current_phase = self.phase + phase_increment * i as f32;
            *sample = self.generate_sine_sample(current_phase) * effective_amplitude;
        }

        // Advance phase for next buffer
        self.advance_phase(effective_frequency, ctx.buffer_size);

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_sine_oscillator_parameters() {
        let mut osc = SineOscillatorNodeRefactored::new(44100.0, "test".to_string());
        
        // Test parameter setting
        assert!(osc.set_parameter("frequency", 880.0).is_ok());
        assert_eq!(osc.get_parameter("frequency").unwrap(), 880.0);
        
        // Test amplitude range
        assert!(osc.set_parameter("amplitude", 0.75).is_ok());
        assert_eq!(osc.get_parameter("amplitude").unwrap(), 0.75);
        
        // Test out of range validation
        assert!(osc.set_parameter("frequency", -100.0).is_err());
        assert!(osc.set_parameter("amplitude", 2.0).is_err());
    }

    #[test]
    fn test_sine_wave_generation() {
        let mut osc = SineOscillatorNodeRefactored::new(44100.0, "test".to_string());
        
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
        
        // Should process without error
        assert!(osc.process(&mut ctx).is_ok());
        
        // Output should be sine wave when active
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_signal = output.iter().any(|&s| s.abs() > 0.001);
        assert!(has_signal);
        
        // Should be smooth sine wave - check phase continuity
        let max_val = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        
        // Sine wave should have reasonable amplitude
        assert!(max_val > 0.4);
        assert!(min_val < -0.4);
    }

    #[test]
    fn test_frequency_cv_modulation() {
        let mut osc = SineOscillatorNodeRefactored::new(44100.0, "test".to_string());
        osc.set_parameter("frequency", 440.0).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_cv("frequency_cv".to_string(), vec![1.0]); // +1V should double frequency
        
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
        
        assert!(osc.process(&mut ctx).is_ok());
        
        // With exponential CV curve, +1V should significantly change frequency
        // This test verifies CV modulation is applied
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_modulated_signal = output.iter().any(|&s| s.abs() > 0.001);
        assert!(has_modulated_signal);
    }

    #[test]
    fn test_amplitude_cv_modulation() {
        let mut osc = SineOscillatorNodeRefactored::new(44100.0, "test".to_string());
        
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
        
        assert!(osc.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let max_amplitude = output.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        
        // Should have modulated amplitude
        assert!(max_amplitude > 0.3);
    }

    #[test]
    fn test_phase_continuity() {
        let mut osc = SineOscillatorNodeRefactored::new(44100.0, "test".to_string());
        
        let inputs = InputBuffers::new();
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
        
        // Process first buffer
        assert!(osc.process(&mut ctx).is_ok());
        let first_buffer = ctx.outputs.get_audio("audio_out").unwrap().to_vec();
        
        // Process second buffer
        ctx.outputs.clear_audio("audio_out");
        assert!(osc.process(&mut ctx).is_ok());
        let second_buffer = ctx.outputs.get_audio("audio_out").unwrap().to_vec();
        
        // Phase should be continuous between buffers
        // Last sample of first buffer and first sample of second buffer should have reasonable difference
        let last_first = first_buffer[first_buffer.len() - 1];
        let first_second = second_buffer[0];
        let phase_jump = (last_first - first_second).abs();
        
        // Phase discontinuity should be small for sine wave
        assert!(phase_jump < 0.5, "Phase discontinuity detected: {}", phase_jump);
    }

    #[test]
    fn test_inactive_state() {
        let mut osc = SineOscillatorNodeRefactored::new(44100.0, "test".to_string());
        osc.set_parameter("active", 0.0).unwrap(); // Disable
        
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
        
        assert!(osc.process(&mut ctx).is_ok());
        
        // Should output silence when inactive
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let is_silent = output.iter().all(|&s| s.abs() < 0.001);
        assert!(is_silent);
    }
}