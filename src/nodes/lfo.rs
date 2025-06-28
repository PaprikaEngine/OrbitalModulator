use crate::nodes::AudioNode;
use crate::graph::{Node, Port, PortType};
use std::any::Any;
use std::collections::HashMap;

/// LFO (Low Frequency Oscillator) Node
/// 
/// Generates low-frequency control voltages for modulation.
/// Typical frequency range: 0.1Hz to 20Hz
pub struct LFONode {
    // 基本ノード情報
    id: String,
    name: String,
    
    // LFOパラメーター
    frequency: f32,      // 0.1Hz ~ 20Hz
    amplitude: f32,      // 0.0 ~ 1.0 (CV出力の振幅)
    waveform: LFOWaveform,
    phase_offset: f32,   // 0.0 ~ 1.0 (位相オフセット)
    active: bool,
    
    // 内部状態
    phase: f32,          // 現在の位相 (0.0 ~ 1.0)
    sample_rate: f32,
    random_value: f32,   // Sample & Hold用のランダム値
    last_phase: f32,     // 前フレームの位相
}

#[derive(Clone, Copy, Debug)]
pub enum LFOWaveform {
    Sine,
    Triangle,
    Sawtooth,
    Square,
    Random,  // Sample & Hold
}

impl LFONode {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            frequency: 1.0,        // 1Hz デフォルト
            amplitude: 1.0,
            waveform: LFOWaveform::Sine,
            phase_offset: 0.0,
            active: true,
            phase: 0.0,
            sample_rate: 44100.0,
            random_value: 0.0,
            last_phase: 0.0,
        }
    }
    
    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq.max(0.01).min(20.0);  // 0.01Hz ~ 20Hz
    }
    
    pub fn set_amplitude(&mut self, amp: f32) {
        self.amplitude = amp.max(0.0).min(1.0);
    }
    
    pub fn set_waveform(&mut self, waveform: LFOWaveform) {
        self.waveform = waveform;
    }
    
    pub fn set_phase_offset(&mut self, offset: f32) {
        self.phase_offset = offset.max(0.0).min(1.0);
    }
    
    fn generate_lfo_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        
        // 位相進行
        let phase_increment = self.frequency / self.sample_rate;
        self.phase += phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        // 位相オフセット適用
        let adjusted_phase = (self.phase + self.phase_offset) % 1.0;
        
        // 波形生成
        let raw_value = match self.waveform {
            LFOWaveform::Sine => {
                (adjusted_phase * 2.0 * std::f32::consts::PI).sin()
            },
            LFOWaveform::Triangle => {
                if adjusted_phase < 0.5 {
                    4.0 * adjusted_phase - 1.0
                } else {
                    3.0 - 4.0 * adjusted_phase
                }
            },
            LFOWaveform::Sawtooth => {
                2.0 * adjusted_phase - 1.0
            },
            LFOWaveform::Square => {
                if adjusted_phase < 0.5 { 1.0 } else { -1.0 }
            },
            LFOWaveform::Random => {
                // Sample & Hold - 新しいランダム値を周期的に生成
                if self.last_phase > adjusted_phase || (self.last_phase < 0.1 && adjusted_phase > 0.9) {
                    // 新しい周期の開始 - 簡単な疑似ランダム
                    let seed = (self.phase * 12345.0) as u32;
                    self.random_value = ((seed.wrapping_mul(1103515245).wrapping_add(12345) >> 16) as f32 / 32768.0) * 2.0 - 1.0;
                }
                self.last_phase = adjusted_phase;
                self.random_value
            },
        };
        
        // 振幅スケーリング
        raw_value * self.amplitude
    }
}

impl AudioNode for LFONode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        // CV入力の処理
        let mut frequency_cv = 0.0;
        let mut amplitude_cv = 0.0;
        
        if let Some(freq_cv) = inputs.get("frequency_cv") {
            if !freq_cv.is_empty() {
                frequency_cv = freq_cv[0];
            }
        }
        
        if let Some(amp_cv) = inputs.get("amplitude_cv") {
            if !amp_cv.is_empty() {
                amplitude_cv = amp_cv[0];
            }
        }
        
        // CV入力によるパラメーター変調
        let modulated_frequency = self.frequency + frequency_cv;
        let modulated_amplitude = (self.amplitude + amplitude_cv).max(0.0).min(1.0);
        
        // 一時的にパラメーター変更
        let original_freq = self.frequency;
        let original_amp = self.amplitude;
        self.frequency = modulated_frequency.max(0.01).min(20.0);
        self.amplitude = modulated_amplitude;
        
        // CV出力の生成
        if let Some(cv_out) = outputs.get_mut("cv_out") {
            let lfo_value = self.generate_lfo_sample();
            for sample in cv_out.iter_mut() {
                *sample = lfo_value;
            }
        }
        
        // パラメーターを元に戻す
        self.frequency = original_freq;
        self.amplitude = original_amp;
    }
    
    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: uuid::Uuid::new_v4(),
            name,
            node_type: "lfo".to_string(),
            parameters: self.get_parameters(),
            input_ports: vec![
                Port { name: "frequency_cv".to_string(), port_type: PortType::CV },
                Port { name: "amplitude_cv".to_string(), port_type: PortType::CV },
            ],
            output_ports: vec![
                Port { name: "cv_out".to_string(), port_type: PortType::CV },
            ],
        }
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl LFONode {
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "frequency" => {
                self.set_frequency(value);
                Ok(())
            },
            "amplitude" => {
                self.set_amplitude(value);
                Ok(())
            },
            "waveform" => {
                let waveform = match value as u8 {
                    0 => LFOWaveform::Sine,
                    1 => LFOWaveform::Triangle,
                    2 => LFOWaveform::Sawtooth,
                    3 => LFOWaveform::Square,
                    4 => LFOWaveform::Random,
                    _ => LFOWaveform::Sine,
                };
                self.set_waveform(waveform);
                Ok(())
            },
            "phase_offset" => {
                self.set_phase_offset(value);
                Ok(())
            },
            "active" => {
                self.active = value != 0.0;
                Ok(())
            },
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "frequency" => Ok(self.frequency),
            "amplitude" => Ok(self.amplitude),
            "waveform" => Ok(self.waveform as u8 as f32),
            "phase_offset" => Ok(self.phase_offset),
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    pub fn get_parameters(&self) -> HashMap<String, f32> {
        let mut params = HashMap::new();
        params.insert("frequency".to_string(), self.frequency);
        params.insert("amplitude".to_string(), self.amplitude);
        params.insert("waveform".to_string(), self.waveform as u8 as f32);
        params.insert("phase_offset".to_string(), self.phase_offset);
        params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
        params
    }
}