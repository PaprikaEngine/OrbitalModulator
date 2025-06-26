mod graph;
mod nodes;
mod audio;
mod cli;

use clap::Parser;
use cli::{Cli, Commands, parse_node_port};
use audio::AudioEngine;
use std::collections::HashMap;
use uuid::Uuid;

struct Application {
    audio_engine: AudioEngine,
    node_name_to_id: HashMap<String, Uuid>,
}

impl Application {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let audio_engine = AudioEngine::new(44100.0, 512)?;
        
        Ok(Self {
            audio_engine,
            node_name_to_id: HashMap::new(),
        })
    }

    fn handle_command(&mut self, command: Commands) -> Result<(), Box<dyn std::error::Error>> {
        match command {
            Commands::Create { node_type, name } => {
                if self.node_name_to_id.contains_key(&name) {
                    eprintln!("Error: Node '{}' already exists", name);
                    return Ok(());
                }

                match self.audio_engine.create_node(&node_type, name.clone()) {
                    Ok(id) => {
                        self.node_name_to_id.insert(name.clone(), id);
                        println!("Created {} node '{}'", node_type, name);
                    },
                    Err(e) => eprintln!("Error creating node: {}", e),
                }
            },

            Commands::Remove { name } => {
                if let Some(&node_id) = self.node_name_to_id.get(&name) {
                    match self.audio_engine.remove_node(node_id) {
                        Ok(_) => {
                            self.node_name_to_id.remove(&name);
                            println!("Removed node '{}'", name);
                        },
                        Err(e) => eprintln!("Error removing node: {}", e),
                    }
                } else {
                    eprintln!("Error: Node '{}' not found", name);
                }
            },

            Commands::Connect { source, target } => {
                let (source_node, source_port) = parse_node_port(&source)?;
                let (target_node, target_port) = parse_node_port(&target)?;

                let source_id = self.node_name_to_id.get(&source_node)
                    .ok_or(format!("Source node '{}' not found", source_node))?;
                let target_id = self.node_name_to_id.get(&target_node)
                    .ok_or(format!("Target node '{}' not found", target_node))?;

                match self.audio_engine.connect_nodes(*source_id, &source_port, *target_id, &target_port) {
                    Ok(_) => println!("Connected {}:{} -> {}:{}", source_node, source_port, target_node, target_port),
                    Err(e) => eprintln!("Error connecting nodes: {}", e),
                }
            },

            Commands::Disconnect { source, target } => {
                let (source_node, source_port) = parse_node_port(&source)?;
                let (target_node, target_port) = parse_node_port(&target)?;

                let source_id = self.node_name_to_id.get(&source_node)
                    .ok_or(format!("Source node '{}' not found", source_node))?;
                let target_id = self.node_name_to_id.get(&target_node)
                    .ok_or(format!("Target node '{}' not found", target_node))?;

                match self.audio_engine.disconnect_nodes(*source_id, &source_port, *target_id, &target_port) {
                    Ok(_) => println!("Disconnected {}:{} -> {}:{}", source_node, source_port, target_node, target_port),
                    Err(e) => eprintln!("Error disconnecting nodes: {}", e),
                }
            },

            Commands::ConnectById { source_id, source_port, target_id, target_port } => {
                match self.audio_engine.connect_nodes_by_id(&source_id, &source_port, &target_id, &target_port) {
                    Ok(_) => println!("Connected {}:{} -> {}:{}", source_id, source_port, target_id, target_port),
                    Err(e) => eprintln!("Error connecting nodes: {}", e),
                }
            },

            Commands::DisconnectById { source_id, source_port, target_id, target_port } => {
                match self.audio_engine.disconnect_nodes_by_id(&source_id, &source_port, &target_id, &target_port) {
                    Ok(_) => println!("Disconnected {}:{} -> {}:{}", source_id, source_port, target_id, target_port),
                    Err(e) => eprintln!("Error disconnecting nodes: {}", e),
                }
            },

            Commands::Set { node, param, value } => {
                if let Some(&node_id) = self.node_name_to_id.get(&node) {
                    match self.audio_engine.set_node_parameter(node_id, &param, value) {
                        Ok(_) => println!("Set {}.{} = {}", node, param, value),
                        Err(e) => eprintln!("Error setting parameter: {}", e),
                    }
                } else {
                    eprintln!("Error: Node '{}' not found", node);
                }
            },

            Commands::Get { node, param } => {
                if let Some(&node_id) = self.node_name_to_id.get(&node) {
                    match self.audio_engine.get_node_parameter(node_id, &param) {
                        Ok(value) => println!("{}.{} = {}", node, param, value),
                        Err(e) => eprintln!("Error getting parameter: {}", e),
                    }
                } else {
                    eprintln!("Error: Node '{}' not found", node);
                }
            },

            Commands::SetById { id, param, value } => {
                match self.audio_engine.set_node_parameter_by_id(&id, &param, value) {
                    Ok(_) => println!("Set {}.{} = {}", id, param, value),
                    Err(e) => eprintln!("Error setting parameter: {}", e),
                }
            },

            Commands::GetById { id, param } => {
                match self.audio_engine.get_node_parameter_by_id(&id, &param) {
                    Ok(value) => println!("{}.{} = {}", id, param, value),
                    Err(e) => eprintln!("Error getting parameter: {}", e),
                }
            },

            Commands::List => {
                let nodes = self.audio_engine.list_nodes();
                if nodes.is_empty() {
                    println!("No nodes found");
                } else {
                    println!("Nodes:");
                    for (id, name, node_type) in nodes {
                        println!("  {} ({}) - {}", name, node_type, id);
                    }
                }
            },

            Commands::Info { name } => {
                if let Some(&node_id) = self.node_name_to_id.get(&name) {
                    if let Some(node_info) = self.audio_engine.get_node_info(node_id) {
                        println!("Node: {} ({})", node_info.name, node_info.node_type);
                        println!("ID: {}", node_info.id);
                        
                        if !node_info.input_ports.is_empty() {
                            println!("Input Ports:");
                            for port in &node_info.input_ports {
                                println!("  {} ({:?})", port.name, port.port_type);
                            }
                        }

                        if !node_info.output_ports.is_empty() {
                            println!("Output Ports:");
                            for port in &node_info.output_ports {
                                println!("  {} ({:?})", port.name, port.port_type);
                            }
                        }

                        if !node_info.parameters.is_empty() {
                            println!("Parameters:");
                            for (param, value) in &node_info.parameters {
                                println!("  {} = {}", param, value);
                            }
                        }
                    }
                } else {
                    eprintln!("Error: Node '{}' not found", name);
                }
            },

            Commands::Graph => {
                let graph_viz = self.audio_engine.get_graph_visualization();
                println!("{}", graph_viz);
            },

            Commands::Tree => {
                let tree_viz = self.audio_engine.get_node_tree();
                println!("{}", tree_viz);
            },

            Commands::Play => {
                match self.audio_engine.start() {
                    Ok(_) => println!("Audio playback started"),
                    Err(e) => eprintln!("Error starting audio: {}", e),
                }
            },

            Commands::Stop => {
                self.audio_engine.stop();
                println!("Audio playback stopped");
            },

            Commands::Save { filename } => {
                match self.audio_engine.save_to_file(&filename) {
                    Ok(_) => println!("Successfully saved to {}", filename),
                    Err(e) => eprintln!("Error saving: {}", e),
                }
            },

            Commands::Load { filename } => {
                match self.audio_engine.load_from_file(&filename) {
                    Ok(_) => println!("Successfully loaded from {}", filename),
                    Err(e) => eprintln!("Error loading: {}", e),
                }
            },

            Commands::Demo => {
                println!("Setting up demo: Sine Oscillator -> Output");
                
                // Create sine oscillator
                let osc_id = match self.audio_engine.create_node("sine_oscillator", "demo_osc".to_string()) {
                    Ok(id) => {
                        self.node_name_to_id.insert("demo_osc".to_string(), id);
                        println!("Created sine oscillator 'demo_osc' (ID: {})", id);
                        id
                    },
                    Err(e) => {
                        eprintln!("Error creating oscillator: {}", e);
                        return Ok(());
                    }
                };

                // Create output
                let out_id = match self.audio_engine.create_node("output", "demo_out".to_string()) {
                    Ok(id) => {
                        self.node_name_to_id.insert("demo_out".to_string(), id);
                        println!("Created output 'demo_out' (ID: {})", id);
                        id
                    },
                    Err(e) => {
                        eprintln!("Error creating output: {}", e);
                        return Ok(());
                    }
                };

                // Connect oscillator to output
                match self.audio_engine.connect_nodes(osc_id, "audio_out", out_id, "audio_in_l") {
                    Ok(_) => println!("Connected demo_osc:audio_out -> demo_out:audio_in_l"),
                    Err(e) => eprintln!("Error connecting nodes: {}", e),
                }

                // Also connect to right channel
                match self.audio_engine.connect_nodes(osc_id, "audio_out", out_id, "audio_in_r") {
                    Ok(_) => println!("Connected demo_osc:audio_out -> demo_out:audio_in_r"),
                    Err(e) => eprintln!("Error connecting nodes: {}", e),
                }

                // Set oscillator frequency
                match self.audio_engine.set_node_parameter(osc_id, "frequency", 440.0) {
                    Ok(_) => println!("Set oscillator frequency to 440Hz"),
                    Err(e) => eprintln!("Error setting frequency: {}", e),
                }

                // Set oscillator amplitude
                match self.audio_engine.set_node_parameter(osc_id, "amplitude", 0.3) {
                    Ok(_) => println!("Set oscillator amplitude to 0.3"),
                    Err(e) => eprintln!("Error setting amplitude: {}", e),
                }

                // Show graph
                println!("\n{}", self.audio_engine.get_graph_visualization());

                // Start audio
                match self.audio_engine.start() {
                    Ok(_) => {
                        println!("Demo started! You should hear a 440Hz sine wave.");
                        println!("Use 'set-by-id {} frequency <value>' to change frequency.", osc_id);
                        println!("Press Ctrl+C to stop...");
                    },
                    Err(e) => eprintln!("Error starting audio: {}", e),
                }
            },

            Commands::Interactive => {
                println!("Starting interactive mode...");
                println!("Type 'help' for available commands, 'exit' to quit");
                
                // Create demo setup
                let osc_id = self.audio_engine.create_node("sine_oscillator", "osc".to_string())?;
                let out_id = self.audio_engine.create_node("output", "out".to_string())?;
                self.audio_engine.connect_nodes(osc_id, "audio_out", out_id, "audio_in_l")?;
                self.audio_engine.connect_nodes(osc_id, "audio_out", out_id, "audio_in_r")?;
                
                println!("Created demo setup: Oscillator (ID: {}) -> Output (ID: {})", osc_id, out_id);
                self.audio_engine.start()?;
                
                loop {
                    print!("orbital> ");
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                    
                    let mut input = String::new();
                    if std::io::stdin().read_line(&mut input).is_err() {
                        break;
                    }
                    
                    let input = input.trim();
                    if input.is_empty() {
                        continue;
                    }
                    
                    match input {
                        "exit" | "quit" => break,
                        "help" => {
                            println!("Available commands:");
                            println!("  freq <value>   - Set oscillator frequency (Hz)");
                            println!("  amp <value>    - Set oscillator amplitude (0.0-1.0)");
                            println!("  vol <value>    - Set output volume (0.0-1.0)");
                            println!("  graph          - Show node graph");
                            println!("  tree           - Show node tree");
                            println!("  save <file>    - Save current setup to file");
                            println!("  load <file>    - Load setup from file");
                            println!("  help           - Show this help");
                            println!("  exit           - Exit interactive mode");
                        },
                        "graph" => {
                            println!("{}", self.audio_engine.get_graph_visualization());
                        },
                        "tree" => {
                            println!("{}", self.audio_engine.get_node_tree());
                        },
                        _ if input.starts_with("save ") => {
                            let filename = &input[5..];
                            match self.audio_engine.save_to_file(filename) {
                                Ok(_) => println!("Saved to {}", filename),
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        },
                        _ if input.starts_with("load ") => {
                            let filename = &input[5..];
                            match self.audio_engine.load_from_file(filename) {
                                Ok(_) => println!("Loaded from {}", filename),
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        },
                        _ if input.starts_with("freq ") => {
                            if let Ok(freq) = input[5..].parse::<f32>() {
                                match self.audio_engine.set_node_parameter(osc_id, "frequency", freq) {
                                    Ok(_) => println!("Set frequency to {}Hz", freq),
                                    Err(e) => eprintln!("Error: {}", e),
                                }
                            } else {
                                eprintln!("Invalid frequency value");
                            }
                        },
                        _ if input.starts_with("amp ") => {
                            if let Ok(amp) = input[4..].parse::<f32>() {
                                match self.audio_engine.set_node_parameter(osc_id, "amplitude", amp) {
                                    Ok(_) => println!("Set amplitude to {}", amp),
                                    Err(e) => eprintln!("Error: {}", e),
                                }
                            } else {
                                eprintln!("Invalid amplitude value");
                            }
                        },
                        _ if input.starts_with("vol ") => {
                            if let Ok(vol) = input[4..].parse::<f32>() {
                                match self.audio_engine.set_node_parameter(out_id, "master_volume", vol) {
                                    Ok(_) => println!("Set volume to {}", vol),
                                    Err(e) => eprintln!("Error: {}", e),
                                }
                            } else {
                                eprintln!("Invalid volume value");
                            }
                        },
                        _ => {
                            eprintln!("Unknown command: {}", input);
                            eprintln!("Type 'help' for available commands");
                        }
                    }
                }
                
                self.audio_engine.stop();
                println!("Exiting interactive mode");
            },
        }

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut app = Application::new()?;
    
    app.handle_command(cli.command)?;

    // If audio is running, keep the application alive
    if app.audio_engine.is_running() {
        println!("Press Ctrl+C to stop...");
        
        // Set up Ctrl+C handler
        let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let r = running.clone();
        
        ctrlc::set_handler(move || {
            r.store(false, std::sync::atomic::Ordering::SeqCst);
        }).expect("Error setting Ctrl+C handler");

        while running.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        app.audio_engine.stop();
    }

    Ok(())
}
