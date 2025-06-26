use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, SampleFormat, SampleRate};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::graph::AudioGraph;
use crate::nodes::{AudioNode, create_node};
use std::collections::{HashMap, HashSet};
use std::fs;
use uuid::Uuid;

pub struct AudioEngine {
    pub graph: Arc<Mutex<AudioGraph>>,
    node_instances: Arc<Mutex<HashMap<Uuid, Box<dyn AudioNode + Send>>>>,
    is_running: Arc<AtomicBool>,
    _stream: Option<Stream>,
    sample_rate: f32,
    buffer_size: usize,
}

// AudioEngineをSend + Syncにするため
unsafe impl Send for AudioEngine {}
unsafe impl Sync for AudioEngine {}

impl AudioEngine {
    pub fn new(sample_rate: f32, buffer_size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            graph: Arc::new(Mutex::new(AudioGraph::new())),
            node_instances: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(AtomicBool::new(false)),
            _stream: None,
            sample_rate,
            buffer_size,
        })
    }

    pub fn create_node(&mut self, node_type: &str, name: String) -> Result<Uuid, String> {
        // Create node instance
        let node_instance = create_node(node_type, name.clone())?;
        let node_info = node_instance.create_node_info(name);
        let node_id = node_info.id;

        // Add to graph
        {
            let mut graph = self.graph.lock().unwrap();
            graph.add_node(node_info);
        }

        // Store node instance
        {
            let mut instances = self.node_instances.lock().unwrap();
            instances.insert(node_id, node_instance);
        }

        Ok(node_id)
    }

    pub fn remove_node(&mut self, node_id: Uuid) -> Result<(), String> {
        {
            let mut graph = self.graph.lock().unwrap();
            graph.remove_node(node_id);
        }

        {
            let mut instances = self.node_instances.lock().unwrap();
            instances.remove(&node_id);
        }

        Ok(())
    }

    pub fn connect_nodes(&mut self, source_node: Uuid, source_port: &str, 
                        target_node: Uuid, target_port: &str) -> Result<(), String> {
        let connection = crate::graph::Connection {
            source_node,
            source_port: source_port.to_string(),
            target_node,
            target_port: target_port.to_string(),
        };

        let mut graph = self.graph.lock().unwrap();
        graph.add_connection(connection)
    }

    pub fn disconnect_nodes(&mut self, source_node: Uuid, source_port: &str,
                           target_node: Uuid, target_port: &str) -> Result<(), String> {
        let mut graph = self.graph.lock().unwrap();
        if graph.remove_connection(source_node, source_port, target_node, target_port) {
            Ok(())
        } else {
            Err("Connection not found".to_string())
        }
    }

    pub fn set_node_parameter(&mut self, node_id: Uuid, param: &str, value: f32) -> Result<(), String> {
        // Update graph parameter
        {
            let mut graph = self.graph.lock().unwrap();
            graph.update_node_parameter(node_id, param, value)?;
        }

        // Update node instance
        {
            let mut instances = self.node_instances.lock().unwrap();
            if let Some(node_instance) = instances.get_mut(&node_id) {
                // Update specific node types
                if let Some(sine_osc) = node_instance.as_any_mut().downcast_mut::<crate::nodes::SineOscillatorNode>() {
                    match param {
                        "frequency" => sine_osc.set_frequency(value),
                        "amplitude" => sine_osc.set_amplitude(value),
                        _ => return Err(format!("Unknown parameter: {}", param)),
                    }
                } else if let Some(osc) = node_instance.as_any_mut().downcast_mut::<crate::nodes::OscillatorNode>() {
                    match param {
                        "frequency" => osc.set_frequency(value),
                        "amplitude" => osc.set_amplitude(value),
                        "waveform" => {
                            let waveform = match value as u8 {
                                0 => crate::nodes::WaveformType::Sine,
                                1 => crate::nodes::WaveformType::Triangle,
                                2 => crate::nodes::WaveformType::Sawtooth,
                                3 => crate::nodes::WaveformType::Pulse,
                                _ => return Err(format!("Invalid waveform value: {}", value)),
                            };
                            osc.set_waveform(waveform);
                        },
                        "pulse_width" => osc.set_pulse_width(value),
                        _ => return Err(format!("Unknown parameter: {}", param)),
                    }
                } else if let Some(output_node) = node_instance.as_any_mut().downcast_mut::<crate::nodes::OutputNode>() {
                    match param {
                        "master_volume" => output_node.set_master_volume(value),
                        "mute" => output_node.set_mute(value != 0.0),
                        _ => return Err(format!("Unknown parameter: {}", param)),
                    }
                }
            }
        }
        
        Ok(())
    }

    pub fn get_node_parameter(&self, node_id: Uuid, param: &str) -> Result<f32, String> {
        let graph = self.graph.lock().unwrap();
        let node = graph.get_node(node_id)
            .ok_or("Node not found")?;
        
        node.parameters.get(param)
            .copied()
            .ok_or("Parameter not found".to_string())
    }

    pub fn set_node_parameter_by_id(&mut self, node_id_str: &str, param: &str, value: f32) -> Result<(), String> {
        let node_id = Uuid::parse_str(node_id_str)
            .map_err(|_| "Invalid UUID format".to_string())?;
        
        self.set_node_parameter(node_id, param, value)
    }

    pub fn get_node_parameter_by_id(&self, node_id_str: &str, param: &str) -> Result<f32, String> {
        let node_id = Uuid::parse_str(node_id_str)
            .map_err(|_| "Invalid UUID format".to_string())?;
        
        self.get_node_parameter(node_id, param)
    }

    pub fn connect_nodes_by_id(&mut self, source_id_str: &str, source_port: &str, 
                              target_id_str: &str, target_port: &str) -> Result<(), String> {
        let source_id = Uuid::parse_str(source_id_str)
            .map_err(|_| "Invalid source UUID format".to_string())?;
        let target_id = Uuid::parse_str(target_id_str)
            .map_err(|_| "Invalid target UUID format".to_string())?;
        
        self.connect_nodes(source_id, source_port, target_id, target_port)
    }

    pub fn disconnect_nodes_by_id(&mut self, source_id_str: &str, source_port: &str,
                                 target_id_str: &str, target_port: &str) -> Result<(), String> {
        let source_id = Uuid::parse_str(source_id_str)
            .map_err(|_| "Invalid source UUID format".to_string())?;
        let target_id = Uuid::parse_str(target_id_str)
            .map_err(|_| "Invalid target UUID format".to_string())?;
        
        self.disconnect_nodes(source_id, source_port, target_id, target_port)
    }

    pub fn list_nodes(&self) -> Vec<(Uuid, String, String)> {
        let graph = self.graph.lock().unwrap();
        graph.nodes.values()
            .map(|node| (node.id, node.name.clone(), node.node_type.clone()))
            .collect()
    }

    pub fn get_node_info(&self, node_id: Uuid) -> Option<crate::graph::Node> {
        let graph = self.graph.lock().unwrap();
        graph.get_node(node_id).cloned()
    }

    pub fn get_node_tree(&self) -> String {
        let graph = self.graph.lock().unwrap();
        let mut output = String::new();
        
        output.push_str("Node Tree:\n");
        output.push_str("==========\n\n");
        
        // Find root nodes (nodes with no inputs from other nodes)
        let mut root_nodes = Vec::new();
        for (node_id, _node) in &graph.nodes {
            let has_inputs = graph.connections.iter()
                .any(|conn| conn.target_node == *node_id);
            if !has_inputs {
                root_nodes.push(*node_id);
            }
        }
        
        // If no root nodes found, use all nodes
        if root_nodes.is_empty() {
            root_nodes = graph.nodes.keys().copied().collect();
        }
        
        // Build tree for each root node
        for root_id in root_nodes {
            if let Some(_root_node) = graph.nodes.get(&root_id) {
                self.build_tree_branch(&graph, root_id, &mut output, 0, &mut HashSet::new());
            }
        }
        
        output
    }
    
    fn build_tree_branch(
        &self,
        graph: &crate::graph::AudioGraph,
        node_id: Uuid,
        output: &mut String,
        depth: usize,
        visited: &mut HashSet<Uuid>
    ) {
        if visited.contains(&node_id) {
            return; // Avoid infinite loops
        }
        visited.insert(node_id);
        
        if let Some(node) = graph.nodes.get(&node_id) {
            // Create indentation
            let indent = "  ".repeat(depth);
            let prefix = if depth == 0 { "🌳" } else { "├─" };
            
            // Display node info
            output.push_str(&format!("{}{} {} ({})\n", indent, prefix, node.name, node.node_type));
            output.push_str(&format!("{}   ID: {}\n", indent, node_id));
            
            // Show parameters if any
            if !node.parameters.is_empty() {
                output.push_str(&format!("{}   Parameters: ", indent));
                let params: Vec<String> = node.parameters.iter()
                    .map(|(k, v)| format!("{}={:.2}", k, v))
                    .collect();
                output.push_str(&params.join(", "));
                output.push('\n');
            }
            
            // Find connected child nodes
            let mut children = Vec::new();
            for connection in &graph.connections {
                if connection.source_node == node_id {
                    children.push((connection.target_node, connection.source_port.clone(), connection.target_port.clone()));
                }
            }
            
            // Display connections and recurse to children
            for (child_id, source_port, target_port) in children {
                output.push_str(&format!("{}   └─ {}:{} → ", indent, source_port, target_port));
                if let Some(child_node) = graph.nodes.get(&child_id) {
                    output.push_str(&format!("{}:{}\n", child_node.name, target_port));
                    self.build_tree_branch(graph, child_id, output, depth + 1, visited);
                }
            }
            
            if depth == 0 {
                output.push('\n');
            }
        }
        
        visited.remove(&node_id);
    }

    pub fn get_graph_visualization(&self) -> String {
        let graph = self.graph.lock().unwrap();
        let mut output = String::new();
        
        output.push_str("Node Graph:\n");
        output.push_str("===========\n\n");
        
        // List all nodes with their details
        for (node_id, node) in &graph.nodes {
            output.push_str(&format!("📦 {} ({})\n", node.name, node.node_type));
            output.push_str(&format!("   ID: {}\n", node_id));
            
            // Show parameters
            if !node.parameters.is_empty() {
                output.push_str("   Parameters:\n");
                for (param, value) in &node.parameters {
                    output.push_str(&format!("     {} = {}\n", param, value));
                }
            }
            
            // Show ports
            if !node.input_ports.is_empty() {
                output.push_str("   Inputs:\n");
                for port in &node.input_ports {
                    output.push_str(&format!("     🔌 {} ({:?})\n", port.name, port.port_type));
                }
            }
            
            if !node.output_ports.is_empty() {
                output.push_str("   Outputs:\n");
                for port in &node.output_ports {
                    output.push_str(&format!("     🔌 {} ({:?})\n", port.name, port.port_type));
                }
            }
            
            output.push_str("\n");
        }
        
        // Show connections
        if !graph.connections.is_empty() {
            output.push_str("Connections:\n");
            output.push_str("============\n\n");
            
            for connection in &graph.connections {
                let source_name = graph.nodes.get(&connection.source_node)
                    .map(|n| n.name.as_str()).unwrap_or("unknown");
                let target_name = graph.nodes.get(&connection.target_node)
                    .map(|n| n.name.as_str()).unwrap_or("unknown");
                
                output.push_str(&format!("🔗 {}:{} -> {}:{}\n", 
                    source_name, connection.source_port,
                    target_name, connection.target_port));
            }
            output.push_str("\n");
        }
        
        // Show processing order
        if !graph.processing_order.is_empty() {
            output.push_str("Processing Order:\n");
            output.push_str("=================\n\n");
            
            for (i, &node_id) in graph.processing_order.iter().enumerate() {
                if let Some(node) = graph.nodes.get(&node_id) {
                    output.push_str(&format!("{}. {} ({})\n", i + 1, node.name, node.node_type));
                }
            }
        }
        
        output
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Initialize CPAL
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or("No default output device found")?;

        let mut supported_configs = device.supported_output_configs()?;
        let config = supported_configs
            .find(|c| c.sample_format() == SampleFormat::F32)
            .ok_or("No supported F32 format found")?
            .with_sample_rate(SampleRate(self.sample_rate as u32));

        let graph = Arc::clone(&self.graph);
        let node_instances = Arc::clone(&self.node_instances);
        let _is_running = Arc::clone(&self.is_running);

        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                Self::audio_callback(data, &graph, &node_instances);
            },
            move |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None
        )?;

        stream.play()?;
        self._stream = Some(stream);
        
        self.is_running.store(true, Ordering::Relaxed);

        println!("Audio engine started - Sample Rate: {}Hz, Buffer Size: {}", 
                self.sample_rate, self.buffer_size);

        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
        self._stream = None;
        println!("Audio engine stopped");
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub fn save_to_file(&self, filename: &str) -> Result<(), String> {
        let graph = self.graph.lock().unwrap();
        
        // Serialize the graph to JSON
        let json = serde_json::to_string_pretty(&*graph)
            .map_err(|e| format!("Failed to serialize graph: {}", e))?;
        
        // Write to file
        fs::write(filename, json)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        
        println!("Saved {} nodes and {} connections to {}", 
                graph.nodes.len(), graph.connections.len(), filename);
        Ok(())
    }

    pub fn load_from_file(&mut self, filename: &str) -> Result<(), String> {
        // Stop audio if running
        let was_running = self.is_running();
        if was_running {
            self.stop();
        }

        // Read file
        let json = fs::read_to_string(filename)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        // Deserialize the graph
        let loaded_graph: crate::graph::AudioGraph = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize graph: {}", e))?;
        
        // Clear current graph and node instances
        {
            let mut graph = self.graph.lock().unwrap();
            *graph = loaded_graph;
        }
        
        // Recreate node instances based on the loaded graph
        self.recreate_node_instances()?;
        
        println!("Loaded {} nodes and {} connections from {}", 
                self.list_nodes().len(), 
                self.graph.lock().unwrap().connections.len(), 
                filename);
        
        // Restart audio if it was running
        if was_running {
            self.start().map_err(|e| format!("Failed to restart audio: {}", e))?;
        }
        
        Ok(())
    }

    fn recreate_node_instances(&mut self) -> Result<(), String> {
        let graph = self.graph.lock().unwrap();
        let nodes_to_create: Vec<(Uuid, String, String, HashMap<String, f32>)> = graph.nodes.iter()
            .map(|(id, node)| (*id, node.node_type.clone(), node.name.clone(), node.parameters.clone()))
            .collect();
        drop(graph); // Release the lock before modifying instances
        
        let mut instances = self.node_instances.lock().unwrap();
        instances.clear();
        
        for (node_id, node_type, node_name, parameters) in nodes_to_create {
            // Create node instance based on node type
            let mut node_instance = create_node(&node_type, node_name)?;
            
            // Apply saved parameters to the node instance
            for (param, value) in &parameters {
                match node_type.as_str() {
                    "sine_oscillator" => {
                        if let Some(sine_osc) = node_instance.as_any_mut().downcast_mut::<crate::nodes::SineOscillatorNode>() {
                            match param.as_str() {
                                "frequency" => sine_osc.set_frequency(*value),
                                "amplitude" => sine_osc.set_amplitude(*value),
                                _ => {}
                            }
                        }
                    }
                    "triangle_oscillator" | "sawtooth_oscillator" | "pulse_oscillator" | "oscillator" => {
                        if let Some(osc) = node_instance.as_any_mut().downcast_mut::<crate::nodes::OscillatorNode>() {
                            match param.as_str() {
                                "frequency" => osc.set_frequency(*value),
                                "amplitude" => osc.set_amplitude(*value),
                                "waveform" => {
                                    let waveform = match *value as u8 {
                                        0 => crate::nodes::WaveformType::Sine,
                                        1 => crate::nodes::WaveformType::Triangle,
                                        2 => crate::nodes::WaveformType::Sawtooth,
                                        3 => crate::nodes::WaveformType::Pulse,
                                        _ => crate::nodes::WaveformType::Sine, // Default
                                    };
                                    osc.set_waveform(waveform);
                                },
                                "pulse_width" => osc.set_pulse_width(*value),
                                _ => {}
                            }
                        }
                    }
                    "output" => {
                        if let Some(output_node) = node_instance.as_any_mut().downcast_mut::<crate::nodes::OutputNode>() {
                            match param.as_str() {
                                "master_volume" => output_node.set_master_volume(*value),
                                "mute" => output_node.set_mute(*value != 0.0),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            instances.insert(node_id, node_instance);
        }
        
        Ok(())
    }

    fn audio_callback(
        output: &mut [f32],
        graph: &Arc<Mutex<AudioGraph>>,
        node_instances: &Arc<Mutex<HashMap<Uuid, Box<dyn AudioNode + Send>>>>
    ) {
        // Clear output buffer
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        // Try to process the graph
        if let (Ok(graph_guard), Ok(mut instances_guard)) = (graph.try_lock(), node_instances.try_lock()) {
            Self::process_graph(&*graph_guard, &mut *instances_guard, output);
        } else {
            // Fallback: generate test tone if locks fail
            let frequency = 440.0;
            let sample_rate = 44100.0;
            
            for (i, sample) in output.iter_mut().enumerate() {
                let t = i as f32 / sample_rate;
                *sample = (t * frequency * 2.0 * std::f32::consts::PI).sin() * 0.1;
            }
        }
    }

    fn process_graph(
        graph: &AudioGraph,
        node_instances: &mut HashMap<Uuid, Box<dyn AudioNode + Send>>,
        final_output: &mut [f32]
    ) {
        let buffer_size = final_output.len();
        
        // Create temporary buffers for inter-node communication
        let mut node_outputs: HashMap<(Uuid, String), Vec<f32>> = HashMap::new();
        
        // Process nodes in topological order
        for &node_id in &graph.processing_order {
            if let (Some(node_info), Some(node_instance)) = (
                graph.get_node(node_id),
                node_instances.get_mut(&node_id)
            ) {
                // Prepare input buffers for this node
                let mut inputs: HashMap<String, &[f32]> = HashMap::new();
                
                for input_port in &node_info.input_ports {
                    // Find connections to this input port
                    for connection in &graph.connections {
                        if connection.target_node == node_id && connection.target_port == input_port.name {
                            let source_key = (connection.source_node, connection.source_port.clone());
                            if let Some(source_data) = node_outputs.get(&source_key) {
                                inputs.insert(input_port.name.clone(), source_data.as_slice());
                            }
                        }
                    }
                }
                
                // Prepare output buffers for this node
                let mut outputs: HashMap<String, Vec<f32>> = HashMap::new();
                for output_port in &node_info.output_ports {
                    outputs.insert(output_port.name.clone(), vec![0.0; buffer_size]);
                }
                
                // Convert outputs to mutable references
                let mut output_refs: HashMap<String, &mut [f32]> = HashMap::new();
                for (port_name, buffer) in &mut outputs {
                    output_refs.insert(port_name.clone(), buffer.as_mut_slice());
                }
                
                // Process this node
                node_instance.process(&inputs, &mut output_refs);
                
                // Store outputs for other nodes to use
                for (port_name, buffer) in outputs {
                    node_outputs.insert((node_id, port_name), buffer);
                }
                
                // If this is an output node, copy its processed output to final output
                if node_info.node_type == "output" {
                    let output_key = (node_id, "mixed_output".to_string());
                    if let Some(mixed_data) = node_outputs.get(&output_key) {
                        for (i, &sample) in mixed_data.iter().enumerate() {
                            if i < final_output.len() {
                                final_output[i] += sample; // Use the output node's processed audio directly
                            }
                        }
                    }
                }
            }
        }
    }
}