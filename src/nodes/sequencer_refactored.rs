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

#[derive(Debug, Clone)]
pub struct SequenceStep {
    pub note: f32,      // Note in Hz (0.0 = rest)
    pub gate: bool,     // Gate on/off
    pub velocity: f32,  // Velocity (0.0 to 1.0)
}

impl Default for SequenceStep {
    fn default() -> Self {
        Self {
            note: 440.0,
            gate: true,
            velocity: 0.8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SequencerMode {
    Forward = 0,     // 1→2→3→4→1...
    Backward = 1,    // 4→3→2→1→4...
    PingPong = 2,    // 1→2→3→4→3→2→1...
    Random = 3,      // Random step selection
}

impl SequencerMode {
    pub fn from_f32(value: f32) -> Self {
        match value as i32 {
            0 => SequencerMode::Forward,
            1 => SequencerMode::Backward,
            2 => SequencerMode::PingPong,
            3 => SequencerMode::Random,
            _ => SequencerMode::Forward,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            SequencerMode::Forward => "Forward",
            SequencerMode::Backward => "Backward",
            SequencerMode::PingPong => "Ping-Pong",
            SequencerMode::Random => "Random",
        }
    }
}

/// リファクタリング済みSequencerNode - プロ品質の16ステップシーケンサー
pub struct SequencerNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Sequencer parameters
    bpm: f32,            // 60.0 ~ 200.0 BPM
    step_count: f32,     // 1 ~ 16 steps
    mode: f32,           // SequencerMode (0-3)
    clock_division: f32, // 1.0 = 16th notes, 2.0 = 8th notes, 4.0 = quarter notes
    swing: f32,          // 0.0 ~ 1.0 (swing amount)
    gate_length: f32,    // 0.1 ~ 1.0 (gate length as fraction of step)
    transpose: f32,      // -24.0 ~ +24.0 semitones
    active: f32,
    
    // CV Modulation parameters
    bpm_param: ModulatableParameter,
    transpose_param: ModulatableParameter,
    
    // Sequence data (16 steps maximum)
    steps: Vec<SequenceStep>,
    
    // Internal state
    current_step: usize,
    sample_counter: usize,
    samples_per_step: usize,
    running: bool,
    direction: i32,      // 1 for forward, -1 for backward (ping-pong mode)
    random_seed: u32,    // For random mode
    
    // Clock and timing
    samples_per_beat: usize,
    gate_samples_remaining: usize,
    
    sample_rate: f32,
}

impl SequencerNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "sequencer_refactored".to_string(),
            category: NodeCategory::Controller,
            description: "Professional 16-step sequencer with multiple modes and swing".to_string(),
            input_ports: vec![
                PortInfo::new("clock_in", PortType::CV)
                    .with_description("External clock input (>2.5V = trigger)")
                    .optional(),
                PortInfo::new("reset_in", PortType::CV)
                    .with_description("Reset trigger input (>2.5V = reset)")
                    .optional(),
                PortInfo::new("run_stop_in", PortType::CV)
                    .with_description("Run/stop trigger input (>2.5V = toggle)")
                    .optional(),
                PortInfo::new("bpm_cv", PortType::CV)
                    .with_description("BPM modulation (0V to +10V)")
                    .optional(),
                PortInfo::new("transpose_cv", PortType::CV)
                    .with_description("Transpose modulation (-10V to +10V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("note_cv", PortType::CV)
                    .with_description("1V/Oct note CV output"),
                PortInfo::new("gate_out", PortType::CV)
                    .with_description("Gate output (0V/5V)"),
                PortInfo::new("velocity_cv", PortType::CV)
                    .with_description("Velocity CV output (0V to +10V)"),
                PortInfo::new("trigger_out", PortType::CV)
                    .with_description("Trigger on each step")
                    .optional(),
                PortInfo::new("end_of_sequence", PortType::CV)
                    .with_description("Trigger at end of sequence")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルシーケンサー用
        let bpm_param = ModulatableParameter::new(
            BasicParameter::new("bpm", 60.0, 200.0, 120.0),
            0.8  // 80% CV modulation range
        );

        let transpose_param = ModulatableParameter::new(
            BasicParameter::new("transpose", -24.0, 24.0, 0.0),
            0.8  // 80% CV modulation range
        );

        // Initialize with a simple C major scale pattern
        let notes = [261.63, 293.66, 329.63, 349.23, 392.00, 440.00, 493.88, 523.25]; // C4 to C5
        let mut steps = Vec::with_capacity(16);
        for i in 0..16 {
            steps.push(SequenceStep {
                note: notes[i % notes.len()],
                gate: i < 8, // First 8 steps active by default
                velocity: 0.8,
            });
        }

        let bpm = 120.0;
        let clock_division = 1.0; // 16th notes
        let samples_per_beat = ((60.0 / bpm) * sample_rate) as usize;
        let samples_per_step = (samples_per_beat as f32 / (4.0 * clock_division)) as usize;

        Self {
            node_info,
            bpm: 120.0,
            step_count: 8.0,     // 8 steps default
            mode: 0.0,           // Forward mode
            clock_division: 1.0, // 16th notes
            swing: 0.0,          // No swing
            gate_length: 0.5,    // 50% gate length
            transpose: 0.0,      // No transpose
            active: 1.0,

            bpm_param,
            transpose_param,
            
            steps,
            
            current_step: 0,
            sample_counter: 0,
            samples_per_step,
            running: false,
            direction: 1,
            random_seed: 12345,
            
            samples_per_beat,
            gate_samples_remaining: 0,
            
            sample_rate,
        }
    }

    /// Update timing based on BPM and clock division
    fn update_timing(&mut self, effective_bpm: f32) {
        self.samples_per_beat = ((60.0 / effective_bpm) * self.sample_rate) as usize;
        self.samples_per_step = (self.samples_per_beat as f32 / (4.0 * self.clock_division)) as usize;
    }

    /// Calculate next step based on sequencer mode
    fn calculate_next_step(&mut self) -> usize {
        let step_count = self.step_count as usize;
        
        match SequencerMode::from_f32(self.mode) {
            SequencerMode::Forward => {
                (self.current_step + 1) % step_count
            },
            SequencerMode::Backward => {
                if self.current_step == 0 {
                    step_count - 1
                } else {
                    self.current_step - 1
                }
            },
            SequencerMode::PingPong => {
                let next_step = (self.current_step as i32 + self.direction) as usize;
                if next_step >= step_count {
                    self.direction = -1;
                    step_count.saturating_sub(2)
                } else if self.current_step == 0 && self.direction == -1 {
                    self.direction = 1;
                    1.min(step_count - 1)
                } else {
                    next_step
                }
            },
            SequencerMode::Random => {
                // Simple pseudo-random number generator
                self.random_seed = self.random_seed.wrapping_mul(1103515245).wrapping_add(12345);
                (self.random_seed / 65536) as usize % step_count
            },
        }
    }

    /// Apply swing timing
    fn get_swing_adjusted_step_length(&self, step_index: usize) -> usize {
        if self.swing == 0.0 {
            return self.samples_per_step;
        }

        // Apply swing to odd-numbered steps (off-beats)
        if step_index % 2 == 1 {
            let swing_factor = 1.0 + (self.swing * 0.5); // Up to 50% longer
            (self.samples_per_step as f32 * swing_factor) as usize
        } else {
            // Compensate even steps to maintain overall timing
            let swing_factor = 1.0 - (self.swing * 0.3); // Slightly shorter
            (self.samples_per_step as f32 * swing_factor) as usize
        }
    }

    /// Convert frequency to 1V/Oct CV
    fn freq_to_cv(&self, freq: f32, transpose_semitones: f32) -> f32 {
        if freq <= 0.0 {
            return 0.0;
        }
        
        // C4 (261.63 Hz) = 0V reference
        let c4_freq = 261.63;
        let octaves = (freq / c4_freq).log2();
        let transposed_octaves = octaves + (transpose_semitones / 12.0);
        transposed_octaves
    }

    /// Process external triggers (with edge detection)
    fn process_triggers(&mut self, clock_signal: f32, reset_signal: f32, run_stop_signal: f32) -> bool {
        let clock_trigger = clock_signal > 2.5;
        let reset_trigger = reset_signal > 2.5;
        let run_stop_trigger = run_stop_signal > 2.5;
        
        let mut step_trigger = false;

        // Reset trigger
        if reset_trigger {
            self.current_step = 0;
            self.sample_counter = 0;
            self.direction = 1;
            self.gate_samples_remaining = 0;
        }

        // Run/stop trigger
        if run_stop_trigger {
            self.running = !self.running;
            if self.running {
                self.sample_counter = 0;
            }
        }

        // External clock trigger
        if clock_trigger && self.running {
            step_trigger = true;
            let was_last_step = self.current_step == (self.step_count as usize - 1) && 
                               SequencerMode::from_f32(self.mode) == SequencerMode::Forward;
            self.advance_step();
        }

        step_trigger
    }

    /// Advance to the next step
    fn advance_step(&mut self) {
        let was_last_step = self.current_step == (self.step_count as usize - 1) && 
                           SequencerMode::from_f32(self.mode) == SequencerMode::Forward;
        
        self.current_step = self.calculate_next_step();
        self.sample_counter = 0;
        
        // Calculate gate length for this step
        let step_length = self.get_swing_adjusted_step_length(self.current_step);
        self.gate_samples_remaining = (step_length as f32 * self.gate_length) as usize;
        
        // End of sequence detection
        if was_last_step {
            // End of sequence reached
        }
    }

    /// Get current step data
    pub fn get_current_step(&self) -> &SequenceStep {
        &self.steps[self.current_step.min(self.steps.len() - 1)]
    }

    /// Set step data
    pub fn set_step(&mut self, step: usize, note: f32, gate: bool, velocity: f32) {
        if step < self.steps.len() {
            self.steps[step].note = note.clamp(20.0, 20000.0);
            self.steps[step].gate = gate;
            self.steps[step].velocity = velocity.clamp(0.0, 1.0);
        }
    }

    /// Start sequencer
    pub fn start(&mut self) {
        self.running = true;
        self.sample_counter = 0;
        self.advance_step(); // Start with first step active
        
        // Set initial gate length for first step
        let step_length = self.get_swing_adjusted_step_length(self.current_step);
        self.gate_samples_remaining = (step_length as f32 * self.gate_length) as usize;
    }

    /// Stop sequencer
    pub fn stop(&mut self) {
        self.running = false;
        self.gate_samples_remaining = 0;
    }

    /// Reset sequencer
    pub fn reset(&mut self) {
        self.current_step = 0;
        self.sample_counter = 0;
        self.direction = 1;
        self.gate_samples_remaining = 0;
    }

    /// Get sequencer running state
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Parameterizable for SequencerNodeRefactored {
    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), crate::parameters::ParameterError> {
        // Handle step-specific parameters
        if name.starts_with("step_") {
            let parts: Vec<&str> = name.split('_').collect();
            if parts.len() >= 3 {
                if let Ok(step_num) = parts[1].parse::<usize>() {
                    if step_num < self.steps.len() {
                        match parts[2] {
                            "note" => {
                                self.steps[step_num].note = value.clamp(20.0, 20000.0);
                                return Ok(());
                            },
                            "gate" => {
                                self.steps[step_num].gate = value > 0.5;
                                return Ok(());
                            },
                            "velocity" => {
                                self.steps[step_num].velocity = value.clamp(0.0, 1.0);
                                return Ok(());
                            },
                            _ => {}
                        }
                    }
                }
            }
        }

        // Handle special control parameters
        match name {
            "running" => {
                if value > 0.5 {
                    self.start();
                } else {
                    self.stop();
                }
                return Ok(());
            },
            "reset" => {
                if value > 0.5 {
                    self.reset();
                }
                return Ok(());
            },
            _ => {}
        }

        // Handle standard parameters manually with simple validation
        match name {
            "bpm" => {
                if value >= 60.0 && value <= 200.0 {
                    self.bpm = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 60.0, max: 200.0
                    })
                }
            },
            "step_count" => {
                if value >= 1.0 && value <= 16.0 {
                    self.step_count = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 1.0, max: 16.0
                    })
                }
            },
            "mode" => {
                if value >= 0.0 && value <= 3.0 {
                    self.mode = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 3.0
                    })
                }
            },
            "clock_division" => {
                if value >= 0.5 && value <= 4.0 {
                    self.clock_division = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.5, max: 4.0
                    })
                }
            },
            "swing" => {
                if value >= 0.0 && value <= 1.0 {
                    self.swing = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 1.0
                    })
                }
            },
            "gate_length" => {
                if value >= 0.1 && value <= 1.0 {
                    self.gate_length = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.1, max: 1.0
                    })
                }
            },
            "transpose" => {
                if value >= -24.0 && value <= 24.0 {
                    self.transpose = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: -24.0, max: 24.0
                    })
                }
            },
            "active" => {
                if value >= 0.0 && value <= 1.0 {
                    self.active = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 1.0
                    })
                }
            },
            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() })
        }
    }

    fn get_parameter(&self, name: &str) -> Result<f32, crate::parameters::ParameterError> {
        // Handle step-specific parameters
        if name.starts_with("step_") {
            let parts: Vec<&str> = name.split('_').collect();
            if parts.len() >= 3 {
                if let Ok(step_num) = parts[1].parse::<usize>() {
                    if step_num < self.steps.len() {
                        return match parts[2] {
                            "note" => Ok(self.steps[step_num].note),
                            "gate" => Ok(if self.steps[step_num].gate { 1.0 } else { 0.0 }),
                            "velocity" => Ok(self.steps[step_num].velocity),
                            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() })
                        };
                    }
                }
            }
        }

        // Handle special parameters
        match name {
            "current_step" => return Ok(self.current_step as f32),
            "running" => return Ok(if self.running { 1.0 } else { 0.0 }),
            _ => {}
        }

        // Handle standard parameters manually
        match name {
            "bpm" => Ok(self.bpm),
            "step_count" => Ok(self.step_count),
            "mode" => Ok(self.mode),
            "clock_division" => Ok(self.clock_division),
            "swing" => Ok(self.swing),
            "gate_length" => Ok(self.gate_length),
            "transpose" => Ok(self.transpose),
            "active" => Ok(self.active),
            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() })
        }
    }

    fn get_all_parameters(&self) -> std::collections::HashMap<String, f32> {
        let mut params = std::collections::HashMap::new();
        params.insert("bpm".to_string(), self.bpm);
        params.insert("step_count".to_string(), self.step_count);
        params.insert("mode".to_string(), self.mode);
        params.insert("clock_division".to_string(), self.clock_division);
        params.insert("swing".to_string(), self.swing);
        params.insert("gate_length".to_string(), self.gate_length);
        params.insert("transpose".to_string(), self.transpose);
        params.insert("active".to_string(), self.active);
        params.insert("current_step".to_string(), self.current_step as f32);
        params.insert("running".to_string(), if self.running { 1.0 } else { 0.0 });
        
        // Add step parameters
        for (i, step) in self.steps.iter().enumerate() {
            params.insert(format!("step_{}_note", i), step.note);
            params.insert(format!("step_{}_gate", i), if step.gate { 1.0 } else { 0.0 });
            params.insert(format!("step_{}_velocity", i), step.velocity);
        }
        
        params
    }

    fn get_parameter_descriptors(&self) -> Vec<Box<dyn ParameterDescriptor>> {
        vec![
            Box::new(BasicParameter::new("bpm", 60.0, 200.0, 120.0)),
            Box::new(BasicParameter::new("step_count", 1.0, 16.0, 8.0)),
            Box::new(BasicParameter::new("mode", 0.0, 3.0, 0.0)),
            Box::new(BasicParameter::new("clock_division", 0.5, 4.0, 1.0)),
            Box::new(BasicParameter::new("swing", 0.0, 1.0, 0.0)),
            Box::new(BasicParameter::new("gate_length", 0.1, 1.0, 0.5)),
            Box::new(BasicParameter::new("transpose", -24.0, 24.0, 0.0)),
            Box::new(BasicParameter::new("active", 0.0, 1.0, 1.0)),
        ]
    }
}

impl AudioNode for SequencerNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - output silence
            if let Some(note_output) = ctx.outputs.get_audio_mut("note_cv") {
                note_output.fill(0.0);
            }
            if let Some(gate_output) = ctx.outputs.get_audio_mut("gate_out") {
                gate_output.fill(0.0);
            }
            if let Some(velocity_output) = ctx.outputs.get_audio_mut("velocity_cv") {
                velocity_output.fill(0.0);
            }
            if let Some(trigger_output) = ctx.outputs.get_audio_mut("trigger_out") {
                trigger_output.fill(0.0);
            }
            if let Some(eos_output) = ctx.outputs.get_audio_mut("end_of_sequence") {
                eos_output.fill(0.0);
            }
            return Ok(());
        }

        // Get input signals
        let clock_input = ctx.inputs.get_audio("clock_in").unwrap_or(&[]);
        let reset_input = ctx.inputs.get_audio("reset_in").unwrap_or(&[]);
        let run_stop_input = ctx.inputs.get_audio("run_stop_in").unwrap_or(&[]);
        
        // Get CV inputs
        let bpm_cv = ctx.inputs.get_cv_value("bpm_cv");
        let transpose_cv = ctx.inputs.get_cv_value("transpose_cv");

        // Apply CV modulation
        let effective_bpm = self.bpm_param.modulate(self.bpm, bpm_cv);
        let effective_transpose = self.transpose_param.modulate(self.transpose, transpose_cv);

        // Update timing
        self.update_timing(effective_bpm);

        // Get buffer size
        let buffer_size = ctx.outputs.get_audio("note_cv")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "note_cv".to_string() 
            })?.len();

        // Process each sample
        let mut note_samples = Vec::with_capacity(buffer_size);
        let mut gate_samples = Vec::with_capacity(buffer_size);
        let mut velocity_samples = Vec::with_capacity(buffer_size);
        let mut trigger_samples = Vec::with_capacity(buffer_size);
        let mut eos_samples = Vec::with_capacity(buffer_size);

        for i in 0..buffer_size {
            // Process triggers
            let clock_signal = if i < clock_input.len() { clock_input[i] } else { 0.0 };
            let reset_signal = if i < reset_input.len() { reset_input[i] } else { 0.0 };
            let run_stop_signal = if i < run_stop_input.len() { run_stop_input[i] } else { 0.0 };
            
            let step_triggered = self.process_triggers(clock_signal, reset_signal, run_stop_signal);

            let mut step_trigger = step_triggered;  // External clock triggered
            let mut end_of_sequence = false;

            // Internal clock (if no external clock)
            if clock_input.is_empty() && self.running {
                self.sample_counter += 1;
                let step_length = self.get_swing_adjusted_step_length(self.current_step);
                
                if self.sample_counter >= step_length {
                    step_trigger = true;
                    let _was_last_step = self.current_step == (self.step_count as usize - 1) && 
                                        SequencerMode::from_f32(self.mode) == SequencerMode::Forward;
                    self.advance_step();
                    self.sample_counter = 0;  // Reset counter after step
                }
            }

            // Set gate length when step is triggered
            if step_trigger && self.running {
                let step_length = self.get_swing_adjusted_step_length(self.current_step);
                self.gate_samples_remaining = (step_length as f32 * self.gate_length) as usize;
            }

            // Get current step data (copy values to avoid borrowing issues)
            let current_step_index = self.current_step.min(self.steps.len() - 1);
            let step_note = self.steps[current_step_index].note;
            let step_gate = self.steps[current_step_index].gate;
            let step_velocity = self.steps[current_step_index].velocity;

            // Generate outputs
            let note_cv = if self.running && step_gate && step_note > 0.0 {
                self.freq_to_cv(step_note, effective_transpose)
            } else {
                0.0
            };

            let gate_cv = if self.running && step_gate && self.gate_samples_remaining > 0 {
                self.gate_samples_remaining = self.gate_samples_remaining.saturating_sub(1);
                5.0
            } else {
                0.0
            };

            let velocity_cv = if self.running && step_gate {
                step_velocity * 10.0
            } else {
                0.0
            };

            note_samples.push(note_cv);
            gate_samples.push(gate_cv);
            velocity_samples.push(velocity_cv);
            trigger_samples.push(if step_trigger { 5.0 } else { 0.0 });
            eos_samples.push(if end_of_sequence { 5.0 } else { 0.0 });
        }

        // Write to output buffers
        if let Some(note_output) = ctx.outputs.get_audio_mut("note_cv") {
            for (i, &sample) in note_samples.iter().enumerate() {
                if i < note_output.len() {
                    note_output[i] = sample;
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

        if let Some(velocity_output) = ctx.outputs.get_audio_mut("velocity_cv") {
            for (i, &sample) in velocity_samples.iter().enumerate() {
                if i < velocity_output.len() {
                    velocity_output[i] = sample;
                }
            }
        }

        if let Some(trigger_output) = ctx.outputs.get_audio_mut("trigger_out") {
            for (i, &sample) in trigger_samples.iter().enumerate() {
                if i < trigger_output.len() {
                    trigger_output[i] = sample;
                }
            }
        }

        if let Some(eos_output) = ctx.outputs.get_audio_mut("end_of_sequence") {
            for (i, &sample) in eos_samples.iter().enumerate() {
                if i < eos_output.len() {
                    eos_output[i] = sample;
                }
            }
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset sequencer state
        self.reset();
    }

    fn latency(&self) -> u32 {
        0 // No latency for sequencing
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_sequencer_parameters() {
        let mut seq = SequencerNodeRefactored::new(44100.0, "test".to_string());
        
        // Test BPM setting
        assert!(seq.set_parameter("bpm", 140.0).is_ok());
        assert_eq!(seq.get_parameter("bpm").unwrap(), 140.0);
        
        // Test step count setting
        assert!(seq.set_parameter("step_count", 12.0).is_ok());
        assert_eq!(seq.get_parameter("step_count").unwrap(), 12.0);
        
        // Test mode setting
        assert!(seq.set_parameter("mode", 2.0).is_ok()); // Ping-pong
        assert_eq!(seq.get_parameter("mode").unwrap(), 2.0);
        
        // Test validation
        assert!(seq.set_parameter("bpm", 300.0).is_err()); // Out of range
        assert!(seq.set_parameter("step_count", 20.0).is_err()); // Out of range
    }

    #[test]
    fn test_step_parameters() {
        let mut seq = SequencerNodeRefactored::new(44100.0, "test".to_string());
        
        // Test step note setting
        assert!(seq.set_parameter("step_0_note", 330.0).is_ok());
        assert_eq!(seq.get_parameter("step_0_note").unwrap(), 330.0);
        
        // Test step gate setting
        assert!(seq.set_parameter("step_1_gate", 0.0).is_ok());
        assert_eq!(seq.get_parameter("step_1_gate").unwrap(), 0.0);
        
        // Test step velocity setting
        assert!(seq.set_parameter("step_2_velocity", 0.6).is_ok());
        assert_eq!(seq.get_parameter("step_2_velocity").unwrap(), 0.6);
        
        // Test invalid step index
        assert!(seq.set_parameter("step_20_note", 440.0).is_err());
    }

    #[test]
    fn test_sequencer_basic_operation() {
        let mut seq = SequencerNodeRefactored::new(44100.0, "test".to_string());
        seq.set_parameter("step_count", 4.0).unwrap();
        seq.set_parameter("bpm", 120.0).unwrap();
        
        // Start sequencer
        seq.set_parameter("running", 1.0).unwrap();
        assert!(seq.is_running());
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("note_cv".to_string(), 512);
        outputs.allocate_audio("gate_out".to_string(), 512);
        outputs.allocate_audio("velocity_cv".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        // Should process without error
        assert!(seq.process(&mut ctx).is_ok());
        
        // Should produce output when running
        let note_output = ctx.outputs.get_audio("note_cv").unwrap();
        let gate_output = ctx.outputs.get_audio("gate_out").unwrap();
        let has_note_output = note_output.iter().any(|&s| s.abs() > 0.1);
        let has_gate_output = gate_output.iter().any(|&s| s > 2.0);
        
        assert!(has_note_output, "Should produce note CV output");
        assert!(has_gate_output, "Should produce gate output");
    }

    #[test]
    fn test_sequencer_modes() {
        let mut seq = SequencerNodeRefactored::new(44100.0, "test".to_string());
        seq.set_parameter("step_count", 4.0).unwrap();
        
        // Test different modes
        for mode in 0..4 {
            seq.set_parameter("mode", mode as f32).unwrap();
            seq.reset();
            seq.start();
            
            // Simulate several steps
            for _ in 0..8 {
                seq.advance_step();
            }
            
            // Should stay within valid step range
            assert!(seq.current_step < 4, "Step should stay within range for mode {}", mode);
        }
    }

    #[test]
    fn test_1v_oct_conversion() {
        let seq = SequencerNodeRefactored::new(44100.0, "test".to_string());
        
        // Test C4 = 0V reference
        let c4_cv = seq.freq_to_cv(261.63, 0.0);
        assert!((c4_cv - 0.0).abs() < 0.01, "C4 should be 0V: {}", c4_cv);
        
        // Test C5 = 1V (one octave up)
        let c5_cv = seq.freq_to_cv(523.25, 0.0);
        assert!((c5_cv - 1.0).abs() < 0.01, "C5 should be 1V: {}", c5_cv);
        
        // Test transpose
        let transposed_cv = seq.freq_to_cv(261.63, 12.0); // C4 + 1 octave
        assert!((transposed_cv - 1.0).abs() < 0.01, "Transposed C4 should be 1V: {}", transposed_cv);
    }

    #[test]
    fn test_external_clock() {
        let mut seq = SequencerNodeRefactored::new(44100.0, "test".to_string());
        seq.set_parameter("step_count", 4.0).unwrap();
        seq.start();
        
        // Create clock signal with triggers
        let clock_signal = vec![0.0; 256].into_iter()
            .chain(vec![5.0; 1])  // Clock trigger
            .chain(vec![0.0; 255])
            .collect();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("clock_in".to_string(), clock_signal);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("note_cv".to_string(), 512);
        outputs.allocate_audio("gate_out".to_string(), 512);
        outputs.allocate_audio("velocity_cv".to_string(), 512);
        outputs.allocate_audio("trigger_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(seq.process(&mut ctx).is_ok());
        
        // Should respond to external clock
        let trigger_output = ctx.outputs.get_audio("trigger_out").unwrap();
        let has_trigger = trigger_output.iter().any(|&s| s > 2.0);
        assert!(has_trigger, "Should respond to external clock");
    }

    #[test]
    fn test_gate_length() {
        let mut seq = SequencerNodeRefactored::new(44100.0, "test".to_string());
        seq.set_parameter("gate_length", 0.25).unwrap(); // 25% gate length
        seq.set_parameter("bpm", 120.0).unwrap();
        seq.start();
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        // Use longer buffer to capture multiple steps
        outputs.allocate_audio("note_cv".to_string(), 8192);
        outputs.allocate_audio("gate_out".to_string(), 8192);
        outputs.allocate_audio("velocity_cv".to_string(), 8192);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 8192,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(seq.process(&mut ctx).is_ok());
        
        // Gate should be 25% of total time when measured over multiple steps
        let gate_output = ctx.outputs.get_audio("gate_out").unwrap();
        let gate_high_samples = gate_output.iter().filter(|&&s| s > 2.0).count();
        let total_samples = gate_output.len();
        let gate_ratio = gate_high_samples as f32 / total_samples as f32;
        
        // Should be approximately 25% (allowing some tolerance for timing and boundaries)
        assert!(gate_ratio > 0.20 && gate_ratio < 0.40, 
                "Gate length should be around 25%: ratio={}", gate_ratio);
    }

    #[test]
    fn test_reset_functionality() {
        let mut seq = SequencerNodeRefactored::new(44100.0, "test".to_string());
        seq.start();
        
        // Advance a few steps
        seq.advance_step();
        seq.advance_step();
        assert!(seq.current_step > 0);
        
        // Reset should go back to step 0
        seq.set_parameter("reset", 1.0).unwrap();
        assert_eq!(seq.current_step, 0);
    }

    #[test]
    fn test_inactive_state() {
        let mut seq = SequencerNodeRefactored::new(44100.0, "test".to_string());
        seq.set_parameter("active", 0.0).unwrap(); // Disable
        seq.start();
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("note_cv".to_string(), 512);
        outputs.allocate_audio("gate_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(seq.process(&mut ctx).is_ok());
        
        // Should output silence when inactive
        let note_output = ctx.outputs.get_audio("note_cv").unwrap();
        let gate_output = ctx.outputs.get_audio("gate_out").unwrap();
        let avg_note = note_output.iter().sum::<f32>() / note_output.len() as f32;
        let avg_gate = gate_output.iter().sum::<f32>() / gate_output.len() as f32;
        
        assert!((avg_note - 0.0).abs() < 0.001, "Should output zero note CV when inactive: {}", avg_note);
        assert!((avg_gate - 0.0).abs() < 0.001, "Should output zero gate when inactive: {}", avg_gate);
    }
}