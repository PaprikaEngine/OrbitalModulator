pub mod output;
pub mod oscillator;
pub mod oscillator_refactored;
pub mod sine_oscillator_refactored;
pub mod noise_refactored;
pub mod vcf_refactored;
pub mod vca_refactored;
pub mod delay_refactored;
pub mod compressor_refactored;
pub mod waveshaper_refactored;
pub mod ring_modulator_refactored;
pub mod adsr_refactored;
pub mod lfo_refactored;
pub mod sequencer_refactored;
pub mod sample_hold_refactored;
pub mod quantizer_refactored;
pub mod attenuverter_refactored;
pub mod multiple_refactored;
pub mod clock_divider_refactored;
pub mod oscilloscope;
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

pub use output::OutputNode;
pub use oscillator::{SineOscillatorNode, OscillatorNode, WaveformType};
pub use oscillator_refactored::OscillatorNodeRefactored;
pub use sine_oscillator_refactored::SineOscillatorNodeRefactored;
pub use noise_refactored::{NoiseNodeRefactored, NoiseType as RefactoredNoiseType};
pub use vcf_refactored::{VCFNodeRefactored, FilterType as RefactoredFilterType};
pub use vca_refactored::{VCANodeRefactored, VCAResponse};
pub use delay_refactored::DelayNodeRefactored;
pub use compressor_refactored::CompressorNodeRefactored;
pub use waveshaper_refactored::{WaveshaperNodeRefactored, WaveshaperType as RefactoredWaveshaperType};
pub use ring_modulator_refactored::RingModulatorNodeRefactored;
pub use adsr_refactored::{ADSRNodeRefactored, EnvelopeState as RefactoredEnvelopeState};
pub use lfo_refactored::{LFONodeRefactored, LFOWaveform as RefactoredLFOWaveform};
pub use sequencer_refactored::{SequencerNodeRefactored, SequenceStep as RefactoredSequenceStep, SequencerMode as RefactoredSequencerMode};
pub use sample_hold_refactored::SampleHoldNodeRefactored;
pub use quantizer_refactored::{QuantizerNodeRefactored, ScaleType as RefactoredScaleType};
pub use attenuverter_refactored::AttenuverterNodeRefactored;
pub use multiple_refactored::MultipleNodeRefactored;
pub use clock_divider_refactored::ClockDividerNodeRefactored;
pub use oscilloscope::{OscilloscopeNode, TriggerMode, TriggerSlope, Measurements};
pub use filter::{VCFNode, FilterType};
pub use envelope::{ADSRNode, EnvelopeState};
pub use lfo::{LFONode, LFOWaveform};
pub use mixer::MixerNode;
pub use delay::DelayNode;
pub use noise::{NoiseNode, NoiseType};
pub use vca::VCANode;
pub use sequencer::{SequencerNode, SequenceStep};
pub use spectrum_analyzer::{SpectrumAnalyzerNode, WindowType};
pub use ring_modulator::RingModulatorNode;
pub use sample_hold::SampleHoldNode;
pub use attenuverter::AttenuverterNode;
pub use multiple::MultipleNode;
pub use clock_divider::ClockDividerNode;
pub use quantizer::{QuantizerNode, Scale};
pub use compressor::CompressorNode;
pub use waveshaper::{WaveshaperNode, WaveshaperType};

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