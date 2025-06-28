use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

pub struct ClockDividerNode {
    id: Uuid,
    name: String,
    
    // Clock divider state
    pub active: bool,
    trigger_threshold: f32,
    last_trigger_state: bool,
    
    // Division counters for each output
    div_counters: Vec<u32>,
    div_ratios: Vec<u32>,  // Division ratios (1, 2, 4, 8, 16, etc.)
    
    // Output states
    output_states: Vec<bool>,
    gate_length: f32,  // Gate length in samples
    gate_counters: Vec<f32>,
    
    sample_rate: f32,
}

impl ClockDividerNode {
    pub fn new(name: String) -> Self {
        // Standard clock division ratios
        let ratios = vec![1, 2, 4, 8, 16, 32];
        let count = ratios.len();
        
        Self {
            id: Uuid::new_v4(),
            name,
            active: true,
            trigger_threshold: 0.1,
            last_trigger_state: false,
            div_counters: vec![0; count],
            div_ratios: ratios,
            output_states: vec![false; count],
            gate_length: 0.05, // 50ms gate length
            gate_counters: vec![0.0; count],
            sample_rate: 44100.0,
        }
    }
    
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "active" => self.active = value != 0.0,
            "trigger_threshold" => self.trigger_threshold = value.clamp(0.01, 1.0),
            "gate_length" => self.gate_length = value.clamp(0.001, 1.0), // 1ms to 1s
            param if param.starts_with("div_ratio_") => {
                if let Some(index_str) = param.strip_prefix("div_ratio_") {
                    if let Ok(index) = index_str.parse::<usize>() {
                        if index < self.div_ratios.len() {
                            let ratio = value as u32;
                            if ratio > 0 && ratio <= 64 {
                                self.div_ratios[index] = ratio;
                                // Reset counter when ratio changes
                                self.div_counters[index] = 0;
                            } else {
                                return Err(format!("Division ratio must be 1-64: {}", ratio));
                            }
                        } else {
                            return Err(format!("Invalid division index: {}", index));
                        }
                    } else {
                        return Err(format!("Invalid division parameter format: {}", param));
                    }
                } else {
                    return Err(format!("Invalid division parameter: {}", param));
                }
            },
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            "trigger_threshold" => Ok(self.trigger_threshold),
            "gate_length" => Ok(self.gate_length),
            param if param.starts_with("div_ratio_") => {
                if let Some(index_str) = param.strip_prefix("div_ratio_") {
                    if let Ok(index) = index_str.parse::<usize>() {
                        if index < self.div_ratios.len() {
                            Ok(self.div_ratios[index] as f32)
                        } else {
                            Err(format!("Invalid division index: {}", index))
                        }
                    } else {
                        Err(format!("Invalid division parameter format: {}", param))
                    }
                } else {
                    Err(format!("Invalid division parameter: {}", param))
                }
            },
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    pub fn get_division_ratios(&self) -> &[u32] {
        &self.div_ratios
    }
    
    pub fn get_output_states(&self) -> &[bool] {
        &self.output_states
    }
    
    fn process_clock_division(&mut self, trigger_sample: f32) {
        // Detect trigger edge (rising edge detection)
        let current_trigger_state = trigger_sample > self.trigger_threshold;
        let trigger_edge = current_trigger_state && !self.last_trigger_state;
        self.last_trigger_state = current_trigger_state;
        
        // On trigger edge, increment counters and check for outputs
        if trigger_edge {
            for i in 0..self.div_ratios.len() {
                self.div_counters[i] += 1;
                
                // Check if this division should trigger
                if self.div_counters[i] >= self.div_ratios[i] {
                    self.div_counters[i] = 0;
                    self.output_states[i] = true;
                    // Reset gate counter for this output
                    self.gate_counters[i] = self.gate_length * self.sample_rate;
                }
            }
        }
        
        // Update gate counters and turn off outputs when gate period expires
        for i in 0..self.gate_counters.len() {
            if self.gate_counters[i] > 0.0 {
                self.gate_counters[i] -= 1.0;
                if self.gate_counters[i] <= 0.0 {
                    self.output_states[i] = false;
                }
            }
        }
    }
}

impl AudioNode for ClockDividerNode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        if !self.active {
            return;
        }
        
        let buffer_size = inputs.get("clock_in")
            .map(|buf| buf.len())
            .unwrap_or(0);
            
        if buffer_size == 0 {
            return;
        }
        
        // Create default silent buffer
        let default_buffer = vec![0.0; buffer_size];
        
        // Get clock input
        let clock_input = inputs.get("clock_in")
            .copied()
            .unwrap_or(&default_buffer);
        
        // Process each sample
        for i in 0..buffer_size {
            let clock_sample = if i < clock_input.len() {
                clock_input[i]
            } else {
                0.0
            };
            
            // Process clock division
            self.process_clock_division(clock_sample);
            
            // Generate outputs
            for (div_index, &ratio) in self.div_ratios.iter().enumerate() {
                let output_name = format!("div_{}", ratio);
                if let Some(output) = outputs.get_mut(&output_name) {
                    if i < output.len() {
                        output[i] = if self.output_states[div_index] { 5.0 } else { 0.0 }; // 5V gate
                    }
                }
            }
        }
        
        // Also provide a pass-through clock output
        if let Some(clock_output) = outputs.get_mut("clock_out") {
            for i in 0..buffer_size.min(clock_output.len()) {
                clock_output[i] = if i < clock_input.len() {
                    clock_input[i]
                } else {
                    0.0
                };
            }
        }
    }
    
    fn create_node_info(&self, name: String) -> Node {
        let mut output_ports = vec![
            Port {
                name: "clock_out".to_string(),
                port_type: PortType::CV,
            }
        ];
        
        // Add division outputs
        for &ratio in &self.div_ratios {
            output_ports.push(Port {
                name: format!("div_{}", ratio),
                port_type: PortType::CV,
            });
        }
        
        Node {
            id: self.id,
            name,
            node_type: "clock_divider".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params.insert("trigger_threshold".to_string(), self.trigger_threshold);
                params.insert("gate_length".to_string(), self.gate_length);
                
                // Add division ratio parameters
                for (i, &ratio) in self.div_ratios.iter().enumerate() {
                    params.insert(format!("div_ratio_{}", i), ratio as f32);
                }
                
                params
            },
            input_ports: vec![
                Port {
                    name: "clock_in".to_string(),
                    port_type: PortType::CV,
                },
            ],
            output_ports,
        }
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Add as_any method for downcast access
impl ClockDividerNode {
    pub fn as_any(&self) -> &dyn Any {
        self
    }
}