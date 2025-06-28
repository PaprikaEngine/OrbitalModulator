use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

pub struct CompressorNode {
    id: Uuid,
    name: String,
    
    // Compressor parameters
    pub active: bool,
    threshold: f32,        // Compression threshold in dB
    ratio: f32,           // Compression ratio (1:1 to 20:1)
    attack: f32,          // Attack time in seconds
    release: f32,         // Release time in seconds
    knee: f32,            // Knee width in dB
    makeup_gain: f32,     // Makeup gain in dB
    
    // Limiter mode
    limiter_mode: bool,   // Enable hard limiting
    limiter_threshold: f32, // Limiter threshold in dB
    
    // Internal state
    envelope: f32,        // Envelope follower output
    gain_reduction: f32,  // Current gain reduction in dB
    sample_rate: f32,
    
    // Coefficients for envelope follower
    attack_coeff: f32,
    release_coeff: f32,
}

impl CompressorNode {
    pub fn new(name: String) -> Self {
        let sample_rate = 44100.0;
        let mut compressor = Self {
            id: Uuid::new_v4(),
            name,
            active: true,
            threshold: -20.0,      // -20dB threshold
            ratio: 4.0,           // 4:1 ratio
            attack: 0.003,        // 3ms attack
            release: 0.1,         // 100ms release
            knee: 2.0,            // 2dB knee
            makeup_gain: 0.0,     // No makeup gain
            limiter_mode: false,
            limiter_threshold: -0.1, // -0.1dB limiter threshold
            envelope: 0.0,
            gain_reduction: 0.0,
            sample_rate,
            attack_coeff: 0.0,
            release_coeff: 0.0,
        };
        
        compressor.update_coefficients();
        compressor
    }
    
    fn update_coefficients(&mut self) {
        // Calculate envelope follower coefficients
        self.attack_coeff = (-1.0 / (self.attack * self.sample_rate)).exp();
        self.release_coeff = (-1.0 / (self.release * self.sample_rate)).exp();
    }
    
    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "active" => self.active = value != 0.0,
            "threshold" => self.threshold = value.clamp(-60.0, 0.0),
            "ratio" => self.ratio = value.clamp(1.0, 20.0),
            "attack" => {
                self.attack = value.clamp(0.0001, 1.0);
                self.update_coefficients();
            },
            "release" => {
                self.release = value.clamp(0.001, 10.0);
                self.update_coefficients();
            },
            "knee" => self.knee = value.clamp(0.0, 10.0),
            "makeup_gain" => self.makeup_gain = value.clamp(-20.0, 20.0),
            "limiter_mode" => self.limiter_mode = value != 0.0,
            "limiter_threshold" => self.limiter_threshold = value.clamp(-20.0, 0.0),
            _ => return Err(format!("Unknown parameter: {}", param)),
        }
        Ok(())
    }
    
    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            "threshold" => Ok(self.threshold),
            "ratio" => Ok(self.ratio),
            "attack" => Ok(self.attack),
            "release" => Ok(self.release),
            "knee" => Ok(self.knee),
            "makeup_gain" => Ok(self.makeup_gain),
            "limiter_mode" => Ok(if self.limiter_mode { 1.0 } else { 0.0 }),
            "limiter_threshold" => Ok(self.limiter_threshold),
            "gain_reduction" => Ok(self.gain_reduction),
            _ => Err(format!("Unknown parameter: {}", param)),
        }
    }
    
    pub fn get_gain_reduction(&self) -> f32 {
        self.gain_reduction
    }
    
    fn linear_to_db(&self, linear: f32) -> f32 {
        if linear > 0.0 {
            20.0 * linear.log10()
        } else {
            -100.0 // Silence
        }
    }
    
    fn db_to_linear(&self, db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }
    
    fn process_compression(&mut self, input: f32) -> f32 {
        // Convert input to dB for processing
        let input_db = self.linear_to_db(input.abs());
        
        // Update envelope follower
        let target = input_db;
        if target > self.envelope {
            // Attack
            self.envelope = target + (self.envelope - target) * self.attack_coeff;
        } else {
            // Release
            self.envelope = target + (self.envelope - target) * self.release_coeff;
        }
        
        // Calculate gain reduction
        let over_threshold = self.envelope - self.threshold;
        
        let compression_gain = if over_threshold > 0.0 {
            if self.knee > 0.0 && over_threshold < self.knee {
                // Soft knee
                let knee_ratio = over_threshold / self.knee;
                let soft_ratio = 1.0 + (self.ratio - 1.0) * knee_ratio * knee_ratio;
                -over_threshold * (1.0 - 1.0 / soft_ratio)
            } else {
                // Hard knee
                -over_threshold * (1.0 - 1.0 / self.ratio)
            }
        } else {
            0.0
        };
        
        self.gain_reduction = compression_gain;
        
        // Apply compression
        let mut output = input * self.db_to_linear(compression_gain);
        
        // Apply makeup gain
        output *= self.db_to_linear(self.makeup_gain);
        
        // Apply limiter if enabled
        if self.limiter_mode {
            let output_db = self.linear_to_db(output.abs());
            if output_db > self.limiter_threshold {
                let limiter_gain = self.limiter_threshold - output_db;
                output *= self.db_to_linear(limiter_gain);
            }
        }
        
        output
    }
}

impl AudioNode for CompressorNode {
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
        
        // Process compression
        if let Some(output) = outputs.get_mut("audio_out") {
            for i in 0..buffer_size.min(output.len()) {
                let input_sample = if i < audio_input.len() {
                    audio_input[i]
                } else {
                    0.0
                };
                
                // Apply compression
                output[i] = self.process_compression(input_sample);
            }
        }
        
        // Provide gain reduction CV output
        if let Some(gr_output) = outputs.get_mut("gain_reduction_out") {
            for i in 0..buffer_size.min(gr_output.len()) {
                // Convert gain reduction to CV (0V = no reduction, negative = reduction)
                gr_output[i] = self.gain_reduction / 10.0; // Scale to reasonable CV range
            }
        }
    }
    
    fn create_node_info(&self, name: String) -> Node {
        Node {
            id: self.id,
            name,
            node_type: "compressor".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });
                params.insert("threshold".to_string(), self.threshold);
                params.insert("ratio".to_string(), self.ratio);
                params.insert("attack".to_string(), self.attack);
                params.insert("release".to_string(), self.release);
                params.insert("knee".to_string(), self.knee);
                params.insert("makeup_gain".to_string(), self.makeup_gain);
                params.insert("limiter_mode".to_string(), if self.limiter_mode { 1.0 } else { 0.0 });
                params.insert("limiter_threshold".to_string(), self.limiter_threshold);
                params.insert("gain_reduction".to_string(), self.gain_reduction);
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
                Port {
                    name: "gain_reduction_out".to_string(),
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

