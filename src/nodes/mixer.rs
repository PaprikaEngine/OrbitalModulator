use crate::nodes::AudioNode;
use crate::graph::{Node, Port, PortType};
use std::any::Any;
use std::collections::HashMap;

/// Mixer Node
/// 
/// 複数のオーディオ信号をミックスして単一の出力に合成する
/// 各チャンネルに個別の音量とパンコントロール付き
pub struct MixerNode {
    // 基本ノード情報
    id: String,
    name: String,
    
    // ミキサーパラメーター
    channel_count: usize,    // チャンネル数（4または8）
    channel_gains: Vec<f32>, // 各チャンネルのゲイン (0.0 ~ 1.0)
    channel_pans: Vec<f32>,  // 各チャンネルのパン (-1.0 ~ 1.0, L < 0 < R)
    master_gain: f32,        // マスターゲイン (0.0 ~ 1.0)
    active: bool,
    
    // 内部バッファ
    temp_buffer_l: Vec<f32>,
    temp_buffer_r: Vec<f32>,
}

impl MixerNode {
    pub fn new(id: String, name: String, channel_count: usize) -> Self {
        let channels = channel_count.min(8).max(2); // 2-8チャンネル
        
        Self {
            id,
            name,
            channel_count: channels,
            channel_gains: vec![0.7; channels], // デフォルト70%
            channel_pans: vec![0.0; channels],  // センター
            master_gain: 0.8,
            active: true,
            temp_buffer_l: vec![0.0; 512],
            temp_buffer_r: vec![0.0; 512],
        }
    }
    
    pub fn set_channel_gain(&mut self, channel: usize, gain: f32) {
        if channel < self.channel_count {
            self.channel_gains[channel] = gain.max(0.0).min(1.0);
        }
    }
    
    pub fn set_channel_pan(&mut self, channel: usize, pan: f32) {
        if channel < self.channel_count {
            self.channel_pans[channel] = pan.max(-1.0).min(1.0);
        }
    }
    
    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain.max(0.0).min(1.0);
    }
    
    /// パンニング計算: コンスタントパワー法
    /// pan: -1.0 (L) ~ 0.0 (C) ~ 1.0 (R)
    fn calculate_pan_gains(&self, pan: f32) -> (f32, f32) {
        let angle = (pan + 1.0) * 0.25 * std::f32::consts::PI; // 0 to π/2
        let left_gain = angle.cos();
        let right_gain = angle.sin();
        (left_gain, right_gain)
    }
    
    /// オーディオミキシング処理
    fn mix_audio(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        if !self.active {
            return;
        }
        
        let buffer_size = outputs.values().next().map(|buf| buf.len()).unwrap_or(512);
        
        // バッファサイズ調整
        if self.temp_buffer_l.len() != buffer_size {
            self.temp_buffer_l.resize(buffer_size, 0.0);
            self.temp_buffer_r.resize(buffer_size, 0.0);
        }
        
        // バッファクリア
        self.temp_buffer_l.fill(0.0);
        self.temp_buffer_r.fill(0.0);
        
        // 各チャンネルをミックス
        for ch in 0..self.channel_count {
            let input_key = format!("audio_in_{}", ch + 1);
            if let Some(input_buffer) = inputs.get(&input_key) {
                let channel_gain = self.channel_gains[ch];
                let channel_pan = self.channel_pans[ch];
                let (left_gain, right_gain) = self.calculate_pan_gains(channel_pan);
                
                let final_left_gain = channel_gain * left_gain;
                let final_right_gain = channel_gain * right_gain;
                
                // ミックス処理
                for (i, &input_sample) in input_buffer.iter().enumerate() {
                    if i >= buffer_size { break; }
                    
                    self.temp_buffer_l[i] += input_sample * final_left_gain;
                    self.temp_buffer_r[i] += input_sample * final_right_gain;
                }
            }
        }
        
        // マスターゲイン適用と出力
        if let Some(output_l) = outputs.get_mut("audio_out_l") {
            for (i, sample) in output_l.iter_mut().enumerate() {
                if i >= buffer_size { break; }
                *sample = self.temp_buffer_l[i] * self.master_gain;
            }
        }
        
        if let Some(output_r) = outputs.get_mut("audio_out_r") {
            for (i, sample) in output_r.iter_mut().enumerate() {
                if i >= buffer_size { break; }
                *sample = self.temp_buffer_r[i] * self.master_gain;
            }
        }
    }
}

impl AudioNode for MixerNode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        self.mix_audio(inputs, outputs);
    }
    
    fn create_node_info(&self, name: String) -> Node {
        let mut input_ports = Vec::new();
        
        // オーディオ入力ポート
        for i in 1..=self.channel_count {
            input_ports.push(Port {
                name: format!("audio_in_{}", i),
                port_type: PortType::AudioMono,
            });
        }
        
        // CV入力ポート（各チャンネルのゲインとパン）
        for i in 1..=self.channel_count {
            input_ports.push(Port {
                name: format!("gain_cv_{}", i),
                port_type: PortType::CV,
            });
            input_ports.push(Port {
                name: format!("pan_cv_{}", i),
                port_type: PortType::CV,
            });
        }
        
        // マスターCV入力
        input_ports.push(Port {
            name: "master_gain_cv".to_string(),
            port_type: PortType::CV,
        });
        
        let output_ports = vec![
            Port { name: "audio_out_l".to_string(), port_type: PortType::AudioMono },
            Port { name: "audio_out_r".to_string(), port_type: PortType::AudioMono },
        ];
        
        Node {
            id: uuid::Uuid::new_v4(),
            name,
            node_type: "mixer".to_string(),
            parameters: self.get_parameters(),
            input_ports,
            output_ports,
        }
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl MixerNode {
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        if param == "master_gain" {
            self.set_master_gain(value);
            return Ok(());
        }
        
        if param == "active" {
            self.active = value != 0.0;
            return Ok(());
        }
        
        // チャンネル固有のパラメーター解析
        if param.starts_with("gain_") {
            if let Ok(channel) = param[5..].parse::<usize>() {
                if channel > 0 && channel <= self.channel_count {
                    self.set_channel_gain(channel - 1, value);
                    return Ok(());
                }
            }
        }
        
        if param.starts_with("pan_") {
            if let Ok(channel) = param[4..].parse::<usize>() {
                if channel > 0 && channel <= self.channel_count {
                    self.set_channel_pan(channel - 1, value);
                    return Ok(());
                }
            }
        }
        
        Err(format!("Unknown parameter: {}", param))
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        if param == "master_gain" {
            return Ok(self.master_gain);
        }
        
        if param == "active" {
            return Ok(if self.active { 1.0 } else { 0.0 });
        }
        
        if param.starts_with("gain_") {
            if let Ok(channel) = param[5..].parse::<usize>() {
                if channel > 0 && channel <= self.channel_count {
                    return Ok(self.channel_gains[channel - 1]);
                }
            }
        }
        
        if param.starts_with("pan_") {
            if let Ok(channel) = param[4..].parse::<usize>() {
                if channel > 0 && channel <= self.channel_count {
                    return Ok(self.channel_pans[channel - 1]);
                }
            }
        }
        
        Err(format!("Unknown parameter: {}", param))
    }
    
    pub fn get_parameters(&self) -> HashMap<String, f32> {
        let mut params = HashMap::new();
        
        params.insert("master_gain".to_string(), self.master_gain);
        params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
        
        // 各チャンネルのパラメーター
        for i in 0..self.channel_count {
            params.insert(format!("gain_{}", i + 1), self.channel_gains[i]);
            params.insert(format!("pan_{}", i + 1), self.channel_pans[i]);
        }
        
        params
    }
}