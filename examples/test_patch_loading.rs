use orbital_modulator::audio::AudioEngine;
use orbital_modulator::tauri_commands::{PatchFile, PatchNode, PatchConnection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing patch file loading logic...");
    
    // Create audio engine
    let engine = AudioEngine::new()?;
    
    // Read and parse the basic synth patch
    let patch_path = "examples/basic_synth_patch.json";
    println!("\n📂 Loading: {}", patch_path);
    
    let json_content = std::fs::read_to_string(patch_path)?;
    let patch: PatchFile = serde_json::from_str(&json_content)?;
    
    println!("📄 Patch: {:?}", patch.patch_name);
    println!("📊 Nodes: {}", patch.nodes.len());
    println!("🔗 Connections: {}", patch.connections.len());
    
    // Clear current graph
    engine.clear_graph()?;
    
    // Test node creation
    println!("\n🔧 Creating nodes...");
    for patch_node in &patch.nodes {
        println!("  Creating: {} '{}' ({})", patch_node.id, patch_node.name, patch_node.node_type);
        match engine.create_builtin_node(&patch_node.node_type, patch_node.id.clone()) {
            Ok(node_id) => {
                println!("    ✅ Created with UUID: {}", node_id);
                
                // Test parameter setting
                for (param_name, param_value) in &patch_node.parameters {
                    match engine.set_node_parameter(&node_id, param_name, *param_value) {
                        Ok(()) => println!("      ✅ Set {} = {}", param_name, param_value),
                        Err(e) => println!("      ❌ Failed to set {}: {}", param_name, e),
                    }
                }
            },
            Err(e) => {
                println!("    ❌ Failed: {}", e);
                return Err(e.into());
            }
        }
    }
    
    // Test connections
    println!("\n🔗 Testing connections...");
    for connection in &patch.connections {
        println!("  Connecting: {} {} -> {} {}", 
                connection.source_node, connection.source_port,
                connection.target_node, connection.target_port);
        
        // Find node IDs by name
        let source_id = engine.find_node_by_name(&connection.source_node);
        let target_id = engine.find_node_by_name(&connection.target_node);
        
        match (source_id, target_id) {
            (Some(src_id), Some(tgt_id)) => {
                println!("    Found nodes: {} [{}] -> {} [{}]", 
                        connection.source_node, src_id,
                        connection.target_node, tgt_id);
                
                match engine.connect_nodes(
                    &src_id.to_string(),
                    &connection.source_port,
                    &tgt_id.to_string(),
                    &connection.target_port
                ) {
                    Ok(()) => println!("    ✅ Connected successfully!"),
                    Err(e) => println!("    ❌ Connection failed: {}", e),
                }
            },
            (None, _) => println!("    ❌ Source node '{}' not found", connection.source_node),
            (_, None) => println!("    ❌ Target node '{}' not found", connection.target_node),
        }
    }
    
    // Show final results
    let nodes = engine.list_nodes();
    println!("\n📊 Final state: {} nodes created", nodes.len());
    
    println!("\n✅ Test completed!");
    Ok(())
}