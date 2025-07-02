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

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::processing::{AudioNode, ProcessContext, ProcessingError, InputPorts, OutputPorts};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PortType {
    AudioMono,
    AudioStereo,
    CV,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub name: String,
    pub port_type: PortType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub source_node: Uuid,
    pub source_port: String,
    pub target_node: Uuid,
    pub target_port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub node_type: String,
    pub name: String,
    pub parameters: HashMap<String, f32>,
    pub input_ports: Vec<Port>,
    pub output_ports: Vec<Port>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AudioGraph {
    pub nodes: HashMap<Uuid, Node>,
    pub connections: Vec<Connection>,
    pub processing_order: Vec<Uuid>,
}

impl AudioGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: Node) -> Uuid {
        let id = node.id;
        self.nodes.insert(id, node);
        if let Err(e) = self.update_processing_order() {
            eprintln!("Warning: {}", e);
        }
        id
    }

    pub fn remove_node(&mut self, id: Uuid) -> Option<Node> {
        // Remove all connections involving this node
        self.connections.retain(|conn| {
            conn.source_node != id && conn.target_node != id
        });
        
        let result = self.nodes.remove(&id);
        if let Err(e) = self.update_processing_order() {
            eprintln!("Warning: {}", e);
        }
        result
    }

    pub fn add_connection(&mut self, connection: Connection) -> Result<(), String> {
        // Validate connection
        let source_node = self.nodes.get(&connection.source_node)
            .ok_or("Source node not found")?;
        let target_node = self.nodes.get(&connection.target_node)
            .ok_or("Target node not found")?;

        // Prevent self-connection
        if connection.source_node == connection.target_node {
            return Err("Cannot connect node to itself".to_string());
        }

        // Check if ports exist and types match
        let source_port = source_node.output_ports.iter()
            .find(|p| p.name == connection.source_port)
            .ok_or("Source port not found")?;
        let target_port = target_node.input_ports.iter()
            .find(|p| p.name == connection.target_port)
            .ok_or("Target port not found")?;

        if source_port.port_type != target_port.port_type {
            return Err("Port types do not match".to_string());
        }

        // Check for existing connection to target port (only one input allowed)
        if self.connections.iter().any(|conn| {
            conn.target_node == connection.target_node && 
            conn.target_port == connection.target_port
        }) {
            return Err("Target port already connected".to_string());
        }

        // Check for cycles before adding connection
        if self.would_create_cycle(&connection) {
            return Err("Connection would create a cycle".to_string());
        }

        self.connections.push(connection);
        self.update_processing_order()?;
        Ok(())
    }

    fn would_create_cycle(&self, new_connection: &Connection) -> bool {
        // Check if adding this connection would create a cycle
        // We need to check if there's already a path from target to source
        self.has_path(new_connection.target_node, new_connection.source_node)
    }

    fn has_path(&self, from: Uuid, to: Uuid) -> bool {
        if from == to {
            return true;
        }

        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![from];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            // Find all nodes this node connects to
            for connection in &self.connections {
                if connection.source_node == current {
                    if connection.target_node == to {
                        return true;
                    }
                    if !visited.contains(&connection.target_node) {
                        stack.push(connection.target_node);
                    }
                }
            }
        }

        false
    }

    pub fn remove_connection(&mut self, source_node: Uuid, source_port: &str, 
                           target_node: Uuid, target_port: &str) -> bool {
        let initial_len = self.connections.len();
        self.connections.retain(|conn| {
            !(conn.source_node == source_node && 
              conn.source_port == source_port &&
              conn.target_node == target_node && 
              conn.target_port == target_port)
        });
        
        let removed = self.connections.len() != initial_len;
        if removed {
            if let Err(e) = self.update_processing_order() {
                eprintln!("Warning: {}", e);
            }
        }
        removed
    }

    pub fn get_node(&self, id: Uuid) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn get_node_by_name(&self, name: &str) -> Option<&Node> {
        self.nodes.values().find(|node| node.name == name)
    }

    pub fn update_node_parameter(&mut self, id: Uuid, param: &str, value: f32) -> Result<(), String> {
        let node = self.nodes.get_mut(&id)
            .ok_or("Node not found")?;
        node.parameters.insert(param.to_string(), value);
        Ok(())
    }

    fn update_processing_order(&mut self) -> Result<(), String> {
        // Simple topological sort for audio processing order
        self.processing_order.clear();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        let node_ids: Vec<Uuid> = self.nodes.keys().copied().collect();
        for node_id in node_ids {
            if !visited.contains(&node_id) {
                if !self.visit_node(node_id, &mut visited, &mut temp_visited) {
                    self.processing_order.clear(); // Clear invalid order
                    return Err("Failed to create processing order due to cycles".to_string());
                }
            }
        }
        Ok(())
    }

    fn visit_node(&mut self, node_id: Uuid, 
                  visited: &mut std::collections::HashSet<Uuid>,
                  temp_visited: &mut std::collections::HashSet<Uuid>) -> bool {
        if temp_visited.contains(&node_id) {
            // Cycle detected - return false to indicate failure
            eprintln!("Warning: Cycle detected involving node {}", node_id);
            return false;
        }
        if visited.contains(&node_id) {
            return true; // Already processed successfully
        }

        temp_visited.insert(node_id);

        // Visit all nodes that this node depends on (inputs)
        let dependencies: Vec<Uuid> = self.connections.iter()
            .filter(|conn| conn.target_node == node_id)
            .map(|conn| conn.source_node)
            .collect();
        
        for dep_node in dependencies {
            if !self.visit_node(dep_node, visited, temp_visited) {
                temp_visited.remove(&node_id);
                return false; // Propagate cycle detection failure
            }
        }

        temp_visited.remove(&node_id);
        visited.insert(node_id);
        self.processing_order.push(node_id);
        true
    }

    pub fn validate_graph(&self) -> Result<(), String> {
        // Check for cycles in the current graph
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        for &node_id in self.nodes.keys() {
            if !visited.contains(&node_id) {
                if !self.check_node_for_cycles(node_id, &mut visited, &mut temp_visited) {
                    return Err(format!("Cycle detected in graph involving node {}", node_id));
                }
            }
        }
        Ok(())
    }

    fn check_node_for_cycles(&self, node_id: Uuid, 
                            visited: &mut std::collections::HashSet<Uuid>,
                            temp_visited: &mut std::collections::HashSet<Uuid>) -> bool {
        if temp_visited.contains(&node_id) {
            return false; // Cycle detected
        }
        if visited.contains(&node_id) {
            return true; // Already processed successfully
        }

        temp_visited.insert(node_id);

        // Check all nodes that this node connects to (outputs)
        for connection in &self.connections {
            if connection.source_node == node_id {
                if !self.check_node_for_cycles(connection.target_node, visited, temp_visited) {
                    return false;
                }
            }
        }

        temp_visited.remove(&node_id);
        visited.insert(node_id);
        true
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.connections.clear();
    }

    pub fn find_node_by_name(&self, name: &str) -> Option<Uuid> {
        for (id, node) in &self.nodes {
            if node.name == name {
                return Some(*id);
            }
        }
        None
    }
}

/// Extended AudioGraph for modern ProcessContext integration
pub struct ProcessingGraph {
    pub audio_nodes: HashMap<Uuid, Box<dyn AudioNode>>,
    pub connections: Vec<Connection>,
    processing_order: Vec<Uuid>,
}

impl ProcessingGraph {
    pub fn new() -> Self {
        Self {
            audio_nodes: HashMap::new(),
            connections: Vec::new(),
            processing_order: Vec::new(),
        }
    }

    /// Add a node instance to the processing graph
    pub fn add_node_instance(&mut self, node: Box<dyn AudioNode>) -> Result<(), String> {
        let node_id = node.node_info().id;
        self.audio_nodes.insert(node_id, node);
        self.update_processing_order()?;
        Ok(())
    }

    /// Remove a node instance
    pub fn remove_node_instance(&mut self, node_id: &str) -> Result<(), String> {
        let uuid = Uuid::parse_str(node_id)
            .map_err(|e| format!("Invalid UUID: {}", e))?;
        
        // Remove all connections involving this node
        self.connections.retain(|conn| {
            conn.source_node != uuid && conn.target_node != uuid
        });
        
        self.audio_nodes.remove(&uuid);
        self.update_processing_order()?;
        Ok(())
    }

    /// Connect two nodes by ID
    pub fn connect_by_id(&mut self, source_id: &str, source_port: &str, 
                         target_id: &str, target_port: &str) -> Result<(), String> {
        let source_uuid = Uuid::parse_str(source_id)
            .map_err(|e| format!("Invalid source UUID: {}", e))?;
        let target_uuid = Uuid::parse_str(target_id)
            .map_err(|e| format!("Invalid target UUID: {}", e))?;

        // Validate nodes exist
        if !self.audio_nodes.contains_key(&source_uuid) {
            return Err("Source node not found".to_string());
        }
        if !self.audio_nodes.contains_key(&target_uuid) {
            return Err("Target node not found".to_string());
        }

        // Prevent self-connection
        if source_uuid == target_uuid {
            return Err("Cannot connect node to itself".to_string());
        }

        let connection = Connection {
            source_node: source_uuid,
            source_port: source_port.to_string(),
            target_node: target_uuid,
            target_port: target_port.to_string(),
        };

        // Check for cycles
        let mut temp_connections = self.connections.clone();
        temp_connections.push(connection.clone());
        if self.would_create_cycle(&temp_connections, source_uuid, target_uuid) {
            return Err("Connection would create a cycle".to_string());
        }

        self.connections.push(connection);
        self.update_processing_order()?;
        Ok(())
    }

    /// Disconnect two nodes by ID
    pub fn disconnect_by_id(&mut self, source_id: &str, source_port: &str,
                            target_id: &str, target_port: &str) -> Result<(), String> {
        let source_uuid = Uuid::parse_str(source_id)
            .map_err(|e| format!("Invalid source UUID: {}", e))?;
        let target_uuid = Uuid::parse_str(target_id)
            .map_err(|e| format!("Invalid target UUID: {}", e))?;

        let initial_len = self.connections.len();
        self.connections.retain(|conn| {
            !(conn.source_node == source_uuid && 
              conn.source_port == source_port &&
              conn.target_node == target_uuid && 
              conn.target_port == target_port)
        });

        if self.connections.len() != initial_len {
            self.update_processing_order()?;
        }

        Ok(())
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&dyn AudioNode> {
        let uuid = Uuid::parse_str(node_id).ok()?;
        self.audio_nodes.get(&uuid).map(|n| n.as_ref())
    }

    /// Get a mutable node by ID
    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut dyn AudioNode> {
        let uuid = Uuid::parse_str(node_id).ok()?;
        if let Some(boxed_node) = self.audio_nodes.get_mut(&uuid) {
            Some(boxed_node.as_mut())
        } else {
            None
        }
    }

    /// Process audio through the entire graph
    pub fn process_audio(&mut self, _inputs: &mut InputPorts, final_outputs: &mut OutputPorts, 
                         sample_rate: f32, buffer_size: usize) -> Result<(), ProcessingError> {
        
        // Storage for node outputs (for signal routing)
        let mut node_output_storage: HashMap<(Uuid, String), Vec<f32>> = HashMap::new();
        
        // Process nodes in dependency order
        for &node_id in &self.processing_order {
            // Prepare input buffers for this node
            let mut node_inputs = InputPorts::new();
            let mut node_outputs = OutputPorts::new();

            // Get node info for buffer initialization
            let node_info = if let Some(node) = self.audio_nodes.get(&node_id) {
                node.node_info().clone()
            } else {
                continue;
            };

            // Initialize output buffers based on node's output ports
            for output_port in &node_info.output_ports {
                match output_port.port_type {
                    crate::graph::PortType::AudioMono => {
                        node_outputs.allocate_audio(output_port.name.clone(), buffer_size);
                    }
                    crate::graph::PortType::AudioStereo => {
                        node_outputs.allocate_audio(format!("{}_left", output_port.name), buffer_size);
                        node_outputs.allocate_audio(format!("{}_right", output_port.name), buffer_size);
                    }
                    crate::graph::PortType::CV => {
                        node_outputs.allocate_cv(output_port.name.clone(), buffer_size);
                    }
                }
            }

            // Route inputs from connected nodes
            for connection in &self.connections {
                if connection.target_node == node_id {
                    // This connection feeds INTO the current node
                    let source_key = (connection.source_node, connection.source_port.clone());
                    
                    if let Some(source_data) = node_output_storage.get(&source_key) {
                        // Determine the target port type from node info
                        let target_port_type = node_info.input_ports.iter()
                            .find(|p| p.name == connection.target_port)
                            .map(|p| p.port_type);
                        
                        match target_port_type {
                            Some(crate::graph::PortType::AudioMono) => {
                                node_inputs.add_audio(connection.target_port.clone(), source_data.clone());
                            }
                            Some(crate::graph::PortType::CV) => {
                                node_inputs.add_cv(connection.target_port.clone(), source_data.clone());
                            }
                            Some(crate::graph::PortType::AudioStereo) => {
                                // For stereo, add to both channels
                                node_inputs.add_audio(format!("{}_left", connection.target_port), source_data.clone());
                                node_inputs.add_audio(format!("{}_right", connection.target_port), source_data.clone());
                            }
                            None => {
                                eprintln!("Warning: Unknown target port {} for node {}", 
                                         connection.target_port, node_id);
                            }
                        }
                    }
                }
            }

            // Create process context and process the node
            let mut ctx = ProcessContext::new(node_inputs, node_outputs, sample_rate, buffer_size);

            if let Some(node) = self.audio_nodes.get_mut(&node_id) {
                // Process the node
                node.process(&mut ctx)?;

                // Store outputs for routing to other nodes
                for output_port in &node_info.output_ports {
                    match output_port.port_type {
                        crate::graph::PortType::AudioMono => {
                            if let Some(output_data) = ctx.outputs.get_audio(&output_port.name) {
                                let key = (node_id, output_port.name.clone());
                                node_output_storage.insert(key, output_data.to_vec());
                            }
                        }
                        crate::graph::PortType::CV => {
                            if let Some(output_data) = ctx.outputs.get_cv(&output_port.name) {
                                let key = (node_id, output_port.name.clone());
                                node_output_storage.insert(key, output_data.to_vec());
                            }
                        }
                        crate::graph::PortType::AudioStereo => {
                            // Handle stereo outputs
                            if let Some(left_data) = ctx.outputs.get_audio(&format!("{}_left", output_port.name)) {
                                let key = (node_id, format!("{}_left", output_port.name));
                                node_output_storage.insert(key, left_data.to_vec());
                            }
                            if let Some(right_data) = ctx.outputs.get_audio(&format!("{}_right", output_port.name)) {
                                let key = (node_id, format!("{}_right", output_port.name));
                                node_output_storage.insert(key, right_data.to_vec());
                            }
                        }
                    }
                }

                // Special handling for OutputNode - route to final output buffers
                if node_info.node_type == "output" {
                    // Route OutputNode's processed inputs (which are stored in mixed_output) to main_left/main_right
                    // Since OutputNode processes stereo to mono in mixed_output, we'll use that for both channels
                    if let Some(mixed_output) = ctx.outputs.get_audio("mixed_output") {
                        final_outputs.allocate_audio("main_left".to_string(), buffer_size);
                        final_outputs.allocate_audio("main_right".to_string(), buffer_size);
                        
                        if let Some(main_left) = final_outputs.get_audio_mut("main_left") {
                            main_left.copy_from_slice(mixed_output);
                        }
                        if let Some(main_right) = final_outputs.get_audio_mut("main_right") {
                            main_right.copy_from_slice(mixed_output);
                        }
                    } else {
                        // Fallback: route inputs directly if no mixed output
                        if let Some(left_input) = ctx.inputs.get_audio("audio_in_l") {
                            final_outputs.allocate_audio("main_left".to_string(), buffer_size);
                            if let Some(main_left) = final_outputs.get_audio_mut("main_left") {
                                main_left.copy_from_slice(left_input);
                            }
                        }
                        if let Some(right_input) = ctx.inputs.get_audio("audio_in_r") {
                            final_outputs.allocate_audio("main_right".to_string(), buffer_size);
                            if let Some(main_right) = final_outputs.get_audio_mut("main_right") {
                                main_right.copy_from_slice(right_input);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if adding a connection would create a cycle
    fn would_create_cycle(&self, connections: &[Connection], from: Uuid, to: Uuid) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![to];

        while let Some(current) = stack.pop() {
            if current == from {
                return true; // Cycle detected
            }
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            // Find all nodes this node connects to
            for connection in connections {
                if connection.source_node == current {
                    if !visited.contains(&connection.target_node) {
                        stack.push(connection.target_node);
                    }
                }
            }
        }

        false
    }

    /// Update processing order using topological sort
    fn update_processing_order(&mut self) -> Result<(), String> {
        self.processing_order.clear();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        let node_ids: Vec<Uuid> = self.audio_nodes.keys().copied().collect();
        for node_id in node_ids {
            if !visited.contains(&node_id) {
                if !self.visit_node(node_id, &mut visited, &mut temp_visited) {
                    self.processing_order.clear();
                    return Err("Failed to create processing order due to cycles".to_string());
                }
            }
        }
        Ok(())
    }

    /// Visit node for topological sort
    fn visit_node(&mut self, node_id: Uuid, 
                  visited: &mut std::collections::HashSet<Uuid>,
                  temp_visited: &mut std::collections::HashSet<Uuid>) -> bool {
        if temp_visited.contains(&node_id) {
            return false; // Cycle detected
        }
        if visited.contains(&node_id) {
            return true; // Already processed
        }

        temp_visited.insert(node_id);

        // Visit all dependencies (nodes that feed into this node)
        let dependencies: Vec<Uuid> = self.connections.iter()
            .filter(|conn| conn.target_node == node_id)
            .map(|conn| conn.source_node)
            .collect();
        
        for dep_node in dependencies {
            if !self.visit_node(dep_node, visited, temp_visited) {
                temp_visited.remove(&node_id);
                return false;
            }
        }

        temp_visited.remove(&node_id);
        visited.insert(node_id);
        self.processing_order.push(node_id);
        true
    }

    /// Remove a node by UUID (wrapper for remove_node_instance)
    pub fn remove_node(&mut self, node_id: Uuid) -> Result<(), String> {
        self.remove_node_instance(&node_id.to_string())
    }

    /// List all node IDs as strings
    pub fn list_nodes(&self) -> Vec<String> {
        self.audio_nodes.keys().map(|id| id.to_string()).collect()
    }

    /// Get node information by string ID
    pub fn get_node_info(&self, node_id: &str) -> Option<crate::processing::NodeInfo> {
        if let Ok(uuid) = Uuid::parse_str(node_id) {
            self.audio_nodes.get(&uuid).map(|node| node.node_info().clone())
        } else {
            None
        }
    }

    /// Find node by name
    pub fn find_node_by_name(&self, name: &str) -> Option<Uuid> {
        for (id, node) in &self.audio_nodes {
            if node.node_info().name == name {
                return Some(*id);
            }
        }
        None
    }

    /// Find node name by ID
    pub fn find_node_name_by_id(&self, node_id: Uuid) -> Option<String> {
        self.audio_nodes.get(&node_id).map(|node| node.node_info().name.clone())
    }

    /// Clear all nodes and connections
    pub fn clear(&mut self) {
        self.audio_nodes.clear();
        self.connections.clear();
        self.processing_order.clear();
    }

    /// Save graph to file (placeholder implementation)
    pub fn save_to_file(&self, _filename: &str) -> Result<(), String> {
        // TODO: Implement actual save functionality
        Ok(())
    }

    /// Load graph from file (placeholder implementation)
    pub fn load_from_file(&mut self, _filename: &str, _sample_rate: f32) -> Result<(), String> {
        // TODO: Implement actual load functionality
        Ok(())
    }
}

impl Default for ProcessingGraph {
    fn default() -> Self {
        Self::new()
    }
}