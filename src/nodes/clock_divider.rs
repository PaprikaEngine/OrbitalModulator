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

/// リファクタリング済みClockDividerNode - プロ品質のクロック分周器
pub struct ClockDividerNode {
    // Node identification
    node_info: NodeInfo,
    
    // Clock divider parameters
    trigger_threshold: f32,  // 0.1V ~ 5.0V (trigger threshold)
    gate_length: f32,        // 0.001 ~ 1.0 (gate length in seconds)
    reset_mode: f32,         // 0.0 = sync reset, 1.0 = async reset
    active: f32,
    
    // CV Modulation parameters
    threshold_param: ModulatableParameter,
    
    // Division configuration
    div_ratios: Vec<u32>,    // Division ratios for each output
    
    // Internal state
    last_trigger_state: bool,
    div_counters: Vec<u32>,       // Counter for each division
    output_states: Vec<bool>,     // Current output state for each division
    gate_counters: Vec<f32>,      // Gate length counter for each output
    reset_pending: bool,          // Reset flag for sync/async reset modes
    
    sample_rate: f32,
}

impl ClockDividerNode {
    pub fn new(sample_rate: f32, name: String) -> Self {
        // Standard division ratios: /1, /2, /4, /8, /16, /32
        let div_ratios = vec![1, 2, 4, 8, 16, 32];
        let num_outputs = div_ratios.len();
        
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "clock_divider".to_string(),
            category: NodeCategory::Utility,
            description: "Professional clock divider with 6 division ratios and sync options".to_string(),
            input_ports: vec![
                PortInfo::new("clock_in", PortType::CV)
                    .with_description("Clock input (>threshold = trigger)"),
                PortInfo::new("reset_in", PortType::CV)
                    .with_description("Reset input (>threshold = reset)")
                    .optional(),
                PortInfo::new("threshold_cv", PortType::CV)
                    .with_description("Trigger threshold modulation")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("clock_out", PortType::CV)
                    .with_description("Clock passthrough output")
                    .optional(),
                PortInfo::new("div_1", PortType::CV)
                    .with_description("Divide by 1 (clock passthrough)"),
                PortInfo::new("div_2", PortType::CV)
                    .with_description("Divide by 2"),
                PortInfo::new("div_4", PortType::CV)
                    .with_description("Divide by 4"),
                PortInfo::new("div_8", PortType::CV)
                    .with_description("Divide by 8"),
                PortInfo::new("div_16", PortType::CV)
                    .with_description("Divide by 16"),
                PortInfo::new("div_32", PortType::CV)
                    .with_description("Divide by 32"),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルクロック分周器用
        let threshold_param = ModulatableParameter::new(
            BasicParameter::new("trigger_threshold", 0.1, 5.0, 1.0),
            0.5  // 50% CV modulation range
        );

        Self {
            node_info,
            trigger_threshold: 1.0,     // 1V default trigger threshold
            gate_length: 0.05,          // 50ms default gate length
            reset_mode: 0.0,            // Sync reset by default
            active: 1.0,

            threshold_param,
            
            div_ratios,
            
            last_trigger_state: false,
            div_counters: vec![0; num_outputs],
            output_states: vec![false; num_outputs],
            gate_counters: vec![0.0; num_outputs],
            reset_pending: false,
            
            sample_rate,
        }
    }

    /// Process clock division logic
    fn process_clock_division(&mut self, clock_signal: f32, reset_signal: f32, effective_threshold: f32) {
        // Detect reset signal
        let reset_trigger = reset_signal > effective_threshold;
        if reset_trigger {
            if self.reset_mode > 0.5 {
                // Async reset - immediate
                self.reset_all_counters();
            } else {
                // Sync reset - wait for next clock edge
                self.reset_pending = true;
            }
        }

        // Detect clock trigger edge (rising edge detection)
        let current_trigger_state = clock_signal > effective_threshold;
        let clock_trigger = current_trigger_state && !self.last_trigger_state;
        self.last_trigger_state = current_trigger_state;

        // Process clock trigger
        if clock_trigger {
            // Handle pending sync reset
            if self.reset_pending {
                self.reset_all_counters();
                self.reset_pending = false;
            }

            // Increment counters and check for outputs
            for i in 0..self.div_ratios.len() {
                self.div_counters[i] += 1;
                
                // Check if this division should trigger
                if self.div_counters[i] >= self.div_ratios[i] {
                    self.div_counters[i] = 0;
                    self.output_states[i] = true;
                    // Set gate length for this output
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

    /// Reset all division counters
    fn reset_all_counters(&mut self) {
        for counter in &mut self.div_counters {
            *counter = 0;
        }
        for state in &mut self.output_states {
            *state = false;
        }
        for gate_counter in &mut self.gate_counters {
            *gate_counter = 0.0;
        }
    }

    /// Get current division counter for debugging
    pub fn get_division_counter(&self, division_index: usize) -> Option<u32> {
        self.div_counters.get(division_index).copied()
    }

    /// Get current output state for debugging  
    pub fn get_output_state(&self, division_index: usize) -> Option<bool> {
        self.output_states.get(division_index).copied()
    }

    /// Get division ratios
    pub fn get_division_ratios(&self) -> &[u32] {
        &self.div_ratios
    }

    /// Set a custom division ratio
    pub fn set_division_ratio(&mut self, index: usize, ratio: u32) -> Result<(), String> {
        if index >= self.div_ratios.len() {
            return Err(format!("Division index {} out of range", index));
        }
        if ratio == 0 || ratio > 64 {
            return Err(format!("Division ratio must be 1-64, got {}", ratio));
        }
        
        self.div_ratios[index] = ratio;
        self.div_counters[index] = 0; // Reset counter when ratio changes
        Ok(())
    }
}

impl Parameterizable for ClockDividerNode {
    fn get_all_parameters(&self) -> std::collections::HashMap<String, f32> {
        let mut params = std::collections::HashMap::new();
        params.insert("trigger_threshold".to_string(), self.trigger_threshold);
        params.insert("gate_length".to_string(), self.gate_length);
        params.insert("reset_mode".to_string(), self.reset_mode);
        params.insert("active".to_string(), self.active);
        
        // Add division ratio parameters
        for (i, &ratio) in self.div_ratios.iter().enumerate() {
            params.insert(format!("div_ratio_{}", i), ratio as f32);
        }
        
        params
    }
    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), crate::parameters::ParameterError> {
        // Handle division ratio parameters
        if name.starts_with("div_ratio_") {
            if let Some(index_str) = name.strip_prefix("div_ratio_") {
                if let Ok(index) = index_str.parse::<usize>() {
                    let ratio = value as u32;
                    self.set_division_ratio(index, ratio)
                        .map_err(|e| crate::parameters::ParameterError::InvalidType { 
                            expected: "valid division ratio".to_string(), 
                            found: e 
                        })?;
                    return Ok(());
                }
            }
            return Err(crate::parameters::ParameterError::NotFound { name: name.to_string() });
        }

        // Handle standard parameters
        match name {
            "trigger_threshold" => {
                if value >= 0.1 && value <= 5.0 {
                    self.trigger_threshold = value;
                    self.threshold_param.set_base_value(value)?;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.1, max: 5.0 
                    })
                }
            },
            "gate_length" => {
                if value >= 0.001 && value <= 1.0 {
                    self.gate_length = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.001, max: 1.0 
                    })
                }
            },
            "reset_mode" => {
                if value >= 0.0 && value <= 1.0 {
                    self.reset_mode = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 1.0 
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
            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() }),
        }
    }

    fn get_parameter(&self, name: &str) -> Result<f32, crate::parameters::ParameterError> {
        // Handle division ratio parameters
        if name.starts_with("div_ratio_") {
            if let Some(index_str) = name.strip_prefix("div_ratio_") {
                if let Ok(index) = index_str.parse::<usize>() {
                    if index < self.div_ratios.len() {
                        return Ok(self.div_ratios[index] as f32);
                    } else {
                        return Err(crate::parameters::ParameterError::InvalidType {
                            expected: "valid division index".to_string(),
                            found: format!("index {}", index)
                        });
                    }
                }
            }
            return Err(crate::parameters::ParameterError::NotFound { name: name.to_string() });
        }

        // Handle standard parameters
        match name {
            "trigger_threshold" => Ok(self.trigger_threshold),
            "gate_length" => Ok(self.gate_length),
            "reset_mode" => Ok(self.reset_mode),
            "active" => Ok(self.active),
            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() }),
        }
    }

    fn get_parameter_descriptors(&self) -> Vec<Box<dyn ParameterDescriptor>> {
        let mut descriptors: Vec<Box<dyn ParameterDescriptor>> = vec![
            Box::new(BasicParameter::new("trigger_threshold", 0.1, 5.0, 1.0)),
            Box::new(BasicParameter::new("gate_length", 0.001, 1.0, 0.05)),
            Box::new(BasicParameter::new("reset_mode", 0.0, 1.0, 0.0)),
            Box::new(BasicParameter::new("active", 0.0, 1.0, 1.0)),
        ];

        // Add division ratio parameters
        for (_i, &ratio) in self.div_ratios.iter().enumerate() {
            descriptors.push(Box::new(BasicParameter::new(
                "div_ratio",  // Use static string for now
                1.0, 64.0, ratio as f32
            )));
        }

        descriptors
    }
}

impl AudioNode for ClockDividerNode {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - zero all outputs
            if let Some(clock_output) = ctx.outputs.get_audio_mut("clock_out") {
                clock_output.fill(0.0);
            }
            for &ratio in &self.div_ratios {
                if let Some(output) = ctx.outputs.get_audio_mut(&format!("div_{}", ratio)) {
                    output.fill(0.0);
                }
            }
            return Ok(());
        }

        // Get input signals
        let clock_input = ctx.inputs.get_audio("clock_in").unwrap_or(&[]);
        let reset_input = ctx.inputs.get_audio("reset_in").unwrap_or(&[]);
        
        // Get CV inputs
        let threshold_cv = ctx.inputs.get_cv_value("threshold_cv");

        // Apply CV modulation
        let effective_threshold = self.threshold_param.modulate(self.trigger_threshold, threshold_cv);

        // Get buffer size
        let buffer_size = clock_input.len();
        if buffer_size == 0 {
            return Ok(());
        }

        // Process each sample
        for i in 0..buffer_size {
            // Get input samples
            let clock_signal = if i < clock_input.len() { 
                clock_input[i] 
            } else { 
                0.0 
            };
            
            let reset_signal = if i < reset_input.len() { 
                reset_input[i] 
            } else { 
                0.0 
            };

            // Process clock division
            self.process_clock_division(clock_signal, reset_signal, effective_threshold);

            // Generate division outputs
            for (div_index, &ratio) in self.div_ratios.iter().enumerate() {
                let output_name = format!("div_{}", ratio);
                if let Some(output) = ctx.outputs.get_audio_mut(&output_name) {
                    if i < output.len() {
                        output[i] = if self.output_states[div_index] { 5.0 } else { 0.0 };
                    }
                }
            }

            // Clock passthrough output
            if let Some(clock_output) = ctx.outputs.get_audio_mut("clock_out") {
                if i < clock_output.len() {
                    clock_output[i] = clock_signal;
                }
            }
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset clock divider state
        self.reset_all_counters();
        self.last_trigger_state = false;
        self.reset_pending = false;
    }

    fn latency(&self) -> u32 {
        0 // No latency for clock division
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
    fn test_clock_divider_parameters() {
        let mut divider = ClockDividerNode::new(44100.0, "test".to_string());
        
        // Test trigger threshold setting
        assert!(divider.set_parameter("trigger_threshold", 2.5).is_ok());
        assert_eq!(divider.get_parameter("trigger_threshold").unwrap(), 2.5);
        
        // Test gate length setting
        assert!(divider.set_parameter("gate_length", 0.1).is_ok());
        assert_eq!(divider.get_parameter("gate_length").unwrap(), 0.1);
        
        // Test reset mode setting
        assert!(divider.set_parameter("reset_mode", 1.0).is_ok());
        assert_eq!(divider.get_parameter("reset_mode").unwrap(), 1.0);
        
        // Test division ratio setting
        assert!(divider.set_parameter("div_ratio_0", 3.0).is_ok());
        assert_eq!(divider.get_parameter("div_ratio_0").unwrap(), 3.0);
        
        // Test validation
        assert!(divider.set_parameter("trigger_threshold", 10.0).is_err()); // Out of range
        assert!(divider.set_parameter("gate_length", 2.0).is_err()); // Out of range
        assert!(divider.set_parameter("div_ratio_0", 0.0).is_err()); // Invalid ratio
    }

    #[test]
    fn test_basic_clock_division() {
        let mut divider = ClockDividerNode::new(44100.0, "test".to_string());
        divider.set_parameter("trigger_threshold", 1.0).unwrap();
        divider.set_parameter("gate_length", 0.1).unwrap(); // 100ms gates
        
        // Create clock signal: 4 pulses
        let clock_signal = vec![
            0.0, 2.0, 0.0, 0.0,  // Pulse 1
            0.0, 2.0, 0.0, 0.0,  // Pulse 2  
            0.0, 2.0, 0.0, 0.0,  // Pulse 3
            0.0, 2.0, 0.0, 0.0,  // Pulse 4
        ];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("clock_in".to_string(), clock_signal);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("div_1".to_string(), 16);  // /1 (every pulse)
        outputs.allocate_audio("div_2".to_string(), 16);  // /2 (every 2nd pulse)
        outputs.allocate_audio("div_4".to_string(), 16);  // /4 (every 4th pulse)
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 16,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(divider.process(&mut ctx).is_ok());
        
        let div_1_output = ctx.outputs.get_audio("div_1").unwrap();
        let div_2_output = ctx.outputs.get_audio("div_2").unwrap();
        let div_4_output = ctx.outputs.get_audio("div_4").unwrap();
        
        // /1 should trigger on every clock pulse
        assert!(div_1_output[1] > 2.0, "div_1 should trigger on pulse 1");
        assert!(div_1_output[5] > 2.0, "div_1 should trigger on pulse 2");
        assert!(div_1_output[9] > 2.0, "div_1 should trigger on pulse 3");
        assert!(div_1_output[13] > 2.0, "div_1 should trigger on pulse 4");
        
        // /2 should trigger on every 2nd pulse
        assert!(div_2_output[5] > 2.0, "div_2 should trigger on pulse 2");
        assert!(div_2_output[13] > 2.0, "div_2 should trigger on pulse 4");
        
        // /4 should trigger on every 4th pulse
        assert!(div_4_output[13] > 2.0, "div_4 should trigger on pulse 4");
    }

    #[test]
    fn test_reset_functionality() {
        let mut divider = ClockDividerNode::new(44100.0, "test".to_string());
        divider.set_parameter("trigger_threshold", 1.0).unwrap();
        divider.set_parameter("reset_mode", 1.0).unwrap(); // Async reset
        
        // Advance some counters
        divider.div_counters[1] = 1; // /2 counter at 1 (next trigger would divide)
        divider.div_counters[2] = 3; // /4 counter at 3 (next trigger would divide)
        
        let clock_signal = vec![0.0, 0.0, 0.0, 2.0]; // Clock pulse at end
        let reset_signal = vec![0.0, 2.0, 0.0, 0.0]; // Reset pulse
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("clock_in".to_string(), clock_signal);
        inputs.add_audio("reset_in".to_string(), reset_signal);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("div_2".to_string(), 4);
        outputs.allocate_audio("div_4".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(divider.process(&mut ctx).is_ok());
        
        // After reset, counters should be at 0, so subsequent clock should start fresh
        assert_eq!(divider.get_division_counter(1).unwrap(), 1); // /2 at 1 after clock
        assert_eq!(divider.get_division_counter(2).unwrap(), 1); // /4 at 1 after clock
    }

    #[test]
    fn test_gate_length() {
        let mut divider = ClockDividerNode::new(44100.0, "test".to_string());
        divider.set_parameter("trigger_threshold", 1.0).unwrap();
        divider.set_parameter("gate_length", 0.001).unwrap(); // Very short gate (1ms)
        
        let clock_signal = vec![0.0, 2.0, 0.0, 0.0]; // Single pulse
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("clock_in".to_string(), clock_signal);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("div_1".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(divider.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("div_1").unwrap();
        
        // Gate should be short - likely only sample 1 should be high
        assert!(output[1] > 2.0, "Gate should be high immediately after trigger");
        // Depending on timing, later samples may or may not be high
    }

    #[test]
    fn test_custom_division_ratios() {
        let mut divider = ClockDividerNode::new(44100.0, "test".to_string());
        
        // Set custom ratios
        assert!(divider.set_division_ratio(0, 3).is_ok()); // /3
        assert!(divider.set_division_ratio(1, 5).is_ok()); // /5
        
        assert_eq!(divider.get_division_ratios()[0], 3);
        assert_eq!(divider.get_division_ratios()[1], 5);
        
        // Test validation
        assert!(divider.set_division_ratio(0, 0).is_err()); // Invalid ratio
        assert!(divider.set_division_ratio(0, 65).is_err()); // Out of range
        assert!(divider.set_division_ratio(10, 2).is_err()); // Invalid index
    }

    #[test]
    fn test_threshold_modulation() {
        let mut divider = ClockDividerNode::new(44100.0, "test".to_string());
        divider.set_parameter("trigger_threshold", 2.0).unwrap(); // Base threshold
        
        let clock_signal = vec![1.5]; // Would not trigger with base threshold
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("clock_in".to_string(), clock_signal);
        inputs.add_cv("threshold_cv".to_string(), vec![-1.0]); // Lower threshold
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("div_1".to_string(), 1);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 1,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(divider.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("div_1").unwrap();
        
        // Should trigger due to CV modulation lowering threshold
        assert!(output[0] > 2.0, "Should trigger with CV-modulated threshold");
    }

    #[test]
    fn test_inactive_state() {
        let mut divider = ClockDividerNode::new(44100.0, "test".to_string());
        divider.set_parameter("active", 0.0).unwrap(); // Disable
        
        let clock_signal = vec![0.0, 5.0, 0.0]; // Strong clock pulse
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("clock_in".to_string(), clock_signal);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("div_1".to_string(), 3);
        outputs.allocate_audio("div_2".to_string(), 3);
        
        let mut ctx = ProcessContext {
            inputs: inputs,
            outputs: outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(divider.process(&mut ctx).is_ok());
        
        let div_1_output = ctx.outputs.get_audio("div_1").unwrap();
        let div_2_output = ctx.outputs.get_audio("div_2").unwrap();
        
        // All outputs should be zero when inactive
        for &sample in div_1_output.iter() {
            assert!((sample - 0.0).abs() < 0.001, "Should be zero when inactive");
        }
        for &sample in div_2_output.iter() {
            assert!((sample - 0.0).abs() < 0.001, "Should be zero when inactive");
        }
    }
}