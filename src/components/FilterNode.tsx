import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface FilterNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const FilterNode: React.FC<FilterNodeProps> = ({ id, data }) => {
  const [isActive, setIsActive] = useState(data.parameters.active === 1.0);
  
  const getFilterTypeName = (value: number) => {
    const types = ['Lowpass', 'Highpass', 'Bandpass'];
    return types[Math.floor(value)] || 'Lowpass';
  };

  const formatFrequency = (freq: number) => {
    if (freq >= 1000) {
      return `${(freq / 1000).toFixed(1)}kHz`;
    }
    return `${freq.toFixed(0)}Hz`;
  };

  const formatResonance = (q: number) => {
    return `Q${q.toFixed(1)}`;
  };

  const toggleActive = useCallback(async () => {
    try {
      const newActiveState = !isActive;
      
      await invoke('set_node_parameter', {
        node_id: id,
        param: 'active',
        value: newActiveState ? 1.0 : 0.0,
      });
      
      setIsActive(newActiveState);
    } catch (error) {
      console.error('Failed to toggle filter active state:', error);
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
          title={isActive ? 'Deactivate filter' : 'Activate filter'}
        >
          {isActive ? '⏹' : '▶'}
        </button>
      </div>
      
      <div className="node-params">
        {data.parameters.cutoff_frequency !== undefined && (
          <div className="param-row">
            <span className="param-label">Cutoff:</span>
            <span className="param-value">{formatFrequency(data.parameters.cutoff_frequency)}</span>
          </div>
        )}
        
        {data.parameters.resonance !== undefined && (
          <div className="param-row">
            <span className="param-label">Res:</span>
            <span className="param-value">{formatResonance(data.parameters.resonance)}</span>
          </div>
        )}
        
        {data.parameters.filter_type !== undefined && (
          <div className="param-row">
            <span className="param-label">Type:</span>
            <span className="param-value">{getFilterTypeName(data.parameters.filter_type)}</span>
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

export default FilterNode;