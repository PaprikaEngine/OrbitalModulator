use uuid::Uuid;

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor, ModulationCurve};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;
use crate::define_parameters;

/// リファクタリング済みSampleHoldNode - プロ品質のサンプル&ホールド
pub struct SampleHoldNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Sample & Hold parameters
    trigger_threshold: f32,   // 0.1V ~ 5.0V (trigger threshold)
    manual_trigger: f32,      // 0.0/1.0 (manual trigger button)
    slew_rate: f32,          // 0.0 ~ 1.0 (slew limiting for smooth transitions)
    track_mode: f32,         // 0.0 = Track/Hold, 1.0 = continuous hold
    active: f32,
    
    // CV Modulation parameters
    threshold_param: ModulatableParameter,
    
    // Internal state
    held_value: f32,           // Currently held sample value
    last_trigger_state: bool,  // Previous trigger state for edge detection
    manual_trigger_processed: bool, // Prevent multiple manual triggers
    slew_target: f32,         // Target value for slewing
    slew_current: f32,        // Current slewed value
    
    sample_rate: f32,
}

impl SampleHoldNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "sample_hold_refactored".to_string(),
            category: NodeCategory::Utility,
            description: "Professional sample and hold module with slew limiting".to_string(),
            input_ports: vec![
                PortInfo::new("signal_in", PortType::AudioMono)
                    .with_description("Input signal to be sampled"),
                PortInfo::new("trigger_in", PortType::CV)
                    .with_description("Trigger input (>threshold = sample)")
                    .optional(),
                PortInfo::new("threshold_cv", PortType::CV)
                    .with_description("Trigger threshold modulation")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("signal_out", PortType::AudioMono)
                    .with_description("Sample and hold output"),
                PortInfo::new("trigger_out", PortType::CV)
                    .with_description("Trigger passthrough for chaining")
                    .optional(),
                PortInfo::new("gate_out", PortType::CV)
                    .with_description("Gate high when holding")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルS&H用
        let threshold_param = ModulatableParameter::new(
            BasicParameter::new("trigger_threshold", 0.1, 5.0, 1.0),
            0.5  // 50% CV modulation range
        );

        Self {
            node_info,
            trigger_threshold: 1.0,     // 1V default trigger threshold
            manual_trigger: 0.0,
            slew_rate: 0.0,            // No slew limiting by default
            track_mode: 0.0,           // Track/Hold mode by default
            active: 1.0,

            threshold_param,
            
            held_value: 0.0,
            last_trigger_state: false,
            manual_trigger_processed: false,
            slew_target: 0.0,
            slew_current: 0.0,
            
            sample_rate,
        }
    }

    /// Process sample and hold logic
    fn process_sample_hold(&mut self, input_sample: f32, trigger_sample: f32, effective_threshold: f32) -> f32 {
        // Check for manual trigger first
        if self.manual_trigger > 0.5 && !self.manual_trigger_processed {
            self.sample_new_value(input_sample);
            self.manual_trigger_processed = true;
        } else if self.manual_trigger <= 0.5 {
            self.manual_trigger_processed = false;
        }
        
        // Detect trigger edge (rising edge detection)
        let current_trigger_state = trigger_sample > effective_threshold;
        let trigger_edge = current_trigger_state && !self.last_trigger_state;
        
        // Update trigger state for next sample
        self.last_trigger_state = current_trigger_state;
        
        // Track mode: continuously track input when trigger is high
        if self.track_mode > 0.5 && current_trigger_state {
            self.sample_new_value(input_sample);
        } else if trigger_edge {
            // Standard S&H: sample on trigger edge
            self.sample_new_value(input_sample);
        }
        
        // Apply slew limiting if enabled
        if self.slew_rate > 0.0 {
            self.apply_slew_limiting()
        } else {
            self.held_value
        }
    }

    /// Sample a new value and set slew target
    fn sample_new_value(&mut self, new_value: f32) {
        self.slew_target = new_value;
        if self.slew_rate == 0.0 {
            // No slew limiting - instant change
            self.held_value = new_value;
            self.slew_current = new_value;
        }
    }

    /// Apply slew limiting to smooth value changes
    fn apply_slew_limiting(&mut self) -> f32 {
        if (self.slew_current - self.slew_target).abs() < 0.001 {
            // Close enough - snap to target
            self.slew_current = self.slew_target;
            self.held_value = self.slew_target;
            return self.held_value;
        }

        // Calculate slew rate in volts per sample
        let slew_rate_per_sample = self.slew_rate * 20.0 / self.sample_rate; // 20V/s maximum slew rate
        
        if self.slew_current < self.slew_target {
            self.slew_current += slew_rate_per_sample;
            if self.slew_current > self.slew_target {
                self.slew_current = self.slew_target;
            }
        } else {
            self.slew_current -= slew_rate_per_sample;
            if self.slew_current < self.slew_target {
                self.slew_current = self.slew_target;
            }
        }

        self.held_value = self.slew_current;
        self.held_value
    }

    /// Get the currently held value (for UI display)
    pub fn get_held_value(&self) -> f32 {
        self.held_value
    }

    /// Check if currently holding a sample
    pub fn is_holding(&self) -> bool {
        // Consider "holding" when not in track mode or when trigger is low in track mode
        if self.track_mode > 0.5 {
            !self.last_trigger_state
        } else {
            true // Always holding in normal S&H mode
        }
    }
}

impl Parameterizable for SampleHoldNodeRefactored {
    define_parameters! {
        trigger_threshold: BasicParameter::new("trigger_threshold", 0.1, 5.0, 1.0),
        manual_trigger: BasicParameter::new("manual_trigger", 0.0, 1.0, 0.0),
        slew_rate: BasicParameter::new("slew_rate", 0.0, 1.0, 0.0),
        track_mode: BasicParameter::new("track_mode", 0.0, 1.0, 0.0),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for SampleHoldNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - pass through input signal
            if let (Some(input), Some(output)) = 
                (ctx.inputs.get_audio("signal_in"), ctx.outputs.get_audio_mut("signal_out")) {
                output.copy_from_slice(&input[..output.len().min(input.len())]);
            }
            return Ok(());
        }

        // Get input signals
        let signal_input = ctx.inputs.get_audio("signal_in").unwrap_or(&[]);
        let trigger_input = ctx.inputs.get_audio("trigger_in").unwrap_or(&[]);
        
        // Get CV inputs
        let threshold_cv = ctx.inputs.get_cv_value("threshold_cv");

        // Apply CV modulation
        let effective_threshold = self.threshold_param.modulate(self.trigger_threshold, threshold_cv);

        // Get buffer size
        let buffer_size = ctx.outputs.get_audio("signal_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "signal_out".to_string() 
            })?.len();

        // Process each sample
        let mut output_samples = Vec::with_capacity(buffer_size);
        let mut gate_samples = Vec::with_capacity(buffer_size);

        for i in 0..buffer_size {
            // Get input samples
            let input_sample = if i < signal_input.len() { 
                signal_input[i] 
            } else { 
                0.0 
            };
            
            let trigger_sample = if i < trigger_input.len() { 
                trigger_input[i] 
            } else { 
                0.0 
            };

            // Process sample and hold
            let output_sample = self.process_sample_hold(input_sample, trigger_sample, effective_threshold);
            output_samples.push(output_sample);

            // Generate gate output (high when holding)
            let gate_output = if self.is_holding() { 5.0 } else { 0.0 };
            gate_samples.push(gate_output);
        }

        // Write to output buffers
        if let Some(signal_output) = ctx.outputs.get_audio_mut("signal_out") {
            for (i, &sample) in output_samples.iter().enumerate() {
                if i < signal_output.len() {
                    signal_output[i] = sample;
                }
            }
        }

        if let Some(trigger_output) = ctx.outputs.get_audio_mut("trigger_out") {
            for i in 0..buffer_size.min(trigger_output.len()) {
                trigger_output[i] = if i < trigger_input.len() {
                    trigger_input[i]
                } else {
                    0.0
                };
            }
        }

        if let Some(gate_output) = ctx.outputs.get_audio_mut("gate_out") {
            for (i, &sample) in gate_samples.iter().enumerate() {
                if i < gate_output.len() {
                    gate_output[i] = sample;
                }
            }
        }

        Ok(())
    }

    fn node_info(&self) -> &NodeInfo {
        &self.node_info
    }

    fn reset(&mut self) {
        // Reset sample and hold state
        self.held_value = 0.0;
        self.last_trigger_state = false;
        self.manual_trigger_processed = false;
        self.slew_target = 0.0;
        self.slew_current = 0.0;
    }

    fn latency(&self) -> u32 {
        0 // No latency for sample and hold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_sample_hold_parameters() {
        let mut sh = SampleHoldNodeRefactored::new(44100.0, "test".to_string());
        
        // Test trigger threshold setting
        assert!(sh.set_parameter("trigger_threshold", 2.5).is_ok());
        assert_eq!(sh.get_parameter("trigger_threshold").unwrap(), 2.5);
        
        // Test slew rate setting
        assert!(sh.set_parameter("slew_rate", 0.5).is_ok());
        assert_eq!(sh.get_parameter("slew_rate").unwrap(), 0.5);
        
        // Test track mode setting
        assert!(sh.set_parameter("track_mode", 1.0).is_ok());
        assert_eq!(sh.get_parameter("track_mode").unwrap(), 1.0);
        
        // Test validation
        assert!(sh.set_parameter("trigger_threshold", -1.0).is_err()); // Out of range
        assert!(sh.set_parameter("slew_rate", 2.0).is_err()); // Out of range
    }

    #[test]
    fn test_sample_hold_basic_operation() {
        let mut sh = SampleHoldNodeRefactored::new(44100.0, "test".to_string());
        sh.set_parameter("trigger_threshold", 1.0).unwrap();
        
        // Create test signals
        let signal_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let trigger_data = vec![0.0, 2.0, 0.0, 2.0, 0.0]; // Triggers at samples 1 and 3
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        inputs.add_audio("trigger_in".to_string(), trigger_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 5);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 5,
            timestamp: 0,
            bpm: 120.0,
        };
        
        // Should process without error
        assert!(sh.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Sample 0: should be 0 (initial state)
        assert!((output[0] - 0.0).abs() < 0.001);
        
        // Sample 1: trigger rising edge - should sample input (2.0)
        assert!((output[1] - 2.0).abs() < 0.001);
        
        // Sample 2: no trigger - should hold previous value (2.0)
        assert!((output[2] - 2.0).abs() < 0.001);
        
        // Sample 3: trigger rising edge - should sample input (4.0)
        assert!((output[3] - 4.0).abs() < 0.001);
        
        // Sample 4: no trigger - should hold previous value (4.0)
        assert!((output[4] - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_manual_trigger() {
        let mut sh = SampleHoldNodeRefactored::new(44100.0, "test".to_string());
        
        let signal_data = vec![1.0, 2.0, 3.0];
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 3);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        // Trigger manual trigger
        sh.set_parameter("manual_trigger", 1.0).unwrap();
        
        assert!(sh.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should sample the first input value
        assert!((output[0] - 1.0).abs() < 0.001);
        
        // Check that held value is accessible
        assert!((sh.get_held_value() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_track_mode() {
        let mut sh = SampleHoldNodeRefactored::new(44100.0, "test".to_string());
        sh.set_parameter("track_mode", 1.0).unwrap(); // Enable track mode
        sh.set_parameter("trigger_threshold", 1.0).unwrap();
        
        let signal_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let trigger_data = vec![0.0, 2.0, 2.0, 0.0, 0.0]; // High for samples 1-2
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        inputs.add_audio("trigger_in".to_string(), trigger_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 5);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 5,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(sh.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should track input while trigger is high, hold when low
        assert!((output[1] - 2.0).abs() < 0.001); // Tracking
        assert!((output[2] - 3.0).abs() < 0.001); // Still tracking
        assert!((output[3] - 3.0).abs() < 0.001); // Holding last tracked value
        assert!((output[4] - 3.0).abs() < 0.001); // Still holding
    }

    #[test]
    fn test_threshold_modulation() {
        let mut sh = SampleHoldNodeRefactored::new(44100.0, "test".to_string());
        sh.set_parameter("trigger_threshold", 1.0).unwrap(); // Base threshold
        
        let signal_data = vec![5.0];
        let trigger_data = vec![1.5]; // Would not trigger with base threshold
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        inputs.add_audio("trigger_in".to_string(), trigger_data);
        inputs.add_cv("threshold_cv".to_string(), vec![-1.0]); // Lower threshold
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 1);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 1,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(sh.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should trigger due to CV modulation lowering threshold
        assert!((output[0] - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_inactive_state() {
        let mut sh = SampleHoldNodeRefactored::new(44100.0, "test".to_string());
        sh.set_parameter("active", 0.0).unwrap(); // Disable
        
        let signal_data = vec![1.0, 2.0, 3.0];
        let trigger_data = vec![0.0, 5.0, 0.0]; // Strong trigger
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data.clone());
        inputs.add_audio("trigger_in".to_string(), trigger_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 3);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(sh.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("signal_out").unwrap();
        
        // Should pass through input signal when inactive
        for (i, &expected) in signal_data.iter().enumerate() {
            assert!((output[i] - expected).abs() < 0.001, 
                    "Sample {}: expected {}, got {}", i, expected, output[i]);
        }
    }

    #[test]
    fn test_gate_output() {
        let mut sh = SampleHoldNodeRefactored::new(44100.0, "test".to_string());
        
        let signal_data = vec![1.0];
        let trigger_data = vec![2.0]; // Trigger
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("signal_in".to_string(), signal_data);
        inputs.add_audio("trigger_in".to_string(), trigger_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("signal_out".to_string(), 1);
        outputs.allocate_audio("gate_out".to_string(), 1);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 1,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(sh.process(&mut ctx).is_ok());
        
        let gate_output = ctx.outputs.get_audio("gate_out").unwrap();
        
        // Should output gate high when holding
        assert!(gate_output[0] > 2.0, "Gate should be high when holding");
    }
}