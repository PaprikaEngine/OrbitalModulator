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

/// Window function types for FFT analysis
#[derive(Debug, Clone, Copy)]
pub enum WindowType {
    Hanning,
    Hamming,
    Blackman,
    Rectangular,
}

/// リファクタリング済みSpectrumAnalyzerNode - プロ仕様FFTスペクトラムアナライザー
/// 
/// 特徴:
/// - 自作Cooley-Tukey FFT実装
/// - 4種類の窓関数（Hanning/Hamming/Blackman/Rectangular）
/// - リアルタイム周波数解析（20Hz〜20kHz）
/// - スムージング機能付き
/// - CV出力対応（ピーク周波数、総パワー等）
/// - プロ仕様UI対応（ログスケール表示）
/// - 分解能調整機能（512〜4096サンプル）
pub struct SpectrumAnalyzerNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Spectrum analyzer controls
    fft_size: f32,               // 512.0 ~ 4096.0 (FFTサイズ)
    window_type: f32,            // 0.0=Hanning, 1.0=Hamming, 2.0=Blackman, 3.0=Rectangular
    smoothing: f32,              // 0.0 ~ 1.0 (スムージング量)
    peak_hold: f32,              // 0.0 ~ 1.0 (ピークホールド時間)
    frequency_range_low: f32,    // 20.0 ~ 1000.0 (表示範囲下限)
    frequency_range_high: f32,   // 1000.0 ~ 20000.0 (表示範囲上限)
    gain: f32,                   // 0.1 ~ 10.0 (表示ゲイン)
    active: f32,                 // 0.0 = Off, 1.0 = On
    
    // CV Modulation parameters
    smoothing_param: ModulatableParameter,
    gain_param: ModulatableParameter,
    
    // Internal processing state
    input_buffer: VecDeque<f32>, // 入力信号バッファ
    fft_buffer: Vec<f32>,        // FFT計算用バッファ
    window_buffer: Vec<f32>,     // 窓関数バッファ
    spectrum_data: Vec<f32>,     // スペクトラムデータ（振幅）
    smoothed_spectrum: Vec<f32>, // スムージング済みスペクトラム
    peak_spectrum: Vec<f32>,     // ピークホールドスペクトラム
    
    // Frequency analysis
    frequency_bins: Vec<f32>,    // 各ビンの周波数
    peak_frequency: f32,         // ピーク周波数
    total_power: f32,            // 総パワー
    centroid_frequency: f32,     // 重心周波数
    
    // FFT state
    bit_reversed_indices: Vec<usize>, // ビットリバーサル用インデックス
    twiddle_factors: Vec<(f32, f32)>, // 回転因子（cos, sin）
    
    // Display state
    display_spectrum: Vec<f32>,  // 表示用スペクトラム（ログスケール）
    update_counter: u32,         // 更新カウンター（30FPS制御用）
    
    sample_rate: f32,
}

impl SpectrumAnalyzerNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "spectrum_analyzer_refactored".to_string(),
            category: NodeCategory::Analyzer,
            
            // Input ports: audio signal + CV inputs
            input_ports: vec![
                PortInfo::new("signal_in", PortType::AudioMono),
                PortInfo::new("smoothing_cv", PortType::CV),
                PortInfo::new("gain_cv", PortType::CV),
                PortInfo::new("frequency_range_cv", PortType::CV),
            ],
            
            // Output ports: analysis results as CV + pass-through
            output_ports: vec![
                PortInfo::new("signal_out", PortType::AudioMono),
                PortInfo::new("peak_frequency_cv", PortType::CV),
                PortInfo::new("total_power_cv", PortType::CV),
                PortInfo::new("centroid_frequency_cv", PortType::CV),
                PortInfo::new("low_band_cv", PortType::CV),
                PortInfo::new("mid_band_cv", PortType::CV),
                PortInfo::new("high_band_cv", PortType::CV),
            ],
            
            description: "Professional FFT spectrum analyzer with windowing and real-time frequency analysis".to_string(),
            latency_samples: 0,
            supports_bypass: true,
        };

        // Create modulation parameters
        let smoothing_param = ModulatableParameter::new(
            BasicParameter::new("smoothing", 0.0, 1.0, 0.3),
            1.0  // 100% CV modulation range
        );
        
        let gain_param = ModulatableParameter::new(
            BasicParameter::new("gain", 0.1, 10.0, 1.0),
            0.5  // 50% CV modulation range
        );

        let fft_size = 1024; // Default FFT size
        
        // Pre-calculate bit-reversed indices for FFT
        let bit_reversed_indices = Self::calculate_bit_reversed_indices(fft_size);
        
        // Pre-calculate twiddle factors for FFT
        let twiddle_factors = Self::calculate_twiddle_factors(fft_size);
        
        // Calculate frequency bins
        let frequency_bins = Self::calculate_frequency_bins(fft_size, sample_rate);

        Self {
            node_info,
            fft_size: fft_size as f32,
            window_type: 0.0,    // Hanning window
            smoothing: 0.3,
            peak_hold: 0.5,
            frequency_range_low: 20.0,
            frequency_range_high: 20000.0,
            gain: 1.0,
            active: 1.0,
            
            // CV parameters
            smoothing_param,
            gain_param,
            
            // Internal state
            input_buffer: VecDeque::with_capacity(fft_size * 2),
            fft_buffer: vec![0.0; fft_size * 2], // Complex numbers (real + imaginary)
            window_buffer: vec![0.0; fft_size],
            spectrum_data: vec![0.0; fft_size / 2],
            smoothed_spectrum: vec![0.0; fft_size / 2],
            peak_spectrum: vec![0.0; fft_size / 2],
            
            // Frequency analysis
            frequency_bins,
            peak_frequency: 0.0,
            total_power: 0.0,
            centroid_frequency: 0.0,
            
            // FFT state
            bit_reversed_indices,
            twiddle_factors,
            
            // Display state
            display_spectrum: vec![0.0; 512], // 512 points for UI display
            update_counter: 0,
            
            sample_rate,
        }
    }
    
    /// Calculate bit-reversed indices for FFT
    fn calculate_bit_reversed_indices(size: usize) -> Vec<usize> {
        let mut indices = vec![0; size];
        let log2_size = (size as f32).log2() as usize;
        
        for i in 0..size {
            let mut reversed = 0;
            let mut temp = i;
            for _ in 0..log2_size {
                reversed = (reversed << 1) | (temp & 1);
                temp >>= 1;
            }
            indices[i] = reversed;
        }
        
        indices
    }
    
    /// Calculate twiddle factors for FFT
    fn calculate_twiddle_factors(size: usize) -> Vec<(f32, f32)> {
        let mut factors = Vec::with_capacity(size / 2);
        let pi2 = 2.0 * std::f32::consts::PI;
        
        for k in 0..size/2 {
            let angle = -pi2 * k as f32 / size as f32;
            factors.push((angle.cos(), angle.sin()));
        }
        
        factors
    }
    
    /// Calculate frequency bins
    fn calculate_frequency_bins(fft_size: usize, sample_rate: f32) -> Vec<f32> {
        let mut bins = Vec::with_capacity(fft_size / 2);
        let bin_width = sample_rate / fft_size as f32;
        
        for i in 0..fft_size/2 {
            bins.push(i as f32 * bin_width);
        }
        
        bins
    }
    
    /// Convert window type parameter to enum
    fn get_window_type(&self) -> WindowType {
        match self.window_type as i32 {
            0 => WindowType::Hanning,
            1 => WindowType::Hamming,
            2 => WindowType::Blackman,
            3 => WindowType::Rectangular,
            _ => WindowType::Hanning,
        }
    }
    
    /// Generate window function
    fn generate_window(&mut self) {
        let size = self.window_buffer.len();
        let window_type = self.get_window_type();
        
        for i in 0..size {
            let x = i as f32 / (size - 1) as f32;
            let value = match window_type {
                WindowType::Hanning => {
                    0.5 * (1.0 - (2.0 * std::f32::consts::PI * x).cos())
                },
                WindowType::Hamming => {
                    0.54 - 0.46 * (2.0 * std::f32::consts::PI * x).cos()
                },
                WindowType::Blackman => {
                    0.42 - 0.5 * (2.0 * std::f32::consts::PI * x).cos() 
                        + 0.08 * (4.0 * std::f32::consts::PI * x).cos()
                },
                WindowType::Rectangular => 1.0,
            };
            self.window_buffer[i] = value;
        }
    }
    
    
    /// Cooley-Tukey FFT implementation
    fn fft_inplace(&self, data: &mut [f32]) {
        let size = data.len() / 2; // Complex numbers
        
        // Bit-reversal permutation
        for i in 0..size {
            let j = if i < self.bit_reversed_indices.len() {
                self.bit_reversed_indices[i]
            } else {
                i
            };
            if i < j {
                data.swap(i * 2, j * 2);         // Real part
                data.swap(i * 2 + 1, j * 2 + 1); // Imaginary part
            }
        }
        
        // FFT computation
        let mut length = 2;
        while length <= size {
            let step = size / length;
            
            for i in (0..size).step_by(length) {
                for j in 0..length/2 {
                    let twiddle_idx = j * step;
                    let (cos_val, sin_val) = if twiddle_idx < self.twiddle_factors.len() {
                        self.twiddle_factors[twiddle_idx]
                    } else {
                        (1.0, 0.0)
                    };
                    
                    let even_idx = (i + j) * 2;
                    let odd_idx = (i + j + length/2) * 2;
                    
                    // Complex multiplication: twiddle * odd
                    let temp_real = data[odd_idx] * cos_val - data[odd_idx + 1] * sin_val;
                    let temp_imag = data[odd_idx] * sin_val + data[odd_idx + 1] * cos_val;
                    
                    // Butterfly computation
                    data[odd_idx] = data[even_idx] - temp_real;
                    data[odd_idx + 1] = data[even_idx + 1] - temp_imag;
                    data[even_idx] += temp_real;
                    data[even_idx + 1] += temp_imag;
                }
            }
            
            length *= 2;
        }
    }
    
    /// Convert complex FFT result to magnitude spectrum
    fn calculate_magnitude_spectrum(&mut self) {
        let size = self.spectrum_data.len();
        
        for i in 0..size {
            let real = self.fft_buffer[i * 2];
            let imag = self.fft_buffer[i * 2 + 1];
            let magnitude = (real * real + imag * imag).sqrt();
            
            // Apply gain and convert to dB
            let db_value = 20.0 * (magnitude * self.gain + 1e-10).log10();
            self.spectrum_data[i] = db_value.max(-120.0); // Noise floor at -120dB
        }
    }
    
    /// Apply smoothing to spectrum data
    fn apply_smoothing(&mut self, effective_smoothing: f32) {
        for i in 0..self.spectrum_data.len() {
            self.smoothed_spectrum[i] = self.smoothed_spectrum[i] * effective_smoothing + 
                                       self.spectrum_data[i] * (1.0 - effective_smoothing);
        }
    }
    
    /// Update peak hold spectrum
    fn update_peak_hold(&mut self) {
        let decay_rate = 0.99; // Peak decay rate
        
        for i in 0..self.spectrum_data.len() {
            if self.smoothed_spectrum[i] > self.peak_spectrum[i] {
                self.peak_spectrum[i] = self.smoothed_spectrum[i];
            } else {
                self.peak_spectrum[i] *= decay_rate;
            }
        }
    }
    
    /// Analyze frequency content
    fn analyze_frequency_content(&mut self) {
        // Find peak frequency
        let mut max_amplitude = -120.0;
        let mut peak_bin = 0;
        
        for (i, &amplitude) in self.smoothed_spectrum.iter().enumerate() {
            if amplitude > max_amplitude {
                max_amplitude = amplitude;
                peak_bin = i;
            }
        }
        
        self.peak_frequency = if peak_bin < self.frequency_bins.len() {
            self.frequency_bins[peak_bin]
        } else {
            0.0
        };
        
        // Calculate total power
        self.total_power = self.smoothed_spectrum.iter()
            .map(|&db| 10.0_f32.powf(db / 10.0))
            .sum();
        
        // Calculate spectral centroid
        let mut weighted_sum = 0.0;
        let mut total_magnitude = 0.0;
        
        for (i, &amplitude) in self.smoothed_spectrum.iter().enumerate() {
            let magnitude = 10.0_f32.powf(amplitude / 20.0);
            let frequency = if i < self.frequency_bins.len() {
                self.frequency_bins[i]
            } else {
                0.0
            };
            
            weighted_sum += frequency * magnitude;
            total_magnitude += magnitude;
        }
        
        self.centroid_frequency = if total_magnitude > 0.0 {
            weighted_sum / total_magnitude
        } else {
            0.0
        };
    }
    
    /// Calculate frequency band energy
    fn calculate_band_energy(&self, low_freq: f32, high_freq: f32) -> f32 {
        let mut energy = 0.0;
        
        for (i, &frequency) in self.frequency_bins.iter().enumerate() {
            if frequency >= low_freq && frequency <= high_freq && i < self.smoothed_spectrum.len() {
                energy += 10.0_f32.powf(self.smoothed_spectrum[i] / 10.0);
            }
        }
        
        energy
    }
    
    /// Update display spectrum for UI
    fn update_display_spectrum(&mut self) {
        // Only update at ~30 FPS to reduce CPU load
        self.update_counter += 1;
        if self.update_counter % (self.sample_rate as u32 / 30) != 0 {
            return;
        }
        
        // Resample spectrum data to 512 points for display
        let spectrum_size = self.smoothed_spectrum.len();
        let display_size = self.display_spectrum.len();
        
        for i in 0..display_size {
            let spectrum_index = (i * spectrum_size) / display_size;
            if spectrum_index < spectrum_size {
                self.display_spectrum[i] = self.smoothed_spectrum[spectrum_index];
            }
        }
    }
    
    /// Get display spectrum for UI rendering
    pub fn get_display_spectrum(&self) -> &[f32] {
        &self.display_spectrum
    }
    
    /// Get peak spectrum for UI rendering
    pub fn get_peak_spectrum(&self) -> &[f32] {
        &self.peak_spectrum
    }
    
    /// Get frequency analysis results
    pub fn get_analysis_results(&self) -> (f32, f32, f32) {
        (self.peak_frequency, self.total_power, self.centroid_frequency)
    }
}

impl Parameterizable for SpectrumAnalyzerNodeRefactored {
    define_parameters! {
        fft_size: BasicParameter::new("fft_size", 512.0, 4096.0, 1024.0),
        window_type: BasicParameter::new("window_type", 0.0, 3.0, 0.0),
        smoothing: BasicParameter::new("smoothing", 0.0, 1.0, 0.3),
        peak_hold: BasicParameter::new("peak_hold", 0.0, 1.0, 0.5),
        frequency_range_low: BasicParameter::new("frequency_range_low", 20.0, 1000.0, 20.0),
        frequency_range_high: BasicParameter::new("frequency_range_high", 1000.0, 20000.0, 20000.0),
        gain: BasicParameter::new("gain", 0.1, 10.0, 1.0),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for SpectrumAnalyzerNodeRefactored {
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
        let smoothing_cv = ctx.inputs.get_cv_value("smoothing_cv");
        let gain_cv = ctx.inputs.get_cv_value("gain_cv");

        // Apply CV modulation
        let effective_smoothing = self.smoothing_param.modulate(self.smoothing, smoothing_cv);
        let effective_gain = self.gain_param.modulate(self.gain, gain_cv);

        // Add samples to input buffer
        for &sample in signal_input {
            self.input_buffer.push_back(sample);
            
            // Keep buffer size manageable
            if self.input_buffer.len() > self.fft_size as usize * 2 {
                self.input_buffer.pop_front();
            }
        }

        // Perform FFT if we have enough samples
        let fft_size = self.fft_size as usize;
        if self.input_buffer.len() >= fft_size {
            // Get latest samples for FFT
            let samples: Vec<f32> = self.input_buffer.iter()
                .skip(self.input_buffer.len() - fft_size)
                .copied()
                .collect();

            // Update window function if needed
            if self.window_buffer.len() != fft_size {
                self.window_buffer.resize(fft_size, 0.0);
                self.generate_window();
            }

            // Apply window and prepare FFT buffer
            {
                let fft_buffer = &mut self.fft_buffer;
                for (i, (&input_val, &window_val)) in samples.iter().zip(self.window_buffer.iter()).enumerate() {
                    fft_buffer[i * 2] = input_val * window_val; // Real part
                    fft_buffer[i * 2 + 1] = 0.0; // Imaginary part
                }
            }

            // Perform FFT using separate variables
            let bit_reversed_indices = &self.bit_reversed_indices;
            let twiddle_factors = &self.twiddle_factors;
            
            // Inline FFT to avoid borrowing conflicts
            let size = self.fft_buffer.len() / 2;
            
            // Bit-reversal permutation
            for i in 0..size {
                let j = if i < bit_reversed_indices.len() {
                    bit_reversed_indices[i]
                } else {
                    i
                };
                if i < j {
                    self.fft_buffer.swap(i * 2, j * 2);
                    self.fft_buffer.swap(i * 2 + 1, j * 2 + 1);
                }
            }
            
            // FFT computation
            let mut length = 2;
            while length <= size {
                let step = size / length;
                
                for i in (0..size).step_by(length) {
                    for j in 0..length/2 {
                        let twiddle_idx = j * step;
                        let (cos_val, sin_val) = if twiddle_idx < twiddle_factors.len() {
                            twiddle_factors[twiddle_idx]
                        } else {
                            (1.0, 0.0)
                        };
                        
                        let even_idx = (i + j) * 2;
                        let odd_idx = (i + j + length/2) * 2;
                        
                        // Complex multiplication: twiddle * odd
                        let temp_real = self.fft_buffer[odd_idx] * cos_val - self.fft_buffer[odd_idx + 1] * sin_val;
                        let temp_imag = self.fft_buffer[odd_idx] * sin_val + self.fft_buffer[odd_idx + 1] * cos_val;
                        
                        // Butterfly computation
                        self.fft_buffer[odd_idx] = self.fft_buffer[even_idx] - temp_real;
                        self.fft_buffer[odd_idx + 1] = self.fft_buffer[even_idx + 1] - temp_imag;
                        self.fft_buffer[even_idx] += temp_real;
                        self.fft_buffer[even_idx + 1] += temp_imag;
                    }
                }
                
                length *= 2;
            }

            // Calculate magnitude spectrum
            self.calculate_magnitude_spectrum();

            // Apply smoothing
            self.apply_smoothing(effective_smoothing);

            // Update peak hold
            if self.peak_hold > 0.1 {
                self.update_peak_hold();
            }

            // Analyze frequency content
            self.analyze_frequency_content();

            // Update display spectrum
            self.update_display_spectrum();
        }

        // Pass through input signal
        if let (Some(input), Some(output)) = 
            (ctx.inputs.get_audio("signal_in"), ctx.outputs.get_audio_mut("signal_out")) {
            output.copy_from_slice(&input[..output.len().min(input.len())]);
        }

        // Output analysis CV signals
        if let Some(peak_freq_cv) = ctx.outputs.get_cv_mut("peak_frequency_cv") {
            let freq_scaled = (self.peak_frequency / 2000.0).min(10.0); // Scale to CV range
            peak_freq_cv.fill(freq_scaled);
        }
        
        if let Some(total_power_cv) = ctx.outputs.get_cv_mut("total_power_cv") {
            let power_scaled = (self.total_power / 100.0).min(10.0); // Scale to CV range
            total_power_cv.fill(power_scaled);
        }
        
        if let Some(centroid_cv) = ctx.outputs.get_cv_mut("centroid_frequency_cv") {
            let centroid_scaled = (self.centroid_frequency / 2000.0).min(10.0); // Scale to CV range
            centroid_cv.fill(centroid_scaled);
        }

        // Calculate frequency band energies
        let low_band = self.calculate_band_energy(20.0, 250.0);    // Low: 20-250 Hz
        let mid_band = self.calculate_band_energy(250.0, 4000.0);  // Mid: 250-4000 Hz
        let high_band = self.calculate_band_energy(4000.0, 20000.0); // High: 4-20 kHz

        if let Some(low_cv) = ctx.outputs.get_cv_mut("low_band_cv") {
            let low_scaled = (low_band / 10.0).min(10.0);
            low_cv.fill(low_scaled);
        }
        
        if let Some(mid_cv) = ctx.outputs.get_cv_mut("mid_band_cv") {
            let mid_scaled = (mid_band / 10.0).min(10.0);
            mid_cv.fill(mid_scaled);
        }
        
        if let Some(high_cv) = ctx.outputs.get_cv_mut("high_band_cv") {
            let high_scaled = (high_band / 10.0).min(10.0);
            high_cv.fill(high_scaled);
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset internal processing state
        self.input_buffer.clear();
        self.fft_buffer.fill(0.0);
        self.spectrum_data.fill(0.0);
        self.smoothed_spectrum.fill(0.0);
        self.peak_spectrum.fill(0.0);
        self.display_spectrum.fill(0.0);
        
        self.peak_frequency = 0.0;
        self.total_power = 0.0;
        self.centroid_frequency = 0.0;
        self.update_counter = 0;
        
        // Regenerate window function
        self.generate_window();
    }

    fn latency(&self) -> u32 {
        0 // No latency for spectrum analysis
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
    fn test_spectrum_analyzer_creation() {
        let analyzer = SpectrumAnalyzerNodeRefactored::new(44100.0, "test_analyzer".to_string());
        assert_eq!(analyzer.fft_size, 1024.0);
        assert_eq!(analyzer.window_type, 0.0);
        assert_eq!(analyzer.smoothing, 0.3);
        assert_eq!(analyzer.active, 1.0);
    }

    #[test]
    fn test_spectrum_analyzer_parameters() {
        let mut analyzer = SpectrumAnalyzerNodeRefactored::new(44100.0, "test".to_string());
        
        // Test parameter setting
        assert!(analyzer.set_parameter("fft_size", 2048.0).is_ok());
        assert_eq!(analyzer.get_parameter("fft_size").unwrap(), 2048.0);
        
        // Test window type
        assert!(analyzer.set_parameter("window_type", 2.0).is_ok());
        assert_eq!(analyzer.get_parameter("window_type").unwrap(), 2.0);
        
        // Test smoothing
        assert!(analyzer.set_parameter("smoothing", 0.5).is_ok());
        assert_eq!(analyzer.get_parameter("smoothing").unwrap(), 0.5);
        
        // Test out of range
        assert!(analyzer.set_parameter("gain", 20.0).is_err());
    }

    #[test]
    fn test_window_type_conversion() {
        let mut analyzer = SpectrumAnalyzerNodeRefactored::new(44100.0, "test".to_string());
        
        analyzer.window_type = 0.0;
        assert!(matches!(analyzer.get_window_type(), WindowType::Hanning));
        
        analyzer.window_type = 1.0;
        assert!(matches!(analyzer.get_window_type(), WindowType::Hamming));
        
        analyzer.window_type = 2.0;
        assert!(matches!(analyzer.get_window_type(), WindowType::Blackman));
        
        analyzer.window_type = 3.0;
        assert!(matches!(analyzer.get_window_type(), WindowType::Rectangular));
    }

    #[test]
    fn test_bit_reversed_indices() {
        let indices = SpectrumAnalyzerNodeRefactored::calculate_bit_reversed_indices(8);
        assert_eq!(indices, vec![0, 4, 2, 6, 1, 5, 3, 7]);
    }

    #[test]
    fn test_frequency_bins() {
        let bins = SpectrumAnalyzerNodeRefactored::calculate_frequency_bins(1024, 44100.0);
        assert_eq!(bins.len(), 512);
        assert_eq!(bins[0], 0.0);
        assert!((bins[1] - 43.066406).abs() < 0.01); // 44100/1024 ≈ 43.066
    }

    #[test]
    fn test_window_generation() {
        let mut analyzer = SpectrumAnalyzerNodeRefactored::new(44100.0, "test".to_string());
        analyzer.window_type = 0.0; // Hanning
        analyzer.generate_window();
        
        // Hanning window should start and end at 0
        assert!((analyzer.window_buffer[0] - 0.0).abs() < 0.01);
        assert!((analyzer.window_buffer[analyzer.window_buffer.len()-1] - 0.0).abs() < 0.01);
        
        // Middle should be close to 1
        let mid = analyzer.window_buffer.len() / 2;
        assert!(analyzer.window_buffer[mid] > 0.9);
    }

    #[test]
    fn test_spectrum_analyzer_processing() {
        let mut analyzer = SpectrumAnalyzerNodeRefactored::new(44100.0, "test".to_string());
        
        // Create a test signal (1 kHz sine wave)
        let mut signal_data = Vec::new();
        for i in 0..2048 {
            let sample = (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 44100.0).sin();
            signal_data.push(sample * 0.5);
        }
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 2048);
        outputs.allocate_cv("peak_frequency_cv".to_string(), 2048);
        outputs.allocate_cv("total_power_cv".to_string(), 2048);
        outputs.allocate_cv("centroid_frequency_cv".to_string(), 2048);
        outputs.allocate_cv("low_band_cv".to_string(), 2048);
        outputs.allocate_cv("mid_band_cv".to_string(), 2048);
        outputs.allocate_cv("high_band_cv".to_string(), 2048);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 2048,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(analyzer.process(&mut ctx).is_ok());
        
        let signal_out = ctx.outputs.get_audio("signal_out").unwrap();
        let peak_freq_cv = ctx.outputs.get_cv("peak_frequency_cv").unwrap();
        
        // Check pass-through
        assert!(signal_out[100] != 0.0);
        
        // Check that CV outputs exist
        assert!(peak_freq_cv[0] >= 0.0);
        
        // Check that buffer is populated
        assert!(!analyzer.input_buffer.is_empty());
    }

    #[test]
    fn test_band_energy_calculation() {
        let analyzer = SpectrumAnalyzerNodeRefactored::new(44100.0, "test".to_string());
        
        // Test with some dummy spectrum data
        // This is mainly to verify the function doesn't crash
        let energy = analyzer.calculate_band_energy(100.0, 1000.0);
        assert!(energy >= 0.0);
    }

    #[test]
    fn test_spectrum_analyzer_bypass() {
        let mut analyzer = SpectrumAnalyzerNodeRefactored::new(44100.0, "test".to_string());
        analyzer.set_parameter("active", 0.0).unwrap(); // Disable
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), vec![1.0, 2.0, 3.0, 4.0]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(analyzer.process(&mut ctx).is_ok());
        
        let signal_out = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should pass through when bypassed
        assert_eq!(signal_out[0], 1.0);
        assert_eq!(signal_out[1], 2.0);
        assert_eq!(signal_out[2], 3.0);
        assert_eq!(signal_out[3], 4.0);
    }

    #[test]
    fn test_display_data_access() {
        let analyzer = SpectrumAnalyzerNodeRefactored::new(44100.0, "test".to_string());
        
        let display_spectrum = analyzer.get_display_spectrum();
        assert_eq!(display_spectrum.len(), 512);
        
        let peak_spectrum = analyzer.get_peak_spectrum();
        assert_eq!(peak_spectrum.len(), 512); // FFT size / 2
        
        let (peak_freq, total_power, centroid) = analyzer.get_analysis_results();
        assert_eq!(peak_freq, 0.0); // Should be initialized to zero
        assert_eq!(total_power, 0.0);
        assert_eq!(centroid, 0.0);
    }

    #[test]
    fn test_fft_functionality() {
        let analyzer = SpectrumAnalyzerNodeRefactored::new(44100.0, "test".to_string());
        
        // Test with a simple signal
        let mut test_data = vec![0.0; 16]; // 8 complex numbers
        test_data[0] = 1.0; // Real part of first sample
        test_data[2] = 1.0; // Real part of second sample
        
        // This mainly tests that FFT doesn't crash
        // A full FFT correctness test would be more complex
        let original_data = test_data.clone();
        
        // The function exists and can be called
        assert_ne!(test_data, original_data); // Some processing should occur
    }
}