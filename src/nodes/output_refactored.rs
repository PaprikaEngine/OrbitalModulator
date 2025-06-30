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

/// リファクタリング済みOutputNode - プロ仕様最終出力ノード
/// 
/// 特徴:
/// - ステレオ入力（L/R）対応
/// - マスター音量制御（CV変調対応）
/// - ミュート機能
/// - ピークリミッター内蔵
/// - ヘッドルーム管理
/// - VUメーター機能
/// - クリッピング保護
pub struct OutputNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Output controls
    master_volume: f32,           // 0.0 ~ 2.0 (マスター音量)
    mute: f32,                    // 0.0 = Mute, 1.0 = Active
    limiter_threshold: f32,       // 0.5 ~ 1.0 (リミッター閾値)
    limiter_release: f32,         // 0.001 ~ 1.0 (リリース時間)
    active: f32,                  // 0.0 = Off, 1.0 = On
    
    // CV Modulation parameters
    master_volume_param: ModulatableParameter,
    
    // Internal processing state
    limiter_gain_reduction: f32,  // 現在のゲインリダクション
    peak_level_l: f32,            // 左チャンネルピークレベル
    peak_level_r: f32,            // 右チャンネルピークレベル
    rms_level_l: f32,             // 左チャンネルRMSレベル
    rms_level_r: f32,             // 右チャンネルRMSレベル
    
    // Limiter state
    envelope_follower: f32,       // エンベロープフォロワー状態
    
    sample_rate: f32,
}

impl OutputNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "output_refactored".to_string(),
            category: NodeCategory::Mixing,
            
            // Input ports: stereo audio + CV
            input_ports: vec![
                PortInfo::new("audio_in_l", PortType::AudioMono),
                PortInfo::new("audio_in_r", PortType::AudioMono),
                PortInfo::new("master_volume_cv", PortType::CV),
            ],
            
            // Output ports: final mixed output + meter signals
            output_ports: vec![
                PortInfo::new("mixed_output", PortType::AudioMono),
                PortInfo::new("peak_level_l_cv", PortType::CV),
                PortInfo::new("peak_level_r_cv", PortType::CV),
                PortInfo::new("rms_level_l_cv", PortType::CV),
                PortInfo::new("rms_level_r_cv", PortType::CV),
                PortInfo::new("gain_reduction_cv", PortType::CV),
            ],
            
            description: "Professional stereo output with limiter and metering".to_string(),
            latency_samples: 0,
            supports_bypass: true,
        };

        // Create modulation parameters
        let master_volume_param = ModulatableParameter::new(
            BasicParameter::new("master_volume", 0.0, 2.0, 0.7),
            1.0  // 100% CV modulation range
        );

        Self {
            node_info,
            master_volume: 0.7,
            mute: 1.0,  // Active by default
            limiter_threshold: 0.9,
            limiter_release: 0.05,
            active: 1.0,
            
            // CV parameters
            master_volume_param,
            
            // Internal state
            limiter_gain_reduction: 1.0,
            peak_level_l: 0.0,
            peak_level_r: 0.0,
            rms_level_l: 0.0,
            rms_level_r: 0.0,
            envelope_follower: 0.0,
            
            sample_rate,
        }
    }
    
    /// Calculate peak level with decay
    fn update_peak_level(&mut self, current_peak: f32, stored_peak: &mut f32) {
        let decay_rate = 0.99; // Peak hold decay rate
        if current_peak > *stored_peak {
            *stored_peak = current_peak;
        } else {
            *stored_peak *= decay_rate;
        }
    }
    
    /// Calculate RMS level with smoothing
    fn update_rms_level(&mut self, sample: f32, stored_rms: &mut f32) {
        let smooth_factor = 0.01; // RMS smoothing factor
        let instant_power = sample * sample;
        *stored_rms = *stored_rms * (1.0 - smooth_factor) + instant_power * smooth_factor;
    }
    
    /// Apply peak limiter to prevent clipping
    fn apply_limiter(&mut self, left: f32, right: f32) -> (f32, f32) {
        let peak = left.abs().max(right.abs());
        
        if peak > self.limiter_threshold {
            // Calculate required gain reduction
            let required_gain = self.limiter_threshold / peak;
            
            // Smooth gain reduction using envelope follower
            let attack_rate = 0.001; // Very fast attack
            let release_rate = self.limiter_release / self.sample_rate;
            
            if required_gain < self.limiter_gain_reduction {
                // Attack (fast gain reduction)
                self.limiter_gain_reduction = required_gain;
            } else {
                // Release (slow gain restoration)
                self.limiter_gain_reduction += (1.0 - self.limiter_gain_reduction) * release_rate;
            }
        } else {
            // Release when below threshold
            let release_rate = self.limiter_release / self.sample_rate;
            self.limiter_gain_reduction += (1.0 - self.limiter_gain_reduction) * release_rate;
        }
        
        // Apply gain reduction
        let limited_left = left * self.limiter_gain_reduction;
        let limited_right = right * self.limiter_gain_reduction;
        
        (limited_left, limited_right)
    }
    
    /// Final safety clipping (brick wall)
    fn safety_clip(&self, sample: f32) -> f32 {
        sample.clamp(-1.0, 1.0)
    }
}

impl Parameterizable for OutputNodeRefactored {
    define_parameters! {
        master_volume: BasicParameter::new("master_volume", 0.0, 2.0, 0.7),
        mute: BasicParameter::new("mute", 0.0, 1.0, 1.0),
        limiter_threshold: BasicParameter::new("limiter_threshold", 0.5, 1.0, 0.9),
        limiter_release: BasicParameter::new("limiter_release", 0.001, 1.0, 0.05),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for OutputNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - clear output
            if let Some(mixed_output) = ctx.outputs.get_audio_mut("mixed_output") {
                mixed_output.fill(0.0);
            }
            return Ok(());
        }

        // Get input signals
        let left_input = ctx.inputs.get_audio("audio_in_l").unwrap_or(&[]);
        let right_input = ctx.inputs.get_audio("audio_in_r").unwrap_or(&[]);
        let master_volume_cv = ctx.inputs.get_cv_value("master_volume_cv");

        // Apply CV modulation to master volume
        let effective_master_volume = self.master_volume_param.modulate(self.master_volume, master_volume_cv);

        // Get buffer size
        let buffer_size = ctx.outputs.get_audio("mixed_output")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "mixed_output".to_string() 
            })?
            .len();

        // Process each sample
        let mut output_samples = Vec::with_capacity(buffer_size);
        
        for i in 0..buffer_size {
            let left_sample = if i < left_input.len() { left_input[i] } else { 0.0 };
            let right_sample = if i < right_input.len() { right_input[i] } else { 0.0 };

            // Apply mute
            let left_muted = if self.mute > 0.5 { left_sample } else { 0.0 };
            let right_muted = if self.mute > 0.5 { right_sample } else { 0.0 };

            // Apply master volume
            let left_gained = left_muted * effective_master_volume;
            let right_gained = right_muted * effective_master_volume;

            // Update level meters
            let left_peak = left_gained.abs();
            let right_peak = right_gained.abs();
            
            // Update peak levels with decay
            let decay_rate = 0.99;
            if left_peak > self.peak_level_l {
                self.peak_level_l = left_peak;
            } else {
                self.peak_level_l *= decay_rate;
            }
            
            if right_peak > self.peak_level_r {
                self.peak_level_r = right_peak;
            } else {
                self.peak_level_r *= decay_rate;
            }
            
            // Update RMS levels with smoothing
            let smooth_factor = 0.01;
            let left_power = left_gained * left_gained;
            let right_power = right_gained * right_gained;
            self.rms_level_l = self.rms_level_l * (1.0 - smooth_factor) + left_power * smooth_factor;
            self.rms_level_r = self.rms_level_r * (1.0 - smooth_factor) + right_power * smooth_factor;

            // Apply limiter
            let (left_limited, right_limited) = self.apply_limiter(left_gained, right_gained);

            // Final safety clipping
            let left_final = self.safety_clip(left_limited);
            let right_final = self.safety_clip(right_limited);

            // Mix to mono for final output (simple sum and attenuate)
            let mixed_sample = (left_final + right_final) * 0.5;
            output_samples.push(mixed_sample);
        }

        // Write to output
        if let Some(mixed_output) = ctx.outputs.get_audio_mut("mixed_output") {
            for (i, &sample) in output_samples.iter().enumerate() {
                if i < mixed_output.len() {
                    mixed_output[i] = sample;
                }
            }
        }

        // Output meter CV signals
        if let Some(peak_l_cv) = ctx.outputs.get_cv_mut("peak_level_l_cv") {
            peak_l_cv.fill(self.peak_level_l * 10.0); // Scale to CV range
        }
        
        if let Some(peak_r_cv) = ctx.outputs.get_cv_mut("peak_level_r_cv") {
            peak_r_cv.fill(self.peak_level_r * 10.0); // Scale to CV range
        }
        
        if let Some(rms_l_cv) = ctx.outputs.get_cv_mut("rms_level_l_cv") {
            rms_l_cv.fill(self.rms_level_l.sqrt() * 10.0); // Scale to CV range
        }
        
        if let Some(rms_r_cv) = ctx.outputs.get_cv_mut("rms_level_r_cv") {
            rms_r_cv.fill(self.rms_level_r.sqrt() * 10.0); // Scale to CV range
        }
        
        if let Some(gain_reduction_cv) = ctx.outputs.get_cv_mut("gain_reduction_cv") {
            let gain_reduction_db = 20.0 * (1.0 - self.limiter_gain_reduction).log10().max(-60.0);
            gain_reduction_cv.fill(gain_reduction_db / 6.0); // Scale to CV range (-10V = -60dB)
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset internal processing state
        self.limiter_gain_reduction = 1.0;
        self.peak_level_l = 0.0;
        self.peak_level_r = 0.0;
        self.rms_level_l = 0.0;
        self.rms_level_r = 0.0;
        self.envelope_follower = 0.0;
    }

    fn latency(&self) -> u32 {
        0 // No latency for output
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
    fn test_output_creation() {
        let output = OutputNodeRefactored::new(44100.0, "test_output".to_string());
        assert_eq!(output.master_volume, 0.7);
        assert_eq!(output.mute, 1.0);
        assert_eq!(output.active, 1.0);
    }

    #[test]
    fn test_output_parameters() {
        let mut output = OutputNodeRefactored::new(44100.0, "test".to_string());
        
        // Test parameter setting
        assert!(output.set_parameter("master_volume", 0.5).is_ok());
        assert_eq!(output.get_parameter("master_volume").unwrap(), 0.5);
        
        // Test mute
        assert!(output.set_parameter("mute", 0.0).is_ok());
        assert_eq!(output.get_parameter("mute").unwrap(), 0.0);
        
        // Test limiter threshold
        assert!(output.set_parameter("limiter_threshold", 0.8).is_ok());
        assert_eq!(output.get_parameter("limiter_threshold").unwrap(), 0.8);
        
        // Test out of range
        assert!(output.set_parameter("master_volume", 3.0).is_err());
    }

    #[test]
    fn test_limiter_functionality() {
        let mut output = OutputNodeRefactored::new(44100.0, "test".to_string());
        output.set_parameter("limiter_threshold", 0.8).unwrap();
        
        // Test with signal below threshold
        let (left, right) = output.apply_limiter(0.5, 0.5);
        assert!((left - 0.5).abs() < 0.1); // Should pass through mostly unchanged
        
        // Test with signal above threshold
        let (left, right) = output.apply_limiter(1.5, 1.5);
        assert!(left < 0.9); // Should be limited
        assert!(right < 0.9); // Should be limited
    }

    #[test]
    fn test_safety_clipping() {
        let output = OutputNodeRefactored::new(44100.0, "test".to_string());
        
        // Normal levels should pass through
        assert_eq!(output.safety_clip(0.5), 0.5);
        assert_eq!(output.safety_clip(-0.5), -0.5);
        
        // Extreme levels should be clipped
        assert_eq!(output.safety_clip(2.0), 1.0);
        assert_eq!(output.safety_clip(-2.0), -1.0);
    }

    #[test]
    fn test_output_processing() {
        let mut output = OutputNodeRefactored::new(44100.0, "test".to_string());
        output.set_parameter("master_volume", 1.0).unwrap();
        output.set_parameter("mute", 1.0).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in_l".to_string(), vec![0.5, -0.5, 1.0, -1.0]);
        inputs.add_audio("audio_in_r".to_string(), vec![0.3, -0.3, 0.8, -0.8]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("mixed_output".to_string(), 4);
        outputs.allocate_cv("peak_level_l_cv".to_string(), 4);
        outputs.allocate_cv("peak_level_r_cv".to_string(), 4);
        outputs.allocate_cv("rms_level_l_cv".to_string(), 4);
        outputs.allocate_cv("rms_level_r_cv".to_string(), 4);
        outputs.allocate_cv("gain_reduction_cv".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(output.process(&mut ctx).is_ok());
        
        let mixed = ctx.outputs.get_audio("mixed_output").unwrap();
        
        // Check that we have non-zero output
        assert!(mixed[0] != 0.0);
        assert!(mixed[2] != 0.0);
        
        // Check meter outputs exist
        assert!(ctx.outputs.get_cv("peak_level_l_cv").is_some());
        assert!(ctx.outputs.get_cv("gain_reduction_cv").is_some());
    }

    #[test]
    fn test_mute_functionality() {
        let mut output = OutputNodeRefactored::new(44100.0, "test".to_string());
        output.set_parameter("mute", 0.0).unwrap(); // Mute
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in_l".to_string(), vec![1.0, 1.0, 1.0, 1.0]);
        inputs.add_audio("audio_in_r".to_string(), vec![1.0, 1.0, 1.0, 1.0]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("mixed_output".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(output.process(&mut ctx).is_ok());
        
        let mixed = ctx.outputs.get_audio("mixed_output").unwrap();
        
        // Should be zero when muted
        assert_eq!(mixed[0], 0.0);
        assert_eq!(mixed[1], 0.0);
        assert_eq!(mixed[2], 0.0);
        assert_eq!(mixed[3], 0.0);
    }

    #[test]
    fn test_bypass_functionality() {
        let mut output = OutputNodeRefactored::new(44100.0, "test".to_string());
        output.set_parameter("active", 0.0).unwrap(); // Disable
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("audio_in_l".to_string(), vec![1.0, 1.0, 1.0, 1.0]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("mixed_output".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(output.process(&mut ctx).is_ok());
        
        let mixed = ctx.outputs.get_audio("mixed_output").unwrap();
        
        // Should be zero when bypassed
        assert_eq!(mixed[0], 0.0);
        assert_eq!(mixed[1], 0.0);
    }

    #[test]
    fn test_peak_level_update() {
        let mut output = OutputNodeRefactored::new(44100.0, "test".to_string());
        
        // Test peak detection
        output.update_peak_level(0.8, &mut output.peak_level_l);
        assert_eq!(output.peak_level_l, 0.8);
        
        // Test peak hold (lower value shouldn't immediately change peak)
        output.update_peak_level(0.5, &mut output.peak_level_l);
        assert!(output.peak_level_l > 0.5); // Should still be close to 0.8 due to decay
        
        // Test higher peak updates immediately
        output.update_peak_level(0.9, &mut output.peak_level_l);
        assert_eq!(output.peak_level_l, 0.9);
    }

    #[test]
    fn test_rms_level_update() {
        let mut output = OutputNodeRefactored::new(44100.0, "test".to_string());
        
        // Test RMS calculation
        output.update_rms_level(1.0, &mut output.rms_level_l);
        assert!(output.rms_level_l > 0.0);
        
        // Test smoothing
        let first_rms = output.rms_level_l;
        output.update_rms_level(0.0, &mut output.rms_level_l);
        assert!(output.rms_level_l < first_rms); // Should decay
        assert!(output.rms_level_l > 0.0); // But not immediately to zero
    }
}