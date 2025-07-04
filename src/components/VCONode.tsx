import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface VCONodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const VCONode: React.FC<VCONodeProps> = ({ id, data, selected }) => {
  const [parameters, setParameters] = useState({
    frequency: data.parameters.frequency || 440,
    amplitude: data.parameters.amplitude || 0.8,
    waveform: data.parameters.waveform || 0, // 0=Triangle, 1=Sawtooth, 2=Sine, 3=Pulse
    active: data.parameters.active || 1,
    ...data.parameters
  });

  const updateParameter = useCallback(async (param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        node_id: id,
        param,
        value,
      });
      
      setParameters(prev => ({
        ...prev,
        [param]: value,
      }));
    } catch (error) {
      console.error('Failed to update parameter:', error);
    }
  }, [id]);

  const waveformNames = ['TRI', 'SAW', 'SIN', 'PUL'];
  const waveformColors = ['#ff6b6b', '#4ecdc4', '#45b7d1', '#96ceb4'];

  return (
    <div className={`eurorack-module vco-module ${selected ? 'selected' : ''}`}>
      {/* Module Header - ドラッグハンドル */}
      <div className="module-header drag-handle">
        <div className="module-brand">ORBITAL</div>
        <div className="module-name">VCO</div>
        <div className="power-led active"></div>
      </div>

      {/* CV Inputs */}
      <Handle
        type="target"
        position={Position.Left}
        id="frequency_cv"
        style={{ top: '25%', background: '#ff6b6b' }}
        className="cv-input"
      />
      <Handle
        type="target"
        position={Position.Left}
        id="amplitude_cv"
        style={{ top: '45%', background: '#4ecdc4' }}
        className="cv-input"
      />

      {/* Main Controls */}
      <div 
        className="control-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Frequency Control */}
        <div className="knob-group">
          <label className="knob-label">FREQ</label>
          <div className="knob-container">
            <input
              type="range"
              min={20}
              max={20000}
              step={1}
              value={parameters.frequency}
              onChange={(e) => updateParameter('frequency', parseFloat(e.target.value))}
              className="frequency-knob"
            />
            <div className="knob-value">
              {parameters.frequency >= 1000 
                ? `${(parameters.frequency / 1000).toFixed(1)}k` 
                : `${Math.round(parameters.frequency)}`}Hz
            </div>
          </div>
        </div>

        {/* Amplitude Control */}
        <div className="knob-group">
          <label className="knob-label">LEVEL</label>
          <div className="knob-container">
            <input
              type="range"
              min={0}
              max={1}
              step={0.01}
              value={parameters.amplitude}
              onChange={(e) => updateParameter('amplitude', parseFloat(e.target.value))}
              className="amplitude-knob"
            />
            <div className="knob-value">{Math.round(parameters.amplitude * 100)}%</div>
          </div>
        </div>
      </div>

      {/* Waveform Selector */}
      <div 
        className="waveform-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        <label className="section-label">WAVEFORM</label>
        <div className="waveform-buttons">
          {waveformNames.map((name, index) => (
            <button
              key={index}
              className={`waveform-btn ${parameters.waveform === index ? 'active' : ''}`}
              style={{ 
                '--waveform-color': waveformColors[index],
                backgroundColor: parameters.waveform === index ? waveformColors[index] : 'transparent',
                borderColor: waveformColors[index],
                color: parameters.waveform === index ? '#fff' : waveformColors[index]
              }}
              onClick={() => updateParameter('waveform', index)}
            >
              {name}
            </button>
          ))}
        </div>
      </div>

      {/* CV Input Labels */}
      <div className="cv-labels">
        <div className="cv-label" style={{ top: '22%' }}>1V/OCT</div>
        <div className="cv-label" style={{ top: '42%' }}>AM</div>
      </div>

      {/* Audio Output */}
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out"
        style={{ top: '70%', background: '#ff6b6b', width: '12px', height: '12px' }}
        className="audio-output"
      />
      <div className="output-label">OUT</div>

      {/* Module Footer */}
      <div className="module-footer">
        <div className="hp-marking">12HP</div>
      </div>
    </div>
  );
};

export default VCONode;