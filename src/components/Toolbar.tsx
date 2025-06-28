import React, { useState } from 'react';

interface ToolbarProps {
  onCreateNode: (nodeType: string, name: string) => void;
  onRemoveNode: () => void;
  onToggleAudioEngine: () => void;
  onSave: () => void;
  onLoad: () => void;
  isAudioEngineRunning: boolean;
  hasSelectedNode: boolean;
}

const Toolbar: React.FC<ToolbarProps> = ({
  onCreateNode,
  onRemoveNode,
  onToggleAudioEngine,
  onSave,
  onLoad,
  isAudioEngineRunning,
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
    { value: 'filter', label: 'VCF Filter' },
    { value: 'adsr', label: 'ADSR Envelope' },
    { value: 'lfo', label: 'LFO' },
    { value: 'mixer', label: 'Mixer (4ch)' },
    { value: 'mixer8', label: 'Mixer (8ch)' },
    { value: 'delay', label: 'Delay Effect' },
    { value: 'noise', label: 'Noise Generator' },
    { value: 'vca', label: 'VCA (Amplifier)' },
    { value: 'sequencer', label: 'Sequencer' },
    { value: 'oscilloscope', label: 'Oscilloscope' },
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
          onClick={onToggleAudioEngine}
          style={{ 
            backgroundColor: isAudioEngineRunning ? '#ff4444' : '#44aa44',
            color: 'white',
            fontWeight: 'bold'
          }}
        >
          {isAudioEngineRunning ? '🔇 Stop Engine' : '🔊 Start Engine'}
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