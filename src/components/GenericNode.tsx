import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface GenericNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

// ノードタイプに基づく色とアイコンの決定
const getNodeStyle = (nodeType: string) => {
  const styles = {
    oscillator: {
      gradient: 'linear-gradient(135deg, #f8f9fa 0%, #f1f3f4 100%)',
      borderColor: '#4285f4',
      icon: '🎵'
    },
    output: {
      gradient: 'linear-gradient(135deg, #fef7e0 0%, #fef3c7 100%)',
      borderColor: '#f59e0b',
      icon: '🔊'
    },
    filter: {
      gradient: 'linear-gradient(135deg, #f3e8ff 0%, #e9d5ff 100%)',
      borderColor: '#8b5cf6',
      icon: '🔧'
    },
    lfo: {
      gradient: 'linear-gradient(135deg, #ecfdf5 0%, #d1fae5 100%)',
      borderColor: '#10b981',
      icon: '🌊'
    },
    adsr: {
      gradient: 'linear-gradient(135deg, #fef2f2 0%, #fecaca 100%)',
      borderColor: '#ef4444',
      icon: '📈'
    },
    mixer: {
      gradient: 'linear-gradient(135deg, #f0f9ff 0%, #dbeafe 100%)',
      borderColor: '#3b82f6',
      icon: '🎚️'
    },
    delay: {
      gradient: 'linear-gradient(135deg, #fffbeb 0%, #fef3c7 100%)',
      borderColor: '#f59e0b',
      icon: '⏱️'
    },
    noise: {
      gradient: 'linear-gradient(135deg, #f9fafb 0%, #f3f4f6 100%)',
      borderColor: '#6b7280',
      icon: '📺'
    },
    default: {
      gradient: 'linear-gradient(135deg, #ffffff 0%, #f8f9fa 100%)',
      borderColor: '#9ca3af',
      icon: '⚙️'
    }
  };
  return styles[nodeType as keyof typeof styles] || styles.default;
};

const GenericNode: React.FC<GenericNodeProps> = ({ id, data }) => {
  const [isActive, setIsActive] = useState(true);
  const nodeStyle = getNodeStyle(data.nodeType);

  const updateParameter = useCallback(async (param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        nodeId: id,
        param,
        value,
      });
    } catch (error) {
      console.error(`Failed to update ${param}:`, error);
    }
  }, [id]);

  const toggleActive = useCallback(async () => {
    try {
      const newActiveState = !isActive;
      await updateParameter('active', newActiveState ? 1.0 : 0.0);
      setIsActive(newActiveState);
    } catch (error) {
      console.error('Failed to toggle active state:', error);
    }
  }, [isActive, updateParameter]);

  return (
    <div 
      className="node-container" 
      style={{
        background: nodeStyle.gradient,
        borderLeft: `4px solid ${nodeStyle.borderColor}`
      }}
    >
      {/* Input handles - 左側 */}
      {data.inputPorts.map((port, index) => (
        <Handle
          key={`input-${port.name}`}
          type="target"
          position={Position.Left}
          id={port.name}
          style={{ 
            top: `${40 + (index * 30)}px`,
            left: '-8px',
            width: '16px',
            height: '16px',
            borderRadius: '50%',
            backgroundColor: port.port_type.includes('cv') ? '#4444ff' : '#ff4444',
            border: '2px solid #fff'
          }}
          title={`${port.name} (${port.port_type})`}
        />
      ))}

      {/* Output handles - 右側 */}
      {data.outputPorts.map((port, index) => (
        <Handle
          key={`output-${port.name}`}
          type="source"
          position={Position.Right}
          id={port.name}
          style={{ 
            top: `${40 + (index * 30)}px`,
            right: '-8px',
            width: '16px',
            height: '16px',
            borderRadius: '50%',
            backgroundColor: port.port_type.includes('cv') ? '#4444ff' : '#ff4444',
            border: '2px solid #fff'
          }}
          title={`${port.name} (${port.port_type})`}
        />
      ))}

      {/* ヘッダー */}
      <div className="node-header">
        <div className="node-title">
          {nodeStyle.icon} {data.label}
        </div>
        <button 
          className={`power-button ${isActive ? 'active' : 'inactive'}`}
          onClick={toggleActive}
          title={isActive ? 'Deactivate' : 'Activate'}
        >
          {isActive ? '●' : '○'}
        </button>
      </div>
      
      {/* パラメーター表示 */}
      <div className="node-controls">
        {Object.entries(data.parameters).map(([key, value]) => (
          <div key={key} className="control-group">
            <div className="parameter-display">
              <span className="control-label">{key}</span>
              <span className="control-value">
                {typeof value === 'number' ? value.toFixed(2) : value}
              </span>
            </div>
          </div>
        )).slice(0, 4)} {/* 最大4つのパラメーターを表示 */}
      </div>

      {/* ポート表示 */}
      <div className="node-ports">
        <div className="ports-left">
          {data.inputPorts.map((port) => (
            <div key={port.name} className="port-label">
              {port.name}
            </div>
          ))}
        </div>
        <div className="ports-right">
          {data.outputPorts.map((port) => (
            <div key={port.name} className="port-label">
              {port.name}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default GenericNode;