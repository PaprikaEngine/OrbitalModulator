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
//! This module provides the unified ProcessContext architecture for all nodes.
//! 
//! ## ProcessContext Architecture (Production Ready)
//! - Unified `ProcessContext` for audio processing
//! - Professional CV modulation with `ModulatableParameter`
//! - Comprehensive error handling and type safety
//! - Full Eurorack compliance (1V/Oct, ±10V CV, 5V gates)
//! - Commercial-grade audio quality
//! 
//! **Status: 21/21 nodes complete (100%)**
//! **Migration: Complete - Ready for production use**

// === Node Module Declarations ===

// Generator Nodes
pub mod oscillator;
pub mod sine_oscillator;
pub mod noise;

// Processor Nodes  
pub mod vcf;
pub mod vca;
pub mod delay;
pub mod compressor;
pub mod waveshaper;
pub mod ring_modulator;

// Controller Nodes
pub mod adsr;
pub mod lfo;
pub mod sequencer;

// Utility Nodes
pub mod sample_hold;
pub mod quantizer;
pub mod attenuverter;
pub mod multiple;
pub mod clock_divider;

// Mixing/Routing Nodes
pub mod mixer;
pub mod output;

// Analyzer Nodes
pub mod oscilloscope;
pub mod spectrum_analyzer;

// === Node Exports ===

// Generator Nodes
pub use oscillator::{OscillatorNode, WaveformType};
pub use sine_oscillator::SineOscillatorNode;
pub use noise::{NoiseNode, NoiseType};

// Processor Nodes
pub use vcf::{VCFNode, FilterType};
pub use vca::{VCANode, VCAResponse};
pub use delay::DelayNode;
pub use compressor::CompressorNode;
pub use waveshaper::{WaveshaperNode, WaveshaperType};
pub use ring_modulator::RingModulatorNode;

// Controller Nodes
pub use adsr::{ADSRNode, EnvelopeState};
pub use lfo::{LFONode, LFOWaveform};
pub use sequencer::{SequencerNode, SequenceStep, SequencerMode};

// Utility Nodes
pub use sample_hold::SampleHoldNode;
pub use quantizer::{QuantizerNode, ScaleType};
pub use attenuverter::AttenuverterNode;
pub use multiple::MultipleNode;
pub use clock_divider::ClockDividerNode;

// Mixing/Routing Nodes
pub use mixer::MixerNode;
pub use output::OutputNode;

// Analyzer Nodes
pub use oscilloscope::{OscilloscopeNode, TriggerMode, TriggerSlope, Measurements};
pub use spectrum_analyzer::{SpectrumAnalyzerNode, WindowType};

// === Node Creation ===
// Note: Node creation is now handled by AudioEngine::create_builtin_node
// This module only exports node types for the unified ProcessContext architecture