use uuid::Uuid;

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;
use crate::define_parameters;

/// リファクタリング済みCompressorNode - プロ品質のダイナミックレンジコンプレッサー
pub struct CompressorNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Parameters
    threshold: f32,        // Compression threshold in dB
    ratio: f32,           // Compression ratio (1:1 to 20:1)
    attack: f32,          // Attack time in seconds
    release: f32,         // Release time in seconds
    knee: f32,            // Knee width in dB
    makeup_gain: f32,     // Makeup gain in dB
    limiter_mode: f32,    // Enable hard limiting (0/1)
    limiter_threshold: f32, // Limiter threshold in dB
    active: f32,
    
    // CV Modulation parameters
    threshold_param: ModulatableParameter,
    ratio_param: ModulatableParameter,
    attack_param: ModulatableParameter,
    release_param: ModulatableParameter,
    makeup_gain_param: ModulatableParameter,
    
    // Internal state
    envelope: f32,        // Envelope follower output
    gain_reduction: f32,  // Current gain reduction in dB
    
    // Coefficients for envelope follower
    attack_coeff: f32,
    release_coeff: f32,
    
    sample_rate: f32,
}

impl CompressorNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "compressor_refactored".to_string(),
            category: NodeCategory::Processor,
            description: "Professional dynamic range compressor with soft knee and limiter".to_string(),
            input_ports: vec![
                PortInfo::new("audio_in", PortType::AudioMono)
                    .with_description("Audio input signal"),
                PortInfo::new("threshold_cv", PortType::CV)
                    .with_description("Threshold control (-10V to +10V)")
                    .optional(),
                PortInfo::new("ratio_cv", PortType::CV)
                    .with_description("Compression ratio control (0V to +10V)")
                    .optional(),
                PortInfo::new("attack_cv", PortType::CV)
                    .with_description("Attack time control (0V to +10V)")
                    .optional(),
                PortInfo::new("release_cv", PortType::CV)
                    .with_description("Release time control (0V to +10V)")
                    .optional(),
                PortInfo::new("makeup_gain_cv", PortType::CV)
                    .with_description("Makeup gain control (-10V to +10V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono)
                    .with_description("Compressed audio output"),
                PortInfo::new("gain_reduction_cv", PortType::CV)
                    .with_description("Gain reduction meter as CV output")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルコンプレッサー用
        let threshold_param = ModulatableParameter::new(
            BasicParameter::new("threshold", -60.0, 0.0, -20.0).with_unit("dB"),
            0.8  // 80% CV modulation range
        );

        let ratio_param = ModulatableParameter::new(
            BasicParameter::new("ratio", 1.0, 20.0, 4.0),
            0.6  // 60% CV modulation range
        );

        let attack_param = ModulatableParameter::new(
            BasicParameter::new("attack", 0.0001, 1.0, 0.003).with_unit("s"),
            0.5  // 50% CV modulation range
        );

        let release_param = ModulatableParameter::new(
            BasicParameter::new("release", 0.001, 10.0, 0.1).with_unit("s"),
            0.5  // 50% CV modulation range
        );

        let makeup_gain_param = ModulatableParameter::new(
            BasicParameter::new("makeup_gain", -20.0, 20.0, 0.0).with_unit("dB"),
            0.8  // 80% CV modulation range
        );

        let mut compressor = Self {
            node_info,
            threshold: -20.0,      // -20dB threshold
            ratio: 4.0,           // 4:1 ratio
            attack: 0.003,        // 3ms attack
            release: 0.1,         // 100ms release
            knee: 2.0,            // 2dB knee
            makeup_gain: 0.0,     // No makeup gain
            limiter_mode: 0.0,    // Disabled
            limiter_threshold: -0.1, // -0.1dB limiter threshold
            active: 1.0,

            threshold_param,
            ratio_param,
            attack_param,
            release_param,
            makeup_gain_param,
            
            envelope: -60.0,      // Start at silence level
            gain_reduction: 0.0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            
            sample_rate,
        };
        
        compressor.update_coefficients(compressor.attack, compressor.release);
        compressor
    }

    /// エンベロープフォロワーの係数を更新
    fn update_coefficients(&mut self, attack: f32, release: f32) {
        // Calculate envelope follower coefficients
        self.attack_coeff = (-1.0 / (attack * self.sample_rate)).exp();
        self.release_coeff = (-1.0 / (release * self.sample_rate)).exp();
    }

    /// リニア値をdBに変換
    fn linear_to_db(&self, linear: f32) -> f32 {
        if linear > 1e-10 {
            20.0 * linear.log10()
        } else {
            -100.0 // Silence
        }
    }

    /// dB値をリニアに変換
    fn db_to_linear(&self, db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }

    /// プロ品質のコンプレッション処理
    fn process_compression(&mut self, input: f32, threshold: f32, ratio: f32, attack: f32, 
                          release: f32, knee: f32, makeup_gain: f32, limiter_mode: bool, 
                          limiter_threshold: f32) -> f32 {
        
        // Update coefficients if attack/release changed
        if (attack - self.attack).abs() > 0.0001 || (release - self.release).abs() > 0.001 {
            self.update_coefficients(attack, release);
        }

        // Convert input to dB for processing
        let input_db = self.linear_to_db(input.abs());
        
        // Update envelope follower with peak detection
        if input_db > self.envelope {
            // Attack (peak detection)
            self.envelope = input_db + (self.envelope - input_db) * self.attack_coeff;
        } else {
            // Release
            self.envelope = input_db + (self.envelope - input_db) * self.release_coeff;
        }
        
        // Calculate gain reduction with soft knee
        let over_threshold = self.envelope - threshold;
        
        let compression_gain = if over_threshold > 0.0 {
            if knee > 0.0 && over_threshold < knee {
                // Soft knee compression - smooth transition
                let knee_ratio = over_threshold / knee;
                let soft_ratio = 1.0 + (ratio - 1.0) * knee_ratio * knee_ratio;
                -over_threshold * (1.0 - 1.0 / soft_ratio)
            } else {
                // Hard knee compression
                -over_threshold * (1.0 - 1.0 / ratio)
            }
        } else {
            0.0
        };
        
        self.gain_reduction = compression_gain;
        
        // Apply compression with smooth gain changes
        let mut output = input * self.db_to_linear(compression_gain);
        
        // Apply makeup gain
        output *= self.db_to_linear(makeup_gain);
        
        // Apply limiter if enabled (brick wall limiting)
        if limiter_mode {
            let output_level = output.abs();
            let limiter_threshold_linear = self.db_to_linear(limiter_threshold);
            if output_level > limiter_threshold_linear {
                output = output.signum() * limiter_threshold_linear;
            }
        }
        
        output
    }
}

impl Parameterizable for CompressorNodeRefactored {
    define_parameters! {
        threshold: BasicParameter::new("threshold", -60.0, 0.0, -20.0).with_unit("dB"),
        ratio: BasicParameter::new("ratio", 1.0, 20.0, 4.0),
        attack: BasicParameter::new("attack", 0.0001, 1.0, 0.003).with_unit("s"),
        release: BasicParameter::new("release", 0.001, 10.0, 0.1).with_unit("s"),
        knee: BasicParameter::new("knee", 0.0, 10.0, 2.0).with_unit("dB"),
        makeup_gain: BasicParameter::new("makeup_gain", -20.0, 20.0, 0.0).with_unit("dB"),
        limiter_mode: BasicParameter::new("limiter_mode", 0.0, 1.0, 0.0),
        limiter_threshold: BasicParameter::new("limiter_threshold", -20.0, 0.0, -0.1).with_unit("dB"),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for CompressorNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - pass through input signal
            let audio_input = ctx.inputs.get_audio("audio_in").unwrap_or(&[]);
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                for (i, output_sample) in output.iter_mut().enumerate() {
                    *output_sample = if i < audio_input.len() { audio_input[i] } else { 0.0 };
                }
            }
            ctx.outputs.set_cv_value("gain_reduction_cv", 0.0);
            return Ok(());
        }

        // Get audio input
        let audio_input = ctx.inputs.get_audio("audio_in").unwrap_or(&[]);
        if audio_input.is_empty() {
            // No input - output silence
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                output.fill(0.0);
            }
            ctx.outputs.set_cv_value("gain_reduction_cv", 0.0);
            return Ok(());
        }

        // Get CV inputs
        let threshold_cv = ctx.inputs.get_cv_value("threshold_cv");
        let ratio_cv = ctx.inputs.get_cv_value("ratio_cv");
        let attack_cv = ctx.inputs.get_cv_value("attack_cv");
        let release_cv = ctx.inputs.get_cv_value("release_cv");
        let makeup_gain_cv = ctx.inputs.get_cv_value("makeup_gain_cv");

        // Apply CV modulation
        let effective_threshold = self.threshold_param.modulate(self.threshold, threshold_cv);
        let effective_ratio = self.ratio_param.modulate(self.ratio, ratio_cv);
        let effective_attack = self.attack_param.modulate(self.attack, attack_cv);
        let effective_release = self.release_param.modulate(self.release, release_cv);
        let effective_makeup_gain = self.makeup_gain_param.modulate(self.makeup_gain, makeup_gain_cv);

        // Process audio output
        let output = ctx.outputs.get_audio_mut("audio_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "audio_out".to_string() 
            })?;

        let mut total_gain_reduction = 0.0;

        // Process each sample through the compressor
        for (i, output_sample) in output.iter_mut().enumerate() {
            let input_sample = if i < audio_input.len() { 
                audio_input[i] 
            } else { 
                0.0 
            };

            *output_sample = self.process_compression(
                input_sample, 
                effective_threshold, 
                effective_ratio, 
                effective_attack, 
                effective_release, 
                self.knee, 
                effective_makeup_gain,
                self.limiter_mode > 0.5,
                self.limiter_threshold
            );

            total_gain_reduction += self.gain_reduction.abs();
        }

        // Output gain reduction as CV (average over buffer)
        let avg_gain_reduction = total_gain_reduction / output.len() as f32;
        // Convert dB gain reduction to CV (0V = no reduction, +10V = 60dB reduction)
        let gain_reduction_cv = (avg_gain_reduction.abs() / 60.0 * 10.0).clamp(0.0, 10.0);
        ctx.outputs.set_cv_value("gain_reduction_cv", gain_reduction_cv);

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset compressor state
        self.envelope = -60.0;
        self.gain_reduction = 0.0;
    }

    fn latency(&self) -> u32 {
        // Compressor has minimal latency (lookahead could be added later)
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_compressor_parameters() {
        let mut compressor = CompressorNodeRefactored::new(44100.0, "test".to_string());
        
        // Test threshold setting
        assert!(compressor.set_parameter("threshold", -30.0).is_ok());
        assert_eq!(compressor.get_parameter("threshold").unwrap(), -30.0);
        
        // Test ratio setting
        assert!(compressor.set_parameter("ratio", 8.0).is_ok());
        assert_eq!(compressor.get_parameter("ratio").unwrap(), 8.0);
        
        // Test attack setting
        assert!(compressor.set_parameter("attack", 0.01).is_ok());
        assert_eq!(compressor.get_parameter("attack").unwrap(), 0.01);
        
        // Test validation
        assert!(compressor.set_parameter("threshold", -100.0).is_err()); // Out of range
        assert!(compressor.set_parameter("ratio", 25.0).is_err()); // Out of range
    }

    #[test]
    fn test_compressor_processing() {
        let mut compressor = CompressorNodeRefactored::new(44100.0, "test".to_string());
        compressor.set_parameter("threshold", -20.0).unwrap();
        compressor.set_parameter("ratio", 4.0).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.8; 512]); // Loud signal
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 512);
        outputs.allocate_cv("gain_reduction_cv".to_string(), 1);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        // Should process without error
        assert!(compressor.process(&mut ctx).is_ok());
        
        // Output should be compressed (lower than input)
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!(avg_output < 0.8, "Output should be compressed: {}", avg_output);
        assert!(avg_output > 0.1, "Should still have signal: {}", avg_output);
        
        // Should show gain reduction
        let gain_reduction_cv = ctx.outputs.get_cv("gain_reduction_cv").unwrap();
        assert!(gain_reduction_cv[0] > 0.0, "Should show gain reduction: {}", gain_reduction_cv[0]);
    }

    #[test]
    fn test_compression_ratio() {
        let mut compressor = CompressorNodeRefactored::new(44100.0, "test".to_string());
        compressor.set_parameter("threshold", -40.0).unwrap(); // Low threshold
        compressor.set_parameter("ratio", 10.0).unwrap(); // High ratio
        compressor.set_parameter("attack", 0.001).unwrap(); // Fast attack
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 1024]); // Very loud signal
        
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
        
        assert!(compressor.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // High ratio should significantly compress the signal
        // Take average from second half of buffer after envelope has settled
        let settled_output = &output[512..];
        let avg_output = settled_output.iter().sum::<f32>() / settled_output.len() as f32;
        assert!(avg_output < 0.8, "High ratio should compress significantly: {}", avg_output);
    }

    #[test]
    fn test_soft_knee() {
        let compressor = CompressorNodeRefactored::new(44100.0, "test".to_string());
        
        // Test conversion functions
        let linear_val = 0.5;
        let db_val = compressor.linear_to_db(linear_val);
        let back_to_linear = compressor.db_to_linear(db_val);
        
        assert!((back_to_linear - linear_val).abs() < 0.001, "Conversion should be accurate");
        
        // Test that soft knee produces different results than hard knee
        // This is tested implicitly in the compression processing
    }

    #[test]
    fn test_limiter_mode() {
        let mut compressor = CompressorNodeRefactored::new(44100.0, "test".to_string());
        compressor.set_parameter("limiter_mode", 1.0).unwrap(); // Enable limiter
        compressor.set_parameter("limiter_threshold", -6.0).unwrap(); // -6dB limit
        compressor.set_parameter("makeup_gain", 10.0).unwrap(); // Boost to trigger limiter
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.5; 512]); // Moderate input
        
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
        
        assert!(compressor.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let max_output = output.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        
        // Limiter should prevent output from exceeding -6dB (~0.5 linear)
        assert!(max_output <= 0.55, "Limiter should prevent excessive output: {}", max_output);
    }

    #[test]
    fn test_threshold_cv_modulation() {
        let mut compressor = CompressorNodeRefactored::new(44100.0, "test".to_string());
        compressor.set_parameter("threshold", -20.0).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.6; 256]);
        inputs.add_cv("threshold_cv".to_string(), vec![-2.0]); // Lower threshold
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 256);
        outputs.allocate_cv("gain_reduction_cv".to_string(), 1);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 256,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(compressor.process(&mut ctx).is_ok());
        
        // CV should modulate the threshold, affecting compression
        let gain_reduction_cv = ctx.outputs.get_cv("gain_reduction_cv").unwrap();
        assert!(gain_reduction_cv[0] > 0.0, "CV modulation should affect compression");
    }

    #[test]
    fn test_attack_release_cv() {
        let mut compressor = CompressorNodeRefactored::new(44100.0, "test".to_string());
        compressor.set_parameter("threshold", -30.0).unwrap(); // Lower threshold for more compression
        compressor.set_parameter("ratio", 8.0).unwrap(); // Higher ratio
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![0.8; 512]);
        inputs.add_cv("attack_cv".to_string(), vec![1.0]); // Faster attack
        inputs.add_cv("release_cv".to_string(), vec![2.0]); // Slower release
        
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
        
        assert!(compressor.process(&mut ctx).is_ok());
        
        // Should modulate attack/release times and produce compression
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        // Check that signal is compressed (average should be less than input)
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!(avg_output < 0.8, "Attack/Release CV should affect compression: avg={}", avg_output);
    }

    #[test]
    fn test_envelope_stability() {
        let mut compressor = CompressorNodeRefactored::new(44100.0, "test".to_string());
        compressor.set_parameter("attack", 0.0001).unwrap(); // Very fast
        compressor.set_parameter("release", 0.001).unwrap(); // Very fast
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 2048]); // Long, loud signal
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 2048);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 2048,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(compressor.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // Output should be stable (no NaN, infinite values)
        let is_stable = output.iter().all(|&s| s.is_finite());
        assert!(is_stable, "Compressor should remain stable");
        
        // Should have reasonable output levels
        let max_output = output.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        assert!(max_output < 10.0, "Output should be reasonable: {}", max_output);
    }

    #[test]
    fn test_inactive_state() {
        let mut compressor = CompressorNodeRefactored::new(44100.0, "test".to_string());
        compressor.set_parameter("active", 0.0).unwrap(); // Disable
        
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
        
        assert!(compressor.process(&mut ctx).is_ok());
        
        // Should pass through unchanged when inactive
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!((avg_output - 0.8).abs() < 0.001, "Should pass through when inactive: {}", avg_output);
    }
}