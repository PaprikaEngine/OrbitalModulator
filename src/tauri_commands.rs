use crate::audio::AudioEngine;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::fs;

// Include plugin commands
pub mod plugin;
pub use plugin::*;

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
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    match engine.create_builtin_node(&node_type, name) {
        Ok(node_id) => Ok(node_id.to_string()),
        Err(e) => Err(format!("Failed to create node: {}", e)),
    }
}

#[tauri::command]
pub async fn remove_node(
    engine: State<'_, AudioEngineState>,
    node_id: String,
) -> Result<(), String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let uuid = Uuid::parse_str(&node_id).map_err(|_| "Invalid UUID format".to_string())?;
    engine.remove_node(uuid)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectNodesRequest {
    pub source_node: String,
    pub source_port: String,
    pub target_node: String,
    pub target_port: String,
}

#[tauri::command]
pub async fn connect_nodes(
    engine: State<'_, AudioEngineState>,
    request: ConnectNodesRequest,
) -> Result<(), String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.connect_nodes(&request.source_node, &request.source_port, &request.target_node, &request.target_port)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisconnectNodesRequest {
    pub source_node: String,
    pub source_port: String,
    pub target_node: String,
    pub target_port: String,
}

#[tauri::command]
pub async fn disconnect_nodes(
    engine: State<'_, AudioEngineState>,
    request: DisconnectNodesRequest,
) -> Result<(), String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.disconnect_nodes(&request.source_node, &request.source_port, &request.target_node, &request.target_port)
}

#[tauri::command]
pub async fn set_node_parameter(
    engine: State<'_, AudioEngineState>,
    node_id: String,
    param: String,
    value: f32,
) -> Result<(), String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.set_node_parameter(&node_id, &param, value)
}

#[tauri::command]
pub async fn get_node_parameter(
    engine: State<'_, AudioEngineState>,
    node_id: String,
    param: String,
) -> Result<f32, String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.get_node_parameter(&node_id, &param)
}

#[tauri::command]
pub async fn list_nodes(
    engine: State<'_, AudioEngineState>,
) -> Result<Vec<NodeInfo>, String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let nodes = engine.list_nodes();
    
    let mut node_infos = Vec::new();
    for node_id in nodes {
        if let Some(node_info) = engine.get_node_info(&node_id) {
            let input_ports = node_info.input_ports.iter().map(|p| PortInfo {
                name: p.name.clone(),
                port_type: format!("{:?}", p.port_type),
            }).collect();
            
            let output_ports = node_info.output_ports.iter().map(|p| PortInfo {
                name: p.name.clone(),
                port_type: format!("{:?}", p.port_type),
            }).collect();
            
            let parameters = engine.get_node_parameters(&node_id).unwrap_or_default();
            
            node_infos.push(NodeInfo {
                id: node_id,
                name: node_info.name.clone(),
                node_type: node_info.node_type.clone(),
                parameters,
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
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
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
    let mut engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.start().map_err(|e| format!("Failed to start audio: {}", e))
}

#[tauri::command]
pub async fn stop_audio(
    engine: State<'_, AudioEngineState>,
) -> Result<(), String> {
    let mut engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let _ = engine.stop();
    Ok(())
}

#[tauri::command]
pub async fn is_audio_running(
    engine: State<'_, AudioEngineState>,
) -> Result<bool, String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    Ok(engine.is_running())
}

#[tauri::command]
pub async fn save_project(
    engine: State<'_, AudioEngineState>,
    filename: String,
) -> Result<(), String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.save_to_file(&filename)
}

#[tauri::command]
pub async fn load_project(
    engine: State<'_, AudioEngineState>,
    filename: String,
) -> Result<(), String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    engine.load_from_file(&filename)
}

#[tauri::command]
pub async fn get_spectrum_data(
    engine: State<'_, AudioEngineState>,
    node_id: String,
) -> Result<Vec<f32>, String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let node_id = Uuid::parse_str(&node_id)
        .map_err(|_| "Invalid UUID format".to_string())?;
    
    let graph = engine.graph.lock()
        .map_err(|e| format!("Failed to lock graph: {}", e))?;
    
    if let Some(node) = graph.get_node(&node_id.to_string()) {
        if let Some(spectrum_node) = node.as_any().downcast_ref::<crate::nodes::SpectrumAnalyzerNode>() {
            return Ok(spectrum_node.get_display_spectrum().to_vec());
        }
    }
    
    Err("Spectrum analyzer node not found".to_string())
}

#[tauri::command]
pub async fn get_spectrum_frequencies(
    engine: State<'_, AudioEngineState>,
    node_id: String,
) -> Result<Vec<f32>, String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let node_id = Uuid::parse_str(&node_id)
        .map_err(|_| "Invalid UUID format".to_string())?;
    
    let graph = engine.graph.lock()
        .map_err(|e| format!("Failed to lock graph: {}", e))?;
    
    if let Some(node) = graph.get_node(&node_id.to_string()) {
        if let Some(_spectrum_node) = node.as_any().downcast_ref::<crate::nodes::SpectrumAnalyzerNode>() {
            // Return frequency bins - we need to implement this or use a placeholder
            let sample_rate = 44100.0; // TODO: get from engine
            let fft_size = 1024; // TODO: get from spectrum analyzer
            let frequency_bins: Vec<f32> = (0..fft_size/2)
                .map(|i| i as f32 * sample_rate / fft_size as f32)
                .collect();
            return Ok(frequency_bins);
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
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    
    // Get current nodes
    let nodes = engine.list_nodes();
    let mut patch_nodes = Vec::new();
    
    for node_id in nodes {
        if let Some(node_info) = engine.get_node_info(&node_id) {
            // Get position from provided positions or use default
            let position = node_positions.as_ref()
                .and_then(|positions| positions.get(&node_info.name))
                .cloned()
                .unwrap_or(PatchPosition { x: 100.0, y: 100.0 });
                
            let parameters = engine.get_node_parameters(&node_id).unwrap_or_default();
            
            let patch_node = PatchNode {
                id: node_info.name.clone(),
                node_type: node_info.node_type.clone(),
                name: node_info.name.clone(),
                position,
                parameters,
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
    
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    
    // Clear current graph
    engine.clear_graph().map_err(|e| format!("Failed to clear graph: {}", e))?;
    
    // Create nodes from patch
    for patch_node in &patch.nodes {
        // Create node using patch ID as name for consistent referencing
        match engine.create_builtin_node(&patch_node.node_type, patch_node.id.clone()) {
            Ok(node_id) => {
                println!("✅ Created node: {} '{}' ({}) with ID: {}", patch_node.id, patch_node.name, patch_node.node_type, node_id);
                
                // Set parameters
                for (param_name, param_value) in &patch_node.parameters {
                    match engine.set_node_parameter(&node_id, param_name, *param_value) {
                        Ok(()) => {
                            println!("  ✅ Set parameter {} = {}", param_name, param_value);
                        },
                        Err(e) => {
                            println!("  ❌ Failed to set parameter {}: {}", param_name, e);
                        }
                    }
                }
            },
            Err(e) => {
                println!("❌ Failed to create node {} '{}': {}", patch_node.id, patch_node.name, e);
                return Err(format!("Failed to create node {} '{}': {}", patch_node.id, patch_node.name, e));
            }
        }
    }
    
    // Create connections
    for connection in &patch.connections {
        // Find node IDs by name
        let source_id = engine.find_node_by_name(&connection.source_node);
        let target_id = engine.find_node_by_name(&connection.target_node);
        
        match (source_id, target_id) {
            (Some(src_id), Some(tgt_id)) => {
                println!("🔗 Attempting connection: {} [{}] {} -> {} [{}] {}", 
                       connection.source_node, src_id, connection.source_port,
                       connection.target_node, tgt_id, connection.target_port);
                match engine.connect_nodes(
                    &src_id.to_string(), 
                    &connection.source_port, 
                    &tgt_id.to_string(), 
                    &connection.target_port
                ) {
                    Ok(()) => {
                        println!("✅ Connected: {} {} -> {} {}", 
                               connection.source_node, connection.source_port,
                               connection.target_node, connection.target_port);
                    },
                    Err(e) => {
                        println!("❌ Connection failed: {} {} -> {} {}: {}", 
                               connection.source_node, connection.source_port,
                               connection.target_node, connection.target_port, e);
                    }
                }
            },
            (None, _) => {
                println!("❌ Source node not found: {}", connection.source_node);
            },
            (_, None) => {
                println!("❌ Target node not found: {}", connection.target_node);
            }
        }
    }
    
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TriggerGateRequest {
    pub node_id: String,
}

#[tauri::command]
pub async fn trigger_gate(
    engine: State<'_, AudioEngineState>,
    request: TriggerGateRequest,
) -> Result<(), String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    
    // ADSRノードのgate_inポートに5Vのゲート信号を送信
    match engine.trigger_node_gate(&request.node_id) {
        Ok(()) => {
            println!("🎹 Gate triggered for node: {}", request.node_id);
            Ok(())
        },
        Err(e) => Err(format!("Failed to trigger gate: {}", e))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetOscilloscopeDataRequest {
    pub node_id: String,
}

#[tauri::command]
pub async fn get_oscilloscope_data(
    engine: State<'_, AudioEngineState>,
    request: GetOscilloscopeDataRequest,
) -> Result<OscilloscopeData, String> {
    let engine = engine.inner().lock().map_err(|e| format!("Failed to lock engine: {}", e))?;
    let uuid = Uuid::parse_str(&request.node_id).map_err(|_| "Invalid UUID format".to_string())?;
    
    // ノードインスタンスを取得
    let mut graph = engine.graph.lock().map_err(|e| format!("Failed to lock graph: {}", e))?;
    
    if let Some(node) = graph.get_node_mut(&uuid.to_string()) {
        if let Some(osc_node) = node.as_any_mut().downcast_mut::<crate::nodes::OscilloscopeNode>() {
            // 波形データ取得
            let waveform_data = osc_node.get_display_data();
            let measurements_data = osc_node.get_measurements();
            
            let waveform = waveform_data.to_vec();
            
            let measurements = MeasurementData {
                vpp: measurements_data.vpp,
                vrms: measurements_data.vrms,
                frequency: measurements_data.frequency,
                period: measurements_data.period,
                duty_cycle: measurements_data.duty_cycle,
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