import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface OscillatorNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const OscillatorNode: React.FC<OscillatorNodeProps> = ({ id, data }) => {
  const [isActive, setIsActive] = useState(false);
  
  const getWaveformName = (value: number) => {
    const waveforms = ['Sine', 'Triangle', 'Sawtooth', 'Pulse'];
    return waveforms[Math.floor(value)] || 'Unknown';
  };

  const formatFrequency = (freq: number) => {
    if (freq >= 1000) {
      return `${(freq / 1000).toFixed(1)}kHz`;
    }
    return `${freq.toFixed(0)}Hz`;
  };

  const toggleActive = useCallback(async () => {
    try {
      const newActiveState = !isActive;
      
      // Tauriコマンドでノードのアクティブ状態を設定
      await invoke('set_node_parameter', {
        nodeId: id,
        param: 'active',
        value: newActiveState ? 1.0 : 0.0,
      });
      
      setIsActive(newActiveState);
    } catch (error) {
      console.error('Failed to toggle oscillator active state:', error);
    }
  }, [id, isActive]);

  return (
    <div className={`react-flow__node react-flow__node-${data.nodeType}`}>
      {/* Input handles */}
      {data.inputPorts.map((port, index) => (
        <Handle
          key={`input-${port.name}`}
          type="target"
          position={Position.Left}
          id={port.name}
          style={{ top: 30 + index * 20 }}
          title={`${port.name} (${port.port_type})`}
        />
      ))}

      {/* Output handles */}
      {data.outputPorts.map((port, index) => (
        <Handle
          key={`output-${port.name}`}
          type="source"
          position={Position.Right}
          id={port.name}
          style={{ top: 30 + index * 20 }}
          title={`${port.name} (${port.port_type})`}
        />
      ))}

      <div className="node-header">
        <div className="node-title">{data.label}</div>
        <button 
          className={`active-button ${isActive ? 'active' : 'inactive'}`}
          onClick={toggleActive}
          title={isActive ? 'Deactivate oscillator' : 'Activate oscillator'}
        >
          {isActive ? '⏹' : '▶'}
        </button>
      </div>
      
      <div className="node-params">
        {data.parameters.frequency !== undefined && (
          <div className="param-row">
            <span className="param-label">Freq:</span>
            <span className="param-value">{formatFrequency(data.parameters.frequency)}</span>
          </div>
        )}
        
        {data.parameters.amplitude !== undefined && (
          <div className="param-row">
            <span className="param-label">Amp:</span>
            <span className="param-value">{(data.parameters.amplitude * 100).toFixed(0)}%</span>
          </div>
        )}
        
        {data.parameters.waveform !== undefined && (
          <div className="param-row">
            <span className="param-label">Wave:</span>
            <span className="param-value">{getWaveformName(data.parameters.waveform)}</span>
          </div>
        )}
        
        {data.parameters.pulse_width !== undefined && data.parameters.waveform === 3 && (
          <div className="param-row">
            <span className="param-label">PWM:</span>
            <span className="param-value">{(data.parameters.pulse_width * 100).toFixed(0)}%</span>
          </div>
        )}
      </div>

      <div className="node-ports">
        <div className="port-list">
          {data.inputPorts.map((port) => (
            <div key={port.name} className="port-item">
              ◀ {port.name}
            </div>
          ))}
          {data.outputPorts.map((port) => (
            <div key={port.name} className="port-item">
              {port.name} ▶
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default OscillatorNode;