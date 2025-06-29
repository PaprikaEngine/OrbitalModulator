use uuid::Uuid;

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor, ModulationCurve};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;
use crate::define_parameters;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LFOWaveform {
    Sine = 0,
    Triangle = 1,
    Sawtooth = 2,
    Square = 3,
    Random = 4,  // Sample & Hold
}

impl LFOWaveform {
    pub fn from_f32(value: f32) -> Self {
        match value as i32 {
            0 => LFOWaveform::Sine,
            1 => LFOWaveform::Triangle,
            2 => LFOWaveform::Sawtooth,
            3 => LFOWaveform::Square,
            4 => LFOWaveform::Random,
            _ => LFOWaveform::Sine,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LFOWaveform::Sine => "Sine",
            LFOWaveform::Triangle => "Triangle",
            LFOWaveform::Sawtooth => "Sawtooth",
            LFOWaveform::Square => "Square",
            LFOWaveform::Random => "Random (S&H)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            LFOWaveform::Sine => "Smooth sinusoidal wave",
            LFOWaveform::Triangle => "Linear ramp up and down",
            LFOWaveform::Sawtooth => "Linear ramp up with sharp reset",
            LFOWaveform::Square => "Stepped high/low wave",
            LFOWaveform::Random => "Sample and hold random values",
        }
    }
}

/// リファクタリング済みLFONode - プロ品質の低周波オシレーター
pub struct LFONodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // LFO parameters
    frequency: f32,      // 0.01Hz ~ 20Hz
    amplitude: f32,      // 0.0 ~ 1.0 (CV出力の振幅)
    waveform: f32,       // 0-4 (LFOWaveform enum values)
    phase_offset: f32,   // 0.0 ~ 1.0 (位相オフセット)
    pulse_width: f32,    // 0.1 ~ 0.9 (square wave duty cycle)
    rate_cv_sensitivity: f32, // 0.0 ~ 1.0 (how much rate CV affects frequency)
    bipolar: f32,        // 0.0 = unipolar (0 to +1), 1.0 = bipolar (-1 to +1)
    active: f32,
    
    // CV Modulation parameters  
    frequency_param: ModulatableParameter,
    amplitude_param: ModulatableParameter,
    phase_offset_param: ModulatableParameter,
    
    // Internal state
    phase: f32,          // 現在の位相 (0.0 ~ 1.0)
    random_value: f32,   // Sample & Hold用のランダム値
    last_phase: f32,     // 前フレームの位相（Random波形用）
    sync_triggered: bool, // 外部同期がトリガーされたか
    
    sample_rate: f32,
}

impl LFONodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "lfo_refactored".to_string(),
            category: NodeCategory::Controller,
            description: "Professional low frequency oscillator with 5 waveforms and sync".to_string(),
            input_ports: vec![
                PortInfo::new("frequency_cv", PortType::CV)
                    .with_description("Frequency modulation (1V/Oct or Linear)")
                    .optional(),
                PortInfo::new("amplitude_cv", PortType::CV)
                    .with_description("Amplitude modulation (0V to +10V)")
                    .optional(),
                PortInfo::new("phase_offset_cv", PortType::CV)
                    .with_description("Phase offset modulation (0V to +10V)")
                    .optional(),
                PortInfo::new("sync_in", PortType::CV)
                    .with_description("Hard sync trigger input (>2.5V)")
                    .optional(),
                PortInfo::new("waveform_cv", PortType::CV)
                    .with_description("Waveform selection (0-4V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("cv_out", PortType::CV)
                    .with_description("LFO CV output (-10V to +10V or 0V to +10V)"),
                PortInfo::new("inverted_out", PortType::CV)
                    .with_description("Inverted CV output")
                    .optional(),
                PortInfo::new("end_of_cycle", PortType::CV)
                    .with_description("Trigger at end of LFO cycle")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナルLFO用
        let frequency_param = ModulatableParameter::new(
            BasicParameter::new("frequency", 0.01, 20.0, 1.0),
            0.8  // 80% CV modulation range
        ).with_curve(ModulationCurve::Exponential); // Exponential for musical frequency response

        let amplitude_param = ModulatableParameter::new(
            BasicParameter::new("amplitude", 0.0, 1.0, 1.0),
            0.8  // 80% CV modulation range
        );

        let phase_offset_param = ModulatableParameter::new(
            BasicParameter::new("phase_offset", 0.0, 1.0, 0.0),
            0.8  // 80% CV modulation range
        );

        Self {
            node_info,
            frequency: 1.0,        // 1Hz デフォルト
            amplitude: 1.0,
            waveform: 0.0,         // Sine default
            phase_offset: 0.0,
            pulse_width: 0.5,      // 50% duty cycle
            rate_cv_sensitivity: 1.0, // Full CV sensitivity
            bipolar: 1.0,          // Bipolar output default
            active: 1.0,

            frequency_param,
            amplitude_param,
            phase_offset_param,
            
            phase: 0.0,
            random_value: 0.5,  // Start with a non-zero value
            last_phase: 0.0,
            sync_triggered: false,
            
            sample_rate,
        }
    }

    /// Generate LFO waveform for given phase
    fn generate_waveform(&mut self, phase: f32, waveform_type: LFOWaveform) -> f32 {
        match waveform_type {
            LFOWaveform::Sine => {
                (phase * 2.0 * std::f32::consts::PI).sin()
            },
            LFOWaveform::Triangle => {
                if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                }
            },
            LFOWaveform::Sawtooth => {
                2.0 * phase - 1.0
            },
            LFOWaveform::Square => {
                if phase < self.pulse_width { 1.0 } else { -1.0 }
            },
            LFOWaveform::Random => {
                // Sample & Hold - generate new random value on phase reset or first time
                if self.last_phase > phase || self.random_value == 0.0 {
                    // New cycle started or first time - generate new random value
                    let seed = ((self.phase * 12345.0) + 
                              (self.sample_rate * 0.0001) + 
                              (self.frequency * 1000.0)) as u32;
                    self.random_value = ((seed.wrapping_mul(1103515245).wrapping_add(12345) >> 16) as f32 / 32768.0) * 2.0 - 1.0;
                }
                self.last_phase = phase;
                self.random_value
            },
        }
    }

    /// Process hard sync signal
    fn process_sync(&mut self, sync_signal: f32) {
        let sync_high = sync_signal > 2.5;
        if sync_high && !self.sync_triggered {
            // Rising edge detected - reset phase
            self.phase = 0.0;
            self.sync_triggered = true;
        } else if !sync_high {
            self.sync_triggered = false;
        }
    }

    /// Get current LFO phase for debugging/display
    pub fn get_phase(&self) -> f32 {
        self.phase
    }

    /// Get current waveform type
    pub fn get_waveform(&self) -> LFOWaveform {
        LFOWaveform::from_f32(self.waveform)
    }
}

impl Parameterizable for LFONodeRefactored {
    define_parameters! {
        frequency: BasicParameter::new("frequency", 0.01, 20.0, 1.0),
        amplitude: BasicParameter::new("amplitude", 0.0, 1.0, 1.0),
        waveform: BasicParameter::new("waveform", 0.0, 4.0, 0.0),
        phase_offset: BasicParameter::new("phase_offset", 0.0, 1.0, 0.0),
        pulse_width: BasicParameter::new("pulse_width", 0.1, 0.9, 0.5),
        rate_cv_sensitivity: BasicParameter::new("rate_cv_sensitivity", 0.0, 1.0, 1.0),
        bipolar: BasicParameter::new("bipolar", 0.0, 1.0, 1.0),
        active: BasicParameter::new("active", 0.0, 1.0, 1.0)
    }
}

impl AudioNode for LFONodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - output zero
            if let Some(cv_output) = ctx.outputs.get_audio_mut("cv_out") {
                cv_output.fill(0.0);
            }
            if let Some(inv_output) = ctx.outputs.get_audio_mut("inverted_out") {
                inv_output.fill(0.0);
            }
            if let Some(eoc_output) = ctx.outputs.get_audio_mut("end_of_cycle") {
                eoc_output.fill(0.0);
            }
            return Ok(());
        }

        // Get input signals
        let sync_input = ctx.inputs.get_audio("sync_in").unwrap_or(&[]);
        
        // Get CV inputs
        let frequency_cv = ctx.inputs.get_cv_value("frequency_cv");
        let amplitude_cv = ctx.inputs.get_cv_value("amplitude_cv");
        let phase_offset_cv = ctx.inputs.get_cv_value("phase_offset_cv");
        let waveform_cv = ctx.inputs.get_cv_value("waveform_cv");

        // Apply CV modulation
        let effective_frequency = self.frequency_param.modulate(self.frequency, 
            frequency_cv * self.rate_cv_sensitivity);
        let effective_amplitude = self.amplitude_param.modulate(self.amplitude, amplitude_cv);
        let effective_phase_offset = self.phase_offset_param.modulate(self.phase_offset, phase_offset_cv);

        // Update waveform from CV if provided
        let current_waveform = if waveform_cv != 0.0 {
            LFOWaveform::from_f32(waveform_cv.clamp(0.0, 4.0))
        } else {
            LFOWaveform::from_f32(self.waveform)
        };

        // Get the buffer size
        let buffer_size = ctx.outputs.get_audio("cv_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "cv_out".to_string() 
            })?.len();

        // Generate samples
        let mut cv_samples = Vec::with_capacity(buffer_size);
        let mut inv_samples = Vec::with_capacity(buffer_size);
        let mut eoc_samples = Vec::with_capacity(buffer_size);

        for i in 0..buffer_size {
            // Process sync input
            let sync_signal = if i < sync_input.len() { 
                sync_input[i] 
            } else { 
                0.0 
            };
            self.process_sync(sync_signal);

            // Calculate phase increment
            let phase_increment = effective_frequency / self.sample_rate;
            
            // Store previous phase for end-of-cycle detection
            let _prev_phase = self.phase;
            
            // Advance phase
            self.phase += phase_increment;
            let mut end_of_cycle = false;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
                end_of_cycle = true;
            }

            // Apply phase offset
            let adjusted_phase = (self.phase + effective_phase_offset) % 1.0;

            // Generate waveform
            let raw_value = self.generate_waveform(adjusted_phase, current_waveform);

            // Apply amplitude scaling
            let scaled_value = raw_value * effective_amplitude;

            // Convert to output voltage based on bipolar setting
            let cv_output = if self.bipolar > 0.5 {
                // Bipolar: -10V to +10V
                scaled_value * 10.0
            } else {
                // Unipolar: 0V to +10V
                (scaled_value + 1.0) * 5.0
            };

            // Inverted output
            let inv_output = -cv_output;

            cv_samples.push(cv_output);
            inv_samples.push(inv_output);
            eoc_samples.push(if end_of_cycle { 5.0 } else { 0.0 });
        }

        // Write to output buffers
        if let Some(cv_output) = ctx.outputs.get_audio_mut("cv_out") {
            for (i, &sample) in cv_samples.iter().enumerate() {
                if i < cv_output.len() {
                    cv_output[i] = sample;
                }
            }
        }

        if let Some(inv_output) = ctx.outputs.get_audio_mut("inverted_out") {
            for (i, &sample) in inv_samples.iter().enumerate() {
                if i < inv_output.len() {
                    inv_output[i] = sample;
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
        // Reset LFO state
        self.phase = 0.0;
        self.random_value = 0.0;
        self.last_phase = 0.0;
        self.sync_triggered = false;
    }

    fn latency(&self) -> u32 {
        0 // No latency for LFO generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::{InputBuffers, OutputBuffers};

    #[test]
    fn test_lfo_parameters() {
        let mut lfo = LFONodeRefactored::new(44100.0, "test".to_string());
        
        // Test frequency setting
        assert!(lfo.set_parameter("frequency", 2.0).is_ok());
        assert_eq!(lfo.get_parameter("frequency").unwrap(), 2.0);
        
        // Test amplitude setting
        assert!(lfo.set_parameter("amplitude", 0.8).is_ok());
        assert_eq!(lfo.get_parameter("amplitude").unwrap(), 0.8);
        
        // Test waveform setting
        assert!(lfo.set_parameter("waveform", 2.0).is_ok()); // Sawtooth
        assert_eq!(lfo.get_parameter("waveform").unwrap(), 2.0);
        
        // Test phase offset setting
        assert!(lfo.set_parameter("phase_offset", 0.25).is_ok());
        assert_eq!(lfo.get_parameter("phase_offset").unwrap(), 0.25);
        
        // Test validation
        assert!(lfo.set_parameter("frequency", -1.0).is_err()); // Out of range
        assert!(lfo.set_parameter("amplitude", 2.0).is_err()); // Out of range
    }

    #[test]
    fn test_lfo_sine_generation() {
        let mut lfo = LFONodeRefactored::new(44100.0, "test".to_string());
        lfo.set_parameter("frequency", 1.0).unwrap(); // 1Hz
        lfo.set_parameter("waveform", 0.0).unwrap(); // Sine
        lfo.set_parameter("bipolar", 1.0).unwrap(); // Bipolar output
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 44100); // 1 second
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 44100,
            timestamp: 0,
            bpm: 120.0,
        };
        
        // Should process without error
        assert!(lfo.process(&mut ctx).is_ok());
        
        // Check sine wave properties
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        
        // Should start near 0 (sine starts at 0)
        assert!(output[0].abs() < 1.0, "Sine should start near zero: {}", output[0]);
        
        // Should have positive peak around quarter cycle
        let quarter_sample = 44100 / 4;
        assert!(output[quarter_sample] > 5.0, "Should have positive peak: {}", output[quarter_sample]);
        
        // Should return to near zero at half cycle
        let half_sample = 44100 / 2;
        assert!(output[half_sample].abs() < 1.0, "Should return to zero at half cycle: {}", output[half_sample]);
        
        // Should have negative peak around three-quarter cycle
        let three_quarter_sample = 3 * 44100 / 4;
        assert!(output[three_quarter_sample] < -5.0, "Should have negative peak: {}", output[three_quarter_sample]);
    }

    #[test]
    fn test_lfo_waveforms() {
        let mut lfo = LFONodeRefactored::new(44100.0, "test".to_string());
        lfo.set_parameter("frequency", 10.0).unwrap(); // 10Hz for faster testing
        
        // Test each waveform
        for waveform in 0..5 {
            lfo.set_parameter("waveform", waveform as f32).unwrap();
            
            let inputs = InputBuffers::new();
            let mut outputs = OutputBuffers::new();
            outputs.allocate_audio("cv_out".to_string(), 512);
            
            let mut ctx = ProcessContext {
                inputs: &inputs,
                outputs: &mut outputs,
                sample_rate: 44100.0,
                buffer_size: 512,
                timestamp: 0,
                bpm: 120.0,
            };
            
            assert!(lfo.process(&mut ctx).is_ok());
            
            let output = ctx.outputs.get_audio("cv_out").unwrap();
            let has_signal = output.iter().any(|&s| s.abs() > 0.1);
            assert!(has_signal, "Waveform {} should produce output", waveform);
            
            // Reset for next waveform test
            lfo.reset();
        }
    }

    #[test]
    fn test_lfo_frequency_cv() {
        let mut lfo = LFONodeRefactored::new(44100.0, "test".to_string());
        lfo.set_parameter("frequency", 1.0).unwrap(); // Base 1Hz
        
        let mut inputs = InputBuffers::new();
        inputs.add_cv("frequency_cv".to_string(), vec![2.0]); // +2V should increase frequency
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(lfo.process(&mut ctx).is_ok());
        
        // CV should modulate the frequency
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        let has_modulated_frequency = output.iter().any(|&s| s.abs() > 0.1);
        assert!(has_modulated_frequency, "Frequency CV should affect the LFO");
    }

    #[test]
    fn test_lfo_sync() {
        let mut lfo = LFONodeRefactored::new(44100.0, "test".to_string());
        lfo.set_parameter("frequency", 1.0).unwrap();
        
        // Create sync signal that triggers mid-cycle
        let sync_signal = vec![0.0; 256].into_iter()
            .chain(vec![5.0; 1])  // Sync trigger
            .chain(vec![0.0; 255])
            .collect();
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("sync_in".to_string(), sync_signal);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(lfo.process(&mut ctx).is_ok());
        
        // Phase should reset at sync trigger
        assert!(lfo.get_phase() >= 0.0 && lfo.get_phase() < 1.0, "Phase should be valid after sync");
    }

    #[test]
    fn test_unipolar_vs_bipolar() {
        let mut lfo = LFONodeRefactored::new(44100.0, "test".to_string());
        lfo.set_parameter("frequency", 20.0).unwrap(); // Maximum allowed frequency for complete cycles
        lfo.set_parameter("waveform", 0.0).unwrap(); // Sine
        
        // Test bipolar output
        lfo.set_parameter("bipolar", 1.0).unwrap();
        
        // Debug: check that bipolar was set correctly
        assert_eq!(lfo.get_parameter("bipolar").unwrap(), 1.0, "Bipolar should be set to 1.0");
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 4410); // 0.1 second = multiple complete cycles
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4410,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(lfo.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        let has_negative = output.iter().any(|&s| s < -1.0);
        let min_value = output.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_value = output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        assert!(has_negative, "Bipolar output should have negative values. Range: {} to {}", min_value, max_value);
        
        // Test unipolar output
        lfo.reset();
        lfo.set_parameter("bipolar", 0.0).unwrap();
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 4410);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4410,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(lfo.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        let all_positive = output.iter().all(|&s| s >= -0.1); // Allow for small rounding errors
        assert!(all_positive, "Unipolar output should be non-negative");
    }

    #[test]
    fn test_inverted_output() {
        let mut lfo = LFONodeRefactored::new(44100.0, "test".to_string());
        lfo.set_parameter("frequency", 5.0).unwrap();
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 256);
        outputs.allocate_audio("inverted_out".to_string(), 256);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 256,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(lfo.process(&mut ctx).is_ok());
        
        // Inverted output should be negative of normal output
        let normal_output = ctx.outputs.get_audio("cv_out").unwrap();
        let inverted_output = ctx.outputs.get_audio("inverted_out").unwrap();
        
        for (i, (&normal, &inverted)) in normal_output.iter().zip(inverted_output.iter()).enumerate() {
            let expected_inverted = -normal;
            assert!((inverted - expected_inverted).abs() < 0.01, 
                "Sample {}: inverted should be negative of normal: {} vs {}", i, inverted, expected_inverted);
        }
    }

    #[test]
    fn test_inactive_state() {
        let mut lfo = LFONodeRefactored::new(44100.0, "test".to_string());
        lfo.set_parameter("active", 0.0).unwrap(); // Disable
        
        let inputs = InputBuffers::new();
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 512);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 512,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(lfo.process(&mut ctx).is_ok());
        
        // Should output zero when inactive
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        let avg_output = output.iter().sum::<f32>() / output.len() as f32;
        assert!((avg_output - 0.0).abs() < 0.001, "Should output zero when inactive: {}", avg_output);
    }
}