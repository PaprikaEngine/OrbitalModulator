import React from 'react';
import { Handle, Position } from 'reactflow';

interface OutputNodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const OutputNode: React.FC<OutputNodeProps> = ({ data }) => {
  return (
    <div className={`react-flow__node react-flow__node-${data.nodeType}`}>
      {/* Input handles */}
      {data.inputPorts.map((port, index) => (
        <Handle
          key={`input-${port.name}`}
          type="target"
          position={Position.Left}
          id={port.name}
          style={{ top: 30 + index * 15 }}
          title={`${port.name} (${port.port_type})`}
        />
      ))}

      <div className="node-header">
        🔊 {data.label}
      </div>
      
      <div className="node-params">
        {data.parameters.master_volume !== undefined && (
          <div className="param-row">
            <span className="param-label">Volume:</span>
            <span className="param-value">{(data.parameters.master_volume * 100).toFixed(0)}%</span>
          </div>
        )}
        
        {data.parameters.mute !== undefined && (
          <div className="param-row">
            <span className="param-label">Mute:</span>
            <span className="param-value">{data.parameters.mute ? 'ON' : 'OFF'}</span>
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
        </div>
      </div>
    </div>
  );
};

export default OutputNode;