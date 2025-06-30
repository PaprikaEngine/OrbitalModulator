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

pub mod audio;
pub mod graph;
pub mod nodes;
pub mod tauri_commands;
pub mod parameters;
pub mod processing;
pub mod errors;
pub mod plugin;

pub use audio::AudioEngine;
pub use graph::{AudioGraph, Node, Port, PortType, Connection, ProcessingGraph};
// Node exports moved to audio::AudioEngine for unified architecture
pub use parameters::{Parameterizable, ParameterDescriptor, ParameterError};
pub use processing::{ProcessContext, ProcessingError, NodeInfo, NodeCategory, InputPorts, OutputPorts};
pub use errors::{AudioEngineError, AudioEngineResult, Logger, ConsoleLogger, LogLevel};
pub use plugin::{PluginManager, PluginError, PluginResult, PluginConfig, PluginStats};