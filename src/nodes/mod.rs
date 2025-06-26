pub mod output;
pub mod oscillator;

pub use output::OutputNode;
pub use oscillator::SineOscillatorNode;

use crate::graph::Node;
use std::collections::HashMap;

pub trait AudioNode: Send {
    fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>);
    fn create_node_info(&self, name: String) -> Node;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

pub fn create_node(node_type: &str, _name: String) -> Result<Box<dyn AudioNode>, String> {
    match node_type {
        "output" => Ok(Box::new(OutputNode::new())),
        "sine_oscillator" => Ok(Box::new(SineOscillatorNode::new(44100.0))),
        _ => Err(format!("Unknown node type: {}", node_type)),
    }
}