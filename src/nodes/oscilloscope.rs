use std::collections::{VecDeque, HashMap};
use std::sync::{Arc, Mutex};
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum TriggerMode {
    Auto,    // 自動トリガー（信号なしでも表示）
    Normal,  // 条件満たした時のみ表示
    Single,  // 1回のみトリガー
}

#[derive(Debug, Clone)]
pub enum TriggerSlope {
    Rising,  // 立ち上がりエッジ
    Falling, // 立ち下がりエッジ
}

#[derive(Debug, Clone)]
pub struct Measurements {
    pub vpp: f32,        // Peak-to-Peak電圧
    pub vrms: f32,       // RMS電圧  
    pub frequency: f32,  // 周波数
    pub period: f32,     // 周期
    pub duty_cycle: f32, // デューティサイクル
}

impl Default for Measurements {
    fn default() -> Self {
        Self {
            vpp: 0.0,
            vrms: 0.0,
            frequency: 0.0,
            period: 0.0,
            duty_cycle: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct TriggerSystem {
    mode: TriggerMode,
    level: f32,
    slope: TriggerSlope,
    triggered: bool,
    last_sample: f32,
    pre_trigger_buffer: VecDeque<f32>,
    pre_trigger_size: usize,
}

impl TriggerSystem {
    pub fn new() -> Self {
        Self {
            mode: TriggerMode::Auto,
            level: 0.0,
            slope: TriggerSlope::Rising,
            triggered: false,
            last_sample: 0.0,
            pre_trigger_buffer: VecDeque::new(),
            pre_trigger_size: 256, // プリトリガーサンプル数
        }
    }

    pub fn set_mode(&mut self, mode: TriggerMode) {
        self.mode = mode;
    }

    pub fn set_level(&mut self, level: f32) {
        self.level = level;
    }

    pub fn set_slope(&mut self, slope: TriggerSlope) {
        self.slope = slope;
    }

    pub fn process_sample(&mut self, sample: f32) -> bool {
        // プリトリガーバッファに追加
        if self.pre_trigger_buffer.len() >= self.pre_trigger_size {
            self.pre_trigger_buffer.pop_front();
        }
        self.pre_trigger_buffer.push_back(sample);

        let trigger_detected = match self.slope {
            TriggerSlope::Rising => {
                self.last_sample < self.level && sample >= self.level
            }
            TriggerSlope::Falling => {
                self.last_sample > self.level && sample <= self.level
            }
        };

        self.last_sample = sample;

        match self.mode {
            TriggerMode::Auto => true, // 常にトリガー
            TriggerMode::Normal => trigger_detected,
            TriggerMode::Single => {
                if trigger_detected && !self.triggered {
                    self.triggered = true;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn reset_single_trigger(&mut self) {
        if matches!(self.mode, TriggerMode::Single) {
            self.triggered = false;
        }
    }

    pub fn get_pre_trigger_data(&self) -> Vec<f32> {
        self.pre_trigger_buffer.iter().cloned().collect()
    }
}

pub struct OscilloscopeNode {
    // 基本ノード情報
    id: String,
    name: String,
    
    // オシロスコープパラメーター
    time_div: f32,      // Time/Div (秒)
    volt_div: f32,      // Volt/Div
    position_h: f32,    // 水平位置 (-50% to +50%)
    position_v: f32,    // 垂直位置 (-50% to +50%)
    
    // トリガーシステム
    trigger: TriggerSystem,
    
    // 波形データバッファ
    waveform_buffer: VecDeque<f32>,
    buffer_size: usize,
    sample_rate: f32,
    
    // 測定データ
    measurements: Measurements,
    measurement_counter: usize,
    measurement_interval: usize, // 測定更新間隔
    
    // フロントエンド共有データ
    shared_waveform: Arc<Mutex<Vec<f32>>>,
    shared_measurements: Arc<Mutex<Measurements>>,
    
    // 入出力ポート - 削除（AudioNodeトレイトで管理）
}

impl OscilloscopeNode {
    pub fn new(id: String, name: String) -> Self {
        let buffer_size = 2048; // 約46ms @ 44.1kHz
        
        Self {
            id,
            name,
            time_div: 0.01,     // 10ms/div
            volt_div: 1.0,      // 1V/div
            position_h: 0.0,
            position_v: 0.0,
            trigger: TriggerSystem::new(),
            waveform_buffer: VecDeque::with_capacity(buffer_size),
            buffer_size,
            sample_rate: 44100.0,
            measurements: Measurements::default(),
            measurement_counter: 0,
            measurement_interval: 1024, // 測定を1024サンプルごとに更新
            shared_waveform: Arc::new(Mutex::new(Vec::new())),
            shared_measurements: Arc::new(Mutex::new(Measurements::default())),
        }
    }

    pub fn set_time_div(&mut self, time_div: f32) {
        self.time_div = time_div.clamp(0.0001, 0.1); // 0.1ms ~ 100ms
        
        // バッファサイズを時間分解能に応じて調整
        let samples_needed = (self.time_div * 10.0 * self.sample_rate) as usize; // 10 divisions
        self.buffer_size = samples_needed.clamp(512, 8192);
        
        // バッファを新しいサイズでリサイズ
        while self.waveform_buffer.len() > self.buffer_size {
            self.waveform_buffer.pop_front();
        }
    }

    pub fn set_volt_div(&mut self, volt_div: f32) {
        self.volt_div = volt_div.clamp(0.1, 10.0);
    }

    pub fn set_position(&mut self, h: f32, v: f32) {
        self.position_h = h.clamp(-0.5, 0.5);
        self.position_v = v.clamp(-0.5, 0.5);
    }

    pub fn set_trigger_mode(&mut self, mode: TriggerMode) {
        self.trigger.set_mode(mode);
    }

    pub fn set_trigger_level(&mut self, level: f32) {
        self.trigger.set_level(level);
    }

    pub fn set_trigger_slope(&mut self, slope: TriggerSlope) {
        self.trigger.set_slope(slope);
    }

    pub fn reset_trigger(&mut self) {
        self.trigger.reset_single_trigger();
    }

    fn calculate_measurements(&mut self) {
        if self.waveform_buffer.is_empty() {
            return;
        }

        let samples: Vec<f32> = self.waveform_buffer.iter().cloned().collect();
        
        // Peak-to-Peak電圧
        let min_val = samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_val = samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        self.measurements.vpp = max_val - min_val;
        
        // RMS電圧
        let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
        self.measurements.vrms = (sum_squares / samples.len() as f32).sqrt();
        
        // 周波数測定（簡易的なゼロクロッシング検出）
        let mut zero_crossings = 0;
        let mut last_sign = samples[0] >= 0.0;
        
        for &sample in &samples[1..] {
            let current_sign = sample >= 0.0;
            if current_sign != last_sign {
                zero_crossings += 1;
            }
            last_sign = current_sign;
        }
        
        if zero_crossings > 0 {
            let duration = samples.len() as f32 / self.sample_rate;
            self.measurements.frequency = (zero_crossings as f32 / 2.0) / duration;
            self.measurements.period = 1.0 / self.measurements.frequency;
        }
        
        // 共有データを更新
        if let Ok(mut shared) = self.shared_measurements.lock() {
            *shared = self.measurements.clone();
        }
    }

    pub fn get_waveform_data(&self) -> Arc<Mutex<Vec<f32>>> {
        self.shared_waveform.clone()
    }

    pub fn get_measurements(&self) -> Arc<Mutex<Measurements>> {
        self.shared_measurements.clone()
    }
}

impl AudioNode for OscilloscopeNode {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        // 入力データを取得
        let audio_input = inputs.get("audio_in").copied().unwrap_or(&[]);
        
        // 出力バッファを取得
        if let Some(audio_output) = outputs.get_mut("audio_out") {
            // 入力をそのまま出力に通す（パススルー）
            let len = audio_input.len().min(audio_output.len());
            audio_output[..len].copy_from_slice(&audio_input[..len]);

            // 波形データを処理
            for &sample in &audio_input[..len] {
                // トリガー処理
                let should_capture = self.trigger.process_sample(sample);
                
                if should_capture {
                    // バッファサイズ制限
                    if self.waveform_buffer.len() >= self.buffer_size {
                        self.waveform_buffer.pop_front();
                    }
                    self.waveform_buffer.push_back(sample);
                }
            }

            // 測定値更新（間隔制御）
            self.measurement_counter += len;
            if self.measurement_counter >= self.measurement_interval {
                self.calculate_measurements();
                self.measurement_counter = 0;
                
                // 共有波形データ更新
                let waveform_data: Vec<f32> = self.waveform_buffer.iter().cloned().collect();
                if let Ok(mut shared) = self.shared_waveform.lock() {
                    *shared = waveform_data;
                }
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        let mut parameters = HashMap::new();
        parameters.insert("time_div".to_string(), self.time_div);
        parameters.insert("volt_div".to_string(), self.volt_div);
        parameters.insert("position_h".to_string(), self.position_h);
        parameters.insert("position_v".to_string(), self.position_v);
        parameters.insert("trigger_level".to_string(), self.trigger.level);

        Node {
            id: Uuid::new_v4(),
            node_type: "oscilloscope".to_string(),
            name,
            parameters,
            input_ports: vec![
                Port {
                    name: "audio_in".to_string(),
                    port_type: PortType::AudioMono,
                },
            ],
            output_ports: vec![
                Port {
                    name: "audio_out".to_string(),
                    port_type: PortType::AudioMono,
                },
            ],
        }
    }
}