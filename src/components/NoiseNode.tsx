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
    <div className={`eurorack-module noise-module ${active ? 'active' : 'inactive'}`}>
      {/* Module Header - ドラッグハンドル */}
      <div className="module-header drag-handle">
        <div className="module-brand">ORBITAL</div>
        <div className="module-name">NOISE</div>
        <div className={`power-led ${active ? 'active' : ''}`}></div>
      </div>

      {/* CV Input */}
      <Handle
        type="target"
        position={Position.Left}
        id="amplitude_cv"
        style={{ top: '35%', background: '#3498db' }}
        className="cv-input"
      />
      <div className="cv-label" style={{ top: '32%' }}>LEVEL</div>

      {/* Main Controls */}
      <div 
        className="control-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Amplitude Control (Large Knob) */}
        <div className="knob-group large-knob">
          <label className="knob-label">LEVEL</label>
          <div className="knob-container">
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={amplitude}
              onChange={(e) => handleAmplitudeChange(Number(e.target.value))}
              className="amplitude-knob"
            />
            <div className="knob-value">{Math.round(amplitude * 100)}%</div>
          </div>
        </div>
      </div>

      {/* Noise Type Selector */}
      <div 
        className="noise-type-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        <label className="section-label">COLOR</label>
        <div className="noise-type-buttons">
          {noiseTypes.map((type, index) => (
            <button
              key={index}
              className={`noise-btn ${noiseType === index ? 'active' : ''}`}
              style={{ 
                backgroundColor: noiseType === index ? getNoiseColor() : 'transparent',
                borderColor: getNoiseColor(),
                color: noiseType === index ? '#fff' : getNoiseColor()
              }}
              onClick={() => handleNoiseTypeChange(index)}
            >
              {type.symbol}
            </button>
          ))}
        </div>
      </div>

      {/* Visual Noise Indicator */}
      <div className="noise-display">
        <div className="noise-bars">
          {[...Array(6)].map((_, i) => (
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
        <div className="noise-type-display">
          <span className="noise-label">{currentNoiseType.label}</span>
        </div>
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
        <div className="hp-marking">4HP</div>
      </div>
    </div>
  );
};

export default NoiseNode;