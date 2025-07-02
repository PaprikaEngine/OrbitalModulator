/*
 * Test signal flow through the graph to verify audio routing
 */

use orbital_modulator::audio::AudioEngine;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔊 Testing audio signal flow...");
    
    // Create audio engine
    let mut engine = AudioEngine::new()?;
    
    // Create a simple oscillator -> output chain
    println!("🎵 Creating oscillator...");
    let osc_id = engine.create_builtin_node("oscillator", "test_osc".to_string())?;
    
    println!("🔊 Creating output...");
    let output_id = engine.create_builtin_node("output", "test_output".to_string())?;
    
    // Set oscillator parameters for audible tone
    engine.set_node_parameter(&osc_id, "frequency", 440.0)?; // A4 note
    engine.set_node_parameter(&osc_id, "amplitude", 0.3)?;    // Safe volume
    engine.set_node_parameter(&osc_id, "waveform", 0.0)?;     // Sine wave
    engine.set_node_parameter(&osc_id, "active", 1.0)?;       // Active
    
    // Set output parameters
    engine.set_node_parameter(&output_id, "master_volume", 0.5)?;
    engine.set_node_parameter(&output_id, "mute", 0.0)?;       // Not muted
    
    // Connect oscillator to output (stereo)
    println!("🔗 Connecting oscillator to output...");
    engine.connect_nodes(&osc_id, "audio_out", &output_id, "audio_in_l")?;
    engine.connect_nodes(&osc_id, "audio_out", &output_id, "audio_in_r")?;
    
    // Start audio engine
    println!("▶️  Starting audio engine...");
    engine.start()?;
    
    println!("🎶 Playing 440Hz sine wave for 3 seconds...");
    println!("   You should hear an audible tone!");
    
    // Play for 3 seconds
    std::thread::sleep(Duration::from_secs(3));
    
    // Stop audio engine
    println!("⏹️  Stopping audio engine...");
    engine.stop()?;
    
    println!("✅ Signal flow test completed!");
    
    Ok(())
}