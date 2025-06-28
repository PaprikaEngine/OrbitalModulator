use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

pub struct MultipleNode {
    id: Uuid,
    name: String,
    
    // Multiple parameters
    pub active: bool,
    pub channel_count: u8,  // Number of output channels (typically 3, 4, or 8)
    
    // Optional gain per output for versatility
    output_gains: Vec<f32>,
}

impl MultipleNode {
    pub fn new(name: String, channel_count: u8) -> Self {
        let channels = channel_count.clamp(2, 8);
        Self {
            id: Uuid::new_v4(),
            name,
            active: true,
            channel_count: channels,
            output_gains: vec![1.0; channels as usize],
        }
    }
    
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "active" => self.active = value != 0.0,
            param if param.starts_with("gain_") => {
                if let Some(channel_str) = param.strip_prefix("gain_") {
                    if let Ok(channel) = channel_str.parse::<usize>() {
                        if channel < self.output_gains.len() {
                            self.output_gains[channel] = value.clamp(0.0, 2.0);
                        } else {
                            return Err(format!("Invalid channel index: {}", channel));
                        }
                    } else {
                        return Err(format!("Invalid gain parameter format: {}", param));
                    }
                } else {
                    return Err(format!("Invalid gain parameter: {}", param));
                }
            },
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            "channel_count" => Ok(self.channel_count as f32),
            param if param.starts_with("gain_") => {
                if let Some(channel_str) = param.strip_prefix("gain_") {
                    if let Ok(channel) = channel_str.parse::<usize>() {
                        if channel < self.output_gains.len() {
                            Ok(self.output_gains[channel])
                        } else {
                            Err(format!("Invalid channel index: {}", channel))
                        }
                    } else {
                        Err(format!("Invalid gain parameter format: {}", param))
                    }
                } else {
                    Err(format!("Invalid gain parameter: {}", param))
                }
            },
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
}

impl AudioNode for MultipleNode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        if !self.active {
            return;
        }
        
        let buffer_size = inputs.get("signal_in")
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
        
        // Distribute signal to all outputs with individual gains
        for channel in 0..self.channel_count {
            let output_name = format!("out_{}", channel + 1);
            if let Some(output) = outputs.get_mut(&output_name) {
                let gain = self.output_gains.get(channel as usize).copied().unwrap_or(1.0);
                
                for i in 0..buffer_size.min(output.len()) {
                    let input_sample = if i < signal_input.len() {
                        signal_input[i]
                    } else {
                        0.0
                    };
                    
                    output[i] = input_sample * gain;
                }
            }
        }
    }
    
    fn create_node_info(&self, name: String) -> Node {
        let mut output_ports = Vec::new();
        for i in 0..self.channel_count {
            output_ports.push(Port {
                name: format!("out_{}", i + 1),
                port_type: PortType::AudioMono,
            });
        }
        
        Node {
            id: self.id,
            name,
            node_type: "multiple".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params.insert("channel_count".to_string(), self.channel_count as f32);
                
                // Add individual gain parameters
                for (i, &gain) in self.output_gains.iter().enumerate() {
                    params.insert(format!("gain_{}", i), gain);
                }
                
                params
            },
            input_ports: vec![
                Port {
                    name: "signal_in".to_string(),
                    port_type: PortType::AudioMono,
                },
            ],
            output_ports,
        }
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

