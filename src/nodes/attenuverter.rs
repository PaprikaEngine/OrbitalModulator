use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

pub struct AttenuverterNode {
    id: Uuid,
    name: String,
    
    // Attenuverter parameters
    pub attenuation: f32,    // -1.0 to +1.0 (negative = invert, positive = attenuate)
    pub offset: f32,         // DC offset -5V to +5V
    pub active: bool,
}

impl AttenuverterNode {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            attenuation: 1.0,    // No attenuation by default
            offset: 0.0,         // No offset by default
            active: true,
        }
    }
    
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "attenuation" => self.attenuation = value.clamp(-1.0, 1.0),
            "offset" => self.offset = value.clamp(-5.0, 5.0),
            "active" => self.active = value != 0.0,
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "attenuation" => Ok(self.attenuation),
            "offset" => Ok(self.offset),
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    fn process_attenuversion(&self, input: f32) -> f32 {
        // Apply attenuation/inversion first, then add offset
        let attenuated = input * self.attenuation;
        let output = attenuated + self.offset;
        
        // Soft clipping to prevent harsh digital clipping
        self.soft_clip(output)
    }
    
    fn soft_clip(&self, input: f32) -> f32 {
        // Soft clipping using tanh function for more musical saturation
        const CLIP_THRESHOLD: f32 = 5.0; // ±5V typical modular range
        
        if input.abs() > CLIP_THRESHOLD {
            CLIP_THRESHOLD * (input / CLIP_THRESHOLD).tanh()
        } else {
            input
        }
    }
}

impl AudioNode for AttenuverterNode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        if !self.active {
            // If inactive, pass through the input signal
            if let (Some(input), Some(output)) = 
                (inputs.get("signal_in"), outputs.get_mut("signal_out")) {
                for i in 0..output.len().min(input.len()) {
                    output[i] = input[i];
                }
            }
            return;
        }
        
        let buffer_size = outputs.get("signal_out")
            .map(|buf| buf.len())
            .unwrap_or(0);
            
        if buffer_size == 0 {
            return;
        }
        
        // Create default silent buffer
        let default_buffer = vec![0.0; buffer_size];
        
        // Get input signal
        let signal_input = inputs.get("signal_in")
            .copied()
            .unwrap_or(&default_buffer);
        
        // Process attenuversion
        if let Some(output) = outputs.get_mut("signal_out") {
            for i in 0..buffer_size.min(output.len()) {
                let input_sample = if i < signal_input.len() {
                    signal_input[i]
                } else {
                    0.0
                };
                
                // Apply attenuation/inversion and offset
                output[i] = self.process_attenuversion(input_sample);
            }
        }
        
        // Also provide an inverted output for convenience
        if let Some(inverted_output) = outputs.get_mut("inverted_out") {
            for i in 0..buffer_size.min(inverted_output.len()) {
                let input_sample = if i < signal_input.len() {
                    signal_input[i]
                } else {
                    0.0
                };
                
                // Inverted version without offset
                inverted_output[i] = self.soft_clip(-input_sample * self.attenuation.abs());
            }
        }
    }
    
    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "attenuverter".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("attenuation".to_string(), self.attenuation);
                params.insert("offset".to_string(), self.offset);
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params
            },
            input_ports: vec![
                Port {
                    name: "signal_in".to_string(),
                    port_type: PortType::AudioMono,
                },
            ],
            output_ports: vec![
                Port {
                    name: "signal_out".to_string(),
                    port_type: PortType::AudioMono,
                },
                Port {
                    name: "inverted_out".to_string(),
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
impl AttenuverterNode {
    pub fn as_any(&self) -> &dyn Any {
        self
    }
}