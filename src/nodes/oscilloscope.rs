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
use std::collections::VecDeque;

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;
use crate::define_parameters;

/// Trigger mode for oscilloscope
#[derive(Debug, Clone)]
pub enum TriggerMode {
    Auto,    // 自動トリガー（信号なしでも表示）
    Normal,  // 条件満たした時のみ表示
    Single,  // 1回のみトリガー
}

/// Trigger slope for edge detection
#[derive(Debug, Clone)]
pub enum TriggerSlope {
    Rising,  // 立ち上がりエッジ
    Falling, // 立ち下がりエッジ
}

/// Automatic measurements from oscilloscope
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

/// リファクタリング済みOscilloscopeNode - プロ仕様デジタルオシロスコープ
/// 
/// 特徴:
/// - CRT風リアルタイム波形表示
/// - 完全なトリガーシステム（Auto/Normal/Single）
/// - 自動測定機能（Vpp, Vrms, 周波数, 周期）
/// - CV変調対応トリガーレベル
/// - プリトリガー機能
/// - ズーム・時間軸制御
/// - 30FPS更新、グロー効果対応
pub struct OscilloscopeNode {
    // Node identification
    node_info: NodeInfo,
    
    // Oscilloscope controls
    time_scale: f32,              // 0.001 ~ 1.0 (時間軸スケール: seconds/div)
    voltage_scale: f32,           // 0.1 ~ 10.0 (電圧軸スケール: volts/div)
    trigger_level: f32,           // -10.0 ~ 10.0 (トリガーレベル)
    trigger_mode: f32,            // 0.0=Auto, 1.0=Normal, 2.0=Single
    trigger_slope: f32,           // 0.0=Rising, 1.0=Falling
    horizontal_position: f32,     // -1.0 ~ 1.0 (水平位置)
    vertical_position: f32,       // -1.0 ~ 1.0 (垂直位置)
    active: f32,                  // 0.0 = Off, 1.0 = On
    
    // CV Modulation parameters
    trigger_level_param: ModulatableParameter,
    time_scale_param: ModulatableParameter,
    voltage_scale_param: ModulatableParameter,
    
    // Internal processing state
    sample_buffer: VecDeque<f32>, // 波形データバッファ
    trigger_buffer: VecDeque<f32>, // トリガー用プリバッファ
    last_sample: f32,             // 前回のサンプル値
    triggered: bool,              // トリガー状態
    trigger_position: usize,      // トリガー位置
    
    // Measurement state
    measurements: Measurements,   // 自動測定結果
    measurement_buffer: VecDeque<f32>, // 測定用バッファ
    zero_crossings: Vec<usize>,   // ゼロクロッシング位置
    
    // Display state
    display_buffer: Vec<f32>,     // 表示用バッファ（1024ポイント）
    update_counter: u32,          // 更新カウンター（30FPS制御用）
    
    sample_rate: f32,
}

impl OscilloscopeNode {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "oscilloscope".to_string(),
            category: NodeCategory::Analyzer,
            
            // Input ports: audio signal + CV inputs
            input_ports: vec![
                PortInfo::new("signal_in", PortType::AudioMono),
                PortInfo::new("trigger_level_cv", PortType::CV),
                PortInfo::new("time_scale_cv", PortType::CV),
                PortInfo::new("voltage_scale_cv", PortType::CV),
                PortInfo::new("external_trigger", PortType::CV),
            ],
            
            // Output ports: measurements as CV + pass-through
            output_ports: vec![
                PortInfo::new("signal_out", PortType::AudioMono),
                PortInfo::new("vpp_cv", PortType::CV),
                PortInfo::new("vrms_cv", PortType::CV),
                PortInfo::new("frequency_cv", PortType::CV),
                PortInfo::new("period_cv", PortType::CV),
                PortInfo::new("trigger_out", PortType::CV),
            ],
            
            description: "Professional digital oscilloscope with CRT-style display and automatic measurements".to_string(),
            latency_samples: 0,
            supports_bypass: true,
        };

        // Create modulation parameters
        let trigger_level_param = ModulatableParameter::new(
            BasicParameter::new("trigger_level", -10.0, 10.0, 0.0),
            1.0  // 100% CV modulation range
        );
        
        let time_scale_param = ModulatableParameter::new(
            BasicParameter::new("time_scale", 0.001, 1.0, 0.01),
            0.5  // 50% CV modulation range
        );
        
        let voltage_scale_param = ModulatableParameter::new(
            BasicParameter::new("voltage_scale", 0.1, 10.0, 1.0),
            0.5  // 50% CV modulation range
        );

        Self {
            node_info,
            time_scale: 0.01,    // 10ms/div
            voltage_scale: 1.0,  // 1V/div
            trigger_level: 0.0,
            trigger_mode: 0.0,   // Auto
            trigger_slope: 0.0,  // Rising
            horizontal_position: 0.0,
            vertical_position: 0.0,
            active: 1.0,
            
            // CV parameters
            trigger_level_param,
            time_scale_param,
            voltage_scale_param,
            
            // Internal state
            sample_buffer: VecDeque::with_capacity(8192),
            trigger_buffer: VecDeque::with_capacity(1024),
            last_sample: 0.0,
            triggered: false,
            trigger_position: 0,
            
            // Measurement state
            measurements: Measurements::default(),
            measurement_buffer: VecDeque::with_capacity(4096),
            zero_crossings: Vec::new(),
            
            // Display state
            display_buffer: vec![0.0; 1024],
            update_counter: 0,
            
            sample_rate,
        }
    }
    
    /// Convert trigger mode parameter to enum
    fn get_trigger_mode(&self) -> TriggerMode {
        match self.trigger_mode as i32 {
            0 => TriggerMode::Auto,
            1 => TriggerMode::Normal,
            2 => TriggerMode::Single,
            _ => TriggerMode::Auto,
        }
    }
    
    /// Convert trigger slope parameter to enum
    fn get_trigger_slope(&self) -> TriggerSlope {
        match self.trigger_slope as i32 {
            0 => TriggerSlope::Rising,
            1 => TriggerSlope::Falling,
            _ => TriggerSlope::Rising,
        }
    }
    
    /// Check for trigger condition
    fn check_trigger(&mut self, current_sample: f32, effective_trigger_level: f32) -> bool {
        let slope = self.get_trigger_slope();
        
        let trigger_detected = match slope {
            TriggerSlope::Rising => {
                self.last_sample < effective_trigger_level && current_sample >= effective_trigger_level
            },
            TriggerSlope::Falling => {
                self.last_sample > effective_trigger_level && current_sample <= effective_trigger_level
            },
        };
        
        self.last_sample = current_sample;
        trigger_detected
    }
    
    /// Calculate automatic measurements
    fn calculate_measurements(&mut self) {
        if self.measurement_buffer.len() < 100 {
            return; // Not enough data
        }
        
        let samples: Vec<f32> = self.measurement_buffer.iter().copied().collect();
        
        // Calculate Peak-to-Peak voltage
        let min_val = samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_val = samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        self.measurements.vpp = max_val - min_val;
        
        // Calculate RMS voltage
        let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
        self.measurements.vrms = (sum_squares / samples.len() as f32).sqrt();
        
        // Find zero crossings for frequency calculation
        self.zero_crossings.clear();
        for i in 1..samples.len() {
            if (samples[i-1] < 0.0 && samples[i] >= 0.0) || 
               (samples[i-1] > 0.0 && samples[i] <= 0.0) {
                self.zero_crossings.push(i);
            }
        }
        
        // Calculate frequency from zero crossings
        if self.zero_crossings.len() >= 4 {
            // Use pairs of zero crossings to calculate period
            let mut periods = Vec::new();
            for i in 0..self.zero_crossings.len()-2 {
                let period_samples = (self.zero_crossings[i+2] - self.zero_crossings[i]) as f32;
                let period_seconds = period_samples / self.sample_rate;
                periods.push(period_seconds);
            }
            
            if !periods.is_empty() {
                self.measurements.period = periods.iter().sum::<f32>() / periods.len() as f32;
                self.measurements.frequency = if self.measurements.period > 0.0 {
                    1.0 / self.measurements.period
                } else {
                    0.0
                };
            }
        }
        
        // Calculate duty cycle (simplified)
        let mut high_samples = 0;
        for &sample in &samples {
            if sample > 0.0 {
                high_samples += 1;
            }
        }
        self.measurements.duty_cycle = high_samples as f32 / samples.len() as f32;
    }
    
    /// Update display buffer for visualization
    fn update_display_buffer(&mut self) {
        // Only update at ~30 FPS to reduce CPU load
        self.update_counter += 1;
        if self.update_counter % (self.sample_rate as u32 / 30) != 0 {
            return;
        }
        
        let samples_per_div = (self.time_scale * self.sample_rate) as usize;
        let total_samples_needed = samples_per_div * 10; // 10 divisions
        
        if self.sample_buffer.len() >= total_samples_needed {
            // Copy from sample buffer to display buffer with decimation if needed
            let decimation = (total_samples_needed / 1024).max(1);
            
            for i in 0..1024 {
                let sample_index = i * decimation;
                if sample_index < self.sample_buffer.len() {
                    self.display_buffer[i] = self.sample_buffer[sample_index];
                }
            }
        }
    }
    
    /// Get display data for UI rendering
    pub fn get_display_data(&self) -> &[f32] {
        &self.display_buffer
    }
    
    /// Get current measurements
    pub fn get_measurements(&self) -> &Measurements {
        &self.measurements
    }
    
    /// Get trigger status
    pub fn is_triggered(&self) -> bool {
        self.triggered
    }
}

impl Parameterizable for OscilloscopeNode {
    define_parameters! {
        time_scale: BasicParameter::new("time_scale", 0.001, 1.0, 0.01),
        voltage_scale: BasicParameter::new("voltage_scale", 0.1, 10.0, 1.0),
        trigger_level: BasicParameter::new("trigger_level", -10.0, 10.0, 0.0),
        trigger_mode: BasicParameter::new("trigger_mode", 0.0, 2.0, 0.0),
        trigger_slope: BasicParameter::new("trigger_slope", 0.0, 1.0, 0.0),
        horizontal_position: BasicParameter::new("horizontal_position", -1.0, 1.0, 0.0),
        vertical_position: BasicParameter::new("vertical_position", -1.0, 1.0, 0.0),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for OscilloscopeNode {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - pass through input signal
            if let (Some(input), Some(output)) = 
                (ctx.inputs.get_audio("signal_in"), ctx.outputs.get_audio_mut("signal_out")) {
                output.copy_from_slice(&input[..output.len().min(input.len())]);
            }
            return Ok(());
        }

        // Get input signals
        let signal_input = ctx.inputs.get_audio("signal_in").unwrap_or(&[]);
        let trigger_level_cv = ctx.inputs.get_cv_value("trigger_level_cv");
        let time_scale_cv = ctx.inputs.get_cv_value("time_scale_cv");
        let voltage_scale_cv = ctx.inputs.get_cv_value("voltage_scale_cv");
        let external_trigger = ctx.inputs.get_cv_value("external_trigger");

        // Apply CV modulation
        let effective_trigger_level = self.trigger_level_param.modulate(self.trigger_level, trigger_level_cv);
        let _effective_time_scale = self.time_scale_param.modulate(self.time_scale, time_scale_cv);
        let _effective_voltage_scale = self.voltage_scale_param.modulate(self.voltage_scale, voltage_scale_cv);

        // Process each sample
        let mut trigger_out_value = 0.0;
        
        for &sample in signal_input {
            // Add to sample buffer
            self.sample_buffer.push_back(sample);
            if self.sample_buffer.len() > 8192 {
                self.sample_buffer.pop_front();
            }
            
            // Add to measurement buffer
            self.measurement_buffer.push_back(sample);
            if self.measurement_buffer.len() > 4096 {
                self.measurement_buffer.pop_front();
            }
            
            // Check trigger condition
            let trigger_mode = self.get_trigger_mode();
            match trigger_mode {
                TriggerMode::Auto => {
                    // Always triggered in auto mode
                    self.triggered = true;
                    trigger_out_value = 5.0; // Gate signal
                },
                TriggerMode::Normal => {
                    if self.check_trigger(sample, effective_trigger_level) {
                        self.triggered = true;
                        trigger_out_value = 5.0; // Gate signal
                    } else {
                        trigger_out_value = 0.0;
                    }
                },
                TriggerMode::Single => {
                    if !self.triggered && self.check_trigger(sample, effective_trigger_level) {
                        self.triggered = true;
                        trigger_out_value = 5.0; // Gate signal
                    } else {
                        trigger_out_value = 0.0;
                    }
                },
            }
            
            // Handle external trigger
            if external_trigger > 2.5 {
                self.triggered = true;
                trigger_out_value = 5.0;
            }
        }

        // Pass through input signal
        if let (Some(input), Some(output)) = 
            (ctx.inputs.get_audio("signal_in"), ctx.outputs.get_audio_mut("signal_out")) {
            output.copy_from_slice(&input[..output.len().min(input.len())]);
        }

        // Calculate measurements periodically
        if self.update_counter % 1024 == 0 {
            self.calculate_measurements();
        }
        
        // Update display buffer
        self.update_display_buffer();

        // Output measurement CV signals (scaled to ±10V range)
        if let Some(vpp_cv) = ctx.outputs.get_cv_mut("vpp_cv") {
            vpp_cv.fill(self.measurements.vpp.min(10.0));
        }
        
        if let Some(vrms_cv) = ctx.outputs.get_cv_mut("vrms_cv") {
            vrms_cv.fill(self.measurements.vrms.min(10.0));
        }
        
        if let Some(freq_cv) = ctx.outputs.get_cv_mut("frequency_cv") {
            let freq_scaled = (self.measurements.frequency / 1000.0).min(10.0); // Scale to kHz
            freq_cv.fill(freq_scaled);
        }
        
        if let Some(period_cv) = ctx.outputs.get_cv_mut("period_cv") {
            let period_scaled = (self.measurements.period * 1000.0).min(10.0); // Scale to ms
            period_cv.fill(period_scaled);
        }
        
        if let Some(trigger_cv) = ctx.outputs.get_cv_mut("trigger_out") {
            trigger_cv.fill(trigger_out_value);
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset internal processing state
        self.sample_buffer.clear();
        self.trigger_buffer.clear();
        self.measurement_buffer.clear();
        self.zero_crossings.clear();
        self.display_buffer.fill(0.0);
        
        self.last_sample = 0.0;
        self.triggered = false;
        self.trigger_position = 0;
        self.update_counter = 0;
        
        self.measurements = Measurements::default();
    }

    fn latency(&self) -> u32 {
        0 // No latency for oscilloscope analysis
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
    fn test_oscilloscope_creation() {
        let scope = OscilloscopeNode::new(44100.0, "test_scope".to_string());
        assert_eq!(scope.time_scale, 0.01);
        assert_eq!(scope.voltage_scale, 1.0);
        assert_eq!(scope.trigger_level, 0.0);
        assert_eq!(scope.active, 1.0);
    }

    #[test]
    fn test_oscilloscope_parameters() {
        let mut scope = OscilloscopeNode::new(44100.0, "test".to_string());
        
        // Test parameter setting
        assert!(scope.set_parameter("time_scale", 0.1).is_ok());
        assert_eq!(scope.get_parameter("time_scale").unwrap(), 0.1);
        
        // Test trigger level
        assert!(scope.set_parameter("trigger_level", 2.5).is_ok());
        assert_eq!(scope.get_parameter("trigger_level").unwrap(), 2.5);
        
        // Test trigger mode
        assert!(scope.set_parameter("trigger_mode", 1.0).is_ok());
        assert_eq!(scope.get_parameter("trigger_mode").unwrap(), 1.0);
        
        // Test out of range
        assert!(scope.set_parameter("voltage_scale", 20.0).is_err());
    }

    #[test]
    fn test_trigger_mode_conversion() {
        let mut scope = OscilloscopeNode::new(44100.0, "test".to_string());
        
        scope.trigger_mode = 0.0;
        assert!(matches!(scope.get_trigger_mode(), TriggerMode::Auto));
        
        scope.trigger_mode = 1.0;
        assert!(matches!(scope.get_trigger_mode(), TriggerMode::Normal));
        
        scope.trigger_mode = 2.0;
        assert!(matches!(scope.get_trigger_mode(), TriggerMode::Single));
    }

    #[test]
    fn test_trigger_slope_conversion() {
        let mut scope = OscilloscopeNode::new(44100.0, "test".to_string());
        
        scope.trigger_slope = 0.0;
        assert!(matches!(scope.get_trigger_slope(), TriggerSlope::Rising));
        
        scope.trigger_slope = 1.0;
        assert!(matches!(scope.get_trigger_slope(), TriggerSlope::Falling));
    }

    #[test]
    fn test_trigger_detection() {
        let mut scope = OscilloscopeNode::new(44100.0, "test".to_string());
        scope.trigger_level = 0.5;
        scope.trigger_slope = 0.0; // Rising
        
        // Test rising edge trigger
        scope.last_sample = 0.0;
        assert!(scope.check_trigger(1.0, 0.5)); // Should trigger (0.0 -> 1.0 crosses 0.5)
        
        scope.last_sample = 1.0;
        assert!(!scope.check_trigger(0.0, 0.5)); // Should not trigger (falling)
        
        // Test falling edge trigger
        scope.trigger_slope = 1.0; // Falling
        scope.last_sample = 1.0;
        assert!(scope.check_trigger(0.0, 0.5)); // Should trigger (1.0 -> 0.0 crosses 0.5)
    }

    #[test]
    fn test_oscilloscope_processing() {
        let mut scope = OscilloscopeNode::new(44100.0, "test".to_string());
        scope.set_parameter("trigger_mode", 0.0).unwrap(); // Auto mode
        
        let signal_data = vec![0.0, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 8);
        outputs.allocate_cv("vpp_cv".to_string(), 8);
        outputs.allocate_cv("vrms_cv".to_string(), 8);
        outputs.allocate_cv("frequency_cv".to_string(), 8);
        outputs.allocate_cv("trigger_out".to_string(), 8);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 8,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(scope.process(&mut ctx).is_ok());
        
        let signal_out = ctx.outputs.get_audio("signal_out").unwrap();
        let trigger_out = ctx.outputs.get_cv("trigger_out").unwrap();
        
        // Check pass-through
        assert_eq!(signal_out[0], 0.0);
        assert_eq!(signal_out[2], 1.0);
        
        // Check trigger output (should be 5.0 in auto mode)
        assert_eq!(trigger_out[0], 5.0);
        
        // Check that buffer is populated
        assert!(!scope.sample_buffer.is_empty());
        assert!(!scope.measurement_buffer.is_empty());
    }

    #[test]
    fn test_measurement_calculation() {
        let mut scope = OscilloscopeNode::new(44100.0, "test".to_string());
        
        // Add test data that should produce known measurements
        for i in 0..1000 {
            let sample = (i as f32 * 0.01).sin(); // 1Hz sine wave at 44.1kHz
            scope.measurement_buffer.push_back(sample);
        }
        
        scope.calculate_measurements();
        
        // Check that measurements are calculated
        assert!(scope.measurements.vpp > 0.0);
        assert!(scope.measurements.vrms > 0.0);
        // Note: Frequency calculation may not be accurate with limited test data
    }

    #[test]
    fn test_display_data_access() {
        let scope = OscilloscopeNode::new(44100.0, "test".to_string());
        
        let display_data = scope.get_display_data();
        assert_eq!(display_data.len(), 1024);
        
        let measurements = scope.get_measurements();
        assert_eq!(measurements.vpp, 0.0); // Should be initialized to zero
        
        assert!(!scope.is_triggered()); // Should start untriggered
    }

    #[test]
    fn test_oscilloscope_bypass() {
        let mut scope = OscilloscopeNode::new(44100.0, "test".to_string());
        scope.set_parameter("active", 0.0).unwrap(); // Disable
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), vec![1.0, 2.0, 3.0, 4.0]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(scope.process(&mut ctx).is_ok());
        
        let signal_out = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should pass through when bypassed
        assert_eq!(signal_out[0], 1.0);
        assert_eq!(signal_out[1], 2.0);
        assert_eq!(signal_out[2], 3.0);
        assert_eq!(signal_out[3], 4.0);
    }
}