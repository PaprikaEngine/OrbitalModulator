use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "orbital-modulator")]
#[command(about = "A modular synthesizer with node-based architecture")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new node
    Create {
        /// Type of node to create (oscillator, output, etc.)
        node_type: String,
        /// Name for the node
        name: String,
    },
    /// Remove a node
    Remove {
        /// Name of the node to remove
        name: String,
    },
    /// Connect two nodes
    Connect {
        /// Source node and port (format: node_name:port_name)
        source: String,
        /// Target node and port (format: node_name:port_name)
        target: String,
    },
    /// Disconnect two nodes
    Disconnect {
        /// Source node and port (format: node_name:port_name)
        source: String,
        /// Target node and port (format: node_name:port_name)
        target: String,
    },
    /// Connect two nodes by ID
    ConnectById {
        /// Source node ID (UUID)
        source_id: String,
        /// Source port name
        source_port: String,
        /// Target node ID (UUID)
        target_id: String,
        /// Target port name
        target_port: String,
    },
    /// Disconnect two nodes by ID
    DisconnectById {
        /// Source node ID (UUID)
        source_id: String,
        /// Source port name
        source_port: String,
        /// Target node ID (UUID)
        target_id: String,
        /// Target port name
        target_port: String,
    },
    /// Set a node parameter
    Set {
        /// Node name
        node: String,
        /// Parameter name
        param: String,
        /// Parameter value
        value: f32,
    },
    /// Set a node parameter by ID
    SetById {
        /// Node ID (UUID)
        id: String,
        /// Parameter name
        param: String,
        /// Parameter value
        value: f32,
    },
    /// Get a node parameter
    Get {
        /// Node name
        node: String,
        /// Parameter name
        param: String,
    },
    /// Get a node parameter by ID
    GetById {
        /// Node ID (UUID)
        id: String,
        /// Parameter name
        param: String,
    },
    /// List all nodes
    List,
    /// Show detailed information about a node
    Info {
        /// Node name
        name: String,
    },
    /// Display the node graph
    Graph,
    /// Display the node tree structure
    Tree,
    /// Start audio playback
    Play,
    /// Stop audio playback
    Stop,
    /// Save current configuration
    Save {
        /// Filename to save to
        filename: String,
    },
    /// Load configuration
    Load {
        /// Filename to load from
        filename: String,
    },
    /// Run demo with sine oscillator connected to output
    Demo,
    /// Run interactive mode for dynamic control
    Interactive,
}

pub fn parse_node_port(input: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid format '{}'. Expected 'node_name:port_name'", input));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}