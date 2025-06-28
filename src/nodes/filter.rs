use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    Lowpass,
    Highpass,
    Bandpass,
}

impl FilterType {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "lowpass" => Some(FilterType::Lowpass),
            "highpass" => Some(FilterType::Highpass),
            "bandpass" => Some(FilterType::Bandpass),
            _ => None,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            FilterType::Lowpass => "lowpass",
            FilterType::Highpass => "highpass", 
            FilterType::Bandpass => "bandpass",
        }
    }
}

#[derive(Debug)]
pub struct VCFNode {
    pub cutoff_frequency: f32,
    pub resonance: f32,
    pub filter_type: FilterType,
    pub active: bool,
    
    // Biquad filter state
    x1: f32, // Previous input sample 1
    x2: f32, // Previous input sample 2
    y1: f32, // Previous output sample 1
    y2: f32, // Previous output sample 2
    
    // Filter coefficients (updated when parameters change)
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
    
    sample_rate: f32,
    coefficients_dirty: bool,
}

impl VCFNode {
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            cutoff_frequency: 1000.0, // 1kHz default
            resonance: 1.0,            // Q = 1.0 default
            filter_type: FilterType::Lowpass,
            active: true,
            
            // Initialize filter state
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            
            // Initialize coefficients
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b1: 0.0,
            b2: 0.0,
            
            sample_rate,
            coefficients_dirty: true,
        };
        
        filter.update_coefficients();
        filter
    }

    pub fn set_cutoff_frequency(&mut self, frequency: f32) {
        self.cutoff_frequency = frequency.clamp(20.0, 20000.0);
        self.coefficients_dirty = true;
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.1, 10.0);
        self.coefficients_dirty = true;
    }

    pub fn set_filter_type(&mut self, filter_type: FilterType) {
        self.filter_type = filter_type;
        self.coefficients_dirty = true;
    }

    fn update_coefficients(&mut self) {
        if !self.coefficients_dirty {
            return;
        }

        let omega = 2.0 * std::f32::consts::PI * self.cutoff_frequency / self.sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * self.resonance);

        match self.filter_type {
            FilterType::Lowpass => {
                // Lowpass biquad coefficients
                let b0 = (1.0 - cos_omega) / 2.0;
                let b1 = 1.0 - cos_omega;
                let b2 = (1.0 - cos_omega) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                self.a0 = b0 / a0;
                self.a1 = b1 / a0;
                self.a2 = b2 / a0;
                self.b1 = a1 / a0;
                self.b2 = a2 / a0;
            },
            FilterType::Highpass => {
                // Highpass biquad coefficients
                let b0 = (1.0 + cos_omega) / 2.0;
                let b1 = -(1.0 + cos_omega);
                let b2 = (1.0 + cos_omega) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                self.a0 = b0 / a0;
                self.a1 = b1 / a0;
                self.a2 = b2 / a0;
                self.b1 = a1 / a0;
                self.b2 = a2 / a0;
            },
            FilterType::Bandpass => {
                // Bandpass biquad coefficients
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                self.a0 = b0 / a0;
                self.a1 = b1 / a0;
                self.a2 = b2 / a0;
                self.b1 = a1 / a0;
                self.b2 = a2 / a0;
            },
        }

        self.coefficients_dirty = false;
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        // Update coefficients if parameters changed
        self.update_coefficients();

        // Biquad filter equation: y[n] = a0*x[n] + a1*x[n-1] + a2*x[n-2] - b1*y[n-1] - b2*y[n-2]
        let output = self.a0 * input + self.a1 * self.x1 + self.a2 * self.x2 - self.b1 * self.y1 - self.b2 * self.y2;

        // Update delay line
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }
}

impl AudioNode for VCFNode {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        // Get CV inputs for cutoff and resonance modulation
        let cutoff_cv = inputs.get("cutoff_cv").copied().unwrap_or(&[]);
        let resonance_cv = inputs.get("resonance_cv").copied().unwrap_or(&[]);
        let audio_input = inputs.get("audio_in").copied().unwrap_or(&[]);

        // Get output buffer
        if let Some(output_buffer) = outputs.get_mut("audio_out") {
            if self.active && !audio_input.is_empty() {
                for (i, output_sample) in output_buffer.iter_mut().enumerate() {
                    // Get input sample (or 0.0 if no input)
                    let input_sample = if i < audio_input.len() { 
                        audio_input[i] 
                    } else { 
                        0.0 
                    };

                    // Calculate effective cutoff frequency (base + CV modulation)
                    let effective_cutoff = if cutoff_cv.is_empty() {
                        self.cutoff_frequency
                    } else {
                        let cv_value = if i < cutoff_cv.len() { cutoff_cv[i] } else { 0.0 };
                        // CV modulation: 1V = 1000Hz (1 octave at 1kHz)
                        (self.cutoff_frequency * (cv_value * 1000.0 / 1000.0).exp2()).clamp(20.0, 20000.0)
                    };

                    // Calculate effective resonance (base + CV modulation)
                    let effective_resonance = if resonance_cv.is_empty() {
                        self.resonance
                    } else {
                        let cv_value = if i < resonance_cv.len() { resonance_cv[i] } else { 0.0 };
                        (self.resonance + cv_value * 2.0).clamp(0.1, 10.0)
                    };

                    // Update filter parameters if they changed
                    if (effective_cutoff - self.cutoff_frequency).abs() > 0.1 {
                        self.cutoff_frequency = effective_cutoff;
                        self.coefficients_dirty = true;
                    }
                    if (effective_resonance - self.resonance).abs() > 0.01 {
                        self.resonance = effective_resonance;
                        self.coefficients_dirty = true;
                    }

                    // Process sample through filter
                    *output_sample = self.process_sample(input_sample);
                }
            } else {
                // If not active or no input, output silence
                for sample in output_buffer.iter_mut() {
                    *sample = 0.0;
                }
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        let mut parameters = HashMap::new();
        parameters.insert("cutoff_frequency".to_string(), self.cutoff_frequency);
        parameters.insert("resonance".to_string(), self.resonance);
        parameters.insert("filter_type".to_string(), self.filter_type as u8 as f32);
        parameters.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });

        Node {
            id: Uuid::new_v4(),
            node_type: "filter".to_string(),
            name,
            parameters,
            input_ports: vec![
                Port {
                    name: "audio_in".to_string(),
                    port_type: PortType::AudioMono,
                },
                Port {
                    name: "cutoff_cv".to_string(),
                    port_type: PortType::CV,
                },
                Port {
                    name: "resonance_cv".to_string(),
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