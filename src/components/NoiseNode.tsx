import React, { useState, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface NoiseNodeProps {
  id: string;
  data: {
    label: string;
    parameters: {
      noise_type: number;
      amplitude: number;
      active: number;
    };
  };
}

const NoiseNode: React.FC<NoiseNodeProps> = ({ id, data }) => {
  const [noiseType, setNoiseType] = useState(data.parameters?.noise_type || 0);
  const [amplitude, setAmplitude] = useState(data.parameters?.amplitude || 0.5);
  const [active, setActive] = useState((data.parameters?.active || 1) !== 0);

  useEffect(() => {
    setNoiseType(data.parameters?.noise_type || 0);
    setAmplitude(data.parameters?.amplitude || 0.5);
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

  const handleNoiseTypeChange = (value: number) => {
    setNoiseType(value);
    updateParameter('noise_type', value);
  };

  const handleAmplitudeChange = (value: number) => {
    setAmplitude(value);
    updateParameter('amplitude', value);
  };

  const toggleActive = () => {
    const newActive = !active;
    setActive(newActive);
    updateParameter('active', newActive ? 1 : 0);
  };

  const noiseTypes = [
    { value: 0, label: 'White', symbol: '⚪' },
    { value: 1, label: 'Pink', symbol: '🌸' },
    { value: 2, label: 'Brown', symbol: '🟤' },
    { value: 3, label: 'Blue', symbol: '🔵' },
  ];

  const getNoiseTypeInfo = (type: number) => {
    return noiseTypes.find(t => t.value === type) || noiseTypes[0];
  };

  const currentNoiseType = getNoiseTypeInfo(noiseType);

  // Visual noise indicator based on amplitude
  const getNoiseBarHeight = () => {
    return Math.max(10, amplitude * 60);
  };

  const getNoiseColor = () => {
    switch (noiseType) {
      case 0: return '#ecf0f1'; // White
      case 1: return '#e91e63'; // Pink
      case 2: return '#8d4004'; // Brown
      case 3: return '#3498db'; // Blue
      default: return '#ecf0f1';
    }
  };

  return (
    <div className="noise-node">
      {/* Input Handle */}
      <Handle
        type="target"
        position={Position.Left}
        id="amplitude_cv"
        style={{ top: '60%', background: '#3498db' }}
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

      {/* Noise Type Display */}
      <div className="noise-type-display">
        <div className="current-noise-type">
          <span className="noise-symbol">{currentNoiseType.symbol}</span>
          <span className="noise-label">{currentNoiseType.label} Noise</span>
        </div>
      </div>

      {/* Visual Noise Indicator */}
      <div className="noise-indicator">
        <div className="noise-bars">
          {[...Array(8)].map((_, i) => (
            <div
              key={i}
              className="noise-bar"
              style={{
                height: `${getNoiseBarHeight() * (0.5 + Math.random() * 0.5)}px`,
                backgroundColor: getNoiseColor(),
                opacity: active ? 0.4 + (amplitude * 0.6) : 0.2,
                animationDelay: `${i * 0.1}s`,
              }}
            />
          ))}
        </div>
        <div className="amplitude-display">
          {(amplitude * 100).toFixed(0)}%
        </div>
      </div>

      {/* Controls */}
      <div className="noise-controls">
        {/* Noise Type Selector */}
        <div className="control-group">
          <label className="control-label">Type</label>
          <select
            value={noiseType}
            onChange={(e) => handleNoiseTypeChange(Number(e.target.value))}
            className="noise-type-select"
          >
            {noiseTypes.map((type) => (
              <option key={type.value} value={type.value}>
                {type.symbol} {type.label}
              </option>
            ))}
          </select>
        </div>

        {/* Amplitude Control */}
        <div className="control-group">
          <label className="control-label">Amplitude</label>
          <div className="amplitude-control">
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={amplitude}
              onChange={(e) => handleAmplitudeChange(Number(e.target.value))}
              className="amplitude-slider"
            />
            <div className="control-value">{(amplitude * 100).toFixed(0)}%</div>
          </div>
        </div>
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
          <div style={{ top: '60%' }}>Amp CV</div>
        </div>
        <div className="output-labels">
          <div>Audio</div>
        </div>
      </div>
    </div>
  );
};

export default NoiseNode;