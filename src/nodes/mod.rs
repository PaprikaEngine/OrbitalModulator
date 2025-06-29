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

//! OrbitalModulator Node Architecture
//! 
//! This module provides two parallel node architectures:
//! 
//! ## New ProcessContext Architecture (Recommended)
//! - All nodes with `*_refactored.rs` suffix
//! - Uses unified `ProcessContext` for audio processing
//! - Professional CV modulation with `ModulatableParameter`
//! - Comprehensive error handling and type safety
//! - Full Eurorack compliance (1V/Oct, ±10V CV, 5V gates)
//! - Ready for commercial deployment
//! 
//! **Status: 21/21 nodes complete (100%)**
//! 
//! ## Legacy HashMap Architecture (Deprecated)
//! - Original node implementations
//! - Used by existing Tauri UI integration
//! - Will be migrated to new architecture in future versions
//! - Maintained for backward compatibility only
//! 
//! **Migration Progress: New architecture ready for production use**

// === Refactored Node Modules (New Architecture) ===

// Generator Nodes
pub mod oscillator;                    // Legacy oscillator (for WaveformType)
pub mod oscillator_refactored;
pub mod sine_oscillator_refactored;
pub mod noise_refactored;

// Processor Nodes  
pub mod vcf_refactored;
pub mod vca_refactored;
pub mod delay_refactored;
pub mod compressor_refactored;
pub mod waveshaper_refactored;
pub mod ring_modulator_refactored;

// Controller Nodes
pub mod adsr_refactored;
pub mod lfo_refactored;
pub mod sequencer_refactored;

// Utility Nodes
pub mod sample_hold_refactored;
pub mod quantizer_refactored;
pub mod attenuverter_refactored;
pub mod multiple_refactored;
pub mod clock_divider_refactored;

// Mixing/Routing Nodes
pub mod mixer_refactored;
pub mod output_refactored;

// Analyzer Nodes
pub mod oscilloscope_refactored;
pub mod spectrum_analyzer_refactored;

// Legacy modules (for backward compatibility with Tauri)
pub mod output;
pub mod filter;
pub mod envelope;
pub mod lfo;
pub mod mixer;
pub mod delay;
pub mod noise;
pub mod vca;
pub mod sequencer;
pub mod spectrum_analyzer;
pub mod ring_modulator;
pub mod sample_hold;
pub mod attenuverter;
pub mod multiple;
pub mod clock_divider;
pub mod quantizer;
pub mod compressor;
pub mod waveshaper;
pub mod oscilloscope;

// === New Refactored Nodes (ProcessContext Architecture) ===

// Generator Nodes
pub use oscillator_refactored::OscillatorNodeRefactored;
pub use sine_oscillator_refactored::SineOscillatorNodeRefactored;
pub use noise_refactored::{NoiseNodeRefactored, NoiseType};

// Processor Nodes
pub use vcf_refactored::{VCFNodeRefactored, FilterType};
pub use vca_refactored::{VCANodeRefactored, VCAResponse};
pub use delay_refactored::DelayNodeRefactored;
pub use compressor_refactored::CompressorNodeRefactored;
pub use waveshaper_refactored::{WaveshaperNodeRefactored, WaveshaperType};
pub use ring_modulator_refactored::RingModulatorNodeRefactored;

// Controller Nodes
pub use adsr_refactored::{ADSRNodeRefactored, EnvelopeState};
pub use lfo_refactored::{LFONodeRefactored, LFOWaveform};
pub use sequencer_refactored::{SequencerNodeRefactored, SequenceStep, SequencerMode};

// Utility Nodes
pub use sample_hold_refactored::SampleHoldNodeRefactored;
pub use quantizer_refactored::{QuantizerNodeRefactored, ScaleType};
pub use attenuverter_refactored::AttenuverterNodeRefactored;
pub use multiple_refactored::MultipleNodeRefactored;
pub use clock_divider_refactored::ClockDividerNodeRefactored;

// Mixing/Routing Nodes
pub use mixer_refactored::MixerNodeRefactored;
pub use output_refactored::OutputNodeRefactored;

// Analyzer Nodes
pub use oscilloscope_refactored::{OscilloscopeNodeRefactored, TriggerMode, TriggerSlope, Measurements};
pub use spectrum_analyzer_refactored::{SpectrumAnalyzerNodeRefactored, WindowType};

// === Legacy Nodes (Backward Compatibility with Tauri) ===
pub use output::OutputNode;
pub use oscillator::{SineOscillatorNode, OscillatorNode, WaveformType};
pub use filter::{VCFNode, FilterType as LegacyFilterType};
pub use envelope::{ADSRNode, EnvelopeState as LegacyEnvelopeState};
pub use lfo::{LFONode, LFOWaveform as LegacyLFOWaveform};
pub use mixer::MixerNode;
pub use delay::DelayNode;
pub use noise::{NoiseNode, NoiseType as LegacyNoiseType};
pub use vca::VCANode;
pub use sequencer::{SequencerNode, SequenceStep as LegacySequenceStep};
pub use spectrum_analyzer::{SpectrumAnalyzerNode, WindowType as LegacyWindowType};
pub use ring_modulator::RingModulatorNode;
pub use sample_hold::SampleHoldNode;
pub use attenuverter::AttenuverterNode;
pub use multiple::MultipleNode;
pub use clock_divider::ClockDividerNode;
pub use quantizer::{QuantizerNode, Scale};
pub use compressor::CompressorNode;
pub use waveshaper::{WaveshaperNode, WaveshaperType as LegacyWaveshaperType};
pub use oscilloscope::{OscilloscopeNode, TriggerMode as LegacyTriggerMode, TriggerSlope as LegacyTriggerSlope, Measurements as LegacyMeasurements};

use crate::graph::Node;
use std::collections::HashMap;

pub trait AudioNode: Send {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>);
    fn create_node_info(&self, name: String) -> Node;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn as_any(&self) -> &dyn std::any::Any;
}

pub fn create_node(node_type: &str, name: String) -> Result<Box<dyn AudioNode>, String> {
    match node_type {
        "output" => Ok(Box::new(OutputNode::new())),
        "sine_oscillator" => Ok(Box::new(SineOscillatorNode::new(44100.0))),
        "triangle_oscillator" => Ok(Box::new(OscillatorNode::new(44100.0, WaveformType::Triangle))),
        "sawtooth_oscillator" => Ok(Box::new(OscillatorNode::new(44100.0, WaveformType::Sawtooth))),
        "pulse_oscillator" => Ok(Box::new(OscillatorNode::new(44100.0, WaveformType::Pulse))),
        "oscillator" => Ok(Box::new(OscillatorNode::new(44100.0, WaveformType::Sine))), // Default to sine
        "oscilloscope" => Ok(Box::new(OscilloscopeNode::new(uuid::Uuid::new_v4().to_string(), name))),
        "filter" => Ok(Box::new(VCFNode::new(44100.0))),
        "adsr" => Ok(Box::new(ADSRNode::new(44100.0))),
        "lfo" => Ok(Box::new(LFONode::new(uuid::Uuid::new_v4().to_string(), name))),
        "mixer" => Ok(Box::new(MixerNode::new(uuid::Uuid::new_v4().to_string(), name, 4))), // 4チャンネルデフォルト
        "mixer8" => Ok(Box::new(MixerNode::new(uuid::Uuid::new_v4().to_string(), name, 8))), // 8チャンネル
        "delay" => Ok(Box::new(DelayNode::new(name))),
        "noise" => Ok(Box::new(NoiseNode::new(name))),
        "vca" => Ok(Box::new(VCANode::new(name))),
        "sequencer" => Ok(Box::new(SequencerNode::new(name))),
        "spectrum_analyzer" => Ok(Box::new(SpectrumAnalyzerNode::new(name))),
        "ring_modulator" => Ok(Box::new(RingModulatorNode::new(name))),
        "sample_hold" => Ok(Box::new(SampleHoldNode::new(name))),
        "attenuverter" => Ok(Box::new(AttenuverterNode::new(name))),
        "multiple" => Ok(Box::new(MultipleNode::new(name, 4))), // 4-channel multiple by default
        "multiple8" => Ok(Box::new(MultipleNode::new(name, 8))), // 8-channel multiple option
        "clock_divider" => Ok(Box::new(ClockDividerNode::new(name))),
        "quantizer" => Ok(Box::new(QuantizerNode::new(name))),
        "compressor" => Ok(Box::new(CompressorNode::new(name))),
        "waveshaper" => Ok(Box::new(WaveshaperNode::new(name))),
        _ => Err(format!("Unknown node type: {}", node_type)),
    }
}