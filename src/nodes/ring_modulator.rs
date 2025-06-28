use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

pub struct RingModulatorNode {
    id: Uuid,
    name: String,
    
    // Ring modulator parameters
    pub mix: f32,           // Dry/wet mix (0.0 = full dry, 1.0 = full modulated)
    pub carrier_gain: f32,  // Gain for the carrier signal
    pub modulator_gain: f32, // Gain for the modulator signal
    pub active: bool,
}

impl RingModulatorNode {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            mix: 1.0,
            carrier_gain: 1.0,
            modulator_gain: 1.0,
            active: true,
        }
    }
    
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "mix" => self.mix = value.clamp(0.0, 1.0),
            "carrier_gain" => self.carrier_gain = value.clamp(0.0, 2.0),
            "modulator_gain" => self.modulator_gain = value.clamp(0.0, 2.0),
            "active" => self.active = value != 0.0,
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "mix" => Ok(self.mix),
            "carrier_gain" => Ok(self.carrier_gain),
            "modulator_gain" => Ok(self.modulator_gain),
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    fn ring_modulate(&self, carrier: f32, modulator: f32) -> f32 {
        // Ring modulation: multiply the two signals
        // Apply gain to each signal before multiplication
        let scaled_carrier = carrier * self.carrier_gain;
        let scaled_modulator = modulator * self.modulator_gain;
        
        // Ring modulation is simple multiplication
        let modulated = scaled_carrier * scaled_modulator;
        
        // Mix between dry (carrier) and modulated signal
        carrier * (1.0 - self.mix) + modulated * self.mix
    }
}

impl AudioNode for RingModulatorNode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        if !self.active {
            // If inactive, just pass through the carrier signal
            if let (Some(carrier_input), Some(output)) = 
                (inputs.get("carrier_in"), outputs.get_mut("audio_out")) {
                for i in 0..output.len().min(carrier_input.len()) {
                    output[i] = carrier_input[i];
                }
            }
            return;
        }
        
        let buffer_size = outputs.get("audio_out")
            .map(|buf| buf.len())
            .unwrap_or(0);
            
        if buffer_size == 0 {
            return;
        }
        
        // Create default silent buffers
        let default_buffer = vec![0.0; buffer_size];
        
        // Get input signals
        let carrier_input = inputs.get("carrier_in")
            .copied()
            .unwrap_or(&default_buffer);
        let modulator_input = inputs.get("modulator_in")
            .copied()
            .unwrap_or(&default_buffer);
        
        // Process ring modulation
        if let Some(output) = outputs.get_mut("audio_out") {
            for i in 0..buffer_size.min(output.len()) {
                let carrier_sample = if i < carrier_input.len() {
                    carrier_input[i]
                } else {
                    0.0
                };
                
                let modulator_sample = if i < modulator_input.len() {
                    modulator_input[i]
                } else {
                    0.0
                };
                
                // Apply ring modulation
                output[i] = self.ring_modulate(carrier_sample, modulator_sample);
            }
        }
    }
    
    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "ring_modulator".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("mix".to_string(), self.mix);
                params.insert("carrier_gain".to_string(), self.carrier_gain);
                params.insert("modulator_gain".to_string(), self.modulator_gain);
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params
            },
            input_ports: vec![
                Port {
                    name: "carrier_in".to_string(),
                    port_type: PortType::AudioMono,
                },
                Port {
                    name: "modulator_in".to_string(),
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
impl RingModulatorNode {
    pub fn as_any(&self) -> &dyn Any {
        self
    }
}