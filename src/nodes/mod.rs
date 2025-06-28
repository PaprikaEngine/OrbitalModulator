pub mod output;
pub mod oscillator;
pub mod oscilloscope;
pub mod filter;
pub mod envelope;

pub use output::OutputNode;
pub use oscillator::{SineOscillatorNode, OscillatorNode, WaveformType};
pub use oscilloscope::{OscilloscopeNode, TriggerMode, TriggerSlope, Measurements};
pub use filter::{VCFNode, FilterType};
pub use envelope::{ADSRNode, EnvelopeState};

use crate::graph::Node;
use std::collections::HashMap;

pub trait AudioNode: Send {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>);
    fn create_node_info(&self, name: String) -> Node;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
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
        _ => Err(format!("Unknown node type: {}", node_type)),
    }
}