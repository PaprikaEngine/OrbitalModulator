import React, { useState, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface VCANodeProps {
  id: string;
  data: {
    label: string;
    parameters: {
      gain: number;
      cv_sensitivity: number;
      active: number;
    };
  };
}

const VCANode: React.FC<VCANodeProps> = ({ id, data }) => {
  const [gain, setGain] = useState(data.parameters?.gain || 1.0);
  const [cvSensitivity, setCvSensitivity] = useState(data.parameters?.cv_sensitivity || 1.0);
  const [active, setActive] = useState((data.parameters?.active || 1) !== 0);

  useEffect(() => {
    setGain(data.parameters?.gain || 1.0);
    setCvSensitivity(data.parameters?.cv_sensitivity || 1.0);
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

  const handleGainChange = (value: number) => {
    setGain(value);
    updateParameter('gain', value);
  };

  const handleCvSensitivityChange = (value: number) => {
    setCvSensitivity(value);
    updateParameter('cv_sensitivity', value);
  };

  const toggleActive = () => {
    const newActive = !active;
    setActive(newActive);
    updateParameter('active', newActive ? 1 : 0);
  };

  // Visual gain meter
  const getGainBarHeight = () => {
    return Math.max(5, gain * 50);
  };

  const getGainColor = () => {
    if (gain < 0.5) return '#27ae60';
    if (gain < 1.0) return '#f39c12';
    if (gain < 1.5) return '#e67e22';
    return '#e74c3c';
  };

  // CV sensitivity indicator
  const getSensitivityColor = () => {
    if (cvSensitivity < 0.5) return '#3498db';
    if (cvSensitivity < 1.0) return '#9b59b6';
    return '#e74c3c';
  };

  return (
    <div className="vca-node">
      {/* Input Handles */}
      <Handle
        type="target"
        position={Position.Left}
        id="audio_in"
        style={{ top: '35%', background: '#e74c3c' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="gain_cv"
        style={{ top: '65%', background: '#3498db' }}
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

      {/* VCA Visual Indicator */}
      <div className="vca-indicator">
        <div className="gain-meter">
          <div 
            className="gain-bar"
            style={{
              height: `${getGainBarHeight()}px`,
              backgroundColor: getGainColor(),
              opacity: active ? 0.8 : 0.3,
            }}
          />
          <div className="unity-line" />
        </div>
        <div className="gain-display">
          {gain.toFixed(2)}x
        </div>
      </div>

      {/* Controls */}
      <div className="vca-controls">
        {/* Gain Control */}
        <div className="control-group">
          <label className="control-label">Gain</label>
          <div className="gain-control">
            <input
              type="range"
              min="0"
              max="2"
              step="0.01"
              value={gain}
              onChange={(e) => handleGainChange(Number(e.target.value))}
              className="gain-slider"
            />
            <div className="control-value">{gain.toFixed(2)}x</div>
          </div>
        </div>

        {/* CV Sensitivity Control */}
        <div className="control-group">
          <label className="control-label">CV Sens</label>
          <div className="cv-sensitivity-control">
            <input
              type="range"
              min="0"
              max="2"
              step="0.01"
              value={cvSensitivity}
              onChange={(e) => handleCvSensitivityChange(Number(e.target.value))}
              className="cv-sensitivity-slider"
              style={{ accentColor: getSensitivityColor() }}
            />
            <div className="control-value">{cvSensitivity.toFixed(2)}</div>
          </div>
        </div>
      </div>

      {/* CV Sensitivity Indicator */}
      <div className="cv-indicator">
        <div 
          className="cv-sensitivity-bar"
          style={{
            width: `${cvSensitivity * 50}%`,
            backgroundColor: getSensitivityColor(),
            opacity: active ? 0.7 : 0.3,
          }}
        />
        <div className="cv-label">CV Response</div>
      </div>

      {/* Output Handle */}
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out"
        style={{ background: '#e74c3c' }}
      />

      {/* Port Labels */}
      <div className="port-labels">
        <div className="input-labels">
          <div style={{ top: '35%' }}>Audio</div>
          <div style={{ top: '65%' }}>Gain CV</div>
        </div>
        <div className="output-labels">
          <div>Audio</div>
        </div>
      </div>
    </div>
  );
};

export default VCANode;