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

use crate::parameters::{BasicParameter, ModulatableParameter, Parameterizable, ParameterDescriptor, ModulationCurve};
use crate::processing::{AudioNode, ProcessContext, ProcessingError, NodeInfo, NodeCategory, PortInfo};
use crate::graph::PortType;
use crate::define_parameters;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScaleType {
    Chromatic = 0,
    Major = 1,
    Minor = 2,
    Pentatonic = 3,
    Blues = 4,
    Dorian = 5,
    Mixolydian = 6,
    Custom = 7,
}

impl ScaleType {
    pub fn from_f32(value: f32) -> Self {
        match value as i32 {
            0 => ScaleType::Chromatic,
            1 => ScaleType::Major,
            2 => ScaleType::Minor,
            3 => ScaleType::Pentatonic,
            4 => ScaleType::Blues,
            5 => ScaleType::Dorian,
            6 => ScaleType::Mixolydian,
            7 => ScaleType::Custom,
            _ => ScaleType::Chromatic,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ScaleType::Chromatic => "Chromatic",
            ScaleType::Major => "Major",
            ScaleType::Minor => "Minor",
            ScaleType::Pentatonic => "Pentatonic",
            ScaleType::Blues => "Blues",
            ScaleType::Dorian => "Dorian",
            ScaleType::Mixolydian => "Mixolydian",
            ScaleType::Custom => "Custom",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ScaleType::Chromatic => "All 12 semitones",
            ScaleType::Major => "Natural major scale",
            ScaleType::Minor => "Natural minor scale",
            ScaleType::Pentatonic => "Five-note pentatonic scale",
            ScaleType::Blues => "Blues scale with blue notes",
            ScaleType::Dorian => "Dorian mode",
            ScaleType::Mixolydian => "Mixolydian mode",
            ScaleType::Custom => "User-defined scale",
        }
    }

    pub fn get_scale_notes(&self) -> Vec<bool> {
        match self {
            ScaleType::Chromatic => vec![true; 12],
            ScaleType::Major => vec![true, false, true, false, true, true, false, true, false, true, false, true],
            ScaleType::Minor => vec![true, false, true, true, false, true, false, true, true, false, true, false],
            ScaleType::Pentatonic => vec![true, false, true, false, true, false, false, true, false, true, false, false],
            ScaleType::Blues => vec![true, false, false, true, false, true, true, true, false, false, true, false],
            ScaleType::Dorian => vec![true, false, true, true, false, true, false, true, false, true, true, false],
            ScaleType::Mixolydian => vec![true, false, true, false, true, true, false, true, false, true, true, false],
            ScaleType::Custom => vec![true; 12], // Will be overridden by custom_scale
        }
    }
}

/// リファクタリング済みQuantizerNode - プロ品質の1V/Oct量子化器
pub struct QuantizerNodeRefactored {
    // Node identification
    node_info: NodeInfo,
    
    // Quantizer parameters
    scale: f32,              // ScaleType (0-7)
    root_note: f32,          // Root note in 1V/Oct format (-5V to +5V)
    transpose: f32,          // Transpose in semitones (-24 to +24)
    slew_rate: f32,          // 0.0 ~ 1.0 (slew limiting for smooth transitions)
    hysteresis: f32,         // 0.0 ~ 1.0 (hysteresis to prevent oscillation)
    active: f32,
    
    // CV Modulation parameters
    root_note_param: ModulatableParameter,
    transpose_param: ModulatableParameter,
    
    // Custom scale definition (12 booleans for each semitone)
    custom_scale: [bool; 12],
    
    // Internal state
    last_quantized_output: f32,   // For slew rate limiting
    last_input_semitone: i32,     // For hysteresis
    last_trigger_state: bool,     // For trigger output generation
    
    sample_rate: f32,
}

impl QuantizerNodeRefactored {
    pub fn new(sample_rate: f32, name: String) -> Self {
        let node_info = NodeInfo {
            id: Uuid::new_v4(),
            name: name.clone(),
            node_type: "quantizer_refactored".to_string(),
            category: NodeCategory::Utility,
            description: "Professional CV quantizer with multiple scales and slew limiting".to_string(),
            input_ports: vec![
                PortInfo::new("cv_in", PortType::CV)
                    .with_description("CV input to be quantized (1V/Oct)"),
                PortInfo::new("root_note_cv", PortType::CV)
                    .with_description("Root note modulation")
                    .optional(),
                PortInfo::new("transpose_cv", PortType::CV)
                    .with_description("Transpose modulation")
                    .optional(),
                PortInfo::new("scale_cv", PortType::CV)
                    .with_description("Scale selection CV (0-7V)")
                    .optional(),
            ],
            output_ports: vec![
                PortInfo::new("cv_out", PortType::CV)
                    .with_description("Quantized CV output (1V/Oct)"),
                PortInfo::new("trigger_out", PortType::CV)
                    .with_description("Trigger on quantization change")
                    .optional(),
                PortInfo::new("gate_out", PortType::CV)
                    .with_description("Gate high when note is in scale")
                    .optional(),
            ],
            latency_samples: 0,
            supports_bypass: true,
        };

        // パラメーター設定 - プロフェッショナル量子化器用
        let root_note_param = ModulatableParameter::new(
            BasicParameter::new("root_note", -5.0, 5.0, 0.0),
            0.8  // 80% CV modulation range
        );

        let transpose_param = ModulatableParameter::new(
            BasicParameter::new("transpose", -24.0, 24.0, 0.0),
            0.8  // 80% CV modulation range
        );

        Self {
            node_info,
            scale: 0.0,           // Chromatic default
            root_note: 0.0,       // C
            transpose: 0.0,
            slew_rate: 0.0,       // No slew by default
            hysteresis: 0.1,      // Small hysteresis by default
            active: 1.0,

            root_note_param,
            transpose_param,
            
            custom_scale: [true; 12], // All notes enabled for custom scale
            
            last_quantized_output: 0.0,
            last_input_semitone: 0,
            last_trigger_state: false,
            
            sample_rate,
        }
    }

    /// Get the scale notes for the current scale type
    fn get_current_scale_notes(&self, scale_type: ScaleType) -> Vec<bool> {
        if scale_type == ScaleType::Custom {
            self.custom_scale.to_vec()
        } else {
            scale_type.get_scale_notes()
        }
    }

    /// Quantize a CV voltage to the nearest note in the current scale
    fn quantize_voltage(&mut self, input_voltage: f32, effective_root: f32, effective_transpose: f32, scale_type: ScaleType) -> (f32, bool) {
        // Apply root note and transpose
        let adjusted_voltage = input_voltage - effective_root + (effective_transpose / 12.0);
        
        // Convert to semitones (12 semitones per volt in 1V/Oct)
        let semitones = adjusted_voltage * 12.0;
        let input_semitone = semitones.round() as i32;
        
        // Apply hysteresis to prevent oscillation around note boundaries
        let semitone_to_quantize = if self.hysteresis > 0.0 {
            let hysteresis_range = (self.hysteresis * 0.5) as i32; // Half semitone hysteresis max
            if (input_semitone - self.last_input_semitone).abs() <= hysteresis_range {
                self.last_input_semitone // Stay with previous semitone
            } else {
                input_semitone
            }
        } else {
            input_semitone
        };
        
        // Get the scale notes
        let scale_notes = self.get_current_scale_notes(scale_type);
        
        // Find the closest valid note in the scale
        let quantized_semitone = self.find_closest_scale_note(semitone_to_quantize, &scale_notes);
        
        // Convert back to voltage
        let target_voltage = (quantized_semitone as f32) / 12.0 + effective_root - (effective_transpose / 12.0);
        
        // Check if quantization changed (for trigger output)
        let quantization_changed = quantized_semitone != self.last_input_semitone;
        self.last_input_semitone = quantized_semitone;
        
        // Apply slew rate limiting
        let final_voltage = if self.slew_rate > 0.0 {
            self.apply_slew_limiting(target_voltage)
        } else {
            self.last_quantized_output = target_voltage;
            target_voltage
        };
        
        (final_voltage, quantization_changed)
    }

    /// Find the closest note in the scale to the given semitone
    fn find_closest_scale_note(&self, target_semitone: i32, scale_notes: &[bool]) -> i32 {
        let note_in_scale = ((target_semitone % 12 + 12) % 12) as usize;
        
        // If the target note is in the scale, use it
        if scale_notes[note_in_scale] {
            return target_semitone;
        }
        
        // Otherwise, find the closest valid note
        let mut closest_distance = 12;
        let mut closest_semitone = target_semitone;
        
        for offset in 1..=6 {
            // Check both directions
            for &direction in &[-1, 1] {
                let test_semitone = target_semitone + (offset * direction);
                let test_note = ((test_semitone % 12 + 12) % 12) as usize;
                if scale_notes[test_note] && offset < closest_distance {
                    closest_distance = offset;
                    closest_semitone = test_semitone;
                    break; // Take the first match in this direction
                }
            }
            if closest_distance < 12 {
                break; // Found a valid note
            }
        }
        
        closest_semitone
    }

    /// Apply slew rate limiting for smooth transitions
    fn apply_slew_limiting(&mut self, target_voltage: f32) -> f32 {
        let max_change_per_second = self.slew_rate * 10.0; // 10V/s maximum slew rate
        let max_change_per_sample = max_change_per_second / self.sample_rate;
        
        let difference = target_voltage - self.last_quantized_output;
        let limited_change = difference.clamp(-max_change_per_sample, max_change_per_sample);
        
        self.last_quantized_output += limited_change;
        self.last_quantized_output
    }

    /// Set a custom scale note
    pub fn set_custom_scale_note(&mut self, note: usize, enabled: bool) -> Result<(), String> {
        if note >= 12 {
            return Err(format!("Invalid note index: {}, must be 0-11", note));
        }
        self.custom_scale[note] = enabled;
        Ok(())
    }

    /// Get custom scale note state
    pub fn get_custom_scale_note(&self, note: usize) -> Result<bool, String> {
        if note >= 12 {
            return Err(format!("Invalid note index: {}, must be 0-11", note));
        }
        Ok(self.custom_scale[note])
    }

    /// Get the current scale type
    pub fn get_scale_type(&self) -> ScaleType {
        ScaleType::from_f32(self.scale)
    }
}

impl Parameterizable for QuantizerNodeRefactored {
    fn get_all_parameters(&self) -> std::collections::HashMap<String, f32> {
        let mut params = std::collections::HashMap::new();
        params.insert("scale".to_string(), self.scale);
        params.insert("root_note".to_string(), self.root_note);
        params.insert("transpose".to_string(), self.transpose);
        params.insert("slew_rate".to_string(), self.slew_rate);
        params.insert("hysteresis".to_string(), self.hysteresis);
        params.insert("active".to_string(), self.active);
        
        // Add custom scale parameters
        for (i, &enabled) in self.custom_scale.iter().enumerate() {
            params.insert(format!("custom_{}", i), if enabled { 1.0 } else { 0.0 });
        }
        
        params
    }
    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), crate::parameters::ParameterError> {
        // Handle custom scale parameters
        if name.starts_with("custom_") {
            if let Some(note_str) = name.strip_prefix("custom_") {
                if let Ok(note) = note_str.parse::<usize>() {
                    self.set_custom_scale_note(note, value > 0.5)
                        .map_err(|e| crate::parameters::ParameterError::InvalidType { 
                            expected: "valid scale type".to_string(), 
                            found: e 
                        })?;
                    return Ok(());
                }
            }
            return Err(crate::parameters::ParameterError::NotFound { name: name.to_string() });
        }

        // Handle standard parameters
        match name {
            "scale" => {
                let scale_value = value.clamp(0.0, 7.0);
                self.scale = scale_value;
                Ok(())
            },
            "root_note" => {
                if value >= -5.0 && value <= 5.0 {
                    self.root_note = value;
                    self.root_note_param.set_base_value(value)?;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: -5.0, max: 5.0 
                    })
                }
            },
            "transpose" => {
                if value >= -24.0 && value <= 24.0 {
                    self.transpose = value;
                    self.transpose_param.set_base_value(value)?;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: -24.0, max: 24.0 
                    })
                }
            },
            "slew_rate" => {
                if value >= 0.0 && value <= 1.0 {
                    self.slew_rate = value;
                    Ok(())
                } else {
                    Err(crate::parameters::ParameterError::OutOfRange { 
                        value, min: 0.0, max: 1.0 
                    })
                }
            },
            "hysteresis" => {
                if value >= 0.0 && value <= 1.0 {
                    self.hysteresis = value;
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
        // Handle custom scale parameters
        if name.starts_with("custom_") {
            if let Some(note_str) = name.strip_prefix("custom_") {
                if let Ok(note) = note_str.parse::<usize>() {
                    return self.get_custom_scale_note(note)
                        .map(|enabled| if enabled { 1.0 } else { 0.0 })
                        .map_err(|e| crate::parameters::ParameterError::InvalidType { 
                            expected: "valid custom scale note".to_string(), 
                            found: e 
                        });
                }
            }
            return Err(crate::parameters::ParameterError::NotFound { name: name.to_string() });
        }

        // Handle standard parameters
        match name {
            "scale" => Ok(self.scale),
            "root_note" => Ok(self.root_note),
            "transpose" => Ok(self.transpose),
            "slew_rate" => Ok(self.slew_rate),
            "hysteresis" => Ok(self.hysteresis),
            "active" => Ok(self.active),
            _ => Err(crate::parameters::ParameterError::NotFound { name: name.to_string() }),
        }
    }

    fn get_parameter_descriptors(&self) -> Vec<Box<dyn ParameterDescriptor>> {
        let mut descriptors: Vec<Box<dyn ParameterDescriptor>> = vec![
            Box::new(BasicParameter::new("scale", 0.0, 7.0, 0.0)),
            Box::new(BasicParameter::new("root_note", -5.0, 5.0, 0.0)),
            Box::new(BasicParameter::new("transpose", -24.0, 24.0, 0.0)),
            Box::new(BasicParameter::new("slew_rate", 0.0, 1.0, 0.0)),
            Box::new(BasicParameter::new("hysteresis", 0.0, 1.0, 0.1)),
            Box::new(BasicParameter::new("active", 0.0, 1.0, 1.0)),
        ];

        // Add custom scale parameters
        for _i in 0..12 {
            descriptors.push(Box::new(BasicParameter::new("custom", 0.0, 1.0, 1.0)));
        }

        descriptors
    }
}

impl AudioNode for QuantizerNodeRefactored {
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError> {
        if !self.is_active() {
            // Inactive - pass through input CV
            if let (Some(input), Some(output)) = 
                (ctx.inputs.get_audio("cv_in"), ctx.outputs.get_audio_mut("cv_out")) {
                output.copy_from_slice(&input[..output.len().min(input.len())]);
            }
            return Ok(());
        }

        // Get input signals
        let cv_input = ctx.inputs.get_audio("cv_in").unwrap_or(&[]);
        
        // Get CV inputs
        let root_note_cv = ctx.inputs.get_cv_value("root_note_cv");
        let transpose_cv = ctx.inputs.get_cv_value("transpose_cv");
        let scale_cv = ctx.inputs.get_cv_value("scale_cv");

        // Apply CV modulation
        let effective_root = self.root_note_param.modulate(
            self.root_note_param.get_base_value(), 
            root_note_cv
        );
        let effective_transpose = self.transpose_param.modulate(
            self.transpose_param.get_base_value(), 
            transpose_cv
        );

        // Update scale from CV if provided
        let current_scale_type = if scale_cv != 0.0 {
            ScaleType::from_f32(scale_cv.clamp(0.0, 7.0))
        } else {
            ScaleType::from_f32(self.scale)
        };

        // Get buffer size
        let buffer_size = ctx.outputs.get_audio("cv_out")
            .ok_or_else(|| ProcessingError::OutputBufferError { 
                port_name: "cv_out".to_string() 
            })?.len();

        // Process each sample
        let mut output_samples = Vec::with_capacity(buffer_size);
        let mut trigger_samples = Vec::with_capacity(buffer_size);
        let mut gate_samples = Vec::with_capacity(buffer_size);

        for i in 0..buffer_size {
            // Get input CV sample
            let input_cv = if i < cv_input.len() { 
                cv_input[i] 
            } else { 
                0.0 
            };

            // Quantize the voltage
            let (quantized_cv, quantization_changed) = self.quantize_voltage(
                input_cv, 
                effective_root, 
                effective_transpose, 
                current_scale_type
            );

            output_samples.push(quantized_cv);

            // Generate trigger on quantization change
            let trigger_output = if quantization_changed && !self.last_trigger_state { 
                self.last_trigger_state = true;
                5.0 
            } else { 
                self.last_trigger_state = false;
                0.0 
            };
            trigger_samples.push(trigger_output);

            // Generate gate (always high when a valid note is quantized)
            let note_in_scale = {
                let semitone = (quantized_cv * 12.0).round() as i32;
                let note = ((semitone % 12 + 12) % 12) as usize;
                let scale_notes = self.get_current_scale_notes(current_scale_type);
                scale_notes[note]
            };
            gate_samples.push(if note_in_scale { 5.0 } else { 0.0 });
        }

        // Write to output buffers
        if let Some(cv_output) = ctx.outputs.get_audio_mut("cv_out") {
            for (i, &sample) in output_samples.iter().enumerate() {
                if i < cv_output.len() {
                    cv_output[i] = sample;
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
        // Reset quantizer state
        self.last_quantized_output = 0.0;
        self.last_input_semitone = 0;
        self.last_trigger_state = false;
    }

    fn latency(&self) -> u32 {
        0 // No latency for quantization
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
    fn test_quantizer_parameters() {
        let mut quant = QuantizerNodeRefactored::new(44100.0, "test".to_string());
        
        // Test scale setting
        assert!(quant.set_parameter("scale", 1.0).is_ok());
        assert_eq!(quant.get_parameter("scale").unwrap(), 1.0);
        
        // Test root note setting
        assert!(quant.set_parameter("root_note", 1.0).is_ok());
        assert_eq!(quant.get_parameter("root_note").unwrap(), 1.0);
        
        // Test transpose setting
        assert!(quant.set_parameter("transpose", 12.0).is_ok());
        assert_eq!(quant.get_parameter("transpose").unwrap(), 12.0);
        
        // Test custom scale note
        assert!(quant.set_parameter("custom_0", 1.0).is_ok());
        assert_eq!(quant.get_parameter("custom_0").unwrap(), 1.0);
        
        // Test validation
        assert!(quant.set_parameter("transpose", 50.0).is_err()); // Out of range
        assert!(quant.set_parameter("slew_rate", -1.0).is_err()); // Out of range
    }

    #[test]
    fn test_chromatic_quantization() {
        let mut quant = QuantizerNodeRefactored::new(44100.0, "test".to_string());
        quant.set_parameter("scale", 0.0).unwrap(); // Chromatic
        
        // Test quantization of various voltages
        let cv_data = vec![0.0, 0.5, 1.0, 1.5, 2.0]; // C, F#, C, F#, C
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("cv_in".to_string(), cv_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 5);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 5,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(quant.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        
        // In chromatic scale, all values should quantize to nearest semitone
        assert!((output[0] - 0.0).abs() < 0.001, "0V should quantize to 0V");
        assert!((output[1] - 0.5).abs() < 0.001, "0.5V should quantize to 0.5V");
        assert!((output[2] - 1.0).abs() < 0.001, "1V should quantize to 1V");
        assert!((output[3] - 1.5).abs() < 0.001, "1.5V should quantize to 1.5V");
        assert!((output[4] - 2.0).abs() < 0.001, "2V should quantize to 2V");
    }

    #[test]
    fn test_major_scale_quantization() {
        let mut quant = QuantizerNodeRefactored::new(44100.0, "test".to_string());
        quant.set_parameter("scale", 1.0).unwrap(); // Major scale
        
        // Test quantization to C major scale
        let cv_data = vec![0.1, 0.2, 0.3, 0.4]; // All should quantize to nearby major scale notes
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("cv_in".to_string(), cv_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(quant.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        
        // All values should quantize to valid major scale notes
        // 0.1V (close to C) should quantize to 0V (C)
        assert!((output[0] - 0.0).abs() < 0.001);
        
        // Values should be quantized to major scale notes only
        for &value in output.iter() {
            let semitone = (value * 12.0).round() as i32;
            let note = ((semitone % 12 + 12) % 12) as usize;
            let major_scale = vec![true, false, true, false, true, true, false, true, false, true, false, true];
            assert!(major_scale[note], "Output {} should be in major scale", value);
        }
    }

    #[test]
    fn test_transpose_and_root_note() {
        let mut quant = QuantizerNodeRefactored::new(44100.0, "test".to_string());
        quant.set_parameter("scale", 1.0).unwrap(); // Major scale
        quant.set_parameter("transpose", 12.0).unwrap(); // +1 octave
        quant.set_parameter("root_note", 1.0).unwrap(); // Root = C
        
        let cv_data = vec![0.0]; // C
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("cv_in".to_string(), cv_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 1);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 1,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(quant.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        
        // Should be affected by transpose and root note
        // The exact value depends on the quantization algorithm
        assert!(output[0] != 0.0, "Output should be affected by transpose and root note");
    }

    #[test]
    fn test_trigger_output() {
        let mut quant = QuantizerNodeRefactored::new(44100.0, "test".to_string());
        quant.set_parameter("scale", 1.0).unwrap(); // Major scale
        
        let cv_data = vec![0.0, 0.1, 0.5, 0.5]; // Should trigger on quantization changes
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("cv_in".to_string(), cv_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 4);
        outputs.allocate_audio("trigger_out".to_string(), 4);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 4,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(quant.process(&mut ctx).is_ok());
        
        let trigger_output = ctx.outputs.get_audio("trigger_out").unwrap();
        
        // Should have triggers when quantization changes
        let has_triggers = trigger_output.iter().any(|&t| t > 2.0);
        assert!(has_triggers, "Should generate triggers on quantization changes");
    }

    #[test]
    fn test_custom_scale() {
        let mut quant = QuantizerNodeRefactored::new(44100.0, "test".to_string());
        quant.set_parameter("scale", 7.0).unwrap(); // Custom scale
        
        // Set up custom scale with only C and G (0 and 7 semitones)
        for i in 0..12 {
            quant.set_parameter(&format!("custom_{}", i), 0.0).unwrap(); // Disable all
        }
        quant.set_parameter("custom_0", 1.0).unwrap(); // Enable C
        quant.set_parameter("custom_7", 1.0).unwrap(); // Enable G
        
        let cv_data = vec![0.25]; // Between C and D, should quantize to C
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("cv_in".to_string(), cv_data);
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 1);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 1,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(quant.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        
        // Should quantize to C (0V) since it's the closest note in the custom scale
        assert!((output[0] - 0.0).abs() < 0.1, "Should quantize to C in custom scale");
    }

    #[test]
    fn test_inactive_state() {
        let mut quant = QuantizerNodeRefactored::new(44100.0, "test".to_string());
        quant.set_parameter("active", 0.0).unwrap(); // Disable
        
        let cv_data = vec![0.123, 0.456, 0.789];
        
        let mut inputs = InputBuffers::new();
        inputs.add_audio("cv_in".to_string(), cv_data.clone());
        
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("cv_out".to_string(), 3);
        
        let mut ctx = ProcessContext {
            inputs: &inputs,
            outputs: &mut outputs,
            sample_rate: 44100.0,
            buffer_size: 3,
            timestamp: 0,
            bpm: 120.0,
        };
        
        assert!(quant.process(&mut ctx).is_ok());
        
        let output = ctx.outputs.get_audio("cv_out").unwrap();
        
        // Should pass through input unchanged when inactive
        for (i, &expected) in cv_data.iter().enumerate() {
            assert!((output[i] - expected).abs() < 0.001, 
                    "Sample {}: expected {}, got {}", i, expected, output[i]);
        }
    }
}