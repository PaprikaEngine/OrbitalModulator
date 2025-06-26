use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub struct SineOscillatorNode {
    pub frequency: f32,
    pub amplitude: f32,
    phase: f32,
    sample_rate: f32,
}

impl SineOscillatorNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            frequency: 440.0,  // A4 default
            amplitude: 0.5,
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
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        let mut parameters = HashMap::new();
        parameters.insert("frequency".to_string(), self.frequency);
        parameters.insert("amplitude".to_string(), self.amplitude);

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