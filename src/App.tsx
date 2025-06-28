import React, { useState, useCallback, useEffect } from 'react';
import ReactFlow, {
  Node,
  Edge,
  addEdge,
  Connection,
  useNodesState,
  useEdgesState,
  Controls,
  Background,
  BackgroundVariant,
  MiniMap,
} from 'reactflow';
import 'reactflow/dist/style.css';

import { invoke } from '@tauri-apps/api/core';
import OscillatorNode from './components/OscillatorNode';
import OutputNode from './components/OutputNode';
import OscilloscopeNode from './components/OscilloscopeNode';
import Toolbar from './components/Toolbar';
import ParameterPanel from './components/ParameterPanel';

const nodeTypes = {
  oscillator: OscillatorNode,
  output: OutputNode,
  oscilloscope: OscilloscopeNode,
  sine_oscillator: OscillatorNode,
  triangle_oscillator: OscillatorNode,
  sawtooth_oscillator: OscillatorNode,
  pulse_oscillator: OscillatorNode,
};

interface NodeInfo {
  id: string;
  name: string;
  node_type: string;
  parameters: Record<string, number>;
  input_ports: Array<{ name: string; port_type: string }>;
  output_ports: Array<{ name: string; port_type: string }>;
}

interface ConnectionInfo {
  source_node: string;
  source_port: string;
  target_node: string;
  target_port: string;
}

function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [isAudioEngineRunning, setIsAudioEngineRunning] = useState(false);
  const [statusMessage, setStatusMessage] = useState('Initializing...');
  const [tauriReady, setTauriReady] = useState(false);

  // Load nodes and connections from Rust backend
  const loadGraph = useCallback(async () => {
    try {
      const nodeInfos: NodeInfo[] = await invoke('list_nodes');
      const connections: ConnectionInfo[] = await invoke('get_connections');
      
      // Convert Rust nodes to ReactFlow nodes
      const flowNodes: Node[] = nodeInfos.map((nodeInfo, index) => ({
        id: nodeInfo.id,
        type: nodeInfo.node_type,
        position: { x: 100 + (index % 3) * 200, y: 100 + Math.floor(index / 3) * 150 },
        data: {
          label: nodeInfo.name,
          nodeType: nodeInfo.node_type,
          parameters: nodeInfo.parameters,
          inputPorts: nodeInfo.input_ports,
          outputPorts: nodeInfo.output_ports,
        },
      }));

      // Convert Rust connections to ReactFlow edges
      const flowEdges: Edge[] = connections.map((conn, index) => ({
        id: `e${index}`,
        source: conn.source_node,
        target: conn.target_node,
        sourceHandle: conn.source_port,
        targetHandle: conn.target_port,
        label: `${conn.source_port} → ${conn.target_port}`,
        labelStyle: { fontSize: 10 },
      }));

      setNodes(flowNodes);
      setEdges(flowEdges);
      setStatusMessage(`Loaded ${flowNodes.length} nodes, ${flowEdges.length} connections`);
    } catch (error) {
      console.error('Failed to load graph:', error);
      setStatusMessage(`Error: ${error}`);
    }
  }, [setNodes, setEdges]);

  // Handle new connections
  const onConnect = useCallback(
    async (params: Connection) => {
      if (!params.source || !params.target || !params.sourceHandle || !params.targetHandle) {
        return;
      }

      try {
        await invoke('connect_nodes', {
          sourceNode: params.source,
          sourcePort: params.sourceHandle,
          targetNode: params.target,
          targetPort: params.targetHandle,
        });
        
        setEdges((eds) => addEdge({
          ...params,
          id: `e${eds.length}`,
          label: `${params.sourceHandle} → ${params.targetHandle}`,
          labelStyle: { fontSize: 10 },
        }, eds));
        
        setStatusMessage('Connection created');
      } catch (error) {
        console.error('Failed to connect nodes:', error);
        setStatusMessage(`Connection failed: ${error}`);
      }
    },
    [setEdges]
  );

  // Handle node selection
  const onNodeClick = useCallback((_event: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
  }, []);

  // Create new node
  const createNode = useCallback(async (nodeType: string, name: string) => {
    try {
      await invoke('create_node', {
        nodeType,
        name,
      });
      
      await loadGraph(); // Reload to get updated node info
      setStatusMessage(`Created ${nodeType} node: ${name}`);
    } catch (error) {
      console.error('Failed to create node:', error);
      setStatusMessage(`Create failed: ${error}`);
    }
  }, [loadGraph]);

  // Remove selected node
  const removeNode = useCallback(async () => {
    if (!selectedNode) return;
    
    try {
      await invoke('remove_node', {
        nodeId: selectedNode.id,
      });
      
      setNodes((nds) => nds.filter((n) => n.id !== selectedNode.id));
      setEdges((eds) => eds.filter((e) => e.source !== selectedNode.id && e.target !== selectedNode.id));
      setSelectedNode(null);
      setStatusMessage(`Removed node: ${selectedNode.data?.label}`);
    } catch (error) {
      console.error('Failed to remove node:', error);
      setStatusMessage(`Remove failed: ${error}`);
    }
  }, [selectedNode, setNodes, setEdges]);

  // Update node parameter
  const updateParameter = useCallback(async (nodeId: string, param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        nodeId,
        param,
        value,
      });
      
      // Update local node data
      setNodes((nds) =>
        nds.map((node) =>
          node.id === nodeId
            ? {
                ...node,
                data: {
                  ...node.data,
                  parameters: {
                    ...node.data.parameters,
                    [param]: value,
                  },
                },
              }
            : node
        )
      );
      
      setStatusMessage(`Set ${param} = ${value}`);
    } catch (error) {
      console.error('Failed to update parameter:', error);
      setStatusMessage(`Update failed: ${error}`);
    }
  }, [setNodes]);

  // Audio engine control
  const toggleAudioEngine = useCallback(async () => {
    try {
      if (isAudioEngineRunning) {
        await invoke('stop_audio');
        setIsAudioEngineRunning(false);
        setStatusMessage('Audio engine stopped');
      } else {
        await invoke('start_audio');
        setIsAudioEngineRunning(true);
        setStatusMessage('Audio engine started');
      }
    } catch (error) {
      console.error('Audio engine control failed:', error);
      setStatusMessage(`Audio engine error: ${error}`);
    }
  }, [isAudioEngineRunning]);

  // Save/Load project
  const saveProject = useCallback(async () => {
    try {
      await invoke('save_project', {
        filename: 'project.json',
      });
      setStatusMessage('Project saved');
    } catch (error) {
      console.error('Save failed:', error);
      setStatusMessage(`Save failed: ${error}`);
    }
  }, []);

  const loadProject = useCallback(async () => {
    try {
      await invoke('load_project', {
        filename: 'project.json',
      });
      await loadGraph();
      setStatusMessage('Project loaded');
    } catch (error) {
      console.error('Load failed:', error);
      setStatusMessage(`Load failed: ${error}`);
    }
  }, [loadGraph]);

  // Initial load
  useEffect(() => {
    const initApp = async () => {
      try {
        setStatusMessage('Connecting to Tauri...');
        await loadGraph();
        setTauriReady(true);
        setStatusMessage('Ready');
      } catch (error) {
        console.error('Failed to initialize app:', error);
        setStatusMessage('Connection failed - check console');
        setTauriReady(false);
      }
    };
    
    initApp();
  }, [loadGraph]);

  // Check audio engine status periodically
  useEffect(() => {
    const checkAudioStatus = async () => {
      try {
        const running: boolean = await invoke('is_audio_running');
        setIsAudioEngineRunning(running);
      } catch (error) {
        console.error('Failed to check audio status:', error);
      }
    };

    const interval = setInterval(checkAudioStatus, 1000);
    return () => clearInterval(interval);
  }, []);

  if (!tauriReady) {
    return (
      <div style={{ 
        width: '100vw', 
        height: '100vh', 
        display: 'flex', 
        flexDirection: 'column',
        justifyContent: 'center', 
        alignItems: 'center',
        fontSize: '18px',
        color: '#333'
      }}>
        <div style={{ marginBottom: '20px' }}>🎵 Orbital Modulator</div>
        <div style={{ fontSize: '14px', opacity: 0.7 }}>{statusMessage}</div>
      </div>
    );
  }

  return (
    <div style={{ width: '100vw', height: '100vh' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClick}
        nodeTypes={nodeTypes}
        fitView
      >
        <Controls />
        <MiniMap />
        <Background variant={BackgroundVariant.Dots} gap={12} size={1} />
      </ReactFlow>

      <Toolbar
        onCreateNode={createNode}
        onRemoveNode={removeNode}
        onToggleAudioEngine={toggleAudioEngine}
        onSave={saveProject}
        onLoad={loadProject}
        isAudioEngineRunning={isAudioEngineRunning}
        hasSelectedNode={!!selectedNode}
      />

      {selectedNode && (
        <ParameterPanel
          node={selectedNode}
          onUpdateParameter={updateParameter}
          onClose={() => setSelectedNode(null)}
        />
      )}

      <div className="status-bar">
        {statusMessage} | Engine: {isAudioEngineRunning ? 'Running' : 'Stopped'} | Nodes: {nodes.length} | Connections: {edges.length}
      </div>
    </div>
  );
}

export default App;