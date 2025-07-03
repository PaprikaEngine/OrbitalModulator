import React, { useState, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface AttenuverterNodeProps {
  id: string;
  data: {
    label: string;
    parameters: {
      attenuation: number;
      offset: number;
      active: number;
    };
  };
}

const AttenuverterNode: React.FC<AttenuverterNodeProps> = ({ id, data }) => {
  const [attenuation, setAttenuation] = useState(data.parameters?.attenuation || 1.0);
  const [offset, setOffset] = useState(data.parameters?.offset || 0.0);
  const [active, setActive] = useState((data.parameters?.active || 1) !== 0);

  useEffect(() => {
    setAttenuation(data.parameters?.attenuation || 1.0);
    setOffset(data.parameters?.offset || 0.0);
    setActive((data.parameters?.active || 1) !== 0);
  }, [data.parameters]);

  const updateParameter = async (param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        node_id: id,
        param,
        value,
      });
    } catch (error) {
      console.error(`Failed to update ${param}:`, error);
    }
  };

  const handleAttenuationChange = (value: number) => {
    setAttenuation(value);
    updateParameter('attenuation', value);
  };

  const handleOffsetChange = (value: number) => {
    setOffset(value);
    updateParameter('offset', value);
  };

  const toggleActive = () => {
    const newActive = !active;
    setActive(newActive);
    updateParameter('active', newActive ? 1 : 0);
  };

  // Calculate visualization values
  const attenuationPercent = Math.abs(attenuation) * 100;
  const isInverted = attenuation < 0;
  const offsetVolts = offset;
  const gainReduction = Math.abs(attenuation) < 1 ? (1 - Math.abs(attenuation)) * 100 : 0;

  return (
    <div className="attenuverter-node">
      {/* Input Handle */}
      <Handle
        type="target"
        position={Position.Left}
        id="signal_in"
        style={{ background: '#3498db' }}
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

      {/* Signal Processing Display */}
      <div className="attenuverter-display">
        <div className="signal-processing">
          {/* Input Signal */}
          <div className="signal-section">
            <div className="signal-label">Input</div>
            <div className="signal-meter">
              <div className="meter-bar input-meter">
                <div className="meter-fill" style={{ height: '60%', background: '#3498db' }} />
              </div>
            </div>
          </div>

          {/* Processing Arrow */}
          <div className="processing-arrow">
            {isInverted ? '⟲' : '→'}
          </div>

          {/* Output Signal */}
          <div className="signal-section">
            <div className="signal-label">Output</div>
            <div className="signal-meter">
              <div className="meter-bar output-meter">
                <div 
                  className="meter-fill" 
                  style={{ 
                    height: `${attenuationPercent * 0.6}%`, 
                    background: isInverted ? '#e74c3c' : '#27ae60',
                    transform: isInverted ? 'scaleY(-1) translateY(-100%)' : 'none'
                  }} 
                />
              </div>
            </div>
          </div>
        </div>

        {/* Status Indicators */}
        <div className="status-indicators">
          <div className="status-item">
            <div className="status-label">Mode</div>
            <div className={`status-value ${isInverted ? 'inverted' : 'normal'}`}>
              {isInverted ? 'INVERT' : 'NORMAL'}
            </div>
          </div>
          {gainReduction > 0 && (
            <div className="status-item">
              <div className="status-label">Reduction</div>
              <div className="status-value">-{gainReduction.toFixed(1)}%</div>
            </div>
          )}
          {Math.abs(offsetVolts) > 0.01 && (
            <div className="status-item">
              <div className="status-label">DC Offset</div>
              <div className="status-value">{offsetVolts >= 0 ? '+' : ''}{offsetVolts.toFixed(2)}V</div>
            </div>
          )}
        </div>
      </div>

      {/* Controls */}
      <div className="attenuverter-controls">
        {/* Attenuation Control */}
        <div className="control-group">
          <label className="control-label">Gain/Atten</label>
          <div className="knob-control">
            <div className="knob-container">
              <input
                type="range"
                min="-1"
                max="1"
                step="0.01"
                value={attenuation}
                onChange={(e) => handleAttenuationChange(Number(e.target.value))}
                className="attenuation-knob"
              />
              <div className="knob-indicator" 
                   style={{ 
                     transform: `rotate(${(attenuation + 1) * 135 - 135}deg)` 
                   }}>
                <div className="knob-pointer"></div>
              </div>
            </div>
            <div className="control-value">
              {attenuation >= 0 ? '+' : ''}{(attenuation * 100).toFixed(0)}%
            </div>
          </div>
        </div>

        {/* Offset Control */}
        <div className="control-group">
          <label className="control-label">DC Offset</label>
          <div className="slider-control">
            <input
              type="range"
              min="-5"
              max="5"
              step="0.01"
              value={offset}
              onChange={(e) => handleOffsetChange(Number(e.target.value))}
              className="offset-slider"
            />
            <div className="control-value">
              {offset >= 0 ? '+' : ''}{offset.toFixed(2)}V
            </div>
          </div>
        </div>
      </div>

      {/* Output Handles */}
      <Handle
        type="source"
        position={Position.Right}
        id="signal_out"
        style={{ background: '#27ae60', top: '35%' }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="inverted_out"
        style={{ background: '#e74c3c', top: '65%' }}
      />

      {/* Port Labels */}
      <div className="port-labels">
        <div className="input-labels">
          <div>Signal</div>
        </div>
        <div className="output-labels">
          <div style={{ top: '35%' }}>Normal</div>
          <div style={{ top: '65%' }}>Inverted</div>
        </div>
      </div>
    </div>
  );
};

export default AttenuverterNode;