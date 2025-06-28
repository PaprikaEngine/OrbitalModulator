use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum Scale {
    Chromatic,
    Major,
    Minor,
    Pentatonic,
    Blues,
    Dorian,
    Mixolydian,
    Custom,
}

pub struct QuantizerNode {
    id: Uuid,
    name: String,
    
    // Quantizer parameters
    pub active: bool,
    scale: Scale,
    root_note: f32,        // Root note in 1V/Oct format (0V = C)
    transpose: f32,        // Transpose in semitones
    
    // Custom scale definition (12 booleans for each semitone)
    custom_scale: [bool; 12],
    
    // Quantization settings
    slew_rate: f32,        // Slew limiting for smooth transitions
    last_output: f32,      // For slew rate limiting
    
    sample_rate: f32,
}

impl QuantizerNode {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            active: true,
            scale: Scale::Chromatic,
            root_note: 0.0,    // C
            transpose: 0.0,
            custom_scale: [true; 12], // All notes enabled for custom scale
            slew_rate: 0.0,    // No slew by default
            last_output: 0.0,
            sample_rate: 44100.0,
        }
    }
    
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "active" => self.active = value != 0.0,
            "scale" => {
                self.scale = match value as u8 {
                    0 => Scale::Chromatic,
                    1 => Scale::Major,
                    2 => Scale::Minor,
                    3 => Scale::Pentatonic,
                    4 => Scale::Blues,
                    5 => Scale::Dorian,
                    6 => Scale::Mixolydian,
                    7 => Scale::Custom,
                    _ => return Err(format!("Invalid scale value: {}", value)),
                };
            },
            "root_note" => self.root_note = value.clamp(-5.0, 5.0),
            "transpose" => self.transpose = value.clamp(-24.0, 24.0),
            "slew_rate" => self.slew_rate = value.clamp(0.0, 1.0),
            param if param.starts_with("custom_") => {
                if let Some(note_str) = param.strip_prefix("custom_") {
                    if let Ok(note) = note_str.parse::<usize>() {
                        if note < 12 {
                            self.custom_scale[note] = value != 0.0;
                        } else {
                            return Err(format!("Invalid custom note index: {}", note));
                        }
                    } else {
                        return Err(format!("Invalid custom parameter format: {}", param));
                    }
                } else {
                    return Err(format!("Invalid custom parameter: {}", param));
                }
            },
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            "scale" => Ok(self.scale as u8 as f32),
            "root_note" => Ok(self.root_note),
            "transpose" => Ok(self.transpose),
            "slew_rate" => Ok(self.slew_rate),
            param if param.starts_with("custom_") => {
                if let Some(note_str) = param.strip_prefix("custom_") {
                    if let Ok(note) = note_str.parse::<usize>() {
                        if note < 12 {
                            Ok(if self.custom_scale[note] { 1.0 } else { 0.0 })
                        } else {
                            Err(format!("Invalid custom note index: {}", note))
                        }
                    } else {
                        Err(format!("Invalid custom parameter format: {}", param))
                    }
                } else {
                    Err(format!("Invalid custom parameter: {}", param))
                }
            },
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    fn get_scale_notes(&self) -> Vec<bool> {
        match self.scale {
            Scale::Chromatic => vec![true; 12],
            Scale::Major => vec![true, false, true, false, true, true, false, true, false, true, false, true],
            Scale::Minor => vec![true, false, true, true, false, true, false, true, true, false, true, false],
            Scale::Pentatonic => vec![true, false, true, false, true, false, false, true, false, true, false, false],
            Scale::Blues => vec![true, false, false, true, false, true, true, true, false, false, true, false],
            Scale::Dorian => vec![true, false, true, true, false, true, false, true, false, true, true, false],
            Scale::Mixolydian => vec![true, false, true, false, true, true, false, true, false, true, true, false],
            Scale::Custom => self.custom_scale.to_vec(),
        }
    }
    
    pub fn get_scale_name(&self) -> &'static str {
        match self.scale {
            Scale::Chromatic => "Chromatic",
            Scale::Major => "Major",
            Scale::Minor => "Minor",
            Scale::Pentatonic => "Pentatonic",
            Scale::Blues => "Blues",
            Scale::Dorian => "Dorian",
            Scale::Mixolydian => "Mixolydian",
            Scale::Custom => "Custom",
        }
    }
    
    fn quantize_voltage(&mut self, input_voltage: f32) -> f32 {
        // Apply root note and transpose
        let adjusted_voltage = input_voltage - self.root_note + (self.transpose / 12.0);
        
        // Convert to semitones (12 semitones per volt in 1V/Oct)
        let semitones = adjusted_voltage * 12.0;
        
        // Get the scale notes
        let scale_notes = self.get_scale_notes();
        
        // Find the closest valid note in the scale
        let base_semitone = semitones.floor() as i32;
        let fractional_part = semitones - base_semitone as f32;
        
        // Check current and next semitone positions in scale
        let current_note = ((base_semitone % 12 + 12) % 12) as usize;
        let next_note = (((base_semitone + 1) % 12 + 12) % 12) as usize;
        
        let quantized_semitone = if scale_notes[current_note] && scale_notes[next_note] {
            // Both notes are in scale, choose based on fractional part
            if fractional_part < 0.5 {
                base_semitone
            } else {
                base_semitone + 1
            }
        } else if scale_notes[current_note] {
            // Only current note is in scale
            base_semitone
        } else if scale_notes[next_note] {
            // Only next note is in scale
            base_semitone + 1
        } else {
            // Neither note is in scale, find closest valid note
            let mut closest_distance = 12;
            let mut closest_semitone = base_semitone;
            
            for offset in 1..=6 {
                // Check both directions
                for &direction in &[-1, 1] {
                    let test_semitone = base_semitone + (offset * direction);
                    let test_note = ((test_semitone % 12 + 12) % 12) as usize;
                    if scale_notes[test_note] && offset < closest_distance {
                        closest_distance = offset;
                        closest_semitone = test_semitone;
                    }
                }
            }
            closest_semitone
        };
        
        // Convert back to voltage
        let target_voltage = (quantized_semitone as f32) / 12.0 + self.root_note - (self.transpose / 12.0);
        
        // Apply slew rate limiting
        if self.slew_rate > 0.0 {
            let max_change = self.slew_rate * (1.0 / self.sample_rate);
            let difference = target_voltage - self.last_output;
            let limited_change = difference.clamp(-max_change, max_change);
            self.last_output += limited_change;
            self.last_output
        } else {
            self.last_output = target_voltage;
            target_voltage
        }
    }
}

impl AudioNode for QuantizerNode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        if !self.active {
            // If inactive, pass through the input signal
            if let (Some(input), Some(output)) = 
                (inputs.get("cv_in"), outputs.get_mut("cv_out")) {
                for i in 0..output.len().min(input.len()) {
                    output[i] = input[i];
                }
            }
            return;
        }
        
        let buffer_size = outputs.get("cv_out")
            .map(|buf| buf.len())
            .unwrap_or(0);
            
        if buffer_size == 0 {
            return;
        }
        
        // Create default silent buffer
        let default_buffer = vec![0.0; buffer_size];
        
        // Get CV input
        let cv_input = inputs.get("cv_in")
            .copied()
            .unwrap_or(&default_buffer);
        
        // Process quantization
        if let Some(output) = outputs.get_mut("cv_out") {
            for i in 0..buffer_size.min(output.len()) {
                let input_sample = if i < cv_input.len() {
                    cv_input[i]
                } else {
                    0.0
                };
                
                // Apply quantization
                output[i] = self.quantize_voltage(input_sample);
            }
        }
        
        // Also provide a trigger output when quantization changes
        if let Some(trigger_output) = outputs.get_mut("trigger_out") {
            for i in 0..buffer_size.min(trigger_output.len()) {
                // Simple trigger logic - could be enhanced
                trigger_output[i] = 0.0; // Placeholder for trigger logic
            }
        }
    }
    
    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "quantizer".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params.insert("scale".to_string(), self.scale as u8 as f32);
                params.insert("root_note".to_string(), self.root_note);
                params.insert("transpose".to_string(), self.transpose);
                params.insert("slew_rate".to_string(), self.slew_rate);
                
                // Add custom scale parameters
                for (i, &enabled) in self.custom_scale.iter().enumerate() {
                    params.insert(format!("custom_{}", i), if enabled { 1.0 } else { 0.0 });
                }
                
                params
            },
            input_ports: vec![
                Port {
                    name: "cv_in".to_string(),
                    port_type: PortType::CV,
                },
            ],
            output_ports: vec![
                Port {
                    name: "cv_out".to_string(),
                    port_type: PortType::CV,
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

