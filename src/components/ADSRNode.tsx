import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface ADSRNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const ADSRNode: React.FC<ADSRNodeProps> = ({ id, data }) => {
  const [isActive, setIsActive] = useState(data.parameters.active === 1.0);
  
  const formatTime = (time: number) => {
    if (time >= 1.0) {
      return `${time.toFixed(1)}s`;
    } else {
      return `${(time * 1000).toFixed(0)}ms`;
    }
  };

  const formatLevel = (level: number) => {
    return `${(level * 100).toFixed(0)}%`;
  };

  const toggleActive = useCallback(async () => {
    try {
      const newActiveState = !isActive;
      
      await invoke('set_node_parameter', {
        nodeId: id,
        param: 'active',
        value: newActiveState ? 1.0 : 0.0,
      });
      
      setIsActive(newActiveState);
    } catch (error) {
      console.error('Failed to toggle ADSR active state:', error);
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
          title={isActive ? 'Deactivate ADSR' : 'Activate ADSR'}
        >
          {isActive ? '⏹' : '▶'}
        </button>
      </div>
      
      <div className="node-params">
        {data.parameters.attack !== undefined && (
          <div className="param-row">
            <span className="param-label">A:</span>
            <span className="param-value">{formatTime(data.parameters.attack)}</span>
          </div>
        )}
        
        {data.parameters.decay !== undefined && (
          <div className="param-row">
            <span className="param-label">D:</span>
            <span className="param-value">{formatTime(data.parameters.decay)}</span>
          </div>
        )}
        
        {data.parameters.sustain !== undefined && (
          <div className="param-row">
            <span className="param-label">S:</span>
            <span className="param-value">{formatLevel(data.parameters.sustain)}</span>
          </div>
        )}
        
        {data.parameters.release !== undefined && (
          <div className="param-row">
            <span className="param-label">R:</span>
            <span className="param-value">{formatTime(data.parameters.release)}</span>
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

export default ADSRNode;