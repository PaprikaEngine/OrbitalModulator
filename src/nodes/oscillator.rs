use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveformType {
    Sine,
    Triangle,
    Sawtooth,
    Pulse,
}

impl WaveformType {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sine" => Some(WaveformType::Sine),
            "triangle" => Some(WaveformType::Triangle),
            "sawtooth" => Some(WaveformType::Sawtooth),
            "pulse" => Some(WaveformType::Pulse),
            _ => None,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            WaveformType::Sine => "sine",
            WaveformType::Triangle => "triangle",
            WaveformType::Sawtooth => "sawtooth",
            WaveformType::Pulse => "pulse",
        }
    }
}

#[derive(Debug)]
pub struct SineOscillatorNode {
    pub frequency: f32,
    pub amplitude: f32,
    pub active: bool,
    phase: f32,
    sample_rate: f32,
}

#[derive(Debug)]
pub struct OscillatorNode {
    pub frequency: f32,
    pub amplitude: f32,
    pub waveform: WaveformType,
    pub pulse_width: f32, // For pulse wave
    pub active: bool,
    phase: f32,
    sample_rate: f32,
}

impl SineOscillatorNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            frequency: 440.0,  // A4 default
            amplitude: 0.5,
            active: false,
            phase: 0.0,
            sample_rate,
        }
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency.clamp(20.0, 20000.0);
    }

    pub fn set_amplitude(&mut self, amplitude: f32) {
        self.amplitude = amplitude.clamp(0.0, 1.0);
    }

    fn advance_phase(&mut self, samples: usize) {
        let phase_increment = 2.0 * std::f32::consts::PI * self.frequency / self.sample_rate;
        self.phase += phase_increment * samples as f32;
        
        // Wrap phase to prevent accumulation errors
        while self.phase >= 2.0 * std::f32::consts::PI {
            self.phase -= 2.0 * std::f32::consts::PI;
        }
    }
}

impl AudioNode for SineOscillatorNode {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        // Get CV inputs for frequency and amplitude modulation
        let frequency_cv = inputs.get("frequency_cv").copied().unwrap_or(&[]);
        let amplitude_cv = inputs.get("amplitude_cv").copied().unwrap_or(&[]);

        // Get output buffer
        if let Some(output_buffer) = outputs.get_mut("audio_out") {
            let buffer_size = output_buffer.len();
            
            if self.active {
                for (i, sample) in output_buffer.iter_mut().enumerate() {
                    // Calculate effective frequency (base + CV modulation)
                    let effective_frequency = if frequency_cv.is_empty() {
                        self.frequency
                    } else {
                        // CV modulation: 1V/Oct standard (CV * 1000Hz per volt)
                        let cv_value = if i < frequency_cv.len() { frequency_cv[i] } else { 0.0 };
                        self.frequency + (cv_value * 1000.0)
                    };

                    // Calculate effective amplitude (base + CV modulation)
                    let effective_amplitude = if amplitude_cv.is_empty() {
                        self.amplitude
                    } else {
                        let cv_value = if i < amplitude_cv.len() { amplitude_cv[i] } else { 0.0 };
                        (self.amplitude + cv_value * 0.1).clamp(0.0, 1.0)
                    };

                    // Generate sine wave sample
                    let phase_increment = 2.0 * std::f32::consts::PI * effective_frequency / self.sample_rate;
                    *sample = (self.phase + phase_increment * i as f32).sin() * effective_amplitude;
                }

                // Advance phase for next buffer
                self.advance_phase(buffer_size);
            } else {
                // If not active, output silence
                for sample in output_buffer.iter_mut() {
                    *sample = 0.0;
                }
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        let mut parameters = HashMap::new();
        parameters.insert("frequency".to_string(), self.frequency);
        parameters.insert("amplitude".to_string(), self.amplitude);
        parameters.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });

        Node {
            id: Uuid::new_v4(),
            node_type: "sine_oscillator".to_string(),
            name,
            parameters,
            input_ports: vec![
                Port {
                    name: "frequency_cv".to_string(),
                    port_type: PortType::CV,
                },
                Port {
                    name: "amplitude_cv".to_string(),
                    port_type: PortType::CV,
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

impl OscillatorNode {
    pub fn new(sample_rate: f32, waveform: WaveformType) -> Self {
        Self {
            frequency: 440.0,  // A4 default
            amplitude: 0.5,
            waveform,
            pulse_width: 0.5,  // 50% duty cycle default
            active: false,
            phase: 0.0,
            sample_rate,
        }
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency.clamp(20.0, 20000.0);
    }

    pub fn set_amplitude(&mut self, amplitude: f32) {
        self.amplitude = amplitude.clamp(0.0, 1.0);
    }

    pub fn set_waveform(&mut self, waveform: WaveformType) {
        self.waveform = waveform;
    }

    pub fn set_pulse_width(&mut self, pulse_width: f32) {
        self.pulse_width = pulse_width.clamp(0.1, 0.9);
    }

    fn advance_phase(&mut self, samples: usize) {
        let phase_increment = 2.0 * std::f32::consts::PI * self.frequency / self.sample_rate;
        self.phase += phase_increment * samples as f32;
        
        // Wrap phase to prevent accumulation errors
        while self.phase >= 2.0 * std::f32::consts::PI {
            self.phase -= 2.0 * std::f32::consts::PI;
        }
    }

    fn generate_sample(&self, phase: f32) -> f32 {
        match self.waveform {
            WaveformType::Sine => phase.sin(),
            WaveformType::Triangle => {
                let normalized_phase = phase / (2.0 * std::f32::consts::PI);
                if normalized_phase < 0.5 {
                    4.0 * normalized_phase - 1.0
                } else {
                    3.0 - 4.0 * normalized_phase
                }
            },
            WaveformType::Sawtooth => {
                let normalized_phase = phase / (2.0 * std::f32::consts::PI);
                2.0 * normalized_phase - 1.0
            },
            WaveformType::Pulse => {
                let normalized_phase = phase / (2.0 * std::f32::consts::PI);
                if normalized_phase < self.pulse_width {
                    1.0
                } else {
                    -1.0
                }
            },
        }
    }
}

impl AudioNode for OscillatorNode {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        // Get CV inputs for frequency and amplitude modulation
        let frequency_cv = inputs.get("frequency_cv").copied().unwrap_or(&[]);
        let amplitude_cv = inputs.get("amplitude_cv").copied().unwrap_or(&[]);
        let waveform_cv = inputs.get("waveform_cv").copied().unwrap_or(&[]);
        let pulse_width_cv = inputs.get("pulse_width_cv").copied().unwrap_or(&[]);

        // Get output buffer
        if let Some(output_buffer) = outputs.get_mut("audio_out") {
            let buffer_size = output_buffer.len();
            
            if self.active {
                for (i, sample) in output_buffer.iter_mut().enumerate() {
                    // Calculate effective frequency (base + CV modulation)
                    let effective_frequency = if frequency_cv.is_empty() {
                        self.frequency
                    } else {
                        // CV modulation: 1V/Oct standard (CV * 1000Hz per volt)
                        let cv_value = if i < frequency_cv.len() { frequency_cv[i] } else { 0.0 };
                        self.frequency + (cv_value * 1000.0)
                    };

                    // Calculate effective amplitude (base + CV modulation)
                    let effective_amplitude = if amplitude_cv.is_empty() {
                        self.amplitude
                    } else {
                        let cv_value = if i < amplitude_cv.len() { amplitude_cv[i] } else { 0.0 };
                        (self.amplitude + cv_value * 0.1).clamp(0.0, 1.0)
                    };

                    // Update waveform from CV if available
                    if !waveform_cv.is_empty() && i < waveform_cv.len() {
                        let waveform_value = (waveform_cv[i] * 4.0).floor() as i32;
                        match waveform_value {
                            0 => self.waveform = WaveformType::Sine,
                            1 => self.waveform = WaveformType::Triangle,
                            2 => self.waveform = WaveformType::Sawtooth,
                            3 => self.waveform = WaveformType::Pulse,
                            _ => {}, // Keep current waveform
                        }
                    }

                    // Update pulse width from CV if available
                    if !pulse_width_cv.is_empty() && i < pulse_width_cv.len() {
                        self.pulse_width = (0.1 + pulse_width_cv[i] * 0.8).clamp(0.1, 0.9);
                    }

                    // Generate sample
                    let phase_increment = 2.0 * std::f32::consts::PI * effective_frequency / self.sample_rate;
                    let current_phase = self.phase + phase_increment * i as f32;
                    *sample = self.generate_sample(current_phase) * effective_amplitude;
                }

                // Advance phase for next buffer
                self.advance_phase(buffer_size);
            } else {
                // If not active, output silence
                for sample in output_buffer.iter_mut() {
                    *sample = 0.0;
                }
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        let mut parameters = HashMap::new();
        parameters.insert("frequency".to_string(), self.frequency);
        parameters.insert("amplitude".to_string(), self.amplitude);
        parameters.insert("waveform".to_string(), self.waveform as u8 as f32);
        parameters.insert("pulse_width".to_string(), self.pulse_width);
        parameters.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });

        Node {
            id: Uuid::new_v4(),
            node_type: "oscillator".to_string(),
            name,
            parameters,
            input_ports: vec![
                Port {
                    name: "frequency_cv".to_string(),
                    port_type: PortType::CV,
                },
                Port {
                    name: "amplitude_cv".to_string(),
                    port_type: PortType::CV,
                },
                Port {
                    name: "waveform_cv".to_string(),
                    port_type: PortType::CV,
                },
                Port {
                    name: "pulse_width_cv".to_string(),
                    port_type: PortType::CV,
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