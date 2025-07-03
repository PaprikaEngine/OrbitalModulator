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
    <div className="lfo-node">
      {/* Input Handles */}
      <Handle
        type="target"
        position={Position.Left}
        id="frequency_cv"
        style={{ top: '30%', background: '#f39c12' }}
        title="Frequency CV"
      />
      <Handle
        type="target"
        position={Position.Left}
        id="amplitude_cv"
        style={{ top: '70%', background: '#f39c12' }}
        title="Amplitude CV"
      />

      {/* Node Content */}
      <div className="node-header">
        <span className="node-title">LFO</span>
        <button 
          className={`active-button ${isActive ? 'active' : 'inactive'}`}
          onClick={toggleActive}
          title={isActive ? 'Click to deactivate' : 'Click to activate'}
        >
          {isActive ? '●' : '○'}
        </button>
      </div>

      <div className="node-content">
        {/* Waveform Selection */}
        <div className="control-group">
          <label>Wave:</label>
          <select value={waveform} onChange={handleWaveformChange} className="waveform-select">
            {waveformOptions.map(option => (
              <option key={option.value} value={option.value}>
                {option.symbol} {option.label}
              </option>
            ))}
          </select>
        </div>

        {/* Frequency Control */}
        <div className="control-group">
          <label>Freq: {frequency.toFixed(2)}Hz</label>
          <input
            type="range"
            min="0.01"
            max="20"
            step="0.01"
            value={frequency}
            onChange={handleFrequencyChange}
            className="slider"
          />
        </div>

        {/* Amplitude Control */}
        <div className="control-group">
          <label>Amp: {(amplitude * 100).toFixed(0)}%</label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={amplitude}
            onChange={handleAmplitudeChange}
            className="slider"
          />
        </div>

        {/* Phase Offset Control */}
        <div className="control-group">
          <label>Phase: {(phaseOffset * 360).toFixed(0)}°</label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={phaseOffset}
            onChange={handlePhaseOffsetChange}
            className="slider"
          />
        </div>

        {/* LFO Rate Indicator */}
        <div className="lfo-indicator">
          <div 
            className="lfo-pulse"
            style={{
              animationDuration: `${1/frequency}s`,
              animationPlayState: isActive ? 'running' : 'paused'
            }}
          />
          <span className="rate-label">{frequency.toFixed(2)}Hz</span>
        </div>
      </div>

      {/* Output Handle */}
      <Handle
        type="source"
        position={Position.Right}
        id="cv_out"
        style={{ background: '#f39c12' }}
        title="CV Output"
      />
    </div>
  );
};

export default LFONode;