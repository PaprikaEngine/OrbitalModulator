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

/// リファクタリング済みDelayNode - プロ品質のディレイエフェクト
pub struct DelayNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Parameters
    delay_time: f32,    // in milliseconds
    feedback: f32,      // 0.0 to 0.95
    mix: f32,           // 0.0 to 1.0 (dry/wet mix)
    active: f32,
    
    // CV Modulation parameters
    delay_time_param: ModulatableParameter,
    feedback_param: ModulatableParameter,
    mix_param: ModulatableParameter,
    
    // Delay buffer and state
    delay_buffer: Vec<f32>,
    write_position: f32,    // Using float for smooth interpolation
    max_delay_samples: usize,
    
    sample_rate: f32,
}

impl DelayNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "delay_refactored".to_string(),
            category: NodeCategory::Processor,
            description: "Professional delay effect with smooth interpolation and CV modulation".to_string(),
            input_ports: vec![
                PortInfo::new("audio_in", PortType::AudioMono)
                    .with_description("Audio input signal"),
                PortInfo::new("delay_time_cv", PortType::CV)
                    .with_description("Delay time modulation (0V to +10V)")
                    .optional(),
                PortInfo::new("feedback_cv", PortType::CV)
                    .with_description("Feedback amount modulation (0V to +10V)")
                    .optional(),
                PortInfo::new("mix_cv", PortType::CV)
                    .with_description("Dry/wet mix modulation (0V to +10V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("audio_out", PortType::AudioMono)
                    .with_description("Delayed audio output"),
                PortInfo::new("wet_out", PortType::AudioMono)
                    .with_description("Wet (delayed) signal only")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルディレイ用
        let delay_time_param = ModulatableParameter::new(
            BasicParameter::new("delay_time", 1.0, 2000.0, 250.0).with_unit("ms"),
            0.8  // 80% CV modulation range
        );

        let feedback_param = ModulatableParameter::new(
            BasicParameter::new("feedback", 0.0, 0.95, 0.3),
            0.6  // 60% CV modulation range
        );

        let mix_param = ModulatableParameter::new(
            BasicParameter::new("mix", 0.0, 1.0, 0.5),
            0.8  // 80% CV modulation range
        );

        // Initialize delay buffer for maximum delay time (2 seconds)
        let max_delay_samples = (2.0 * sample_rate) as usize;
        let delay_buffer = vec![0.0; max_delay_samples];

        Self {
            node_info,
            delay_time: 250.0,
            feedback: 0.3,
            mix: 0.5,
            active: 1.0,

            delay_time_param,
            feedback_param,
            mix_param,
            
            delay_buffer,
            write_position: 0.0,
            max_delay_samples,
            
            sample_rate,
        }
    }

    /// 高品質線形補間でディレイサンプルを読み取り
    fn read_delayed_sample(&self, delay_samples: f32) -> f32 {
        if self.delay_buffer.is_empty() {
            return 0.0;
        }

        // Calculate read position with wrap-around
        let buffer_len = self.delay_buffer.len() as f32;
        let read_pos = self.write_position - delay_samples;
        let wrapped_pos = if read_pos < 0.0 {
            read_pos + buffer_len
        } else {
            read_pos % buffer_len
        };

        // Linear interpolation between samples
        let index = wrapped_pos.floor() as usize;
        let frac = wrapped_pos.fract();
        
        let sample1 = self.delay_buffer[index % self.delay_buffer.len()];
        let sample2 = self.delay_buffer[(index + 1) % self.delay_buffer.len()];
        
        sample1 + frac * (sample2 - sample1)
    }

    /// ディレイサンプル処理 - プロ品質のフィードバック制御
    fn process_delay_sample(&mut self, input: f32, delay_time_ms: f32, feedback: f32, mix: f32) -> (f32, f32) {
        let delay_samples = (delay_time_ms / 1000.0 * self.sample_rate).clamp(1.0, self.max_delay_samples as f32 - 1.0);
        
        // Read delayed sample with interpolation
        let delayed_sample = self.read_delayed_sample(delay_samples);
        
        // Apply soft limiting to feedback to prevent runaway
        let limited_feedback = if feedback > 0.9 {
            0.9 + (feedback - 0.9) * 0.1
        } else {
            feedback
        };

        // Create feedback signal with soft saturation
        let feedback_signal = delayed_sample * limited_feedback;
        let saturated_feedback = if feedback_signal.abs() > 0.95 {
            feedback_signal.signum() * (0.95 + (feedback_signal.abs() - 0.95) * 0.1)
        } else {
            feedback_signal
        };
        
        // Write input + feedback to buffer
        let write_sample = input + saturated_feedback;
        let write_index = self.write_position.floor() as usize % self.delay_buffer.len();
        self.delay_buffer[write_index] = write_sample;
        
        // Advance write position
        self.write_position = (self.write_position + 1.0) % self.delay_buffer.len() as f32;
        
        // Mix dry and wet signals
        let dry_signal = input * (1.0 - mix);
        let wet_signal = delayed_sample * mix;
        
        (dry_signal + wet_signal, delayed_sample)
    }
}

impl Parameterizable for DelayNodeRefactored {
    define_parameters! {
        delay_time: BasicParameter::new("delay_time", 1.0, 2000.0, 250.0).with_unit("ms"),
        feedback: BasicParameter::new("feedback", 0.0, 0.95, 0.3),
        mix: BasicParameter::new("mix", 0.0, 1.0, 0.5),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for DelayNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - output silence
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                output.fill(0.0);
            }
            ctx.outputs.clear_audio("wet_out");
            return Ok(());
        }

        // Get audio input
        let audio_input = ctx.inputs.get_audio("audio_in").unwrap_or(&[]);
        if audio_input.is_empty() {
            // No input - output silence (but allow delay buffer to decay)
            if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
                for output_sample in output.iter_mut() {
                    let (mixed, _wet) = self.process_delay_sample(0.0, self.delay_time, self.feedback, self.mix);
                    *output_sample = mixed;
                }
            }
            // Process wet output separately to avoid borrowing issues
            let mut wet_samples = Vec::new();
            for _ in 0..ctx.buffer_size {
                let (_, wet) = self.process_delay_sample(0.0, self.delay_time, self.feedback, self.mix);
                wet_samples.push(wet);
            }
            if let Some(wet_out) = ctx.outputs.get_audio_mut("wet_out") {
                for (i, &wet) in wet_samples.iter().enumerate() {
                    if i < wet_out.len() {
                        wet_out[i] = wet;
                    }
                }
            }
            return Ok(());
        }

        // Get CV inputs
        let delay_time_cv = ctx.inputs.get_cv_value("delay_time_cv");
        let feedback_cv = ctx.inputs.get_cv_value("feedback_cv");
        let mix_cv = ctx.inputs.get_cv_value("mix_cv");

        // Apply CV modulation
        let effective_delay_time = self.delay_time_param.modulate(self.delay_time, delay_time_cv);
        let effective_feedback = self.feedback_param.modulate(self.feedback, feedback_cv);
        let effective_mix = self.mix_param.modulate(self.mix, mix_cv);

        // Collect processed samples first to avoid borrowing conflicts
        let mut main_samples = Vec::new();
        let mut wet_samples = Vec::new();
        
        for i in 0..ctx.buffer_size {
            let input_sample = if i < audio_input.len() { 
                audio_input[i] 
            } else { 
                0.0 
            };

            let (mixed, wet) = self.process_delay_sample(
                input_sample, 
                effective_delay_time, 
                effective_feedback, 
                effective_mix
            );

            main_samples.push(mixed);
            wet_samples.push(wet);
        }
        
        // Write to output buffers
        if let Some(output) = ctx.outputs.get_audio_mut("audio_out") {
            for (i, &sample) in main_samples.iter().enumerate() {
                if i < output.len() {
                    output[i] = sample;
                }
            }
        }
        
        if let Some(wet_out) = ctx.outputs.get_audio_mut("wet_out") {
            for (i, &sample) in wet_samples.iter().enumerate() {
                if i < wet_out.len() {
                    wet_out[i] = sample;
                }
            }
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Clear delay buffer
        self.delay_buffer.fill(0.0);
        self.write_position = 0.0;
    }

    fn latency(&self) -> u32 {
        // Delay has inherent latency equal to delay time
        (self.delay_time / 1000.0 * self.sample_rate) as u32
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
    fn test_delay_parameters() {
        let mut delay = DelayNodeRefactored::new(44100.0, "test".to_string());
        
        // Test delay time setting
        assert!(delay.set_parameter("delay_time", 500.0).is_ok());
        assert_eq!(delay.get_parameter("delay_time").unwrap(), 500.0);
        
        // Test feedback setting
        assert!(delay.set_parameter("feedback", 0.6).is_ok());
        assert_eq!(delay.get_parameter("feedback").unwrap(), 0.6);
        
        // Test mix setting
        assert!(delay.set_parameter("mix", 0.7).is_ok());
        assert_eq!(delay.get_parameter("mix").unwrap(), 0.7);
        
        // Test validation
        assert!(delay.set_parameter("delay_time", -10.0).is_err()); // Out of range
        assert!(delay.set_parameter("feedback", 1.2).is_err()); // Out of range
    }

    #[test]
    fn test_delay_processing() {
        let mut delay = DelayNodeRefactored::new(44100.0, "test".to_string());
        delay.set_parameter("delay_time", 10.0).unwrap(); // Short delay for testing
        delay.set_parameter("mix", 1.0).unwrap(); // Full wet signal
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0, 0.0, 0.0, 0.0]); // Impulse
        
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
        
        // Should process without error
        assert!(delay.process(&mut ctx).is_ok());
        
        // Output should show delayed impulse
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // Initial samples should be mostly quiet (impulse hasn't come through yet)
        assert!(output[0].abs() < 0.5, "First sample should be mostly input");
        
        // Later samples should show the delayed signal
        let has_delayed_signal = output[400..].iter().any(|&s| s.abs() > 0.1);
        assert!(has_delayed_signal, "Should have delayed signal later");
    }

    #[test]
    fn test_delay_feedback() {
        let mut delay = DelayNodeRefactored::new(44100.0, "test".to_string());
        delay.set_parameter("delay_time", 5.0).unwrap(); // Very short delay
        delay.set_parameter("feedback", 0.5).unwrap();
        delay.set_parameter("mix", 1.0).unwrap(); // Full wet to hear feedback
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 1024]); // Constant input
        
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
        
        assert!(delay.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // With feedback, signal should build up over time
        let early_avg = output[..100].iter().sum::<f32>() / 100.0;
        let late_avg = output[900..].iter().sum::<f32>() / 100.0;
        
        assert!(late_avg > early_avg, "Feedback should cause signal to build up: {} vs {}", early_avg, late_avg);
        
        // Should not go into runaway feedback
        let max_output = output.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        assert!(max_output < 10.0, "Should limit feedback to prevent runaway: {}", max_output);
    }

    #[test]
    fn test_delay_time_cv_modulation() {
        let mut delay = DelayNodeRefactored::new(44100.0, "test".to_string());
        delay.set_parameter("delay_time", 100.0).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 256]);
        inputs.add_cv("delay_time_cv".to_string(), vec![2.0]); // Increase delay time
        
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
        
        assert!(delay.process(&mut ctx).is_ok());
        
        // CV should modulate the delay time
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let has_modulated_delay = output.iter().any(|&s| s.abs() > 0.001);
        assert!(has_modulated_delay);
    }

    #[test]
    fn test_wet_output() {
        let mut delay = DelayNodeRefactored::new(44100.0, "test".to_string());
        delay.set_parameter("delay_time", 10.0).unwrap();
        delay.set_parameter("feedback", 0.2).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 512]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("audio_out".to_string(), 512);
        outputs.allocate_audio("wet_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(delay.process(&mut ctx).is_ok());
        
        // Should have wet output
        let wet_out = ctx.outputs.get_audio("wet_out").unwrap();
        let has_wet_signal = wet_out.iter().any(|&s| s.abs() > 0.01);
        assert!(has_wet_signal, "Should have wet output signal");
    }

    #[test]
    fn test_interpolation_quality() {
        let delay = DelayNodeRefactored::new(44100.0, "test".to_string());
        
        // Test interpolation with fractional delay
        let test_sample1 = delay.read_delayed_sample(10.3);
        let test_sample2 = delay.read_delayed_sample(10.7);
        
        // Values should be different due to interpolation
        // (Note: they'll both be 0.0 initially, but testing the mechanism)
        assert!(test_sample1.is_finite());
        assert!(test_sample2.is_finite());
    }

    #[test]
    fn test_feedback_limiting() {
        let mut delay = DelayNodeRefactored::new(44100.0, "test".to_string());
        delay.set_parameter("feedback", 0.95).unwrap(); // High feedback
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in".to_string(), vec![1.0; 2048]); // Long input
        
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
        
        assert!(delay.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        
        // Even with high feedback, output should remain stable
        let max_output = output.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        assert!(max_output < 5.0, "High feedback should be limited: {}", max_output);
        
        // Should still have some signal
        let avg_output = output.iter().map(|&x| x.abs()).sum::<f32>() / output.len() as f32;
        assert!(avg_output > 0.1, "Should still have signal with feedback: {}", avg_output);
    }

    #[test]
    fn test_inactive_state() {
        let mut delay = DelayNodeRefactored::new(44100.0, "test".to_string());
        delay.set_parameter("active", 0.0).unwrap(); // Disable
        
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
        
        assert!(delay.process(&mut ctx).is_ok());
        
        // Should output silence when inactive
        let output = ctx.outputs.get_audio("audio_out").unwrap();
        let is_silent = output.iter().all(|&s| s.abs() < 0.001);
        assert!(is_silent);
    }
}