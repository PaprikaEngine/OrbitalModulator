use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub struct OutputNode {
    pub master_volume: f32,
    pub mute: bool,
}

impl OutputNode {
    pub fn new() -> Self {
        Self {
            master_volume: 0.7,
            mute: false,
        }
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_mute(&mut self, mute: bool) {
        self.mute = mute;
    }
}

impl AudioNode for OutputNode {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        // Get input signals
        let left_input = inputs.get("audio_in_l").copied().unwrap_or(&[]);
        let right_input = inputs.get("audio_in_r").copied().unwrap_or(&[]);
        let volume_cv = inputs.get("master_volume_cv").copied().unwrap_or(&[]);

        // Get the mixed output buffer - we'll use this to store our final processed audio
        if let Some(mixed_output) = outputs.get_mut("mixed_output") {
            // Clear the output buffer
            for sample in mixed_output.iter_mut() {
                *sample = 0.0;
            }

            if self.mute {
                return; // No output when muted - buffer stays at zero
            }

            // Calculate effective volume (parameter + CV modulation)
            let effective_volume = if volume_cv.is_empty() {
                self.master_volume
            } else {
                (self.master_volume + volume_cv[0] * 0.1).clamp(0.0, 1.0)
            };

            // Mix left and right inputs and apply volume
            let buffer_size = mixed_output.len();
            
            for i in 0..buffer_size {
                let mut sample = 0.0;
                
                // Mix left channel
                if i < left_input.len() {
                    sample += left_input[i];
                }
                
                // Mix right channel
                if i < right_input.len() {
                    sample += right_input[i];
                }
                
                // Apply master volume
                mixed_output[i] = sample * effective_volume;
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        let mut parameters = HashMap::new();
        parameters.insert("master_volume".to_string(), self.master_volume);
        parameters.insert("mute".to_string(), if self.mute { 1.0 } else { 0.0 });

        Node {
            id: Uuid::new_v4(),
            node_type: "output".to_string(),
            name,
            parameters,
            input_ports: vec![
                Port {
                    name: "audio_in_l".to_string(),
                    port_type: PortType::AudioMono,
                },
                Port {
                    name: "audio_in_r".to_string(),
                    port_type: PortType::AudioMono,
                },
                Port {
                    name: "master_volume_cv".to_string(),
                    port_type: PortType::CV,
                },
            ],
            output_ports: vec![
                Port {
                    name: "mixed_output".to_string(),
                    port_type: PortType::AudioMono,
                },
            ]
        }
    }
}