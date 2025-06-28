import React, { useState, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface MultipleNodeProps {
  id: string;
  data: {
    label: string;
    parameters: {
      active: number;
      channel_count: number;
      gain_0?: number;
      gain_1?: number;
      gain_2?: number;
      gain_3?: number;
      gain_4?: number;
      gain_5?: number;
      gain_6?: number;
      gain_7?: number;
    };
  };
}

const MultipleNode: React.FC<MultipleNodeProps> = ({ id, data }) => {
  const [active, setActive] = useState((data.parameters?.active || 1) !== 0);
  const [channelCount, setChannelCount] = useState(data.parameters?.channel_count || 4);
  const [gains, setGains] = useState<number[]>(() => {
    const count = data.parameters?.channel_count || 4;
    const initialGains = [];
    for (let i = 0; i < count; i++) {
      const gainParam = `gain_${i}` as keyof typeof data.parameters;
      initialGains.push(data.parameters?.[gainParam] || 1.0);
    }
    return initialGains;
  });

  useEffect(() => {
    setActive((data.parameters?.active || 1) !== 0);
    setChannelCount(data.parameters?.channel_count || 4);
    
    const count = data.parameters?.channel_count || 4;
    const updatedGains = [];
    for (let i = 0; i < count; i++) {
      const gainParam = `gain_${i}` as keyof typeof data.parameters;
      updatedGains.push(data.parameters?.[gainParam] || 1.0);
    }
    setGains(updatedGains);
  }, [data.parameters]);

  const updateParameter = async (param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        nodeId: id,
        param,
        value,
      });
    } catch (error) {
      console.error(`Failed to update ${param}:`, error);
    }
  };

  const handleGainChange = (channel: number, value: number) => {
    const newGains = [...gains];
    newGains[channel] = value;
    setGains(newGains);
    updateParameter(`gain_${channel}`, value);
  };

  const toggleActive = () => {
    const newActive = !active;
    setActive(newActive);
    updateParameter('active', newActive ? 1 : 0);
  };

  // Generate output handles based on channel count
  const renderOutputHandles = () => {
    const handles = [];
    const spacing = channelCount <= 4 ? 60 / channelCount : 60 / channelCount;
    
    for (let i = 0; i < channelCount; i++) {
      const topPercent = 30 + (i * spacing);
      handles.push(
        <Handle
          key={`out_${i + 1}`}
          type="source"
          position={Position.Right}
          id={`out_${i + 1}`}
          style={{ 
            background: '#3498db', 
            top: `${topPercent}%`,
            transform: 'translateY(-50%)'
          }}
        />
      );
    }
    return handles;
  };

  const renderGainControls = () => {
    return gains.map((gain, index) => (
      <div key={`gain_${index}`} className="gain-control-row">
        <div className="channel-label">Ch{index + 1}</div>
        <input
          type="range"
          min="0"
          max="2"
          step="0.01"
          value={gain}
          onChange={(e) => handleGainChange(index, Number(e.target.value))}
          className="gain-mini-slider"
        />
        <div className="gain-value-small">{gain.toFixed(2)}</div>
      </div>
    ));
  };

  return (
    <div className="multiple-node">
      {/* Input Handle */}
      <Handle
        type="target"
        position={Position.Left}
        id="signal_in"
        style={{ background: '#e74c3c' }}
      />

      {/* Header */}
      <div className="node-header">
        <div className="node-title">{data.label}</div>
        <button
          className={`active-button ${active ? 'active' : 'inactive'}`}
          onClick={toggleActive}
          title={active ? 'Click to deactivate' : 'Click to activate'}
        >
          {active ? 'ON' : 'OFF'}
        </button>
      </div>

      {/* Signal Distribution Visualization */}
      <div className="multiple-display">
        <div className="signal-flow-visual">
          <div className="input-signal">
            <div className="signal-dot input-dot" />
            <div className="signal-label">IN</div>
          </div>
          
          <div className="distribution-hub">
            <div className="hub-circle">
              <div className="hub-center" />
            </div>
            <div className="signal-lines">
              {Array.from({ length: channelCount }, (_, i) => (
                <div 
                  key={i}
                  className="signal-line" 
                  style={{
                    transform: `rotate(${(360 / channelCount) * i}deg)`,
                    opacity: active ? 1 : 0.3
                  }}
                />
              ))}
            </div>
          </div>
          
          <div className="output-signals">
            {Array.from({ length: channelCount }, (_, i) => (
              <div key={i} className="output-channel">
                <div className="signal-dot output-dot" style={{
                  background: active ? '#3498db' : '#7f8c8d'
                }} />
                <div className="channel-number">{i + 1}</div>
              </div>
            ))}
          </div>
        </div>
        
        <div className="channel-info">
          <div className="info-item">
            <span className="info-label">Channels:</span>
            <span className="info-value">{channelCount}</span>
          </div>
          <div className="info-item">
            <span className="info-label">Type:</span>
            <span className="info-value">1→{channelCount}</span>
          </div>
        </div>
      </div>

      {/* Individual Gain Controls */}
      <div className="multiple-controls">
        <div className="controls-header">Channel Gains</div>
        <div className="gain-controls-grid">
          {renderGainControls()}
        </div>
      </div>

      {/* Output Handles */}
      {renderOutputHandles()}

      {/* Port Labels */}
      <div className="port-labels">
        <div className="input-labels">
          <div>Signal</div>
        </div>
        <div className="output-labels">
          {Array.from({ length: channelCount }, (_, i) => (
            <div key={i} style={{ 
              top: `${30 + (i * (60 / channelCount))}%`,
              transform: 'translateY(-50%)'
            }}>
              Out{i + 1}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default MultipleNode;