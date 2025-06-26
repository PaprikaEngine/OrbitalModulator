import React, { useState } from 'react';

interface ToolbarProps {
  onCreateNode: (nodeType: string, name: string) => void;
  onRemoveNode: () => void;
  onToggleAudio: () => void;
  onSave: () => void;
  onLoad: () => void;
  isAudioRunning: boolean;
  hasSelectedNode: boolean;
}

const Toolbar: React.FC<ToolbarProps> = ({
  onCreateNode,
  onRemoveNode,
  onToggleAudio,
  onSave,
  onLoad,
  isAudioRunning,
  hasSelectedNode,
}) => {
  const [nodeName, setNodeName] = useState('');
  const [nodeType, setNodeType] = useState('oscillator');

  const handleCreateNode = () => {
    if (nodeName.trim()) {
      onCreateNode(nodeType, nodeName.trim());
      setNodeName('');
    }
  };

  const nodeTypes = [
    { value: 'oscillator', label: 'Multi Oscillator' },
    { value: 'sine_oscillator', label: 'Sine Oscillator' },
    { value: 'triangle_oscillator', label: 'Triangle Oscillator' },
    { value: 'sawtooth_oscillator', label: 'Sawtooth Oscillator' },
    { value: 'pulse_oscillator', label: 'Pulse Oscillator' },
    { value: 'output', label: 'Audio Output' },
  ];

  return (
    <div className="toolbar">
      <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
        <select
          value={nodeType}
          onChange={(e) => setNodeType(e.target.value)}
          style={{ padding: '6px 8px', borderRadius: '4px', border: '1px solid #ddd' }}
        >
          {nodeTypes.map((type) => (
            <option key={type.value} value={type.value}>
              {type.label}
            </option>
          ))}
        </select>
        
        <input
          type="text"
          placeholder="Node name"
          value={nodeName}
          onChange={(e) => setNodeName(e.target.value)}
          onKeyPress={(e) => e.key === 'Enter' && handleCreateNode()}
          style={{ padding: '6px 8px', borderRadius: '4px', border: '1px solid #ddd', width: '120px' }}
        />
        
        <button onClick={handleCreateNode} disabled={!nodeName.trim()}>
          Create Node
        </button>
      </div>

      <div style={{ display: 'flex', gap: '8px' }}>
        <button 
          onClick={onRemoveNode} 
          disabled={!hasSelectedNode}
          style={{ 
            backgroundColor: hasSelectedNode ? '#ff4444' : undefined,
            color: hasSelectedNode ? 'white' : undefined 
          }}
        >
          Remove Selected
        </button>

        <button 
          onClick={onToggleAudio}
          style={{ 
            backgroundColor: isAudioRunning ? '#ff4444' : '#44ff44',
            color: 'white',
            fontWeight: 'bold'
          }}
        >
          {isAudioRunning ? '⏹ Stop Audio' : '▶ Start Audio'}
        </button>

        <button onClick={onSave}>
          💾 Save
        </button>

        <button onClick={onLoad}>
          📂 Load
        </button>
      </div>
    </div>
  );
};

export default Toolbar;