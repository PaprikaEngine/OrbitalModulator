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
import './styles.css';
import './styles/oscilloscope.css';
import './styles/eurorack.css';

import { invoke } from '@tauri-apps/api/core';
import OscillatorNode from './components/OscillatorNode';
import OutputNode from './components/OutputNode';
import GenericNode from './components/GenericNode';
import OscilloscopeNode from './components/OscilloscopeNode';
import FilterNode from './components/FilterNode';
import ADSRNode from './components/ADSRNode';
import LFONode from './components/LFONode';
import MixerNode from './components/MixerNode';
import DelayNode from './components/DelayNode';
import VCONode from './components/VCONode';
import VCFNode from './components/VCFNode';
import EurorackADSRNode from './components/EurorackADSRNode';
import EurorackLFONode from './components/EurorackLFONode';
import NoiseNode from './components/NoiseNode';
import VCANode from './components/VCANode';
import SequencerNode from './components/SequencerNode';
import SpectrumAnalyzerNode from './components/SpectrumAnalyzerNode';
import RingModulatorNode from './components/RingModulatorNode';
import SampleHoldNode from './components/SampleHoldNode';
import AttenuverterNode from './components/AttenuverterNode';
import MultipleNode from './components/MultipleNode';
import Toolbar from './components/Toolbar';
import ParameterPanel from './components/ParameterPanel';

const nodeTypes = {
  // Generator Nodes - Eurorack Style
  oscillator: VCONode,
  sine_oscillator: GenericNode,
  triangle_oscillator: GenericNode,
  sawtooth_oscillator: GenericNode,
  pulse_oscillator: GenericNode,
  noise: GenericNode,
  
  // Processor Nodes - Eurorack Style  
  vcf: VCFNode,
  filter: VCFNode, // Alias for vcf
  vca: GenericNode,
  delay: GenericNode,
  compressor: GenericNode,
  waveshaper: GenericNode,
  ring_modulator: GenericNode,
  
  // Controller Nodes - Eurorack Style
  adsr: EurorackADSRNode,
  lfo: EurorackLFONode,
  sequencer: GenericNode,
  
  // Utility Nodes
  sample_hold: GenericNode,
  quantizer: GenericNode,
  attenuverter: GenericNode,
  multiple: GenericNode,
  multiple8: GenericNode,
  clock_divider: GenericNode,
  
  // Mixing/Routing Nodes
  mixer: GenericNode,
  mixer8: GenericNode,
  output: GenericNode,
  
  // Analysis Nodes
  oscilloscope: OscilloscopeNode,
  spectrum_analyzer: GenericNode,
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

// Cable color mapping based on port types
const getCableColor = (sourcePort: string, targetPort: string): string => {
  // Audio connections (red spectrum)
  if (sourcePort.includes('audio') || targetPort.includes('audio') ||
      sourcePort.includes('out') || targetPort.includes('in')) {
    return '#ff4444'; // Red for audio
  }
  
  // CV connections (blue spectrum)
  if (sourcePort.includes('cv') || targetPort.includes('cv') ||
      sourcePort.includes('control') || targetPort.includes('control')) {
    return '#4444ff'; // Blue for CV
  }
  
  // Gate/Trigger connections (green spectrum)
  if (sourcePort.includes('gate') || targetPort.includes('gate') ||
      sourcePort.includes('trigger') || targetPort.includes('trigger')) {
    return '#44ff44'; // Green for gates/triggers
  }
  
  // Clock connections (orange spectrum)
  if (sourcePort.includes('clock') || targetPort.includes('clock')) {
    return '#ff8844'; // Orange for clock
  }
  
  // Frequency/Pitch connections (purple spectrum)
  if (sourcePort.includes('frequency') || targetPort.includes('frequency') ||
      sourcePort.includes('pitch') || targetPort.includes('pitch')) {
    return '#8844ff'; // Purple for frequency/pitch
  }
  
  // Default color (gray)
  return '#888888';
};

// Cable styling based on connection type
const getCableStyle = (sourcePort: string, targetPort: string) => ({
  stroke: getCableColor(sourcePort, targetPort),
  strokeWidth: 3,
  strokeDasharray: sourcePort.includes('cv') || targetPort.includes('cv') ? '5,5' : undefined,
});

function App() {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [isAudioEngineRunning, setIsAudioEngineRunning] = useState(false);
  const [statusMessage, setStatusMessage] = useState('Initializing...');
  const [tauriReady, setTauriReady] = useState(false);

  // Trigger gate for ADSR nodes
  const triggerGate = useCallback(async (nodeId: string) => {
    try {
      await invoke('trigger_gate', { request: { node_id: nodeId } });
      setStatusMessage(`Gate triggered for node: ${nodeId}`);
    } catch (error) {
      console.error('Failed to trigger gate:', error);
      setStatusMessage(`Trigger failed: ${error}`);
    }
  }, []);

  // Node drag event handlers
  const onNodeDragStart = useCallback((event: React.MouseEvent, node: Node) => {
    console.log('Node drag started:', node.id);
  }, []);

  const onNodeDrag = useCallback((event: React.MouseEvent, node: Node) => {
    // Optional: Add debug logging if needed
  }, []);

  const onNodeDragStop = useCallback((event: React.MouseEvent, node: Node) => {
    console.log('Node drag stopped:', node.id);
  }, []);

  // Global mouse event handlers to ensure drag state is properly reset
  useEffect(() => {
    const handleGlobalMouseUp = (event: MouseEvent) => {
      // Force release any pointer capture
      if (document.body.style.cursor === 'grabbing') {
        document.body.style.cursor = '';
      }
      
      // Clear any ReactFlow drag states by triggering a re-render
      setStatusMessage(prev => prev); // Trigger state update
    };

    const handleGlobalMouseLeave = () => {
      handleGlobalMouseUp({} as MouseEvent);
    };

    document.addEventListener('mouseup', handleGlobalMouseUp, true); // Use capture phase
    document.addEventListener('mouseleave', handleGlobalMouseLeave);
    window.addEventListener('blur', handleGlobalMouseLeave);

    return () => {
      document.removeEventListener('mouseup', handleGlobalMouseUp, true);
      document.removeEventListener('mouseleave', handleGlobalMouseLeave);
      window.removeEventListener('blur', handleGlobalMouseLeave);
    };
  }, []);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setSelectedNode(null);
        console.log('Node selection cleared by Escape key');
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);

  // Load nodes and connections from Rust backend
  const loadGraph = useCallback(async () => {
    try {
      const nodeInfos: NodeInfo[] = await invoke('list_nodes');
      const connections: ConnectionInfo[] = await invoke('get_connections');
      
      // Convert Rust nodes to ReactFlow nodes
      const flowNodes: Node[] = nodeInfos.map((nodeInfo, index) => {
        console.log('Creating node:', nodeInfo);
        return {
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
        };
      });

      // Convert Rust connections to ReactFlow edges
      const flowEdges: Edge[] = connections.map((conn) => ({
        id: `${conn.source_node}:${conn.source_port}->${conn.target_node}:${conn.target_port}`,
        source: conn.source_node,
        target: conn.target_node,
        sourceHandle: conn.source_port,
        targetHandle: conn.target_port,
        style: getCableStyle(conn.source_port, conn.target_port),
        animated: conn.source_port.includes('clock') || conn.target_port.includes('clock'),
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
        const connectionParams = {
          source_node: params.source,
          source_port: params.sourceHandle,
          target_node: params.target,
          target_port: params.targetHandle,
        };
        console.log('Connecting with params:', connectionParams);
        await invoke('connect_nodes', { request: connectionParams });
        
        setEdges((eds) => addEdge({
          ...params,
          id: `${params.source}:${params.sourceHandle}->${params.target}:${params.targetHandle}`,
          style: getCableStyle(params.sourceHandle || '', params.targetHandle || ''),
          animated: (params.sourceHandle?.includes('clock') || params.targetHandle?.includes('clock')) || false,
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
  const onNodeClick = useCallback((event: React.MouseEvent, node: Node) => {
    // Prevent event from interfering with other interactions
    event.stopPropagation();
    
    // Only select node if not already performing other operations
    if (!event.defaultPrevented) {
      setSelectedNode(node);
      console.log('Node selected:', node.id);
    }
  }, []);

  // Handle pane (background) click to deselect nodes
  const onPaneClick = useCallback((event: React.MouseEvent) => {
    // Clear selection when clicking on background
    setSelectedNode(null);
    console.log('Node selection cleared');
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

  // Handle edge double click (delete on double click)
  const onEdgeDoubleClick = useCallback(async (_event: React.MouseEvent, edge: Edge) => {
    try {
      // Parse the edge ID to get source and target information
      // Edge ID format: "sourceNodeId:sourcePort->targetNodeId:targetPort"
      const [sourceInfo, targetInfo] = edge.id.split('->');
      const [sourceNodeId, sourcePort] = sourceInfo.split(':');
      const [targetNodeId, targetPort] = targetInfo.split(':');

      // Call Tauri to disconnect the nodes
      const disconnectParams = {
        source_node: sourceNodeId,
        source_port: sourcePort,
        target_node: targetNodeId,
        target_port: targetPort,
      };
      console.log('Disconnecting with params:', disconnectParams);
      await invoke('disconnect_nodes', { request: disconnectParams });

      // Update local edges state
      setEdges((eds) => eds.filter((e) => e.id !== edge.id));
      
      setStatusMessage(`Disconnected ${sourceNodeId}:${sourcePort} from ${targetNodeId}:${targetPort}`);
    } catch (error) {
      console.error('Failed to disconnect nodes:', error);
      setStatusMessage(`Failed to disconnect: ${error}`);
    }
  }, [setEdges]);

  // Handle edge deletion
  const onEdgesDelete = useCallback(async (deletedEdges: Edge[]) => {
    for (const edge of deletedEdges) {
      try {
        // Parse the edge ID to get source and target information
        // Edge ID format: "sourceNodeId:sourcePort->targetNodeId:targetPort"
        const [sourceInfo, targetInfo] = edge.id.split('->');
        const [sourceNodeId, sourcePort] = sourceInfo.split(':');
        const [targetNodeId, targetPort] = targetInfo.split(':');

        // Call Tauri to disconnect the nodes
        const disconnectParams = {
          source_node: sourceNodeId,
          source_port: sourcePort,
          target_node: targetNodeId,
          target_port: targetPort,
        };
        console.log('Disconnecting (delete) with params:', disconnectParams);
        await invoke('disconnect_nodes', { request: disconnectParams });

        setStatusMessage(`Disconnected ${sourceNodeId}:${sourcePort} from ${targetNodeId}:${targetPort}`);
      } catch (error) {
        console.error('Failed to disconnect nodes:', error);
        setStatusMessage(`Failed to disconnect: ${error}`);
      }
    }

    // Update local edges state
    setEdges((eds) => eds.filter((e) => !deletedEdges.some((deleted) => deleted.id === e.id)));
  }, [setEdges]);

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
      // Open file dialog to select save location
      const { save } = await import('@tauri-apps/plugin-dialog');
      
      const filePath = await save({
        title: 'Save Patch File',
        defaultPath: 'my_patch.json',
        filters: [
          {
            name: 'JSON Patch Files',
            extensions: ['json']
          }
        ]
      });

      if (filePath) {
        // Prepare node positions from ReactFlow
        const nodePositions: Record<string, {x: number, y: number}> = {};
        nodes.forEach(node => {
          nodePositions[node.data?.label || node.id] = {
            x: node.position.x,
            y: node.position.y
          };
        });

        // Prepare connection states from ReactFlow edges
        const connectionStates = edges.map(edge => ({
          source_node: edge.source,
          source_port: edge.sourceHandle,
          target_node: edge.target,
          target_port: edge.targetHandle,
          id: edge.id,
          style: edge.style || {},
          animated: edge.animated || false
        }));

        // Save the current patch configuration
        await invoke('save_patch_file', {
          filePath: filePath,
          patchName: 'My Patch',
          description: 'Patch created with Orbital Modulator',
          nodePositions: nodePositions,
          connectionStates: connectionStates
        });
        
        setStatusMessage(`Patch saved: ${filePath.split('/').pop() || 'file'}`);
      }
    } catch (error) {
      console.error('Save failed:', error);
      setStatusMessage(`Save failed: ${error}`);
    }
  }, [nodes, edges]);

  const loadProject = useCallback(async () => {
    try {
      // Open file dialog to select patch file
      const { open } = await import('@tauri-apps/plugin-dialog');
      
      const selected = await open({
        title: 'Load Patch File',
        filters: [
          {
            name: 'JSON Patch Files',
            extensions: ['json']
          },
          {
            name: 'All Files',
            extensions: ['*']
          }
        ],
        multiple: false,
      });

      if (selected) {
        // Load the selected patch file
        await invoke('load_patch_file', {
          filePath: selected,
        });
        
        await loadGraph(); // Refresh the graph view
        setStatusMessage(`Patch loaded: ${selected.split('/').pop() || 'file'}`);
      }
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
        onPaneClick={onPaneClick}
        onNodeDragStart={onNodeDragStart}
        onNodeDrag={onNodeDrag}
        onNodeDragStop={onNodeDragStop}
        onEdgesDelete={onEdgesDelete}
        onEdgeDoubleClick={onEdgeDoubleClick}
        nodeTypes={nodeTypes}
        nodesDraggable={true}
        nodesConnectable={true}
        elementsSelectable={true}
        selectNodesOnDrag={false}
        multiSelectionKeyCode={null}
        panOnDrag={true}
        nodeDragHandle=".drag-handle"
        preventScrolling={false}
        fitView
        deleteKeyCode="Delete"
        nodeDragThreshold={5}
        nodeOrigin={[0.5, 0]}
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
          onTriggerGate={triggerGate}
          onClose={() => setSelectedNode(null)}
        />
      )}

      <div className="status-bar">
        {statusMessage} | Engine: {isAudioEngineRunning ? 'Running' : 'Stopped'} | Nodes: {nodes.length} | Connections: {edges.length} | Tip: Double-click connection to disconnect
      </div>

      {/* Cable Color Legend */}
      <div className="cable-legend">
        <div className="legend-title">Cable Colors:</div>
        <div className="legend-items">
          <div className="legend-item">
            <div className="legend-color" style={{ backgroundColor: '#ff4444' }}></div>
            <span>Audio</span>
          </div>
          <div className="legend-item">
            <div className="legend-color" style={{ backgroundColor: '#4444ff' }}></div>
            <span>CV</span>
          </div>
          <div className="legend-item">
            <div className="legend-color" style={{ backgroundColor: '#44ff44' }}></div>
            <span>Gate/Trigger</span>
          </div>
          <div className="legend-item">
            <div className="legend-color" style={{ backgroundColor: '#ff8844' }}></div>
            <span>Clock</span>
          </div>
          <div className="legend-item">
            <div className="legend-color" style={{ backgroundColor: '#8844ff' }}></div>
            <span>Frequency</span>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;