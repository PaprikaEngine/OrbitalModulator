use crate::audio::AudioEngine;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

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