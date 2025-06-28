use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

pub struct VCANode {
    gain: f32,
    cv_sensitivity: f32,
    sample_rate: f32,
    pub active: bool,
    id: Uuid,
    name: String,
}

impl VCANode {
    pub fn new(name: String) -> Self {
        Self {
            gain: 1.0,
            cv_sensitivity: 1.0,
            sample_rate: 44100.0,
            active: true,
            id: Uuid::new_v4(),
            name,
        }
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(0.0, 2.0);
    }

    pub fn set_cv_sensitivity(&mut self, sensitivity: f32) {
        self.cv_sensitivity = sensitivity.clamp(0.0, 2.0);
    }

    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "gain" => self.set_gain(value),
            "cv_sensitivity" => self.set_cv_sensitivity(value),
            "active" => self.active = value != 0.0,
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }

    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "gain" => Ok(self.gain),
            "cv_sensitivity" => Ok(self.cv_sensitivity),
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }

    fn process_vca_sample(&self, audio_sample: f32, cv_value: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Convert CV (-10V to +10V) to gain multiplier (0.0 to 2.0)
        let cv_gain = if cv_value != 0.0 {
            // CV modulation: 0V = 1.0x gain, +10V = 2.0x gain, -10V = 0.0x gain
            let normalized_cv = (cv_value + 10.0) / 20.0; // -10V..+10V -> 0.0..1.0
            normalized_cv.clamp(0.0, 1.0) * 2.0 * self.cv_sensitivity
        } else {
            1.0
        };

        // Apply gain and CV modulation
        audio_sample * self.gain * cv_gain
    }
}

impl AudioNode for VCANode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        let buffer_size = outputs.get("audio_out")
            .map(|buf| buf.len())
            .unwrap_or(0);

        if buffer_size == 0 {
            return;
        }

        // Create default buffers
        let default_buffer = vec![0.0; buffer_size];

        // Get input audio
        let input_audio = inputs.get("audio_in")
            .copied()
            .unwrap_or(&default_buffer);

        // Get CV input for gain control
        let gain_cv = inputs.get("gain_cv")
            .copied()
            .unwrap_or(&default_buffer);

        // Process audio output
        if let Some(output) = outputs.get_mut("audio_out") {
            for i in 0..buffer_size.min(output.len()) {
                let audio_sample = if i < input_audio.len() {
                    input_audio[i]
                } else {
                    0.0
                };

                let cv_value = if i < gain_cv.len() {
                    gain_cv[i]
                } else {
                    0.0
                };

                // Process VCA
                output[i] = self.process_vca_sample(audio_sample, cv_value);
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "vca".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("gain".to_string(), self.gain);
                params.insert("cv_sensitivity".to_string(), self.cv_sensitivity);
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params
            },
            input_ports: vec![
                Port {
                    name: "audio_in".to_string(),
                    port_type: PortType::AudioMono,
                },
                Port {
                    name: "gain_cv".to_string(),
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