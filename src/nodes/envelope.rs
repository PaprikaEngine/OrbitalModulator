use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvelopeState {
    Idle,    // 待機状態
    Attack,  // アタック段階
    Decay,   // ディケイ段階  
    Sustain, // サステイン段階
    Release, // リリース段階
}

#[derive(Debug)]
pub struct ADSRNode {
    // ADSR parameters (in seconds, except sustain)
    pub attack: f32,   // Attack time (0.0 - 10.0 seconds)
    pub decay: f32,    // Decay time (0.0 - 10.0 seconds)
    pub sustain: f32,  // Sustain level (0.0 - 1.0)
    pub release: f32,  // Release time (0.0 - 10.0 seconds)
    pub active: bool,
    
    // Internal state
    state: EnvelopeState,
    current_level: f32,      // Current envelope output level (0.0 - 1.0)
    stage_progress: f32,     // Progress through current stage (0.0 - 1.0)
    gate_was_high: bool,     // Previous gate state for edge detection
    
    sample_rate: f32,
}

impl ADSRNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            attack: 0.1,   // 100ms default attack
            decay: 0.3,    // 300ms default decay
            sustain: 0.7,  // 70% sustain level
            release: 0.5,  // 500ms default release
            active: true,
            
            state: EnvelopeState::Idle,
            current_level: 0.0,
            stage_progress: 0.0,
            gate_was_high: false,
            
            sample_rate,
        }
    }

    pub fn set_attack(&mut self, attack: f32) {
        self.attack = attack.clamp(0.001, 10.0); // Minimum 1ms to avoid division by zero
    }

    pub fn set_decay(&mut self, decay: f32) {
        self.decay = decay.clamp(0.001, 10.0);
    }

    pub fn set_sustain(&mut self, sustain: f32) {
        self.sustain = sustain.clamp(0.0, 1.0);
    }

    pub fn set_release(&mut self, release: f32) {
        self.release = release.clamp(0.001, 10.0);
    }

    fn process_gate(&mut self, gate_high: bool) {
        // Detect gate edges
        let gate_rising = gate_high && !self.gate_was_high;
        let gate_falling = !gate_high && self.gate_was_high;
        
        self.gate_was_high = gate_high;

        // State transitions based on gate
        match self.state {
            EnvelopeState::Idle => {
                if gate_rising {
                    self.state = EnvelopeState::Attack;
                    self.stage_progress = 0.0;
                }
            },
            EnvelopeState::Attack => {
                if gate_falling {
                    self.state = EnvelopeState::Release;
                    self.stage_progress = 0.0;
                } else if self.stage_progress >= 1.0 {
                    self.state = EnvelopeState::Decay;
                    self.stage_progress = 0.0;
                }
            },
            EnvelopeState::Decay => {
                if gate_falling {
                    self.state = EnvelopeState::Release;
                    self.stage_progress = 0.0;
                } else if self.stage_progress >= 1.0 {
                    self.state = EnvelopeState::Sustain;
                    self.stage_progress = 0.0;
                }
            },
            EnvelopeState::Sustain => {
                if gate_falling {
                    self.state = EnvelopeState::Release;
                    self.stage_progress = 0.0;
                }
                // Stay in sustain while gate is high
            },
            EnvelopeState::Release => {
                if gate_rising {
                    self.state = EnvelopeState::Attack;
                    self.stage_progress = 0.0;
                } else if self.stage_progress >= 1.0 {
                    self.state = EnvelopeState::Idle;
                    self.stage_progress = 0.0;
                    self.current_level = 0.0;
                }
            },
        }
    }

    fn calculate_envelope_level(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Idle => {
                self.current_level = 0.0;
            },
            EnvelopeState::Attack => {
                // Linear attack from 0 to 1
                self.current_level = self.stage_progress;
                
                // Advance progress
                let attack_samples = self.attack * self.sample_rate;
                self.stage_progress += 1.0 / attack_samples;
            },
            EnvelopeState::Decay => {
                // Exponential decay from 1 to sustain level
                let decay_range = 1.0 - self.sustain;
                self.current_level = 1.0 - (decay_range * self.stage_progress);
                
                // Advance progress
                let decay_samples = self.decay * self.sample_rate;
                self.stage_progress += 1.0 / decay_samples;
            },
            EnvelopeState::Sustain => {
                // Hold at sustain level
                self.current_level = self.sustain;
                // No progress advancement needed
            },
            EnvelopeState::Release => {
                // Exponential release from current level to 0
                let release_start_level = if self.stage_progress == 0.0 {
                    self.current_level // Capture the level when release started
                } else {
                    self.current_level // Use current level for continued calculation
                };
                
                self.current_level = release_start_level * (1.0 - self.stage_progress);
                
                // Advance progress
                let release_samples = self.release * self.sample_rate;
                self.stage_progress += 1.0 / release_samples;
            },
        }

        self.current_level.clamp(0.0, 1.0)
    }
}

impl AudioNode for ADSRNode {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        let gate_input = inputs.get("gate_in").copied().unwrap_or(&[]);

        if let Some(cv_output) = outputs.get_mut("cv_out") {
            if self.active {
                for (i, cv_sample) in cv_output.iter_mut().enumerate() {
                    // Get gate signal (treat > 0.5 as high)
                    let gate_value = if i < gate_input.len() { 
                        gate_input[i] 
                    } else { 
                        0.0 
                    };
                    let gate_high = gate_value > 0.5;

                    // Process gate signal and update envelope state
                    self.process_gate(gate_high);

                    // Calculate and output envelope level
                    *cv_sample = self.calculate_envelope_level();
                }
            } else {
                // If not active, output zero CV
                for cv_sample in cv_output.iter_mut() {
                    *cv_sample = 0.0;
                }
                // Reset state when inactive
                self.state = EnvelopeState::Idle;
                self.current_level = 0.0;
                self.stage_progress = 0.0;
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        let mut parameters = HashMap::new();
        parameters.insert("attack".to_string(), self.attack);
        parameters.insert("decay".to_string(), self.decay);
        parameters.insert("sustain".to_string(), self.sustain);
        parameters.insert("release".to_string(), self.release);
        parameters.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });

        Node {
            id: Uuid::new_v4(),
            node_type: "adsr".to_string(),
            name,
            parameters,
            input_ports: vec![
                Port {
                    name: "gate_in".to_string(),
                    port_type: PortType::CV,
                },
            ],
            output_ports: vec![
                Port {
                    name: "cv_out".to_string(),
                    port_type: PortType::CV,
                },
            ],
        }
    }
}