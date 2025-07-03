/*
 * OrbitalModulator - Professional Modular Synthesizer
 * Copyright (c) 2025 MACHIKO LAB
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use uuid::Uuid;

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;
use crate::define_parameters;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvelopeState {
    Idle,    // 待機状態
    Attack,  // アタック段階
    Decay,   // ディケイ段階  
    Sustain, // サステイン段階
    Release, // リリース段階
}

/// リファクタリング済みADSRNode - プロ品質のエンベロープジェネレーター
pub struct ADSRNode {
    // Node identification
    node_info: NodeInfo,
    
    // ADSR parameters (in seconds, except sustain)
    attack: f32,   // Attack time (0.001 - 10.0 seconds)
    decay: f32,    // Decay time (0.001 - 10.0 seconds)
    sustain: f32,  // Sustain level (0.0 - 1.0)
    release: f32,  // Release time (0.001 - 10.0 seconds)
    curve: f32,    // Envelope curve (0.5 = linear, < 0.5 = exponential, > 0.5 = logarithmic)
    velocity_sensitivity: f32, // How much velocity affects envelope amplitude
    active: f32,
    
    // CV Modulation parameters
    attack_param: ModulatableParameter,
    decay_param: ModulatableParameter,
    sustain_param: ModulatableParameter,
    release_param: ModulatableParameter,
    
    // Internal state
    state: EnvelopeState,
    current_level: f32,      // Current envelope output level (0.0 - 1.0)
    stage_progress: f32,     // Progress through current stage (0.0 - 1.0)
    gate_was_high: bool,     // Previous gate state for edge detection
    release_start_level: f32, // Level when release phase started
    velocity: f32,           // Current note velocity (0.0 - 1.0)
    
    sample_rate: f32,
}

impl ADSRNode {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "adsr".to_string(),
            category: NodeCategory::Controller,
            description: "Professional ADSR envelope generator with CV modulation and velocity sensitivity".to_string(),
            input_ports: vec![
                PortInfo::new("gate_in", PortType::CV)
                    .with_description("Gate input signal (>2.5V = trigger)"),
                PortInfo::new("velocity_in", PortType::CV)
                    .with_description("Velocity input (0V to +10V)")
                    .optional(),
                PortInfo::new("attack_cv", PortType::CV)
                    .with_description("Attack time modulation (0V to +10V)")
                    .optional(),
                PortInfo::new("decay_cv", PortType::CV)
                    .with_description("Decay time modulation (0V to +10V)")
                    .optional(),
                PortInfo::new("sustain_cv", PortType::CV)
                    .with_description("Sustain level modulation (0V to +10V)")
                    .optional(),
                PortInfo::new("release_cv", PortType::CV)
                    .with_description("Release time modulation (0V to +10V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("cv_out", PortType::CV)
                    .with_description("Envelope CV output (0V to +10V)"),
                PortInfo::new("gate_out", PortType::CV)
                    .with_description("Gate pass-through output")
                    .optional(),
                PortInfo::new("end_of_cycle", PortType::CV)
                    .with_description("Trigger at end of envelope cycle")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルADSR用
        let attack_param = ModulatableParameter::new(
            BasicParameter::new("attack", 0.001, 10.0, 0.1),
            0.8  // 80% CV modulation range
        );

        let decay_param = ModulatableParameter::new(
            BasicParameter::new("decay", 0.001, 10.0, 0.3),
            0.8  // 80% CV modulation range
        );

        let sustain_param = ModulatableParameter::new(
            BasicParameter::new("sustain", 0.0, 1.0, 0.7),
            0.8  // 80% CV modulation range
        );

        let release_param = ModulatableParameter::new(
            BasicParameter::new("release", 0.001, 10.0, 0.5),
            0.8  // 80% CV modulation range
        );

        Self {
            node_info,
            attack: 0.1,   // 100ms default attack
            decay: 0.3,    // 300ms default decay
            sustain: 0.7,  // 70% sustain level
            release: 0.5,  // 500ms default release
            curve: 0.5,    // Linear curve default
            velocity_sensitivity: 1.0, // Full velocity sensitivity
            active: 1.0,

            attack_param,
            decay_param,
            sustain_param,
            release_param,
            
            state: EnvelopeState::Idle,
            current_level: 0.0,
            stage_progress: 0.0,
            gate_was_high: false,
            release_start_level: 0.0,
            velocity: 1.0,
            
            sample_rate,
        }
    }

    /// Process gate signal and handle state transitions
    fn process_gate(&mut self, gate_high: bool, velocity: f32) {
        // Detect gate edges
        let gate_rising = gate_high && !self.gate_was_high;
        let gate_falling = !gate_high && self.gate_was_high;
        
        self.gate_was_high = gate_high;

        // Update velocity on gate rising
        if gate_rising {
            self.velocity = velocity.clamp(0.0, 1.0);
        }

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
                    self.release_start_level = self.current_level;
                } else if self.stage_progress >= 1.0 {
                    self.state = EnvelopeState::Decay;
                    self.stage_progress = 0.0;
                }
            },
            EnvelopeState::Decay => {
                if gate_falling {
                    self.state = EnvelopeState::Release;
                    self.stage_progress = 0.0;
                    self.release_start_level = self.current_level;
                } else if self.stage_progress >= 1.0 {
                    self.state = EnvelopeState::Sustain;
                    self.stage_progress = 0.0;
                }
            },
            EnvelopeState::Sustain => {
                if gate_falling {
                    self.state = EnvelopeState::Release;
                    self.stage_progress = 0.0;
                    self.release_start_level = self.current_level;
                }
                // Stay in sustain while gate is high
            },
            EnvelopeState::Release => {
                if gate_rising {
                    self.state = EnvelopeState::Attack;
                    self.stage_progress = 0.0;
                    self.velocity = velocity.clamp(0.0, 1.0);
                } else if self.stage_progress >= 1.0 {
                    self.state = EnvelopeState::Idle;
                    self.stage_progress = 0.0;
                    self.current_level = 0.0;
                }
            },
        }
    }

    /// Apply envelope curve shaping
    fn apply_curve(&self, linear_progress: f32) -> f32 {
        if self.curve < 0.5 {
            // Exponential curve (fast start, slow end)
            let curve_amount = (0.5 - self.curve) * 2.0; // 0.0 to 1.0
            let exp_factor = 1.0 + curve_amount * 4.0; // 1.0 to 5.0
            (linear_progress.powf(exp_factor)).clamp(0.0, 1.0)
        } else if self.curve > 0.5 {
            // Logarithmic curve (slow start, fast end)
            let curve_amount = (self.curve - 0.5) * 2.0; // 0.0 to 1.0
            let log_factor = 1.0 + curve_amount * 4.0; // 1.0 to 5.0
            (1.0 - (1.0 - linear_progress).powf(log_factor)).clamp(0.0, 1.0)
        } else {
            // Linear curve
            linear_progress
        }
    }

    /// Calculate envelope level for current state and advance progress
    fn calculate_envelope_level(&mut self, attack_time: f32, decay_time: f32, 
                                sustain_level: f32, release_time: f32) -> (f32, bool) {
        let mut end_of_cycle = false;
        
        match self.state {
            EnvelopeState::Idle => {
                self.current_level = 0.0;
            },
            EnvelopeState::Attack => {
                // Attack from 0 to velocity level with curve
                let curved_progress = self.apply_curve(self.stage_progress);
                self.current_level = curved_progress * self.velocity * 
                    (1.0 - self.velocity_sensitivity + self.velocity_sensitivity * self.velocity);
                
                // Advance progress
                let attack_samples = attack_time * self.sample_rate;
                self.stage_progress += 1.0 / attack_samples.max(1.0);
            },
            EnvelopeState::Decay => {
                // Decay from attack peak to sustain level with curve
                let attack_peak = self.velocity * 
                    (1.0 - self.velocity_sensitivity + self.velocity_sensitivity * self.velocity);
                let decay_range = attack_peak - sustain_level;
                let curved_progress = self.apply_curve(self.stage_progress);
                self.current_level = attack_peak - (decay_range * curved_progress);
                
                // Advance progress
                let decay_samples = decay_time * self.sample_rate;
                self.stage_progress += 1.0 / decay_samples.max(1.0);
            },
            EnvelopeState::Sustain => {
                // Hold at sustain level (modified by velocity)
                self.current_level = sustain_level * self.velocity * 
                    (1.0 - self.velocity_sensitivity + self.velocity_sensitivity * self.velocity);
                // No progress advancement needed
            },
            EnvelopeState::Release => {
                // Release from current level to 0 with curve
                let curved_progress = self.apply_curve(self.stage_progress);
                self.current_level = self.release_start_level * (1.0 - curved_progress);
                
                // Advance progress
                let release_samples = release_time * self.sample_rate;
                self.stage_progress += 1.0 / release_samples.max(1.0);
                
                // Check for end of cycle
                if self.stage_progress >= 1.0 {
                    end_of_cycle = true;
                }
            },
        }

        (self.current_level.clamp(0.0, 1.0), end_of_cycle)
    }

    /// Get current envelope state for debugging/display
    pub fn get_state(&self) -> EnvelopeState {
        self.state
    }

    /// Get current stage progress for debugging/display
    pub fn get_stage_progress(&self) -> f32 {
        self.stage_progress
    }
}

impl Parameterizable for ADSRNode {
    define_parameters! {
        attack: BasicParameter::new("attack", 0.001, 10.0, 0.1),
        decay: BasicParameter::new("decay", 0.001, 10.0, 0.3),
        sustain: BasicParameter::new("sustain", 0.0, 1.0, 0.7),
        release: BasicParameter::new("release", 0.001, 10.0, 0.5),
        curve: BasicParameter::new("curve", 0.0, 1.0, 0.5),
        velocity_sensitivity: BasicParameter::new("velocity_sensitivity", 0.0, 1.0, 1.0),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for ADSRNode {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - output zero
            if let Some(cv_output) = ctx.outputs.get_cv_mut("cv_out") {
                cv_output.fill(0.0);
            }
            if let Some(gate_output) = ctx.outputs.get_audio_mut("gate_out") {
                gate_output.fill(0.0);
            }
            if let Some(eoc_output) = ctx.outputs.get_audio_mut("end_of_cycle") {
                eoc_output.fill(0.0);
            }
            
            // Reset state when inactive
            self.state = EnvelopeState::Idle;
            self.current_level = 0.0;
            self.stage_progress = 0.0;
            
            return Ok(());
        }

        // Get input signals
        let gate_input = ctx.inputs.get_audio("gate_in").unwrap_or(&[]);
        let velocity_input = ctx.inputs.get_audio("velocity_in").unwrap_or(&[]);
        
        if gate_input.is_empty() {
            // No gate input - output zero
            if let Some(cv_output) = ctx.outputs.get_cv_mut("cv_out") {
                cv_output.fill(0.0);
            }
            if let Some(gate_output) = ctx.outputs.get_audio_mut("gate_out") {
                gate_output.fill(0.0);
            }
            if let Some(eoc_output) = ctx.outputs.get_audio_mut("end_of_cycle") {
                eoc_output.fill(0.0);
            }
            return Ok(());
        }

        // Get CV inputs
        let attack_cv = ctx.inputs.get_cv_value("attack_cv");
        let decay_cv = ctx.inputs.get_cv_value("decay_cv");
        let sustain_cv = ctx.inputs.get_cv_value("sustain_cv");
        let release_cv = ctx.inputs.get_cv_value("release_cv");

        // Apply CV modulation
        let effective_attack = self.attack_param.modulate(self.attack, attack_cv);
        let effective_decay = self.decay_param.modulate(self.decay, decay_cv);
        let effective_sustain = self.sustain_param.modulate(self.sustain, sustain_cv);
        let effective_release = self.release_param.modulate(self.release, release_cv);

        // Get the buffer size from the first output
        let buffer_size = ctx.outputs.get_cv("cv_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "cv_out".to_string() 
            })?.len();

        // Collect the processed samples first to avoid borrowing conflicts
        let mut cv_samples = Vec::with_capacity(buffer_size);
        let mut gate_samples = Vec::with_capacity(buffer_size);
        let mut eoc_samples = Vec::with_capacity(buffer_size);

        // Process each sample
        for i in 0..buffer_size {
            // Get gate signal (treat > 2.5V as high for better noise immunity)
            let gate_value = if i < gate_input.len() { 
                gate_input[i] 
            } else { 
                0.0 
            };
            let gate_high = gate_value > 2.5; // Eurorack standard gate threshold

            // Get velocity (normalize from 10V scale)
            let velocity_value = if i < velocity_input.len() { 
                velocity_input[i] 
            } else { 
                10.0 // Default full velocity
            };
            let velocity = (velocity_value / 10.0).clamp(0.0, 1.0);

            // Process gate signal and update envelope state
            self.process_gate(gate_high, velocity);

            // Calculate and output envelope level
            let (envelope_level, end_of_cycle) = self.calculate_envelope_level(
                effective_attack, effective_decay, effective_sustain, effective_release
            );
            
            // Convert to 10V CV scale (Eurorack standard)
            cv_samples.push(envelope_level * 10.0);
            gate_samples.push(gate_value);
            eoc_samples.push(if end_of_cycle { 5.0 } else { 0.0 });
        }

        // Now write to the output buffers
        if let Some(cv_output) = ctx.outputs.get_cv_mut("cv_out") {
            for (i, &sample) in cv_samples.iter().enumerate() {
                if i < cv_output.len() {
                    cv_output[i] = sample;
                }
            }
        }

        if let Some(gate_output) = ctx.outputs.get_audio_mut("gate_out") {
            for (i, &sample) in gate_samples.iter().enumerate() {
                if i < gate_output.len() {
                    gate_output[i] = sample;
                }
            }
        }

        if let Some(eoc_output) = ctx.outputs.get_audio_mut("end_of_cycle") {
            for (i, &sample) in eoc_samples.iter().enumerate() {
                if i < eoc_output.len() {
                    eoc_output[i] = sample;
                }
            }
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset envelope state
        self.state = EnvelopeState::Idle;
        self.current_level = 0.0;
        self.stage_progress = 0.0;
        self.gate_was_high = false;
        self.release_start_level = 0.0;
        self.velocity = 1.0;
    }

    fn latency(&self) -> u32 {
        0 // No latency for envelope generation
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ADSRNode {
    /// Manually trigger the ADSR envelope (for testing/manual triggering)
    pub fn trigger_gate(&mut self) {
        // Trigger attack phase
        self.state = EnvelopeState::Attack;
        self.stage_progress = 0.0;
        self.velocity = 1.0; // Full velocity
        self.gate_was_high = false; // Reset gate state so it triggers properly
        println!("🎹 ADSR gate manually triggered: {}", self.node_info.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_adsr_parameters() {
        let mut adsr = ADSRNode::new(44100.0, "test".to_string());
        
        // Test attack setting
        assert!(adsr.set_parameter("attack", 0.5).is_ok());
        assert_eq!(adsr.get_parameter("attack").unwrap(), 0.5);
        
        // Test decay setting
        assert!(adsr.set_parameter("decay", 0.8).is_ok());
        assert_eq!(adsr.get_parameter("decay").unwrap(), 0.8);
        
        // Test sustain setting
        assert!(adsr.set_parameter("sustain", 0.6).is_ok());
        assert_eq!(adsr.get_parameter("sustain").unwrap(), 0.6);
        
        // Test release setting
        assert!(adsr.set_parameter("release", 1.0).is_ok());
        assert_eq!(adsr.get_parameter("release").unwrap(), 1.0);
        
        // Test validation
        assert!(adsr.set_parameter("attack", -0.1).is_err()); // Out of range
        assert!(adsr.set_parameter("sustain", 1.5).is_err()); // Out of range
    }

    #[test]
    fn test_adsr_gate_processing() {
        let mut adsr = ADSRNode::new(44100.0, "test".to_string());
        adsr.set_parameter("attack", 0.1).unwrap(); // 100ms attack (longer for this test)
        adsr.set_parameter("sustain", 0.5).unwrap();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("gate_in".to_string(), vec![5.0; 512]); // Gate high
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_cv("cv_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        // Should process without error
        assert!(adsr.process(&mut ctx).is_ok());
        
        // Output should start from 0 and increase (attack phase)
        let output = ctx.outputs.get_cv("cv_out").unwrap();
        assert!(output[0] >= 0.0, "Should start at zero");
        assert!(output[100] > output[0], "Should increase during attack");
        
        // Should be in attack state initially
        assert_eq!(adsr.get_state(), EnvelopeState::Attack);
    }

    #[test]
    fn test_adsr_full_cycle() {
        let mut adsr = ADSRNode::new(44100.0, "test".to_string());
        adsr.set_parameter("attack", 0.01).unwrap();  // 10ms
        adsr.set_parameter("decay", 0.01).unwrap();   // 10ms  
        adsr.set_parameter("sustain", 0.5).unwrap();  // 50%
        adsr.set_parameter("release", 0.02).unwrap(); // 20ms
        
        // Attack + Decay phase (gate high)
        let mut inputs = InputBuffers::new();
        inputs.add_audio("gate_in".to_string(), vec![5.0; 1024]); // Gate high
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_cv("cv_out".to_string(), 1024);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 1024,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(adsr.process(&mut ctx).is_ok());
        
        // Should progress through attack and into decay/sustain
        let output = ctx.outputs.get_cv("cv_out").unwrap();
        let final_level = output[1023];
        assert!(final_level > 0.0, "Should be in sustain phase: {}", final_level);
        
        // Release phase (gate low) - create new buffers
        let mut inputs = InputBuffers::new();
        inputs.add_audio("gate_in".to_string(), vec![0.0; 1024]); // Gate low
        let mut outputs = OutputBuffers::new();
        outputs.allocate_cv("cv_out".to_string(), 1024);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 1024,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(adsr.process(&mut ctx).is_ok());
        
        // Should decay toward zero in release
        let output = ctx.outputs.get_cv("cv_out").unwrap();
        assert!(output[1023] < final_level, "Should decay during release");
    }

    #[test]
    fn test_velocity_sensitivity() {
        let mut adsr = ADSRNode::new(44100.0, "test".to_string());
        adsr.set_parameter("attack", 0.01).unwrap();
        adsr.set_parameter("velocity_sensitivity", 1.0).unwrap(); // Full sensitivity
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("gate_in".to_string(), vec![5.0; 256]);
        inputs.add_audio("velocity_in".to_string(), vec![5.0; 256]); // 50% velocity
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_cv("cv_out".to_string(), 256);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 256,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(adsr.process(&mut ctx).is_ok());
        
        // With 50% velocity, output should be scaled down
        let output = ctx.outputs.get_cv("cv_out").unwrap();
        let max_output = output.iter().fold(0.0f32, |a, &b| a.max(b));
        assert!(max_output < 8.0, "Should be scaled by velocity: {}", max_output); // Less than 80% of 10V
    }

    #[test]
    fn test_cv_modulation() {
        let mut adsr = ADSRNode::new(44100.0, "test".to_string());
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("gate_in".to_string(), vec![5.0; 256]);
        inputs.add_cv("attack_cv".to_string(), vec![3.0]); // Increase attack time
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_cv("cv_out".to_string(), 256);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 256,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(adsr.process(&mut ctx).is_ok());
        
        // CV should modulate the attack time
        let output = ctx.outputs.get_cv("cv_out").unwrap();
        let has_modulated_attack = output.iter().any(|&s| s > 0.0);
        assert!(has_modulated_attack, "Attack CV should affect the envelope");
    }

    #[test]
    fn test_gate_edges() {
        let mut adsr = ADSRNode::new(44100.0, "test".to_string());
        
        // Test gate rising edge
        let mut inputs = InputBuffers::new();
        inputs.add_audio("gate_in".to_string(), vec![0.0, 0.0, 5.0, 5.0]); // Rising edge
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_cv("cv_out".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(adsr.process(&mut ctx).is_ok());
        assert_eq!(adsr.get_state(), EnvelopeState::Attack);
        
        // Test gate falling edge - create new buffers
        let mut inputs = InputBuffers::new();
        inputs.add_audio("gate_in".to_string(), vec![5.0, 5.0, 0.0, 0.0]); // Falling edge
        let mut outputs = OutputBuffers::new();
        outputs.allocate_cv("cv_out".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(adsr.process(&mut ctx).is_ok());
        assert_eq!(adsr.get_state(), EnvelopeState::Release);
    }

    #[test]
    fn test_inactive_state() {
        let mut adsr = ADSRNode::new(44100.0, "test".to_string());
        adsr.set_parameter("active", 0.0).unwrap(); // Disable
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("gate_in".to_string(), vec![5.0; 512]);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_cv("cv_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(adsr.process(&mut ctx).is_ok());
        
        // Should output zero when inactive
        let output = ctx.outputs.get_cv("cv_out").unwrap();
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!((avg_output - 0.0).abs() < 0.001, "Should output zero when inactive: {}", avg_output);
        
        // Should be in idle state
        assert_eq!(adsr.get_state(), EnvelopeState::Idle);
    }
}