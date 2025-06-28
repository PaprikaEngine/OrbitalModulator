use std::any::Any;
use std::collections::HashMap;
use crate::graph::{Node, Port, PortType};
use crate::nodes::AudioNode;
use uuid::Uuid;

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

pub struct SequencerNode {
    steps: Vec<SequenceStep>,
    current_step: usize,
    step_count: usize,
    bpm: f32,
    sample_rate: f32,
    samples_per_step: usize,
    sample_counter: usize,
    pub active: bool,
    id: Uuid,
    name: String,
    running: bool,
}

impl SequencerNode {
    pub fn new(name: String) -> Self {
        let step_count = 8;
        let mut steps = Vec::with_capacity(step_count);
        
        // Initialize with a simple C major scale pattern
        let notes = [261.63, 293.66, 329.63, 349.23, 392.00, 440.00, 493.88, 523.25]; // C4 to C5
        for i in 0..step_count {
            steps.push(SequenceStep {
                note: notes[i % notes.len()],
                gate: true,
                velocity: 0.8,
            });
        }

        let bpm = 120.0;
        let sample_rate = 44100.0;
        let samples_per_step = ((60.0 / bpm) * sample_rate / 4.0) as usize; // 16th notes

        Self {
            steps,
            current_step: 0,
            step_count,
            bpm,
            sample_rate,
            samples_per_step,
            sample_counter: 0,
            active: true,
            id: Uuid::new_v4(),
            name,
            running: false,
        }
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(60.0, 200.0);
        self.samples_per_step = ((60.0 / self.bpm) * self.sample_rate / 4.0) as usize;
    }

    pub fn set_step_count(&mut self, count: usize) {
        let count = count.clamp(1, 16);
        if count != self.step_count {
            self.step_count = count;
            self.steps.resize(count, SequenceStep::default());
            if self.current_step >= count {
                self.current_step = 0;
            }
        }
    }

    pub fn set_step_note(&mut self, step: usize, note: f32) {
        if step < self.steps.len() {
            self.steps[step].note = note.clamp(20.0, 20000.0);
        }
    }

    pub fn set_step_gate(&mut self, step: usize, gate: bool) {
        if step < self.steps.len() {
            self.steps[step].gate = gate;
        }
    }

    pub fn set_step_velocity(&mut self, step: usize, velocity: f32) {
        if step < self.steps.len() {
            self.steps[step].velocity = velocity.clamp(0.0, 1.0);
        }
    }

    pub fn start(&mut self) {
        self.running = true;
        self.current_step = 0;
        self.sample_counter = 0;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn reset(&mut self) {
        self.current_step = 0;
        self.sample_counter = 0;
    }

    pub fn set_parameter(&mut self, param: &str, value: f32) -> Result<(), String> {
        match param {
            "bpm" => self.set_bpm(value),
            "step_count" => self.set_step_count(value as usize),
            "running" => {
                if value != 0.0 {
                    self.start();
                } else {
                    self.stop();
                }
            },
            "reset" => {
                if value != 0.0 {
                    self.reset();
                }
            },
            "active" => self.active = value != 0.0,
            _ => {
                // Handle step-specific parameters
                if param.starts_with("step_") {
                    let parts: Vec<&str> = param.split('_').collect();
                    if parts.len() >= 3 {
                        if let Ok(step_num) = parts[1].parse::<usize>() {
                            if step_num < self.steps.len() {
                                match parts[2] {
                                    "note" => self.set_step_note(step_num, value),
                                    "gate" => self.set_step_gate(step_num, value != 0.0),
                                    "velocity" => self.set_step_velocity(step_num, value),
                                    _ => return Err(format!("Unknown step parameter: {}", param)),
                                }
                            } else {
                                return Err(format!("Step index out of range: {}", step_num));
                            }
                        } else {
                            return Err(format!("Invalid step number in parameter: {}", param));
                        }
                    } else {
                        return Err(format!("Invalid step parameter format: {}", param));
                    }
                } else {
                    return Err(format!("Unknown parameter: {}", param));
                }
            }
        }
        Ok(())
    }

    pub fn get_parameter(&self, param: &str) -> Result<f32, String> {
        match param {
            "bpm" => Ok(self.bpm),
            "step_count" => Ok(self.step_count as f32),
            "current_step" => Ok(self.current_step as f32),
            "running" => Ok(if self.running { 1.0 } else { 0.0 }),
            "active" => Ok(if self.active { 1.0 } else { 0.0 }),
            _ => {
                if param.starts_with("step_") {
                    let parts: Vec<&str> = param.split('_').collect();
                    if parts.len() >= 3 {
                        if let Ok(step_num) = parts[1].parse::<usize>() {
                            if step_num < self.steps.len() {
                                match parts[2] {
                                    "note" => Ok(self.steps[step_num].note),
                                    "gate" => Ok(if self.steps[step_num].gate { 1.0 } else { 0.0 }),
                                    "velocity" => Ok(self.steps[step_num].velocity),
                                    _ => Err(format!("Unknown step parameter: {}", param)),
                                }
                            } else {
                                Err(format!("Step index out of range: {}", step_num))
                            }
                        } else {
                            Err(format!("Invalid step number in parameter: {}", param))
                        }
                    } else {
                        Err(format!("Invalid step parameter format: {}", param))
                    }
                } else {
                    Err(format!("Unknown parameter: {}", param))
                }
            }
        }
    }

    fn process_sequence(&mut self) -> (f32, f32, f32) {
        if !self.active || !self.running {
            return (0.0, 0.0, 0.0);
        }

        let current_step_data = &self.steps[self.current_step % self.step_count];
        
        // Convert frequency to CV (1V/Oct standard)
        let note_cv = if current_step_data.gate && current_step_data.note > 0.0 {
            // Convert Hz to CV: C4 (261.63 Hz) = 0V, each octave = 1V
            let c4_freq = 261.63;
            let octaves = (current_step_data.note / c4_freq).log2();
            octaves
        } else {
            0.0
        };

        let gate_cv = if current_step_data.gate { 5.0 } else { 0.0 }; // 5V gate standard
        let velocity_cv = current_step_data.velocity * 10.0; // 0-10V velocity

        // Advance step counter
        self.sample_counter += 1;
        if self.sample_counter >= self.samples_per_step {
            self.sample_counter = 0;
            self.current_step = (self.current_step + 1) % self.step_count;
        }

        (note_cv, gate_cv, velocity_cv)
    }
}

impl AudioNode for SequencerNode {
    fn process(&mut self, _inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
        let buffer_size = outputs.get("note_cv")
            .map(|buf| buf.len())
            .unwrap_or(0);

        if buffer_size == 0 {
            return;
        }

        for i in 0..buffer_size {
            let (note_cv, gate_cv, velocity_cv) = self.process_sequence();

            // Output CV signals
            if let Some(note_out) = outputs.get_mut("note_cv") {
                if i < note_out.len() {
                    note_out[i] = note_cv;
                }
            }

            if let Some(gate_out) = outputs.get_mut("gate_out") {
                if i < gate_out.len() {
                    gate_out[i] = gate_cv;
                }
            }

            if let Some(velocity_out) = outputs.get_mut("velocity_cv") {
                if i < velocity_out.len() {
                    velocity_out[i] = velocity_cv;
                }
            }
        }
    }

    fn create_node_info(&self, name: String) -> Node {
        let mut params = HashMap::new();
        params.insert("bpm".to_string(), self.bpm);
        params.insert("step_count".to_string(), self.step_count as f32);
        params.insert("current_step".to_string(), self.current_step as f32);
        params.insert("running".to_string(), if self.running { 1.0 } else { 0.0 });
        params.insert("active".to_string(), if self.active { 1.0 } else { 0.0 });

        // Add step parameters
        for (i, step) in self.steps.iter().enumerate() {
            if i < self.step_count {
                params.insert(format!("step_{}_note", i), step.note);
                params.insert(format!("step_{}_gate", i), if step.gate { 1.0 } else { 0.0 });
                params.insert(format!("step_{}_velocity", i), step.velocity);
            }
        }

        Node {
            id: self.id,
            name,
            node_type: "sequencer".to_string(),
            parameters: params,
            input_ports: vec![],
            output_ports: vec![
                Port {
                    name: "note_cv".to_string(),
                    port_type: PortType::CV,
                },
                Port {
                    name: "gate_out".to_string(),
                    port_type: PortType::CV,
                },
                Port {
                    name: "velocity_cv".to_string(),
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