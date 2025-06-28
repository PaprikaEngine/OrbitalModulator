use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum NoiseType {
    White = 0,
    Pink = 1,
    Brown = 2,
    Blue = 3,
}

pub struct NoiseNode {
    noise_type: NoiseType,
    amplitude: f32,
    sample_rate: f32,
    pub active: bool,
    id: Uuid,
    name: String,
    
    // Noise generation state
    rng_state: u32,
    pink_state: [f32; 7],
    brown_state: f32,
}

impl NoiseNode {
    pub fn new(name: String) -> Self {
        Self {
            noise_type: NoiseType::White,
            amplitude: 0.5,
            sample_rate: 44100.0,
            active: true,
            id: Uuid::new_v4(),
            name,
            rng_state: 1,
            pink_state: [0.0; 7],
            brown_state: 0.0,
        }
    }

    pub fn set_noise_type(&mut self, noise_type: NoiseType) {
        self.noise_type = noise_type;
    }

    pub fn set_amplitude(&mut self, amplitude: f32) {
        self.amplitude = amplitude.clamp(0.0, 1.0);
    }

    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "noise_type" => {
                let noise_type = match value as u8 {
                    0 => NoiseType::White,
                    1 => NoiseType::Pink,
                    2 => NoiseType::Brown,
                    3 => NoiseType::Blue,
                    _ => return Err(format!("Invalid noise type value: {}", value)),
                };
                self.set_noise_type(noise_type);
            },
            "amplitude" => self.set_amplitude(value),
            "active" => self.active = value != 0.0,
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }

    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "noise_type" => Ok(self.noise_type as u8 as f32),
            "amplitude" => Ok(self.amplitude),
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }

    // Simple linear congruential generator for deterministic noise
    fn next_random(&mut self) -> f32 {
        self.rng_state = self.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.rng_state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn generate_white_noise(&mut self) -> f32 {
        self.next_random()
    }

    fn generate_pink_noise(&mut self) -> f32 {
        let white = self.next_random();
        
        // Pink noise using Paul Kellet's algorithm
        self.pink_state[0] = 0.99886 * self.pink_state[0] + white * 0.0555179;
        self.pink_state[1] = 0.99332 * self.pink_state[1] + white * 0.0750759;
        self.pink_state[2] = 0.96900 * self.pink_state[2] + white * 0.1538520;
        self.pink_state[3] = 0.86650 * self.pink_state[3] + white * 0.3104856;
        self.pink_state[4] = 0.55000 * self.pink_state[4] + white * 0.5329522;
        self.pink_state[5] = -0.7616 * self.pink_state[5] - white * 0.0168980;
        
        let pink = self.pink_state[0] + self.pink_state[1] + self.pink_state[2] + 
                   self.pink_state[3] + self.pink_state[4] + self.pink_state[5] + 
                   self.pink_state[6] + white * 0.5362;
        
        self.pink_state[6] = white * 0.115926;
        
        pink * 0.11
    }

    fn generate_brown_noise(&mut self) -> f32 {
        let white = self.next_random();
        self.brown_state = (self.brown_state + white * 0.02).clamp(-1.0, 1.0);
        self.brown_state
    }

    fn generate_blue_noise(&mut self) -> f32 {
        // Blue noise is the derivative of pink noise
        let current_pink = self.generate_pink_noise();
        let blue = current_pink - self.pink_state[6];
        self.pink_state[6] = current_pink;
        blue * 2.0
    }

    fn generate_noise_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let noise = match self.noise_type {
            NoiseType::White => self.generate_white_noise(),
            NoiseType::Pink => self.generate_pink_noise(),
            NoiseType::Brown => self.generate_brown_noise(),
            NoiseType::Blue => self.generate_blue_noise(),
        };

        noise * self.amplitude
    }
}

impl AudioNode for NoiseNode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        let buffer_size = outputs.get("audio_out")
            .map(|buf| buf.len())
            .unwrap_or(0);

        if buffer_size == 0 {
            return;
        }

        // Create default buffers
        let default_buffer = vec![0.0; buffer_size];

        // Get CV inputs for modulation
        let amplitude_cv = inputs.get("amplitude_cv")
            .copied()
            .unwrap_or(&default_buffer);

        // Process audio output
        if let Some(output) = outputs.get_mut("audio_out") {
            for i in 0..buffer_size.min(output.len()) {
                // Apply CV modulation to amplitude
                let mut current_amplitude = self.amplitude;
                if i < amplitude_cv.len() && amplitude_cv[i] != 0.0 {
                    current_amplitude = (self.amplitude + amplitude_cv[i] * 0.1).clamp(0.0, 1.0);
                }

                // Generate noise sample
                let original_amplitude = self.amplitude;
                self.amplitude = current_amplitude;
                output[i] = self.generate_noise_sample();
                self.amplitude = original_amplitude;
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "noise".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("noise_type".to_string(), self.noise_type as u8 as f32);
                params.insert("amplitude".to_string(), self.amplitude);
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params
            },
            input_ports: vec![
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

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}