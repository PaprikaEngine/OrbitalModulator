use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

pub struct SampleHoldNode {
    id: Uuid,
    name: String,
    
    // Sample & Hold state
    held_value: f32,           // The currently held sample value
    last_trigger_state: bool,  // Previous trigger state for edge detection
    trigger_threshold: f32,    // Threshold for trigger detection
    pub active: bool,
    
    // Manual trigger
    manual_trigger: bool,      // For manual triggering from UI
    manual_trigger_processed: bool, // To prevent multiple triggers
}

impl SampleHoldNode {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            held_value: 0.0,
            last_trigger_state: false,
            trigger_threshold: 0.1, // 100mV trigger threshold
            active: true,
            manual_trigger: false,
            manual_trigger_processed: false,
        }
    }
    
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "trigger_threshold" => self.trigger_threshold = value.clamp(0.01, 1.0),
            "manual_trigger" => {
                // Trigger on rising edge
                if value > 0.5 && !self.manual_trigger {
                    self.manual_trigger = true;
                    self.manual_trigger_processed = false;
                } else if value <= 0.5 {
                    self.manual_trigger = false;
                }
            },
            "active" => self.active = value != 0.0,
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "trigger_threshold" => Ok(self.trigger_threshold),
            "held_value" => Ok(self.held_value),
            "manual_trigger" => Ok(if self.manual_trigger { 1.0 } else { 0.0 }),
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    pub fn get_held_value(&self) -> f32 {
        self.held_value
    }
    
    fn process_sample_hold(&mut self, input_sample: f32, trigger_sample: f32) -> f32 {
        // Check for manual trigger first
        if self.manual_trigger && !self.manual_trigger_processed {
            self.held_value = input_sample;
            self.manual_trigger_processed = true;
            return self.held_value;
        }
        
        // Detect trigger edge (rising edge detection)
        let current_trigger_state = trigger_sample > self.trigger_threshold;
        let trigger_edge = current_trigger_state && !self.last_trigger_state;
        
        // Update trigger state for next sample
        self.last_trigger_state = current_trigger_state;
        
        // Sample new value on trigger edge
        if trigger_edge {
            self.held_value = input_sample;
        }
        
        // Always output the held value
        self.held_value
    }
}

impl AudioNode for SampleHoldNode {
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
        
        // Create default silent buffers
        let default_buffer = vec![0.0; buffer_size];
        
        // Get input signals
        let signal_input = inputs.get("signal_in")
            .copied()
            .unwrap_or(&default_buffer);
        let trigger_input = inputs.get("trigger_in")
            .copied()
            .unwrap_or(&default_buffer);
        
        // Process sample & hold
        if let Some(output) = outputs.get_mut("signal_out") {
            for i in 0..buffer_size.min(output.len()) {
                let signal_sample = if i < signal_input.len() {
                    signal_input[i]
                } else {
                    0.0
                };
                
                let trigger_sample = if i < trigger_input.len() {
                    trigger_input[i]
                } else {
                    0.0
                };
                
                // Process sample & hold
                output[i] = self.process_sample_hold(signal_sample, trigger_sample);
            }
        }
        
        // Also output the trigger signal for monitoring
        if let Some(trigger_output) = outputs.get_mut("trigger_out") {
            for i in 0..buffer_size.min(trigger_output.len()) {
                trigger_output[i] = if i < trigger_input.len() {
                    trigger_input[i]
                } else {
                    0.0
                };
            }
        }
    }
    
    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "sample_hold".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("trigger_threshold".to_string(), self.trigger_threshold);
                params.insert("held_value".to_string(), self.held_value);
                params.insert("manual_trigger".to_string(), if self.manual_trigger { 1.0 } else { 0.0 });
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params
            },
            input_ports: vec![
                Port {
                    name: "signal_in".to_string(),
                    port_type: PortType::AudioMono,
                },
                Port {
                    name: "trigger_in".to_string(),
                    port_type: PortType::CV,
                },
            ],
            output_ports: vec![
                Port {
                    name: "signal_out".to_string(),
                    port_type: PortType::AudioMono,
                },
                Port {
                    name: "trigger_out".to_string(),
                    port_type: PortType::CV,
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

