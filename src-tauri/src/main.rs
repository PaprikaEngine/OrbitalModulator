#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex};
use tauri::Manager;

use orbital_modulator::audio::AudioEngine;
use orbital_modulator::tauri_commands::*;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Initialize audio engine
            let sample_rate = 44100.0;
            let buffer_size = 512;
            
            let audio_engine = AudioEngine::new(sample_rate, buffer_size)
                .map_err(|e| format!("Failed to create audio engine: {}", e))?;
            
            let engine_state = Arc::new(Mutex::new(audio_engine));
            app.manage(engine_state);
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_node,
            remove_node,
            connect_nodes,
            disconnect_nodes,
            set_node_parameter,
            get_node_parameter,
            list_nodes,
            get_connections,
            start_audio,
            stop_audio,
            is_audio_running,
            save_project,
            load_project,
            get_oscilloscope_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}