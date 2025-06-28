use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

pub struct DelayNode {
    delay_buffer: Vec<f32>,
    buffer_position: usize,
    delay_time_ms: f32,
    feedback: f32,
    mix: f32,
    sample_rate: f32,
    pub active: bool,
    id: Uuid,
    name: String,
}

impl DelayNode {
    pub fn new(name: String) -> Self {
        let delay_time_ms = 250.0; // 250ms default delay
        let sample_rate = 44100.0;
        let buffer_size = ((delay_time_ms / 1000.0) * sample_rate) as usize;
        
        Self {
            delay_buffer: vec![0.0; buffer_size],
            buffer_position: 0,
            delay_time_ms,
            feedback: 0.3,
            mix: 0.5,
            sample_rate,
            active: true,
            id: Uuid::new_v4(),
            name,
        }
    }

    pub fn set_delay_time(&mut self, delay_time_ms: f32) {
        self.delay_time_ms = delay_time_ms.clamp(1.0, 2000.0);
        
        // Resize buffer if needed
        let new_buffer_size = ((self.delay_time_ms / 1000.0) * self.sample_rate) as usize;
        if new_buffer_size != self.delay_buffer.len() {
            self.delay_buffer.resize(new_buffer_size, 0.0);
            self.buffer_position = self.buffer_position % new_buffer_size.max(1);
        }
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.95); // Limit to prevent instability
    }

    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "delay_time" => self.set_delay_time(value),
            "feedback" => self.set_feedback(value),
            "mix" => self.set_mix(value),
            "active" => self.active = value != 0.0,
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }

    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "delay_time" => Ok(self.delay_time_ms),
            "feedback" => Ok(self.feedback),
            "mix" => Ok(self.mix),
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }

    fn process_delay_sample(&mut self, input: f32) -> f32 {
        if !self.active || self.delay_buffer.is_empty() {
            return input;
        }

        // Get delayed sample from buffer
        let delayed_sample = self.delay_buffer[self.buffer_position];
        
        // Create feedback sample (input + delayed sample with feedback)
        let feedback_sample = input + (delayed_sample * self.feedback);
        
        // Store in buffer
        self.delay_buffer[self.buffer_position] = feedback_sample;
        
        // Advance buffer position
        self.buffer_position = (self.buffer_position + 1) % self.delay_buffer.len();
        
        // Mix dry and wet signals
        let dry_signal = input * (1.0 - self.mix);
        let wet_signal = delayed_sample * self.mix;
        
        dry_signal + wet_signal
    }
}

impl AudioNode for DelayNode {
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

        // Get CV inputs for modulation
        let delay_time_cv = inputs.get("delay_time_cv")
            .copied()
            .unwrap_or(&default_buffer);
        let feedback_cv = inputs.get("feedback_cv")
            .copied()
            .unwrap_or(&default_buffer);
        let mix_cv = inputs.get("mix_cv")
            .copied()
            .unwrap_or(&default_buffer);

        // Process audio output
        if let Some(output) = outputs.get_mut("audio_out") {
            for i in 0..buffer_size.min(output.len()) {
                let input_sample = if i < input_audio.len() {
                    input_audio[i]
                } else {
                    0.0
                };

                // Apply CV modulation
                if i < delay_time_cv.len() && delay_time_cv[i] != 0.0 {
                    let modulated_delay = self.delay_time_ms + (delay_time_cv[i] * 100.0);
                    self.set_delay_time(modulated_delay);
                }

                if i < feedback_cv.len() && feedback_cv[i] != 0.0 {
                    let modulated_feedback = self.feedback + (feedback_cv[i] * 0.1);
                    self.set_feedback(modulated_feedback);
                }

                if i < mix_cv.len() && mix_cv[i] != 0.0 {
                    let modulated_mix = self.mix + (mix_cv[i] * 0.1);
                    self.set_mix(modulated_mix);
                }

                // Process delay effect
                output[i] = self.process_delay_sample(input_sample);
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "delay".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("delay_time".to_string(), self.delay_time_ms);
                params.insert("feedback".to_string(), self.feedback);
                params.insert("mix".to_string(), self.mix);
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params
            },
            input_ports: vec![
                Port {
                    name: "audio_in".to_string(),
                    port_type: PortType::AudioMono,
                },
                Port {
                    name: "delay_time_cv".to_string(),
                    port_type: PortType::CV,
                },
                Port {
                    name: "feedback_cv".to_string(),
                    port_type: PortType::CV,
                },
                Port {
                    name: "mix_cv".to_string(),
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
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}