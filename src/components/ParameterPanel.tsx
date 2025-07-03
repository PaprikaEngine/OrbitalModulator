import React, { useState, useEffect } from 'react';
import { Node } from 'reactflow';

interface ParameterPanelProps {
  node: Node;
  onUpdateParameter: (nodeId: string, param: string, value: number) => void;
  onTriggerGate?: (nodeId: string) => void;
  onClose: () => void;
}

const ParameterPanel: React.FC<ParameterPanelProps> = ({ node, onUpdateParameter, onTriggerGate, onClose }) => {
  const [localParams, setLocalParams] = useState<Record<string, number>>(node.data.parameters || {});

  useEffect(() => {
    setLocalParams(node.data.parameters || {});
  }, [node.data.parameters]);

  const handleParameterChange = (param: string, value: number) => {
    setLocalParams(prev => ({ ...prev, [param]: value }));
    onUpdateParameter(node.id, param, value);
  };

  const getParameterConfig = (param: string, _value: number) => {
    switch (param) {
      case 'frequency':
        return {
          min: 20,
          max: 20000,
          step: 1,
          label: 'Frequency (Hz)',
          format: (val: number) => val >= 1000 ? `${(val / 1000).toFixed(1)}kHz` : `${val.toFixed(0)}Hz`
        };
      case 'amplitude':
        return {
          min: 0,
          max: 1,
          step: 0.01,
          label: 'Amplitude',
          format: (val: number) => `${(val * 100).toFixed(0)}%`
        };
      case 'master_volume':
        return {
          min: 0,
          max: 1,
          step: 0.01,
          label: 'Master Volume',
          format: (val: number) => `${(val * 100).toFixed(0)}%`
        };
      case 'waveform':
        return {
          min: 0,
          max: 3,
          step: 1,
          label: 'Waveform',
          format: (val: number) => ['Sine', 'Triangle', 'Sawtooth', 'Pulse'][Math.floor(val)] || 'Unknown'
        };
      case 'pulse_width':
        return {
          min: 0.1,
          max: 0.9,
          step: 0.01,
          label: 'Pulse Width',
          format: (val: number) => `${(val * 100).toFixed(0)}%`
        };
      case 'mute':
        return {
          min: 0,
          max: 1,
          step: 1,
          label: 'Mute',
          format: (val: number) => val ? 'ON' : 'OFF'
        };
      default:
        return {
          min: 0,
          max: 1,
          step: 0.01,
          label: param,
          format: (val: number) => val.toFixed(2)
        };
    }
  };

  const renderParameterControl = (param: string, value: number) => {
    const config = getParameterConfig(param, value);
    
    if (param === 'mute') {
      return (
        <div key={param} className="parameter-control">
          <label>{config.label}:</label>
          <input
            type="checkbox"
            checked={value > 0}
            onChange={(e) => handleParameterChange(param, e.target.checked ? 1 : 0)}
          />
          <span style={{ marginLeft: '8px' }}>{config.format(value)}</span>
        </div>
      );
    }

    if (param === 'waveform') {
      return (
        <div key={param} className="parameter-control">
          <label>{config.label}:</label>
          <select
            value={Math.floor(value)}
            onChange={(e) => handleParameterChange(param, parseInt(e.target.value))}
            style={{ width: '100%', padding: '6px 8px', border: '1px solid #ddd', borderRadius: '4px' }}
          >
            <option value={0}>Sine</option>
            <option value={1}>Triangle</option>
            <option value={2}>Sawtooth</option>
            <option value={3}>Pulse</option>
          </select>
        </div>
      );
    }

    return (
      <div key={param} className="parameter-control">
        <label>{config.label}:</label>
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <input
            type="range"
            min={config.min}
            max={config.max}
            step={config.step}
            value={value}
            onChange={(e) => handleParameterChange(param, parseFloat(e.target.value))}
            style={{ flex: 1 }}
          />
          <span style={{ minWidth: '60px', fontSize: '12px', textAlign: 'right' }}>
            {config.format(value)}
          </span>
        </div>
        <input
          type="number"
          min={config.min}
          max={config.max}
          step={config.step}
          value={value}
          onChange={(e) => handleParameterChange(param, parseFloat(e.target.value) || 0)}
          style={{ width: '100%', marginTop: '4px', padding: '4px 6px', fontSize: '12px' }}
        />
      </div>
    );
  };

  return (
    <div className="parameter-panel">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
        <h3>{node.data.label || 'Node Parameters'}</h3>
        <button onClick={onClose} style={{ padding: '4px 8px', fontSize: '12px' }}>
          ✕
        </button>
      </div>
      
      <div style={{ fontSize: '12px', color: '#666', marginBottom: '12px' }}>
        Type: {node.data.nodeType}<br />
        ID: {node.id.slice(0, 8)}...
      </div>

      {Object.entries(localParams).map(([param, value]) => renderParameterControl(param, value))}

      {Object.keys(localParams).length === 0 && (
        <div style={{ color: '#666', fontStyle: 'italic' }}>
          No parameters available for this node.
        </div>
      )}

      {/* ADSR Trigger Button */}
      {node.data.nodeType === 'adsr' && onTriggerGate && (
        <div style={{ marginTop: '16px' }}>
          <button 
            onClick={() => onTriggerGate(node.id)}
            style={{ 
              width: '100%', 
              padding: '8px 16px', 
              backgroundColor: '#4CAF50', 
              color: 'white', 
              border: 'none', 
              borderRadius: '4px',
              fontSize: '14px',
              fontWeight: 'bold',
              cursor: 'pointer'
            }}
          >
            🎹 Trigger Gate
          </button>
        </div>
      )}

      <div style={{ marginTop: '16px', padding: '8px', background: '#f5f5f5', borderRadius: '4px', fontSize: '11px' }}>
        <strong>Ports:</strong><br />
        <div style={{ marginTop: '4px' }}>
          <strong>Inputs:</strong> {node.data.inputPorts?.map((p: any) => p.name).join(', ') || 'None'}<br />
          <strong>Outputs:</strong> {node.data.outputPorts?.map((p: any) => p.name).join(', ') || 'None'}
        </div>
      </div>
    </div>
  );
};

export default ParameterPanel;