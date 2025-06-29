use uuid::Uuid;

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;
use crate::define_parameters;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VCAResponse {
    Linear = 0,
    Exponential = 1,
}

impl VCAResponse {
    pub fn from_f32(value: f32) -> Self {
        match value as i32 {
            0 => VCAResponse::Linear,
            1 => VCAResponse::Exponential,
            _ => VCAResponse::Linear,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            VCAResponse::Linear => "Linear",
            VCAResponse::Exponential => "Exponential",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            VCAResponse::Linear => "Linear response (constant gain per volt)",
            VCAResponse::Exponential => "Exponential response (constant dB per volt)",
        }
    }
}

/// リファクタリング済みVCANode - プロ品質の電圧制御アンプ
pub struct VCANodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Parameters
    gain: f32,
    cv_sensitivity: f32,
    response: f32,
    active: f32,
    
    // CV Modulation parameters
    gain_param: ModulatableParameter,
    cv_sensitivity_param: ModulatableParameter,
    
    sample_rate: f32,
}

impl VCANodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "vca_refactored".to_string(),
            category: NodeCategory::Processor,
            description: "Professional voltage controlled amplifier with linear/exponential response".to_string(),
            input_ports: vec![
                PortInfo::new("audio_in", PortType::AudioMono)
                    .with_description("Audio input signal"),
                PortInfo::new("gain_cv", PortType::CV)
                    .with_description("Gain control voltage (0V to +10V)")
                    .optional(),
                PortInfo::new("cv_cv", PortType::CV)
                    .with_description("CV sensitivity modulation")
                    .optional(),
                PortInfo::new("response_cv", PortType::CV)
                    .with_description("Response curve selection (0-1V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono)
                    .with_description("Amplified audio output"),
                PortInfo::new("gain_cv_out", PortType::CV)
                    .with_description("Current gain level as CV output")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルVCA用
        let gain_param = ModulatableParameter::new(
            BasicParameter::new("gain", 0.0, 2.0, 1.0),
            0.8  // 80% CV modulation range
        );

        let cv_sensitivity_param = ModulatableParameter::new(
            BasicParameter::new("cv_sensitivity", 0.0, 2.0, 1.0),
            0.5  // 50% CV modulation range
        );

        Self {
            node_info,
            gain: 1.0,
            cv_sensitivity: 1.0,
            response: 0.0, // Linear default
            active: 1.0,

            gain_param,
            cv_sensitivity_param,
            
            sample_rate,
        }
    }

    /// Calculate gain factor from CV input based on response curve
    fn calculate_cv_gain(&self, cv_value: f32, cv_sensitivity: f32, response: VCAResponse) -> f32 {
        if cv_value == 0.0 {
            return 1.0;
        }

        let normalized_cv = (cv_value.clamp(0.0, 10.0) / 10.0) * cv_sensitivity;

        match response {
            VCAResponse::Linear => {
                // Linear response: 0V = 0x gain, 10V = 2x gain
                normalized_cv * 2.0
            },
            VCAResponse::Exponential => {
                // Exponential response: more musical (dB per volt)
                // 0V = 0x gain, 5V = 1x gain, 10V = 4x gain
                if normalized_cv <= 0.5 {
                    normalized_cv * 2.0 // 0 to 1
                } else {
                    (normalized_cv - 0.5) * 6.0 + 1.0 // 1 to 4
                }
            },
        }
    }

    /// Process single VCA sample with high-quality amplification
    fn process_vca_sample(&self, audio_sample: f32, gain: f32, cv_gain: f32) -> f32 {
        // Apply both manual gain and CV gain
        let total_gain = gain * cv_gain;
        
        // Soft clipping to prevent harsh distortion at high gains
        let amplified = audio_sample * total_gain;
        
        // Gentle tanh soft clipping
        if amplified.abs() > 1.0 {
            amplified.signum() * amplified.abs().tanh()
        } else {
            amplified
        }
    }
}

impl Parameterizable for VCANodeRefactored {
    define_parameters! {
        gain: BasicParameter::new("gain", 0.0, 2.0, 1.0),
        cv_sensitivity: BasicParameter::new("cv_sensitivity", 0.0, 2.0, 1.0),
        response: BasicParameter::new("response", 0.0, 1.0, 0.0),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for VCANodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - output silence
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                output.fill(0.0);
            }
            ctx.outputs.set_cv_value("gain_cv_out", 0.0);
            return Ok(());
        }

        // Get audio input
        let audio_input = ctx.inputs.get_audio("audio_in").unwrap_or(&[]);
        if audio_input.is_empty() {
            // No input - output silence
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                output.fill(0.0);
            }
            ctx.outputs.set_cv_value("gain_cv_out", 0.0);
            return Ok(());
        }

        // Get CV inputs
        let gain_cv = ctx.inputs.get_cv_value("gain_cv");
        let cv_cv = ctx.inputs.get_cv_value("cv_cv");
        let response_cv = ctx.inputs.get_cv_value("response_cv");

        // Apply CV modulation
        let effective_gain = self.gain_param.modulate(self.gain, gain_cv);
        let effective_cv_sensitivity = self.cv_sensitivity_param.modulate(self.cv_sensitivity, cv_cv);

        // Update response curve from CV if provided
        let current_response = if response_cv != 0.0 {
            VCAResponse::from_f32(response_cv.clamp(0.0, 1.0))
        } else {
            VCAResponse::from_f32(self.response)
        };

        // Calculate CV gain based on response curve
        let cv_gain = self.calculate_cv_gain(gain_cv, effective_cv_sensitivity, current_response);

        // Process audio output
        let output = ctx.outputs.get_audio_mut("audio_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "audio_out".to_string() 
            })?;

        // Process each sample through the VCA
        for (i, output_sample) in output.iter_mut().enumerate() {
            let input_sample = if i < audio_input.len() { 
                audio_input[i] 
            } else { 
                0.0 
            };

            *output_sample = self.process_vca_sample(input_sample, effective_gain, cv_gain);
        }

        // Output current gain level as CV
        let gain_level = (effective_gain * cv_gain).clamp(0.0, 10.0);
        ctx.outputs.set_cv_value("gain_cv_out", gain_level);

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // VCA has no internal state to reset
    }

    fn latency(&self) -> u32 {
        0 // No latency for VCA
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_vca_parameters() {
        let mut vca = VCANodeRefactored::new(44100.0, "test".to_string());
        
        // Test gain setting
        assert!(vca.set_parameter("gain", 1.5).is_ok());
        assert_eq!(vca.get_parameter("gain").unwrap(), 1.5);
        
        // Test CV sensitivity setting
        assert!(vca.set_parameter("cv_sensitivity", 0.8).is_ok());
        assert_eq!(vca.get_parameter("cv_sensitivity").unwrap(), 0.8);
        
        // Test validation
        assert!(vca.set_parameter("gain", -0.5).is_err()); // Out of range
        assert!(vca.set_parameter("cv_sensitivity", 3.0).is_err()); // Out of range
    }

    #[test]
    fn test_vca_processing() {
        let mut vca = VCANodeRefactored::new(44100.0, "test".to_string());
        
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
        assert!(vca.process(&mut ctx).is_ok());
        
        // Output should be amplified input
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_output = output.iter().any(|&s| s.abs() > 0.1);
        assert!(has_output);
        
        // Should approximately match input * gain
        assert!((output[0] - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_gain_cv_modulation() {
        let mut vca = VCANodeRefactored::new(44100.0, "test".to_string());
        vca.set_parameter("gain", 1.0).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 64]);
        inputs.add_cv("gain_cv".to_string(), vec![5.0]); // Mid-range CV
        
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
        
        assert!(vca.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // CV should modulate the gain
        let output_level = output[0].abs();
        assert!(output_level > 0.5, "CV should increase gain: {}", output_level);
    }

    #[test]
    fn test_response_curves() {
        let vca = VCANodeRefactored::new(44100.0, "test".to_string());
        
        // Test linear response
        let linear_gain = vca.calculate_cv_gain(5.0, 1.0, VCAResponse::Linear);
        assert!((linear_gain - 1.0).abs() < 0.1, "Linear response at 5V should be ~1.0: {}", linear_gain);
        
        // Test exponential response
        let exp_gain = vca.calculate_cv_gain(5.0, 1.0, VCAResponse::Exponential);
        assert!((exp_gain - 1.0).abs() < 0.1, "Exponential response at 5V should be ~1.0: {}", exp_gain);
        
        // High CV should give higher gain in exponential mode
        let exp_high = vca.calculate_cv_gain(10.0, 1.0, VCAResponse::Exponential);
        let lin_high = vca.calculate_cv_gain(10.0, 1.0, VCAResponse::Linear);
        assert!(exp_high > lin_high, "Exponential should give higher gain at 10V: {} vs {}", exp_high, lin_high);
    }

    #[test]
    fn test_cv_sensitivity() {
        let mut vca = VCANodeRefactored::new(44100.0, "test".to_string());
        vca.set_parameter("cv_sensitivity", 0.5).unwrap(); // Reduced sensitivity
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 64]);
        inputs.add_cv("gain_cv".to_string(), vec![10.0]); // Max CV
        
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
        
        assert!(vca.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // Reduced sensitivity should limit the gain increase
        let output_level = output[0].abs();
        assert!(output_level < 2.0, "Reduced CV sensitivity should limit gain: {}", output_level);
    }

    #[test]
    fn test_soft_clipping() {
        let vca = VCANodeRefactored::new(44100.0, "test".to_string());
        
        // Test with very high gain that would cause clipping
        let clipped_output = vca.process_vca_sample(1.0, 5.0, 2.0); // Total gain = 10x
        
        // Should not exceed reasonable bounds due to soft clipping
        assert!(clipped_output.abs() <= 1.5, "Should apply soft clipping: {}", clipped_output);
        assert!(clipped_output > 0.0, "Should still be positive");
    }

    #[test]
    fn test_gain_cv_output() {
        let mut vca = VCANodeRefactored::new(44100.0, "test".to_string());
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 64]);
        inputs.add_cv("gain_cv".to_string(), vec![8.0]); // High CV
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 64);
        outputs.allocate_cv("gain_cv_out".to_string(), 64);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 64,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(vca.process(&mut ctx).is_ok());
        
        // Should output current gain level as CV
        let gain_cv_out = ctx.outputs.get_cv("gain_cv_out").unwrap();
        assert!(gain_cv_out[0] > 1.0, "Should output gain level as CV: {}", gain_cv_out[0]);
        assert!(gain_cv_out[0] <= 10.0, "Should be within CV range: {}", gain_cv_out[0]);
    }

    #[test]
    fn test_inactive_state() {
        let mut vca = VCANodeRefactored::new(44100.0, "test".to_string());
        vca.set_parameter("active", 0.0).unwrap(); // Disable
        
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
        
        assert!(vca.process(&mut ctx).is_ok());
        
        // Should output silence when inactive
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let is_silent = output.iter().all(|&s| s.abs() < 0.001);
        assert!(is_silent);
    }
}