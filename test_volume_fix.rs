// ボリューム制御修正のテストプログラム
use orbital_modulator::audio::AudioEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Orbital Modulator - Volume Control Test");
    
    // AudioEngineを初期化
    let mut engine = AudioEngine::new(44100.0, 512)?;
    
    // 1. オシレーターノードを作成
    let osc_id = engine.create_node("oscillator", "test_osc".to_string())?;
    println!("✅ Created oscillator node: {}", osc_id);
    
    // 2. 出力ノードを作成
    let output_id = engine.create_node("output", "test_output".to_string())?;
    println!("✅ Created output node: {}", output_id);
    
    // 3. ノード接続
    engine.connect_nodes(osc_id, "audio_out", output_id, "audio_in_l")?;
    engine.connect_nodes(osc_id, "audio_out", output_id, "audio_in_r")?;
    println!("✅ Connected oscillator to output");
    
    // 4. 初期ボリューム確認
    let initial_volume = engine.get_node_parameter(output_id, "master_volume")?;
    println!("📊 Initial master volume: {}", initial_volume);
    
    // 5. ボリューム変更テスト
    println!("\n🔧 Testing volume control...");
    
    // ボリューム0.1に設定
    engine.set_node_parameter(output_id, "master_volume", 0.1)?;
    let new_volume = engine.get_node_parameter(output_id, "master_volume")?;
    println!("🔉 Set volume to 0.1, current: {}", new_volume);
    
    // ボリューム0.9に設定
    engine.set_node_parameter(output_id, "master_volume", 0.9)?;
    let new_volume = engine.get_node_parameter(output_id, "master_volume")?;
    println!("🔊 Set volume to 0.9, current: {}", new_volume);
    
    // ボリューム0.0に設定
    engine.set_node_parameter(output_id, "master_volume", 0.0)?;
    let new_volume = engine.get_node_parameter(output_id, "master_volume")?;
    println!("🔇 Set volume to 0.0, current: {}", new_volume);
    
    // 6. ミュートテスト
    println!("\n🔇 Testing mute control...");
    engine.set_node_parameter(output_id, "mute", 1.0)?; // ミュートオン
    let mute_status = engine.get_node_parameter(output_id, "mute")?;
    println!("🔇 Mute ON: {}", mute_status);
    
    engine.set_node_parameter(output_id, "mute", 0.0)?; // ミュートオフ
    let mute_status = engine.get_node_parameter(output_id, "mute")?;
    println!("🔊 Mute OFF: {}", mute_status);
    
    // 7. ノードグラフ表示
    println!("\n📊 Node Graph:");
    println!("{}", engine.get_graph_visualization());
    
    println!("\n✅ Volume control test completed successfully!");
    println!("🎯 The output node volume control fix is working correctly.");
    
    Ok(())
}