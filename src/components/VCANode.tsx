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
    <div className={`eurorack-module vca-module ${active ? 'active' : 'inactive'}`}>
      {/* Module Header - ドラッグハンドル */}
      <div className="module-header drag-handle">
        <div className="module-brand">ORBITAL</div>
        <div className="module-name">VCA</div>
        <div className={`power-led ${active ? 'active' : ''}`}></div>
      </div>

      {/* Audio Input */}
      <Handle
        type="target"
        position={Position.Left}
        id="audio_in"
        style={{ top: '20%', background: '#e74c3c', width: '12px', height: '12px' }}
        className="audio-input"
      />
      <div className="input-label" style={{ top: '17%' }}>IN</div>

      {/* CV Input */}
      <Handle
        type="target"
        position={Position.Left}
        id="gain_cv"
        style={{ top: '35%', background: '#3498db' }}
        className="cv-input"
      />

      {/* Main Controls */}
      <div 
        className="control-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Gain Control (Large Knob) */}
        <div className="knob-group large-knob">
          <label className="knob-label">GAIN</label>
          <div className="knob-container">
            <input
              type="range"
              min="0"
              max="2"
              step="0.01"
              value={gain}
              onChange={(e) => handleGainChange(Number(e.target.value))}
              className="gain-knob"
            />
            <div className="knob-value">{gain.toFixed(2)}x</div>
          </div>
        </div>

        {/* CV Sensitivity Control */}
        <div className="knob-group">
          <label className="knob-label">CV SENS</label>
          <div className="knob-container">
            <input
              type="range"
              min="0"
              max="2"
              step="0.01"
              value={cvSensitivity}
              onChange={(e) => handleCvSensitivityChange(Number(e.target.value))}
              className="cv-sensitivity-knob"
            />
            <div className="knob-value">{cvSensitivity.toFixed(2)}</div>
          </div>
        </div>
      </div>

      {/* VCA Visual Indicator */}
      <div className="vca-display">
        <div className="gain-meter">
          <div className="meter-background"></div>
          <div 
            className="meter-fill"
            style={{
              height: `${Math.min(100, gain * 50)}%`,
              backgroundColor: getGainColor(),
              opacity: active ? 0.8 : 0.3,
            }}
          />
        </div>
        <div className="gain-readout">
          {gain.toFixed(2)}x
        </div>
      </div>

      {/* CV Input Labels */}
      <div className="cv-labels">
        <div className="cv-label" style={{ top: '32%' }}>CV</div>
      </div>

      {/* Audio Output */}
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out"
        style={{ top: '70%', background: '#e74c3c', width: '12px', height: '12px' }}
        className="audio-output"
      />
      <div className="output-label">OUT</div>

      {/* Module Footer */}
      <div className="module-footer">
        <div className="hp-marking">6HP</div>
      </div>
    </div>
  );
};

export default VCANode;