pub mod audio;
pub mod graph;
pub mod nodes;
pub mod tauri_commands;
pub mod parameters;
pub mod processing;
pub mod errors;

pub use audio::AudioEngine;
pub use graph::{AudioGraph, Node, Port, PortType, Connection};
pub use nodes::{AudioNode, create_node};
pub use parameters::{Parameterizable, ParameterDescriptor, ParameterError};
pub use processing::{ProcessContext, ProcessingError, NodeInfo, NodeCategory};
pub use errors::{AudioEngineError, AudioEngineResult, Logger, ConsoleLogger, LogLevel};