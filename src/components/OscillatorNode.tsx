import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface OscillatorNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const OscillatorNode: React.FC<OscillatorNodeProps> = ({ id, data }) => {
  const [isActive, setIsActive] = useState(false);
  const [frequency, setFrequency] = useState(data.parameters.frequency || 440);
  const [amplitude, setAmplitude] = useState(data.parameters.amplitude || 0.5);
  const [waveform, setWaveform] = useState(data.parameters.waveform || 0);
  const [pulseWidth, setPulseWidth] = useState(data.parameters.pulse_width || 0.5);
  
  const getWaveformName = (value: number) => {
    const waveforms = ['Sine', 'Triangle', 'Sawtooth', 'Pulse'];
    return waveforms[Math.floor(value)] || 'Unknown';
  };

  const formatFrequency = (freq: number) => {
    if (freq >= 1000) {
      return `${(freq / 1000).toFixed(1)}kHz`;
    }
    return `${freq.toFixed(0)}Hz`;
  };

  const updateParameter = useCallback(async (param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        node_id: id,
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
      console.error('Failed to toggle oscillator active state:', error);
    }
  }, [isActive, updateParameter]);

  const handleFrequencyChange = useCallback(async (value: number) => {
    setFrequency(value);
    await updateParameter('frequency', value);
  }, [updateParameter]);

  const handleAmplitudeChange = useCallback(async (value: number) => {
    setAmplitude(value);
    await updateParameter('amplitude', value);
  }, [updateParameter]);

  const handleWaveformChange = useCallback(async (value: number) => {
    setWaveform(value);
    await updateParameter('waveform', value);
  }, [updateParameter]);

  const handlePulseWidthChange = useCallback(async (value: number) => {
    setPulseWidth(value);
    await updateParameter('pulse_width', value);
  }, [updateParameter]);

  return (
    <div className="node-container oscillator-node">
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
        <div className="node-title">{data.label}</div>
        <button 
          className={`power-button ${isActive ? 'active' : 'inactive'}`}
          onClick={toggleActive}
          title={isActive ? 'Deactivate' : 'Activate'}
        >
          {isActive ? '●' : '○'}
        </button>
      </div>
      
      {/* パラメーター調整UI */}
      <div className="node-controls">
        {/* 周波数 */}
        <div className="control-group">
          <label className="control-label">Frequency</label>
          <input
            type="range"
            min="20"
            max="20000"
            step="1"
            value={frequency}
            onChange={(e) => handleFrequencyChange(Number(e.target.value))}
            className="control-slider"
          />
          <span className="control-value">{formatFrequency(frequency)}</span>
        </div>

        {/* 振幅 */}
        <div className="control-group">
          <label className="control-label">Amplitude</label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={amplitude}
            onChange={(e) => handleAmplitudeChange(Number(e.target.value))}
            className="control-slider"
          />
          <span className="control-value">{(amplitude * 100).toFixed(0)}%</span>
        </div>

        {/* 波形選択 */}
        <div className="control-group">
          <label className="control-label">Waveform</label>
          <select
            value={Math.floor(waveform)}
            onChange={(e) => handleWaveformChange(Number(e.target.value))}
            className="control-select"
          >
            <option value={0}>Sine</option>
            <option value={1}>Triangle</option>
            <option value={2}>Sawtooth</option>
            <option value={3}>Pulse</option>
          </select>
        </div>

        {/* パルス幅（パルス波選択時のみ） */}
        {Math.floor(waveform) === 3 && (
          <div className="control-group">
            <label className="control-label">Pulse Width</label>
            <input
              type="range"
              min="0.1"
              max="0.9"
              step="0.01"
              value={pulseWidth}
              onChange={(e) => handlePulseWidthChange(Number(e.target.value))}
              className="control-slider"
            />
            <span className="control-value">{(pulseWidth * 100).toFixed(0)}%</span>
          </div>
        )}
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

export default OscillatorNode;