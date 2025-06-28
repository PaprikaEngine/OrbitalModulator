use crate::audio::AudioEngine;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub parameters: std::collections::HashMap<String, f32>,
    pub input_ports: Vec<PortInfo>,
    pub output_ports: Vec<PortInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortInfo {
    pub name: String,
    pub port_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub source_node: String,
    pub source_port: String,
    pub target_node: String,
    pub target_port: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub name: String,
    pub position: PatchPosition,
    pub parameters: std::collections::HashMap<String, f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatchPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchConnection {
    pub source_node: String,
    pub source_port: String,
    pub target_node: String,
    pub target_port: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchFile {
    pub patch_name: Option<String>,
    pub description: Option<String>,
    pub nodes: Vec<PatchNode>,
    pub connections: Vec<PatchConnection>,
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OscilloscopeData {
    pub waveform: Vec<f32>,
    pub measurements: MeasurementData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeasurementData {
    pub vpp: f32,
    pub vrms: f32,
    pub frequency: f32,
    pub period: f32,
    pub duty_cycle: f32,
}

pub type AudioEngineState = Arc<Mutex<AudioEngine>>;

#[tauri::command]
pub async fn create_node(
    engine: State<'_, AudioEngineState>,
    node_type: String,
    name: String,
) -> Result<String, String> {
    let mut engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    match engine.create_node(&node_type, name) {
        Ok(node_id) => Ok(node_id.to_string()),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn remove_node(
    engine: State<'_, AudioEngineState>,
    node_id: String,
) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let uuid = Uuid::parse_str(&node_id).map_err(|_| "Invalid UUID format".to_string())?;
    engine.remove_node(uuid)
}

#[tauri::command]
pub async fn connect_nodes(
    engine: State<'_, AudioEngineState>,
    source_node: String,
    source_port: String,
    target_node: String,
    target_port: String,
) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let source_uuid = Uuid::parse_str(&source_node).map_err(|_| "Invalid source UUID format".to_string())?;
    let target_uuid = Uuid::parse_str(&target_node).map_err(|_| "Invalid target UUID format".to_string())?;
    engine.connect_nodes(source_uuid, &source_port, target_uuid, &target_port)
}

#[tauri::command]
pub async fn disconnect_nodes(
    engine: State<'_, AudioEngineState>,
    source_node: String,
    source_port: String,
    target_node: String,
    target_port: String,
) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let source_uuid = Uuid::parse_str(&source_node).map_err(|_| "Invalid source UUID format".to_string())?;
    let target_uuid = Uuid::parse_str(&target_node).map_err(|_| "Invalid target UUID format".to_string())?;
    engine.disconnect_nodes(source_uuid, &source_port, target_uuid, &target_port)
}

#[tauri::command]
pub async fn set_node_parameter(
    engine: State<'_, AudioEngineState>,
    node_id: String,
    param: String,
    value: f32,
) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let uuid = Uuid::parse_str(&node_id).map_err(|_| "Invalid UUID format".to_string())?;
    engine.set_node_parameter(uuid, &param, value)
}

#[tauri::command]
pub async fn get_node_parameter(
    engine: State<'_, AudioEngineState>,
    node_id: String,
    param: String,
) -> Result<f32, String> {
    let engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let uuid = Uuid::parse_str(&node_id).map_err(|_| "Invalid UUID format".to_string())?;
    engine.get_node_parameter(uuid, &param)
}

#[tauri::command]
pub async fn list_nodes(
    engine: State<'_, AudioEngineState>,
) -> Result<Vec<NodeInfo>, String> {
    let engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let nodes = engine.list_nodes();
    
    let mut node_infos = Vec::new();
    for (node_id, name, node_type) in nodes {
        if let Some(node_info) = engine.get_node_info(node_id) {
            let input_ports = node_info.input_ports.iter().map(|p| PortInfo {
                name: p.name.clone(),
                port_type: format!("{:?}", p.port_type),
            }).collect();
            
            let output_ports = node_info.output_ports.iter().map(|p| PortInfo {
                name: p.name.clone(),
                port_type: format!("{:?}", p.port_type),
            }).collect();
            
            node_infos.push(NodeInfo {
                id: node_id.to_string(),
                name,
                node_type,
                parameters: node_info.parameters,
                input_ports,
                output_ports,
            });
        }
    }
    
    Ok(node_infos)
}

#[tauri::command]
pub async fn get_connections(
    engine: State<'_, AudioEngineState>,
) -> Result<Vec<ConnectionInfo>, String> {
    let engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let graph = engine.graph.lock().map_err(|e| format!("Failed to lock graph: {}", e))?;
    
    let connections = graph.connections.iter().map(|conn| ConnectionInfo {
        source_node: conn.source_node.to_string(),
        source_port: conn.source_port.clone(),
        target_node: conn.target_node.to_string(),
        target_port: conn.target_port.clone(),
    }).collect();
    
    Ok(connections)
}

#[tauri::command]
pub async fn start_audio(
    engine: State<'_, AudioEngineState>,
) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.start().map_err(|e| format!("Failed to start audio: {}", e))
}

#[tauri::command]
pub async fn stop_audio(
    engine: State<'_, AudioEngineState>,
) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.stop();
    Ok(())
}

#[tauri::command]
pub async fn is_audio_running(
    engine: State<'_, AudioEngineState>,
) -> Result<bool, String> {
    let engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    Ok(engine.is_running())
}

#[tauri::command]
pub async fn save_project(
    engine: State<'_, AudioEngineState>,
    filename: String,
) -> Result<(), String> {
    let engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.save_to_file(&filename)
}

#[tauri::command]
pub async fn load_project(
    engine: State<'_, AudioEngineState>,
    filename: String,
) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.load_from_file(&filename)
}

#[tauri::command]
pub async fn get_spectrum_data(
    engine: State<'_, AudioEngineState>,
    node_id: String,
) -> Result<Vec<f32>, String> {
    let engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let node_id = Uuid::parse_str(&node_id)
        .map_err(|_| "Invalid UUID format".to_string())?;
    
    let node_instances = engine.node_instances.lock()
        .map_err(|e| format!("Failed to lock node instances: {}", e))?;
    
    if let Some(node_instance) = node_instances.get(&node_id) {
        if let Some(spectrum_node) = node_instance.as_any().downcast_ref::<crate::nodes::SpectrumAnalyzerNode>() {
            return Ok(spectrum_node.get_magnitude_spectrum().to_vec());
        }
    }
    
    Err("Spectrum analyzer node not found".to_string())
}

#[tauri::command]
pub async fn get_spectrum_frequencies(
    engine: State<'_, AudioEngineState>,
    node_id: String,
) -> Result<Vec<f32>, String> {
    let engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let node_id = Uuid::parse_str(&node_id)
        .map_err(|_| "Invalid UUID format".to_string())?;
    
    let node_instances = engine.node_instances.lock()
        .map_err(|e| format!("Failed to lock node instances: {}", e))?;
    
    if let Some(node_instance) = node_instances.get(&node_id) {
        if let Some(spectrum_node) = node_instance.as_any().downcast_ref::<crate::nodes::SpectrumAnalyzerNode>() {
            return Ok(spectrum_node.get_frequency_bins());
        }
    }
    
    Err("Spectrum analyzer node not found".to_string())
}

#[tauri::command]
pub async fn save_patch_file(
    engine: State<'_, AudioEngineState>,
    file_path: String,
    patch_name: Option<String>,
    description: Option<String>,
    node_positions: Option<std::collections::HashMap<String, PatchPosition>>,
) -> Result<(), String> {
    let engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    
    // Get current nodes
    let nodes = engine.list_nodes();
    let mut patch_nodes = Vec::new();
    
    for (node_id, name, node_type) in nodes {
        if let Some(node_info) = engine.get_node_info(node_id) {
            // Get position from provided positions or use default
            let position = node_positions.as_ref()
                .and_then(|positions| positions.get(&name))
                .cloned()
                .unwrap_or(PatchPosition { x: 100.0, y: 100.0 });
                
            let patch_node = PatchNode {
                id: name.clone(),
                node_type: node_type.clone(),
                name: name.clone(),
                position,
                parameters: node_info.parameters,
            };
            patch_nodes.push(patch_node);
        }
    }
    
    // Get current connections
    let graph = engine.graph.lock().map_err(|e| format!("Failed to lock graph: {}", e))?;
    let mut patch_connections = Vec::new();
    
    for conn in &graph.connections {
        // Find node names by ID
        let source_name = engine.find_node_name_by_id(conn.source_node);
        let target_name = engine.find_node_name_by_id(conn.target_node);
        
        if let (Some(src_name), Some(tgt_name)) = (source_name, target_name) {
            let patch_conn = PatchConnection {
                source_node: src_name,
                source_port: conn.source_port.clone(),
                target_node: tgt_name,
                target_port: conn.target_port.clone(),
            };
            patch_connections.push(patch_conn);
        }
    }
    
    // Save lengths before moving values
    let nodes_count = patch_nodes.len();
    let connections_count = patch_connections.len();
    
    // Create patch file structure
    let patch = PatchFile {
        patch_name,
        description,
        nodes: patch_nodes,
        connections: patch_connections,
        notes: Some(vec![
            "Generated patch file".to_string(),
            format!("Created with {} nodes and {} connections", 
                   nodes_count, connections_count)
        ]),
    };
    
    // Write to file
    let json_content = serde_json::to_string_pretty(&patch)
        .map_err(|e| format!("Failed to serialize patch: {}", e))?;
        
    fs::write(&file_path, json_content)
        .map_err(|e| format!("Failed to write file {}: {}", file_path, e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn load_patch_file(
    engine: State<'_, AudioEngineState>,
    file_path: String,
) -> Result<(), String> {
    // Read the JSON file
    let json_content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;
    
    // Parse the JSON
    let patch: PatchFile = serde_json::from_str(&json_content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    let mut engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    
    // Clear current graph
    engine.clear_graph().map_err(|e| format!("Failed to clear graph: {}", e))?;
    
    // Create nodes from patch
    for patch_node in &patch.nodes {
        // Create node (engine will assign new UUID)
        let node_id = engine.create_node(&patch_node.node_type, patch_node.name.clone())
            .map_err(|e| format!("Failed to create node {}: {}", patch_node.name, e))?;
        
        // Set parameters
        for (param_name, param_value) in &patch_node.parameters {
            let _ = engine.set_node_parameter(node_id, param_name, *param_value);
            // Ignore parameter errors to allow partial loading
        }
    }
    
    // Create connections
    for connection in &patch.connections {
        // Find node IDs by name
        let source_id = engine.find_node_by_name(&connection.source_node);
        let target_id = engine.find_node_by_name(&connection.target_node);
        
        if let (Some(src_id), Some(tgt_id)) = (source_id, target_id) {
            let _ = engine.connect_nodes(
                src_id, 
                &connection.source_port, 
                tgt_id, 
                &connection.target_port
            );
            // Ignore connection errors to allow partial loading
        }
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_oscilloscope_data(
    engine: State<'_, AudioEngineState>,
    node_id: String,
) -> Result<OscilloscopeData, String> {
    use crate::nodes::OscilloscopeNode;
    
    let engine = engine.lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let uuid = Uuid::parse_str(&node_id).map_err(|_| "Invalid UUID format".to_string())?;
    
    // ノードインスタンスを取得
    let mut node_instances = engine.node_instances.lock().map_err(|e| format!("Failed to lock node instances: {}", e))?;
    
    if let Some(node_instance) = node_instances.get_mut(&uuid) {
        if let Some(osc_node) = node_instance.as_any_mut().downcast_mut::<OscilloscopeNode>() {
            // 波形データ取得
            let waveform_data = osc_node.get_waveform_data();
            let measurements_data = osc_node.get_measurements();
            
            let waveform = if let Ok(data) = waveform_data.lock() {
                data.clone()
            } else {
                Vec::new()
            };
            
            let measurements = if let Ok(data) = measurements_data.lock() {
                MeasurementData {
                    vpp: data.vpp,
                    vrms: data.vrms,
                    frequency: data.frequency,
                    period: data.period,
                    duty_cycle: data.duty_cycle,
                }
            } else {
                MeasurementData {
                    vpp: 0.0,
                    vrms: 0.0,
                    frequency: 0.0,
                    period: 0.0,
                    duty_cycle: 0.0,
                }
            };
            
            Ok(OscilloscopeData {
                waveform,
                measurements,
            })
        } else {
            Err("Node is not an oscilloscope node".to_string())
        }
    } else {
        Err("Node not found".to_string())
    }
}