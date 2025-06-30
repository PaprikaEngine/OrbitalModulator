/*
 * OrbitalModulator - Professional Modular Synthesizer
 * Copyright (c) 2025 MACHIKO LAB
 *
 * Tauri Commands for Plugin Management
 */

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

use crate::plugin::{
    PluginManager, PluginMetadata, PluginStats, PluginConfig, 
    PluginCategory, PluginSearchCriteria, SecurityViolation
};

/// Plugin info for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUIInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub is_loaded: bool,
    pub node_types: Vec<String>,
    pub tags: Vec<String>,
}

/// Plugin statistics for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginUIStats {
    pub plugin_id: String,
    pub cpu_usage: f32,
    pub memory_usage: usize,
    pub audio_dropouts: u64,
    pub processing_time: f64,
    pub calls_per_second: f64,
    pub last_error: Option<String>,
    pub violations_count: usize,
}

/// Global plugin manager instance
static PLUGIN_MANAGER: Mutex<Option<Arc<Mutex<PluginManager>>>> = Mutex::new(None);

/// Initialize plugin manager
#[tauri::command]
pub fn init_plugin_manager() -> Result<String, String> {
    let mut manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    if manager_opt.is_none() {
        let manager = Arc::new(Mutex::new(PluginManager::new("1.0.0".to_string())));
        *manager_opt = Some(manager);
        Ok("Plugin manager initialized".to_string())
    } else {
        Ok("Plugin manager already initialized".to_string())
    }
}

/// Scan for available plugins
#[tauri::command]
pub fn scan_plugins() -> Result<Vec<PluginUIInfo>, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let available = manager.scan_available_plugins()
        .map_err(|e| format!("Failed to scan plugins: {}", e))?;
    
    let loaded_plugins = manager.list_loaded_plugins();
    
    let ui_info: Vec<PluginUIInfo> = available.into_iter().map(|plugin| {
        PluginUIInfo {
            id: plugin.id.clone(),
            name: plugin.name,
            version: plugin.version,
            description: plugin.description,
            author: plugin.author,
            category: format!("{:?}", plugin.category),
            is_loaded: loaded_plugins.contains(&plugin.id),
            node_types: plugin.node_types,
            tags: plugin.tags,
        }
    }).collect();
    
    Ok(ui_info)
}

/// Load a plugin
#[tauri::command]
pub fn load_plugin(plugin_id: String) -> Result<String, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    manager.load_plugin(&plugin_id)
        .map_err(|e| format!("Failed to load plugin: {}", e))?;
    
    Ok(format!("Plugin {} loaded successfully", plugin_id))
}

/// Unload a plugin
#[tauri::command]
pub fn unload_plugin(plugin_id: String) -> Result<String, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    manager.unload_plugin(&plugin_id)
        .map_err(|e| format!("Failed to unload plugin: {}", e))?;
    
    Ok(format!("Plugin {} unloaded successfully", plugin_id))
}

/// Get plugin statistics
#[tauri::command]
pub fn get_plugin_stats() -> Result<Vec<PluginUIStats>, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let loaded_plugins = manager.list_loaded_plugins();
    let mut stats = Vec::new();
    
    for plugin_id in loaded_plugins {
        if let Some(plugin_stats) = manager.get_plugin_stats(&plugin_id) {
            let violations = manager.get_security_violations(&plugin_id);
            
            stats.push(PluginUIStats {
                plugin_id: plugin_id.clone(),
                cpu_usage: plugin_stats.cpu_usage,
                memory_usage: plugin_stats.memory_usage,
                audio_dropouts: plugin_stats.audio_dropouts,
                processing_time: plugin_stats.processing_time,
                calls_per_second: plugin_stats.calls_per_second,
                last_error: plugin_stats.last_error,
                violations_count: violations.len(),
            });
        }
    }
    
    Ok(stats)
}

/// Search plugins by criteria
#[tauri::command]
pub fn search_plugins(
    category: Option<String>,
    author: Option<String>,
    tags: Vec<String>,
    node_types: Vec<String>,
) -> Result<Vec<PluginUIInfo>, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    // Convert category string to PluginCategory
    let plugin_category = category.map(|cat| {
        match cat.as_str() {
            "Generator" => PluginCategory::Generator,
            "Processor" => PluginCategory::Processor,
            "Controller" => PluginCategory::Controller,
            "Utility" => PluginCategory::Utility,
            "Mixing" => PluginCategory::Mixing,
            "Analyzer" => PluginCategory::Analyzer,
            custom => PluginCategory::Custom(custom.to_string()),
        }
    });
    
    let criteria = PluginSearchCriteria {
        category: plugin_category,
        author,
        tags,
        node_types,
        min_version: None,
        max_version: None,
    };
    
    let results = manager.search_plugins(criteria)
        .map_err(|e| format!("Failed to search plugins: {}", e))?;
    
    let loaded_plugins = manager.list_loaded_plugins();
    
    let ui_info: Vec<PluginUIInfo> = results.into_iter().map(|plugin| {
        PluginUIInfo {
            id: plugin.id.clone(),
            name: plugin.name,
            version: plugin.version,
            description: plugin.description,
            author: plugin.author,
            category: format!("{:?}", plugin.category),
            is_loaded: loaded_plugins.contains(&plugin.id),
            node_types: plugin.node_types,
            tags: plugin.tags,
        }
    }).collect();
    
    Ok(ui_info)
}

/// Get plugin configuration
#[tauri::command]
pub fn get_plugin_config(plugin_id: String) -> Result<PluginConfig, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    manager.get_plugin_config(&plugin_id)
        .ok_or_else(|| format!("Plugin {} not found", plugin_id))
}

/// Configure plugin settings
#[tauri::command]
pub fn configure_plugin(plugin_id: String, config: PluginConfig) -> Result<String, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    manager.configure_plugin(&plugin_id, config)
        .map_err(|e| format!("Failed to configure plugin: {}", e))?;
    
    Ok(format!("Plugin {} configured successfully", plugin_id))
}

/// Get security violations for a plugin
#[tauri::command]
pub fn get_plugin_violations(plugin_id: String) -> Result<Vec<SecurityViolation>, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    Ok(manager.get_security_violations(&plugin_id))
}

/// Auto-disable misbehaving plugins
#[tauri::command]
pub fn auto_disable_plugins() -> Result<Vec<String>, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    Ok(manager.auto_disable_check())
}

/// Get all supported node types from loaded plugins
#[tauri::command]
pub fn get_supported_node_types() -> Result<HashMap<String, Vec<String>>, String> {
    let manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_ref()
        .ok_or("Plugin manager not initialized")?;
    
    let manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    Ok(manager.get_all_supported_node_types())
}

/// Add plugin directory
#[tauri::command]
pub fn add_plugin_directory(path: String) -> Result<String, String> {
    let mut manager_opt = PLUGIN_MANAGER.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    let manager_arc = manager_opt.as_mut()
        .ok_or("Plugin manager not initialized")?;
    
    let mut manager = manager_arc.lock()
        .map_err(|e| format!("Failed to lock plugin manager: {}", e))?;
    
    manager.add_plugin_directory(&path);
    Ok(format!("Added plugin directory: {}", path))
}