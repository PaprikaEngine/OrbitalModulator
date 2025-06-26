use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Default)]
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
        self.update_processing_order();
        id
    }

    pub fn remove_node(&mut self, id: Uuid) -> Option<Node> {
        // Remove all connections involving this node
        self.connections.retain(|conn| {
            conn.source_node != id && conn.target_node != id
        });
        
        let result = self.nodes.remove(&id);
        self.update_processing_order();
        result
    }

    pub fn add_connection(&mut self, connection: Connection) -> Result<(), String> {
        // Validate connection
        let source_node = self.nodes.get(&connection.source_node)
            .ok_or("Source node not found")?;
        let target_node = self.nodes.get(&connection.target_node)
            .ok_or("Target node not found")?;

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

        self.connections.push(connection);
        self.update_processing_order();
        Ok(())
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
            self.update_processing_order();
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

    fn update_processing_order(&mut self) {
        // Simple topological sort for audio processing order
        self.processing_order.clear();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        let node_ids: Vec<Uuid> = self.nodes.keys().copied().collect();
        for node_id in node_ids {
            if !visited.contains(&node_id) {
                self.visit_node(node_id, &mut visited, &mut temp_visited);
            }
        }
    }

    fn visit_node(&mut self, node_id: Uuid, 
                  visited: &mut std::collections::HashSet<Uuid>,
                  temp_visited: &mut std::collections::HashSet<Uuid>) {
        if temp_visited.contains(&node_id) {
            // Cycle detected - for now just skip
            return;
        }
        if visited.contains(&node_id) {
            return;
        }

        temp_visited.insert(node_id);

        // Visit all nodes that this node depends on (inputs)
        let dependencies: Vec<Uuid> = self.connections.iter()
            .filter(|conn| conn.target_node == node_id)
            .map(|conn| conn.source_node)
            .collect();
        
        for dep_node in dependencies {
            self.visit_node(dep_node, visited, temp_visited);
        }

        temp_visited.remove(&node_id);
        visited.insert(node_id);
        self.processing_order.push(node_id);
    }
}