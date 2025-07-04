import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface LFONodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const LFONode: React.FC<LFONodeProps> = ({ id, data }) => {
  const [frequency, setFrequency] = useState(data.parameters?.frequency || 1.0);
  const [amplitude, setAmplitude] = useState(data.parameters?.amplitude || 1.0);
  const [waveform, setWaveform] = useState(data.parameters?.waveform || 0);
  const [phaseOffset, setPhaseOffset] = useState(data.parameters?.phase_offset || 0.0);
  const [isActive, setIsActive] = useState((data.parameters?.active || 1.0) !== 0.0);

  const waveformOptions = [
    { value: 0, label: 'Sine', symbol: '~' },
    { value: 1, label: 'Triangle', symbol: '/\\' },
    { value: 2, label: 'Sawtooth', symbol: '/|' },
    { value: 3, label: 'Square', symbol: '⌐' },
    { value: 4, label: 'Random', symbol: '?' },
  ];

  const updateParameter = useCallback(async (param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        node_id: id,
        param,
        value,
      });
    } catch (error) {
      console.error('Failed to update parameter:', error);
    }
  }, [id]);

  const handleFrequencyChange = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseFloat(e.target.value);
    setFrequency(value);
    await updateParameter('frequency', value);
  }, [updateParameter]);

  const handleAmplitudeChange = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseFloat(e.target.value);
    setAmplitude(value);
    await updateParameter('amplitude', value);
  }, [updateParameter]);

  const handleWaveformChange = useCallback(async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = parseInt(e.target.value);
    setWaveform(value);
    await updateParameter('waveform', value);
  }, [updateParameter]);

  const handlePhaseOffsetChange = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseFloat(e.target.value);
    setPhaseOffset(value);
    await updateParameter('phase_offset', value);
  }, [updateParameter]);

  const toggleActive = useCallback(async () => {
    const newActiveState = !isActive;
    await updateParameter('active', newActiveState ? 1.0 : 0.0);
    setIsActive(newActiveState);
  }, [isActive, updateParameter]);

  return (
    <div className={`eurorack-module lfo-module ${isActive ? 'active' : 'inactive'}`}>
      {/* Module Header - ドラッグハンドル */}
      <div className="module-header drag-handle">
        <div className="module-brand">ORBITAL</div>
        <div className="module-name">LFO</div>
        <div className={`power-led ${isActive ? 'active' : ''}`}></div>
      </div>

      {/* CV Inputs */}
      <Handle
        type="target"
        position={Position.Left}
        id="frequency_cv"
        style={{ top: '25%', background: '#3498db' }}
        className="cv-input"
      />
      <Handle
        type="target"
        position={Position.Left}
        id="amplitude_cv"
        style={{ top: '40%', background: '#2ecc71' }}
        className="cv-input"
      />

      {/* Main Controls */}
      <div 
        className="control-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Frequency Control (Large Knob) */}
        <div className="knob-group large-knob">
          <label className="knob-label">RATE</label>
          <div className="knob-container">
            <input
              type="range"
              min="0.01"
              max="20"
              step="0.01"
              value={frequency}
              onChange={handleFrequencyChange}
              className="rate-knob"
            />
            <div className="knob-value">
              {frequency < 1 ? `${(frequency * 1000).toFixed(0)}m` : `${frequency.toFixed(1)}`}Hz
            </div>
          </div>
        </div>

        {/* Amplitude Control */}
        <div className="knob-group">
          <label className="knob-label">LEVEL</label>
          <div className="knob-container">
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={amplitude}
              onChange={handleAmplitudeChange}
              className="amplitude-knob"
            />
            <div className="knob-value">{Math.round(amplitude * 100)}%</div>
          </div>
        </div>
      </div>

      {/* Waveform Selector */}
      <div 
        className="waveform-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        <label className="section-label">WAVE</label>
        <div className="lfo-waveform-grid">
          {waveformOptions.map((option, index) => (
            <button
              key={index}
              className={`lfo-wave-btn ${waveform === index ? 'active' : ''}`}
              style={{ 
                backgroundColor: waveform === index ? '#e74c3c' : 'transparent',
                borderColor: '#e74c3c',
                color: waveform === index ? '#fff' : '#e74c3c'
              }}
              onClick={() => handleWaveformChange({ target: { value: index.toString() } } as any)}
              title={option.label}
            >
              <span className="wave-symbol">{option.symbol}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Secondary Controls */}
      <div 
        className="secondary-controls"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Phase Offset */}
        <div className="small-knob-group">
          <label className="small-label">PHASE</label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={phaseOffset}
            onChange={handlePhaseOffsetChange}
            className="small-knob"
          />
          <div className="small-value">{Math.round(phaseOffset * 360)}°</div>
        </div>
      </div>

      {/* CV Input Labels */}
      <div className="cv-labels">
        <div className="cv-label" style={{ top: '22%' }}>RATE</div>
        <div className="cv-label" style={{ top: '37%' }}>LEVEL</div>
      </div>

      {/* CV Outputs */}
      <Handle
        type="source"
        position={Position.Right}
        id="cv_out"
        style={{ top: '40%', background: '#e74c3c', width: '12px', height: '12px' }}
        className="cv-output"
      />
      <div className="output-label" style={{ top: '37%' }}>CV</div>

      <Handle
        type="source"
        position={Position.Right}
        id="bipolar_out"
        style={{ top: '55%', background: '#9b59b6', width: '12px', height: '12px' }}
        className="cv-output"
      />
      <div className="output-label" style={{ top: '52%' }}>BI</div>

      {/* Phase LED indicator */}
      <div className="phase-led" style={{
        backgroundColor: isActive ? '#e74c3c' : '#666',
        opacity: isActive ? 0.8 : 0.3
      }}></div>

      {/* Module Footer */}
      <div className="module-footer">
        <div className="hp-marking">6HP</div>
      </div>
    </div>
  );
};

export default LFONode;