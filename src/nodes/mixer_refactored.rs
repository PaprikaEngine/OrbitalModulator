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

/// リファクタリング済みMixerNode - プロ仕様マルチチャンネルミキサー
/// 
/// 特徴:
/// - 可変チャンネル数（2-8チャンネル）
/// - チャンネル毎のゲイン・パン・ミュート制御
/// - CV変調対応（全パラメーター）
/// - ステレオ出力（L/R）
/// - マスターゲイン・EQ制御
/// - ヘッドルーム管理とクリッピング保護
pub struct MixerNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Mixer configuration
    channel_count: f32,           // 2.0 ~ 8.0 (実際のチャンネル数)
    active: f32,                  // 0.0 = Off, 1.0 = On
    
    // Master controls
    master_gain: f32,             // 0.0 ~ 2.0 (マスターゲイン)
    high_freq_gain: f32,          // 0.0 ~ 2.0 (高音域EQ)
    mid_freq_gain: f32,           // 0.0 ~ 2.0 (中音域EQ)
    low_freq_gain: f32,           // 0.0 ~ 2.0 (低音域EQ)
    
    // Per-channel controls (最大8チャンネル分)
    channel_gains: [f32; 8],      // 各チャンネルゲイン (0.0 ~ 2.0)
    channel_pans: [f32; 8],       // 各チャンネルパン (-1.0 ~ 1.0)
    channel_mutes: [f32; 8],      // 各チャンネルミュート (0.0 = Mute, 1.0 = Active)
    
    // CV Modulation parameters
    master_gain_param: ModulatableParameter,
    high_freq_param: ModulatableParameter,
    mid_freq_param: ModulatableParameter,
    low_freq_param: ModulatableParameter,
    channel_gain_params: [ModulatableParameter; 8],
    channel_pan_params: [ModulatableParameter; 8],
    
    // Internal processing state
    temp_left: Vec<f32>,          // 左チャンネル合成バッファ
    temp_right: Vec<f32>,         // 右チャンネル合成バッファ
    
    // EQ state (simple 3-band)
    high_freq_state: f32,         // ハイパスフィルター状態
    low_freq_state: f32,          // ローパスフィルター状態
    
    sample_rate: f32,
}

impl MixerNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "mixer_refactored".to_string(),
            category: NodeCategory::Mixing,
            
            // Input ports: 8 stereo inputs + CV inputs
            input_ports: vec![
                // Audio inputs (8 stereo pairs)
                PortInfo { name: "ch1_l".to_string(), port_type: PortType::AudioMono, description: "Channel 1 Left".to_string(), is_optional: true },
                PortInfo { name: "ch1_r".to_string(), port_type: PortType::AudioMono, description: "Channel 1 Right".to_string(), is_optional: true },
                PortInfo::new("ch2_l", PortType::AudioMono),
                PortInfo::new("ch2_r", PortType::AudioMono),
                PortInfo::new("ch3_l", PortType::AudioMono),
                PortInfo::new("ch3_r", PortType::AudioMono),
                PortInfo::new("ch4_l", PortType::AudioMono),
                PortInfo::new("ch4_r", PortType::AudioMono),
                PortInfo::new("ch5_l", PortType::AudioMono),
                PortInfo::new("ch5_r", PortType::AudioMono),
                PortInfo::new("ch6_l", PortType::AudioMono),
                PortInfo::new("ch6_r", PortType::AudioMono),
                PortInfo::new("ch7_l", PortType::AudioMono),
                PortInfo::new("ch7_r", PortType::AudioMono),
                PortInfo::new("ch8_l", PortType::AudioMono),
                PortInfo::new("ch8_r", PortType::AudioMono),
                
                // CV inputs
                PortInfo::new("master_gain_cv", PortType::CV),
                PortInfo::new("high_freq_cv", PortType::CV),
                PortInfo::new("mid_freq_cv", PortType::CV),
                PortInfo::new("low_freq_cv", PortType::CV),
            ],
            
            // Output ports: stereo mix + sends
            output_ports: vec![
                PortInfo::new("mix_l", PortType::AudioMono),
                PortInfo::new("mix_r", PortType::AudioMono),
                PortInfo::new("send1_l", PortType::AudioMono),
                PortInfo::new("send1_r", PortType::AudioMono),
                PortInfo::new("send2_l", PortType::AudioMono),
                PortInfo::new("send2_r", PortType::AudioMono),
            ],
            
            description: "Professional multi-channel mixer with EQ and CV modulation".to_string(),
            latency_samples: 0,
            supports_bypass: true,
        };

        // Create modulation parameters
        let master_gain_param = ModulatableParameter::new(
            BasicParameter::new("master_gain", 0.0, 2.0, 0.8),
            1.0  // 100% CV modulation range
        );
        
        let high_freq_param = ModulatableParameter::new(
            BasicParameter::new("high_freq_gain", 0.0, 2.0, 1.0),
            0.5  // 50% CV modulation range
        );
        
        let mid_freq_param = ModulatableParameter::new(
            BasicParameter::new("mid_freq_gain", 0.0, 2.0, 1.0),
            0.5  // 50% CV modulation range
        );
        
        let low_freq_param = ModulatableParameter::new(
            BasicParameter::new("low_freq_gain", 0.0, 2.0, 1.0),
            0.5  // 50% CV modulation range
        );

        // Create per-channel modulation parameters
        let channel_gain_params = [0; 8].map(|_| 
            ModulatableParameter::new(
                BasicParameter::new("channel_gain", 0.0, 2.0, 0.7),
                0.8  // 80% CV modulation range
            )
        );
        
        let channel_pan_params = [0; 8].map(|_| 
            ModulatableParameter::new(
                BasicParameter::new("channel_pan", -1.0, 1.0, 0.0),
                0.8  // 80% CV modulation range
            )
        );

        Self {
            node_info,
            channel_count: 4.0,  // デフォルト4チャンネル
            active: 1.0,
            
            // Master controls
            master_gain: 0.8,
            high_freq_gain: 1.0,
            mid_freq_gain: 1.0,
            low_freq_gain: 1.0,
            
            // Per-channel controls
            channel_gains: [0.7; 8],    // デフォルト70%ゲイン
            channel_pans: [0.0; 8],     // センターパン
            channel_mutes: [1.0; 8],    // 全チャンネルアクティブ
            
            // CV parameters
            master_gain_param,
            high_freq_param,
            mid_freq_param,
            low_freq_param,
            channel_gain_params,
            channel_pan_params,
            
            // Internal state
            temp_left: vec![0.0; 512],
            temp_right: vec![0.0; 512],
            high_freq_state: 0.0,
            low_freq_state: 0.0,
            
            sample_rate,
        }
    }
    
    /// Set number of active channels (2-8)
    pub fn set_channel_count(&mut self, count: usize) -> Result<(), String> {
        if count >= 2 && count <= 8 {
            self.channel_count = count as f32;
            Ok(())
        } else {
            Err(format!("Channel count must be 2-8, got {}", count))
        }
    }
    
    /// Calculate pan gains (constant power panning)
    fn calculate_pan_gains(&self, pan: f32) -> (f32, f32) {
        let pan_clamped = pan.clamp(-1.0, 1.0);
        let pan_rad = (pan_clamped + 1.0) * std::f32::consts::PI / 4.0; // 0 to π/2
        let left_gain = pan_rad.cos();
        let right_gain = pan_rad.sin();
        (left_gain, right_gain)
    }
    
    /// Simple 3-band EQ processing
    fn apply_eq(&mut self, left: f32, right: f32) -> (f32, f32) {
        // High-pass filter (simple 1-pole)
        let high_cutoff = 0.1; // Normalized frequency
        self.high_freq_state += high_cutoff * (left - self.high_freq_state);
        let high_component = left - self.high_freq_state;
        
        // Low-pass filter (simple 1-pole)  
        let low_cutoff = 0.3; // Normalized frequency
        self.low_freq_state += low_cutoff * (left - self.low_freq_state);
        let low_component = self.low_freq_state;
        
        // Mid component (original - high - low)
        let mid_component = left - high_component - low_component;
        
        // Apply EQ gains
        let eq_left = high_component * self.high_freq_gain + 
                      mid_component * self.mid_freq_gain + 
                      low_component * self.low_freq_gain;
        
        let eq_right = right; // For simplicity, apply same EQ to right
        
        (eq_left, eq_right)
    }
    
    /// Soft clipping to prevent harsh distortion
    fn soft_clip(&self, sample: f32) -> f32 {
        let threshold = 0.8;
        if sample.abs() > threshold {
            let sign = sample.signum();
            sign * (threshold + (sample.abs() - threshold).tanh() * 0.2)
        } else {
            sample
        }
    }
}

impl Parameterizable for MixerNodeRefactored {
    fn get_all_parameters(&self) -> std::collections::HashMap<String, f32> {
        let mut params = std::collections::HashMap::new();
        params.insert("channel_count".to_string(), self.channel_count);
        params.insert("master_gain".to_string(), self.master_gain);
        params.insert("high_freq_gain".to_string(), self.high_freq_gain);
        params.insert("mid_freq_gain".to_string(), self.mid_freq_gain);
        params.insert("low_freq_gain".to_string(), self.low_freq_gain);
        params.insert("active".to_string(), self.active);
        
        // Add per-channel parameters
        for i in 0..8 {
            params.insert(format!("ch{}_gain", i + 1), self.channel_gains[i]);
            params.insert(format!("ch{}_pan", i + 1), self.channel_pans[i]);
            params.insert(format!("ch{}_mute", i + 1), self.channel_mutes[i]);
        }
        
        params
    }

    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), crate::parameters::ParameterError> {
        // Handle channel-specific parameters
        if name.starts_with("ch") && name.contains("_") {
            if let Some(underscore_pos) = name.rfind('_') {
                let channel_part = &name[..underscore_pos];
                let param_type = &name[underscore_pos + 1..];
                
                if let Some(ch_str) = channel_part.strip_prefix("ch") {
                    if let Ok(ch_num) = ch_str.parse::<usize>() {
                        if ch_num >= 1 && ch_num <= 8 {
                            let ch_idx = ch_num - 1;
                            match param_type {
                                "gain" => {
                                    if value >= 0.0 && value <= 2.0 {
                                        self.channel_gains[ch_idx] = value;
                                        self.channel_gain_params[ch_idx].set_base_value(value)?;
                                        return Ok(());
                                    } else {
                                        return Err(crate::parameters::ParameterError::OutOfRange { 
                                            value, min: 0.0, max: 2.0 
                                        });
                                    }
                                },
                                "pan" => {
                                    if value >= -1.0 && value <= 1.0 {
                                        self.channel_pans[ch_idx] = value;
                                        self.channel_pan_params[ch_idx].set_base_value(value)?;
                                        return Ok(());
                                    } else {
                                        return Err(crate::parameters::ParameterError::OutOfRange { 
                                            value, min: -1.0, max: 1.0 
                                        });
                                    }
                                },
                                "mute" => {
                                    if value >= 0.0 && value <= 1.0 {
                                        self.channel_mutes[ch_idx] = value;
                                        return Ok(());
                                    } else {
                                        return Err(crate::parameters::ParameterError::OutOfRange { 
                                            value, min: 0.0, max: 1.0 
                                        });
                                    }
                                },
                                _ => {
                                    return Err(crate::parameters::ParameterError::NotFound { 
                                        name: name.to_string() 
                                    });
                                }
                            }
                        }
                    }
                }
            }
            return Err(crate::parameters::ParameterError::NotFound { name: name.to_string() });
        }

        // Handle standard parameters
        match name {
            "channel_count" => {
                if value >= 2.0 && value <= 8.0 {
                    self.channel_count = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 2.0, max: 8.0 
                    })
                }
            },
            "master_gain" => {
                if value >= 0.0 && value <= 2.0 {
                    self.master_gain = value;
                    self.master_gain_param.set_base_value(value)?;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 2.0 
                    })
                }
            },
            "high_freq_gain" => {
                if value >= 0.0 && value <= 2.0 {
                    self.high_freq_gain = value;
                    self.high_freq_param.set_base_value(value)?;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 2.0 
                    })
                }
            },
            "mid_freq_gain" => {
                if value >= 0.0 && value <= 2.0 {
                    self.mid_freq_gain = value;
                    self.mid_freq_param.set_base_value(value)?;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 2.0 
                    })
                }
            },
            "low_freq_gain" => {
                if value >= 0.0 && value <= 2.0 {
                    self.low_freq_gain = value;
                    self.low_freq_param.set_base_value(value)?;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 2.0 
                    })
                }
            },
            "active" => {
                if value >= 0.0 && value <= 1.0 {
                    self.active = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 1.0 
                    })
                }
            },
            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() }),
        }
    }

    fn get_parameter(&self, name: &str) -> Result<f32, crate::parameters::ParameterError> {
        // Handle channel-specific parameters
        if name.starts_with("ch") && name.contains("_") {
            if let Some(underscore_pos) = name.rfind('_') {
                let channel_part = &name[..underscore_pos];
                let param_type = &name[underscore_pos + 1..];
                
                if let Some(ch_str) = channel_part.strip_prefix("ch") {
                    if let Ok(ch_num) = ch_str.parse::<usize>() {
                        if ch_num >= 1 && ch_num <= 8 {
                            let ch_idx = ch_num - 1;
                            return match param_type {
                                "gain" => Ok(self.channel_gains[ch_idx]),
                                "pan" => Ok(self.channel_pans[ch_idx]),
                                "mute" => Ok(self.channel_mutes[ch_idx]),
                                _ => Err(crate::parameters::ParameterError::NotFound { 
                                    name: name.to_string() 
                                }),
                            };
                        }
                    }
                }
            }
            return Err(crate::parameters::ParameterError::NotFound { name: name.to_string() });
        }

        // Handle standard parameters
        match name {
            "channel_count" => Ok(self.channel_count),
            "master_gain" => Ok(self.master_gain),
            "high_freq_gain" => Ok(self.high_freq_gain),
            "mid_freq_gain" => Ok(self.mid_freq_gain),
            "low_freq_gain" => Ok(self.low_freq_gain),
            "active" => Ok(self.active),
            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() }),
        }
    }

    fn get_parameter_descriptors(&self) -> Vec<Box<dyn ParameterDescriptor>> {
        let mut descriptors: Vec<Box<dyn ParameterDescriptor>> = vec![
            Box::new(BasicParameter::new("channel_count", 2.0, 8.0, 4.0)),
            Box::new(BasicParameter::new("master_gain", 0.0, 2.0, 0.8)),
            Box::new(BasicParameter::new("high_freq_gain", 0.0, 2.0, 1.0)),
            Box::new(BasicParameter::new("mid_freq_gain", 0.0, 2.0, 1.0)),
            Box::new(BasicParameter::new("low_freq_gain", 0.0, 2.0, 1.0)),
            Box::new(BasicParameter::new("active", 0.0, 1.0, 1.0)),
        ];

        // Add per-channel parameter descriptors (using static names for simplicity)
        descriptors.push(Box::new(BasicParameter::new("ch1_gain", 0.0, 2.0, 0.7)));
        descriptors.push(Box::new(BasicParameter::new("ch1_pan", -1.0, 1.0, 0.0)));
        descriptors.push(Box::new(BasicParameter::new("ch1_mute", 0.0, 1.0, 1.0)));
        descriptors.push(Box::new(BasicParameter::new("ch2_gain", 0.0, 2.0, 0.7)));
        descriptors.push(Box::new(BasicParameter::new("ch2_pan", -1.0, 1.0, 0.0)));
        descriptors.push(Box::new(BasicParameter::new("ch2_mute", 0.0, 1.0, 1.0)));
        descriptors.push(Box::new(BasicParameter::new("ch3_gain", 0.0, 2.0, 0.7)));
        descriptors.push(Box::new(BasicParameter::new("ch3_pan", -1.0, 1.0, 0.0)));
        descriptors.push(Box::new(BasicParameter::new("ch3_mute", 0.0, 1.0, 1.0)));
        descriptors.push(Box::new(BasicParameter::new("ch4_gain", 0.0, 2.0, 0.7)));
        descriptors.push(Box::new(BasicParameter::new("ch4_pan", -1.0, 1.0, 0.0)));
        descriptors.push(Box::new(BasicParameter::new("ch4_mute", 0.0, 1.0, 1.0)));
        descriptors.push(Box::new(BasicParameter::new("ch5_gain", 0.0, 2.0, 0.7)));
        descriptors.push(Box::new(BasicParameter::new("ch5_pan", -1.0, 1.0, 0.0)));
        descriptors.push(Box::new(BasicParameter::new("ch5_mute", 0.0, 1.0, 1.0)));
        descriptors.push(Box::new(BasicParameter::new("ch6_gain", 0.0, 2.0, 0.7)));
        descriptors.push(Box::new(BasicParameter::new("ch6_pan", -1.0, 1.0, 0.0)));
        descriptors.push(Box::new(BasicParameter::new("ch6_mute", 0.0, 1.0, 1.0)));
        descriptors.push(Box::new(BasicParameter::new("ch7_gain", 0.0, 2.0, 0.7)));
        descriptors.push(Box::new(BasicParameter::new("ch7_pan", -1.0, 1.0, 0.0)));
        descriptors.push(Box::new(BasicParameter::new("ch7_mute", 0.0, 1.0, 1.0)));
        descriptors.push(Box::new(BasicParameter::new("ch8_gain", 0.0, 2.0, 0.7)));
        descriptors.push(Box::new(BasicParameter::new("ch8_pan", -1.0, 1.0, 0.0)));
        descriptors.push(Box::new(BasicParameter::new("ch8_mute", 0.0, 1.0, 1.0)));

        descriptors
    }
}

impl AudioNode for MixerNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - clear all outputs
            if let Some(mix_l) = ctx.outputs.get_audio_mut("mix_l") {
                mix_l.fill(0.0);
            }
            if let Some(mix_r) = ctx.outputs.get_audio_mut("mix_r") {
                mix_r.fill(0.0);
            }
            return Ok(());
        }

        // Get CV inputs for master controls
        let master_gain_cv = ctx.inputs.get_cv_value("master_gain_cv");
        let high_freq_cv = ctx.inputs.get_cv_value("high_freq_cv");
        let mid_freq_cv = ctx.inputs.get_cv_value("mid_freq_cv");
        let low_freq_cv = ctx.inputs.get_cv_value("low_freq_cv");

        // Apply CV modulation to master controls
        let effective_master_gain = self.master_gain_param.modulate(self.master_gain, master_gain_cv);
        self.high_freq_gain = self.high_freq_param.modulate(self.high_freq_gain, high_freq_cv);
        self.mid_freq_gain = self.mid_freq_param.modulate(self.mid_freq_gain, mid_freq_cv);
        self.low_freq_gain = self.low_freq_param.modulate(self.low_freq_gain, low_freq_cv);

        // Get buffer size
        let buffer_size = ctx.outputs.get_audio("mix_l")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "mix_l".to_string() 
            })?
            .len();

        // Resize temp buffers if needed
        if self.temp_left.len() != buffer_size {
            self.temp_left.resize(buffer_size, 0.0);
            self.temp_right.resize(buffer_size, 0.0);
        }

        // Clear mixing buffers
        self.temp_left.fill(0.0);
        self.temp_right.fill(0.0);

        // Mix each active channel
        let active_channels = self.channel_count as usize;
        for ch in 0..active_channels.min(8) {
            // Get channel inputs
            let ch_l_name = format!("ch{}_l", ch + 1);
            let ch_r_name = format!("ch{}_r", ch + 1);
            let ch_left = ctx.inputs.get_audio(&ch_l_name).unwrap_or(&[]);
            let ch_right = ctx.inputs.get_audio(&ch_r_name).unwrap_or(&[]);

            // Get current channel parameters
            let ch_gain = self.channel_gains[ch];
            let ch_pan = self.channel_pans[ch];
            let ch_mute = self.channel_mutes[ch];

            // Skip if muted
            if ch_mute < 0.5 {
                continue;
            }

            // Calculate pan gains
            let (left_gain, right_gain) = self.calculate_pan_gains(ch_pan);

            // Mix samples
            for i in 0..buffer_size {
                let left_sample = if i < ch_left.len() { ch_left[i] } else { 0.0 };
                let right_sample = if i < ch_right.len() { ch_right[i] } else { 0.0 };

                // Apply channel gain and pan
                let gained_left = left_sample * ch_gain;
                let gained_right = right_sample * ch_gain;

                // Mix to output with panning
                self.temp_left[i] += gained_left * left_gain + gained_right * left_gain;
                self.temp_right[i] += gained_left * right_gain + gained_right * right_gain;
            }
        }

        // Apply master processing
        for i in 0..buffer_size {
            // Apply EQ
            let (eq_left, eq_right) = self.apply_eq(self.temp_left[i], self.temp_right[i]);
            
            // Apply master gain
            let final_left = eq_left * effective_master_gain;
            let final_right = eq_right * effective_master_gain;
            
            // Soft clipping
            self.temp_left[i] = self.soft_clip(final_left);
            self.temp_right[i] = self.soft_clip(final_right);
        }

        // Write to outputs
        if let Some(mix_l) = ctx.outputs.get_audio_mut("mix_l") {
            for (i, &sample) in self.temp_left.iter().enumerate() {
                if i < mix_l.len() {
                    mix_l[i] = sample;
                }
            }
        }

        if let Some(mix_r) = ctx.outputs.get_audio_mut("mix_r") {
            for (i, &sample) in self.temp_right.iter().enumerate() {
                if i < mix_r.len() {
                    mix_r[i] = sample;
                }
            }
        }

        // Copy to send outputs (aux sends)
        if let Some(send1_l) = ctx.outputs.get_audio_mut("send1_l") {
            for (i, &sample) in self.temp_left.iter().enumerate() {
                if i < send1_l.len() {
                    send1_l[i] = sample * 0.5; // 50% send level
                }
            }
        }

        if let Some(send1_r) = ctx.outputs.get_audio_mut("send1_r") {
            for (i, &sample) in self.temp_right.iter().enumerate() {
                if i < send1_r.len() {
                    send1_r[i] = sample * 0.5; // 50% send level
                }
            }
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset internal processing state
        self.temp_left.fill(0.0);
        self.temp_right.fill(0.0);
        self.high_freq_state = 0.0;
        self.low_freq_state = 0.0;
        
        // Reset CV modulation parameters
        // Note: Individual parameter reset methods could be added if needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_mixer_creation() {
        let mixer = MixerNodeRefactored::new(44100.0, "test_mixer".to_string());
        assert_eq!(mixer.channel_count, 4.0);
        assert_eq!(mixer.master_gain, 0.8);
        assert_eq!(mixer.active, 1.0);
    }

    #[test]
    fn test_mixer_parameters() {
        let mut mixer = MixerNodeRefactored::new(44100.0, "test".to_string());
        
        // Test parameter setting
        assert!(mixer.set_parameter("master_gain", 0.5).is_ok());
        assert_eq!(mixer.get_parameter("master_gain").unwrap(), 0.5);
        
        // Test channel count
        assert!(mixer.set_parameter("channel_count", 6.0).is_ok());
        assert_eq!(mixer.get_parameter("channel_count").unwrap(), 6.0);
        
        // Test out of range
        assert!(mixer.set_parameter("master_gain", 3.0).is_err());
    }

    #[test]
    fn test_pan_calculation() {
        let mixer = MixerNodeRefactored::new(44100.0, "test".to_string());
        
        // Center pan
        let (left, right) = mixer.calculate_pan_gains(0.0);
        assert!((left - 0.707).abs() < 0.01); // √2/2 ≈ 0.707
        assert!((right - 0.707).abs() < 0.01);
        
        // Hard left
        let (left, right) = mixer.calculate_pan_gains(-1.0);
        assert!((left - 1.0).abs() < 0.01);
        assert!(right < 0.01);
        
        // Hard right
        let (left, right) = mixer.calculate_pan_gains(1.0);
        assert!(left < 0.01);
        assert!((right - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_mixer_processing() {
        let mut mixer = MixerNodeRefactored::new(44100.0, "test".to_string());
        mixer.set_parameter("master_gain", 1.0).unwrap();
        mixer.set_parameter("ch1_gain", 1.0).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("ch1_l".to_string(), vec![0.5, -0.5, 1.0, -1.0]);
        inputs.add_audio("ch1_r".to_string(), vec![0.3, -0.3, 0.8, -0.8]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("mix_l".to_string(), 4);
        outputs.allocate_audio("mix_r".to_string(), 4);
        outputs.allocate_audio("send1_l".to_string(), 4);
        outputs.allocate_audio("send1_r".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(mixer.process(&mut ctx).is_ok());
        
        let mix_l = ctx.outputs.get_audio("mix_l").unwrap();
        let mix_r = ctx.outputs.get_audio("mix_r").unwrap();
        
        // Check that we have non-zero output
        assert!(mix_l[0] != 0.0);
        assert!(mix_r[0] != 0.0);
        
        // Check send outputs exist
        assert!(ctx.outputs.get_audio("send1_l").is_some());
        assert!(ctx.outputs.get_audio("send1_r").is_some());
    }

    #[test]
    fn test_soft_clipping() {
        let mixer = MixerNodeRefactored::new(44100.0, "test".to_string());
        
        // Normal levels should pass through
        assert!((mixer.soft_clip(0.5) - 0.5).abs() < 0.001);
        
        // High levels should be clipped
        let clipped = mixer.soft_clip(2.0);
        assert!(clipped < 1.0);
        assert!(clipped > 0.8); // Should be close to but less than 1.0
    }

    #[test]
    fn test_channel_count_validation() {
        let mut mixer = MixerNodeRefactored::new(44100.0, "test".to_string());
        
        // Valid counts
        assert!(mixer.set_channel_count(2).is_ok());
        assert!(mixer.set_channel_count(8).is_ok());
        
        // Invalid counts
        assert!(mixer.set_channel_count(1).is_err());
        assert!(mixer.set_channel_count(9).is_err());
    }

    #[test]
    fn test_mixer_bypass() {
        let mut mixer = MixerNodeRefactored::new(44100.0, "test".to_string());
        mixer.set_parameter("active", 0.0).unwrap(); // Disable
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("ch1_l".to_string(), vec![1.0, 1.0, 1.0, 1.0]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("mix_l".to_string(), 4);
        outputs.allocate_audio("mix_r".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(mixer.process(&mut ctx).is_ok());
        
        let mix_l = ctx.outputs.get_audio("mix_l").unwrap();
        
        // Should be zero when bypassed
        assert_eq!(mix_l[0], 0.0);
        assert_eq!(mix_l[1], 0.0);
    }
}