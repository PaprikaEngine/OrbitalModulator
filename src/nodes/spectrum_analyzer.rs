use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

pub struct SpectrumAnalyzerNode {
    sample_rate: f32,
    buffer_size: usize,
    fft_size: usize,
    pub active: bool,
    id: Uuid,
    name: String,
    
    // FFT processing
    input_buffer: Vec<f32>,
    fft_buffer: Vec<f32>,
    magnitude_spectrum: Vec<f32>,
    window: Vec<f32>,
    buffer_index: usize,
    
    // Analysis parameters
    window_type: WindowType,
    smoothing: f32,
    gain: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum WindowType {
    Hanning,
    Hamming,
    Blackman,
    Rectangular,
}

impl SpectrumAnalyzerNode {
    pub fn new(name: String) -> Self {
        let fft_size = 1024;
        let sample_rate = 44100.0;
        let buffer_size = 512;
        
        let mut analyzer = Self {
            sample_rate,
            buffer_size,
            fft_size,
            active: true,
            id: Uuid::new_v4(),
            name,
            input_buffer: vec![0.0; fft_size],
            fft_buffer: vec![0.0; fft_size * 2], // Real + imaginary
            magnitude_spectrum: vec![0.0; fft_size / 2],
            window: vec![0.0; fft_size],
            buffer_index: 0,
            window_type: WindowType::Hanning,
            smoothing: 0.8,
            gain: 1.0,
        };
        
        analyzer.generate_window();
        analyzer
    }
    
    fn generate_window(&mut self) {
        let n = self.fft_size as f32;
        for i in 0..self.fft_size {
            let phase = 2.0 * std::f32::consts::PI * i as f32 / (n - 1.0);
            self.window[i] = match self.window_type {
                WindowType::Hanning => 0.5 * (1.0 - phase.cos()),
                WindowType::Hamming => 0.54 - 0.46 * phase.cos(),
                WindowType::Blackman => {
                    0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos()
                },
                WindowType::Rectangular => 1.0,
            };
        }
    }
    
    pub fn set_window_type(&mut self, window_type: WindowType) {
        self.window_type = window_type;
        self.generate_window();
    }
    
    pub fn set_smoothing(&mut self, smoothing: f32) {
        self.smoothing = smoothing.clamp(0.0, 0.99);
    }
    
    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.1, 10.0);
    }
    
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "window_type" => {
                let window_type = match value as u8 {
                    0 => WindowType::Hanning,
                    1 => WindowType::Hamming,
                    2 => WindowType::Blackman,
                    3 => WindowType::Rectangular,
                    _ => return Err(format!("Invalid window type value: {}", value)),
                };
                self.set_window_type(window_type);
            },
            "smoothing" => self.set_smoothing(value),
            "gain" => self.set_gain(value),
            "active" => self.active = value != 0.0,
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "window_type" => Ok(self.window_type as u8 as f32),
            "smoothing" => Ok(self.smoothing),
            "gain" => Ok(self.gain),
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    // Simple FFT implementation (Cooley-Tukey algorithm)
    fn fft_inplace(&self, data: &mut [f32], n: usize, inverse: bool) {
        // Bit-reversal permutation
        let mut j = 0;
        for i in 1..n {
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
            if i < j {
                data.swap(2 * i, 2 * j);
                data.swap(2 * i + 1, 2 * j + 1);
            }
        }
        
        // FFT computation
        let mut length = 2;
        while length <= n {
            let angle = if inverse { 2.0 * std::f32::consts::PI / length as f32 } 
                       else { -2.0 * std::f32::consts::PI / length as f32 };
            let wlen_cos = angle.cos();
            let wlen_sin = angle.sin();
            
            let mut i = 0;
            while i < n {
                let mut w_cos = 1.0;
                let mut w_sin = 0.0;
                
                for j in 0..length/2 {
                    let u_idx = 2 * (i + j);
                    let v_idx = 2 * (i + j + length / 2);
                    
                    let u_real = data[u_idx];
                    let u_imag = data[u_idx + 1];
                    let v_real = data[v_idx];
                    let v_imag = data[v_idx + 1];
                    
                    let temp_real = v_real * w_cos - v_imag * w_sin;
                    let temp_imag = v_real * w_sin + v_imag * w_cos;
                    
                    data[u_idx] = u_real + temp_real;
                    data[u_idx + 1] = u_imag + temp_imag;
                    data[v_idx] = u_real - temp_real;
                    data[v_idx + 1] = u_imag - temp_imag;
                    
                    let new_w_cos = w_cos * wlen_cos - w_sin * wlen_sin;
                    let new_w_sin = w_cos * wlen_sin + w_sin * wlen_cos;
                    w_cos = new_w_cos;
                    w_sin = new_w_sin;
                }
                i += length;
            }
            length <<= 1;
        }
        
        if inverse {
            let scale = 1.0 / n as f32;
            for i in 0..2*n {
                data[i] *= scale;
            }
        }
    }
    
    fn process_fft(&mut self) {
        // Apply window and copy to FFT buffer
        for i in 0..self.fft_size {
            self.fft_buffer[2 * i] = self.input_buffer[i] * self.window[i];
            self.fft_buffer[2 * i + 1] = 0.0; // Imaginary part
        }
        
        // Perform FFT
        self.fft_inplace(&mut self.fft_buffer, self.fft_size, false);
        
        // Calculate magnitude spectrum
        for i in 0..self.fft_size / 2 {
            let real = self.fft_buffer[2 * i];
            let imag = self.fft_buffer[2 * i + 1];
            let magnitude = (real * real + imag * imag).sqrt() * self.gain;
            
            // Apply smoothing
            self.magnitude_spectrum[i] = self.magnitude_spectrum[i] * self.smoothing + 
                                       magnitude * (1.0 - self.smoothing);
        }
    }
    
    pub fn get_magnitude_spectrum(&self) -> &[f32] {
        &self.magnitude_spectrum
    }
    
    pub fn get_frequency_bins(&self) -> Vec<f32> {
        let mut frequencies = Vec::with_capacity(self.fft_size / 2);
        let freq_resolution = self.sample_rate / self.fft_size as f32;
        
        for i in 0..self.fft_size / 2 {
            frequencies.push(i as f32 * freq_resolution);
        }
        
        frequencies
    }
}

impl AudioNode for SpectrumAnalyzerNode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        let buffer_size = inputs.get("audio_in")
            .map(|buf| buf.len())
            .unwrap_or(0);
            
        if buffer_size == 0 || !self.active {
            return;
        }
        
        // Create default buffer
        let default_buffer = vec![0.0; buffer_size];
        
        // Get input audio
        let input_audio = inputs.get("audio_in")
            .copied()
            .unwrap_or(&default_buffer);
        
        // Pass through audio (analyzer doesn't modify the signal)
        if let Some(output) = outputs.get_mut("audio_out") {
            for i in 0..buffer_size.min(output.len()) {
                let sample = if i < input_audio.len() {
                    input_audio[i]
                } else {
                    0.0
                };
                output[i] = sample;
                
                // Collect samples for FFT analysis
                self.input_buffer[self.buffer_index] = sample;
                self.buffer_index += 1;
                
                // Process FFT when buffer is full
                if self.buffer_index >= self.fft_size {
                    self.process_fft();
                    
                    // Overlap by 50%
                    let overlap = self.fft_size / 2;
                    for j in 0..overlap {
                        self.input_buffer[j] = self.input_buffer[overlap + j];
                    }
                    self.buffer_index = overlap;
                }
            }
        }
    }
    
    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "spectrum_analyzer".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("window_type".to_string(), self.window_type as u8 as f32);
                params.insert("smoothing".to_string(), self.smoothing);
                params.insert("gain".to_string(), self.gain);
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params
            },
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
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Add as_any method for downcast access
impl SpectrumAnalyzerNode {
    pub fn as_any(&self) -> &dyn Any {
        self
    }
}