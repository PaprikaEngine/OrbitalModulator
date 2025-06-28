use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub enum WaveshaperType {
    Tanh,
    ArcTan,
    Sine,
    Cubic,
    HardClip,
    SoftClip,
    Tube,
    Asymmetric,
}

pub struct WaveshaperNode {
    id: Uuid,
    name: String,
    
    // Waveshaper parameters
    pub active: bool,
    drive: f32,              // Input drive/gain
    shape_type: WaveshaperType,
    shape_amount: f32,       // Shaping intensity
    bias: f32,              // DC bias for asymmetric distortion
    output_gain: f32,       // Output level compensation
    
    // Tone shaping
    pre_filter_cutoff: f32,  // Pre-distortion filter
    post_filter_cutoff: f32, // Post-distortion filter
    
    // Internal state for filters
    pre_filter_state: f32,
    post_filter_state: f32,
    sample_rate: f32,
}

impl WaveshaperNode {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            active: true,
            drive: 1.0,
            shape_type: WaveshaperType::Tanh,
            shape_amount: 0.5,
            bias: 0.0,
            output_gain: 1.0,
            pre_filter_cutoff: 20000.0,  // No filtering by default
            post_filter_cutoff: 20000.0, // No filtering by default
            pre_filter_state: 0.0,
            post_filter_state: 0.0,
            sample_rate: 44100.0,
        }
    }
    
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "active" => self.active = value != 0.0,
            "drive" => self.drive = value.clamp(0.1, 10.0),
            "shape_type" => {
                self.shape_type = match value as u8 {
                    0 => WaveshaperType::Tanh,
                    1 => WaveshaperType::ArcTan,
                    2 => WaveshaperType::Sine,
                    3 => WaveshaperType::Cubic,
                    4 => WaveshaperType::HardClip,
                    5 => WaveshaperType::SoftClip,
                    6 => WaveshaperType::Tube,
                    7 => WaveshaperType::Asymmetric,
                    _ => return Err(format!("Invalid shape type value: {}", value)),
                };
            },
            "shape_amount" => self.shape_amount = value.clamp(0.0, 1.0),
            "bias" => self.bias = value.clamp(-1.0, 1.0),
            "output_gain" => self.output_gain = value.clamp(0.1, 2.0),
            "pre_filter_cutoff" => self.pre_filter_cutoff = value.clamp(20.0, 20000.0),
            "post_filter_cutoff" => self.post_filter_cutoff = value.clamp(20.0, 20000.0),
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            "drive" => Ok(self.drive),
            "shape_type" => Ok(self.shape_type as u8 as f32),
            "shape_amount" => Ok(self.shape_amount),
            "bias" => Ok(self.bias),
            "output_gain" => Ok(self.output_gain),
            "pre_filter_cutoff" => Ok(self.pre_filter_cutoff),
            "post_filter_cutoff" => Ok(self.post_filter_cutoff),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    pub fn get_shape_name(&self) -> &'static str {
        match self.shape_type {
            WaveshaperType::Tanh => "Tanh",
            WaveshaperType::ArcTan => "ArcTan",
            WaveshaperType::Sine => "Sine",
            WaveshaperType::Cubic => "Cubic",
            WaveshaperType::HardClip => "Hard Clip",
            WaveshaperType::SoftClip => "Soft Clip",
            WaveshaperType::Tube => "Tube",
            WaveshaperType::Asymmetric => "Asymmetric",
        }
    }
    
    fn apply_waveshaping(&self, input: f32) -> f32 {
        // Apply bias
        let biased_input = input + self.bias;
        
        // Apply drive
        let driven_input = biased_input * self.drive;
        
        // Apply waveshaping
        let shaped = match self.shape_type {
            WaveshaperType::Tanh => {
                let amount = self.shape_amount * 10.0; // Scale for tanh
                driven_input.tanh() * amount + driven_input * (1.0 - self.shape_amount)
            },
            WaveshaperType::ArcTan => {
                let amount = self.shape_amount * 5.0;
                (driven_input * amount).atan() / (amount.atan()) // Normalize
            },
            WaveshaperType::Sine => {
                let amount = self.shape_amount;
                let sine_shaped = (driven_input * std::f32::consts::PI).sin();
                sine_shaped * amount + driven_input * (1.0 - amount)
            },
            WaveshaperType::Cubic => {
                let amount = self.shape_amount;
                let cubic = driven_input - (driven_input.powi(3) / 3.0);
                cubic * amount + driven_input * (1.0 - amount)
            },
            WaveshaperType::HardClip => {
                let threshold = 1.0 - self.shape_amount * 0.8; // Keep some headroom
                driven_input.clamp(-threshold, threshold)
            },
            WaveshaperType::SoftClip => {
                let amount = self.shape_amount * 2.0 + 0.1;
                if driven_input.abs() < amount {
                    driven_input
                } else {
                    amount * driven_input.signum()
                }
            },
            WaveshaperType::Tube => {
                // Tube-like saturation curve
                let amount = self.shape_amount * 3.0 + 0.1;
                let x = driven_input / amount;
                if x.abs() < 1.0 {
                    driven_input * (1.0 - x.abs().powi(2) / 3.0)
                } else {
                    (2.0 / 3.0) * amount * x.signum()
                }
            },
            WaveshaperType::Asymmetric => {
                // Asymmetric clipping (diode-like)
                let pos_threshold = 0.7 - self.shape_amount * 0.3;
                let neg_threshold = 1.2 - self.shape_amount * 0.5;
                
                if driven_input > pos_threshold {
                    pos_threshold + (driven_input - pos_threshold) * 0.1
                } else if driven_input < -neg_threshold {
                    -neg_threshold + (driven_input + neg_threshold) * 0.1
                } else {
                    driven_input
                }
            },
        };
        
        shaped
    }
    
    fn simple_lowpass_static(input: f32, cutoff: f32, state: &mut f32, sample_rate: f32) -> f32 {
        // Simple one-pole lowpass filter
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);
        
        *state += alpha * (input - *state);
        *state
    }
}

impl AudioNode for WaveshaperNode {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        if !self.active {
            // If inactive, pass through the input signal
            if let (Some(input), Some(output)) = 
                (inputs.get("audio_in"), outputs.get_mut("audio_out")) {
                for i in 0..output.len().min(input.len()) {
                    output[i] = input[i];
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
        
        // Create default silent buffer
        let default_buffer = vec![0.0; buffer_size];
        
        // Get audio input
        let audio_input = inputs.get("audio_in")
            .copied()
            .unwrap_or(&default_buffer);
        
        // Process waveshaping
        if let Some(output) = outputs.get_mut("audio_out") {
            for i in 0..buffer_size.min(output.len()) {
                let input_sample = if i < audio_input.len() {
                    audio_input[i]
                } else {
                    0.0
                };
                
                // Apply pre-filter
                let pre_cutoff = self.pre_filter_cutoff;
                let pre_filtered = Self::simple_lowpass_static(
                    input_sample, 
                    pre_cutoff, 
                    &mut self.pre_filter_state,
                    self.sample_rate
                );
                
                // Apply waveshaping
                let shaped = self.apply_waveshaping(pre_filtered);
                
                // Apply post-filter
                let post_cutoff = self.post_filter_cutoff;
                let post_filtered = Self::simple_lowpass_static(
                    shaped, 
                    post_cutoff, 
                    &mut self.post_filter_state,
                    self.sample_rate
                );
                
                // Apply output gain and final clipping protection
                output[i] = (post_filtered * self.output_gain).clamp(-2.0, 2.0);
            }
        }
    }
    
    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "waveshaper".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params.insert("drive".to_string(), self.drive);
                params.insert("shape_type".to_string(), self.shape_type as u8 as f32);
                params.insert("shape_amount".to_string(), self.shape_amount);
                params.insert("bias".to_string(), self.bias);
                params.insert("output_gain".to_string(), self.output_gain);
                params.insert("pre_filter_cutoff".to_string(), self.pre_filter_cutoff);
                params.insert("post_filter_cutoff".to_string(), self.post_filter_cutoff);
                params
            },
            input_ports: vec![
                Port {
                    name: "audio_in".to_string(),
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
impl WaveshaperNode {
    pub fn as_any(&self) -> &dyn Any {
        self
    }
}