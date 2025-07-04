import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface EurorackLFONodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const EurorackLFONode: React.FC<EurorackLFONodeProps> = ({ id, data, selected }) => {
  const [parameters, setParameters] = useState({
    frequency: data.parameters.frequency || 1.0,
    amplitude: data.parameters.amplitude || 1.0,
    waveform: data.parameters.waveform || 0, // 0=Sine, 1=Triangle, 2=Sawtooth, 3=Square, 4=Random
    offset: data.parameters.offset || 0.0,
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

  const waveformSymbols = ['∼', '△', '⟋', '⌐', '≈'];
  const waveformNames = ['SIN', 'TRI', 'SAW', 'SQR', 'RND'];
  const waveformColors = ['#3498db', '#e74c3c', '#f39c12', '#9b59b6', '#2ecc71'];

  const formatFrequency = (freq: number) => {
    if (freq < 0.1) return `${(freq * 1000).toFixed(0)}mHz`;
    if (freq < 1) return `${freq.toFixed(2)}Hz`;
    return `${freq.toFixed(1)}Hz`;
  };

  return (
    <div className={`eurorack-module lfo-module ${selected ? 'selected' : ''}`}>
      {/* Module Header - ドラッグハンドル */}
      <div className="module-header drag-handle">
        <div className="module-brand">ORBITAL</div>
        <div className="module-name">LFO</div>
        <div className="power-led active"></div>
      </div>

      {/* Rate Control (Large Knob) */}
      <div 
        className="rate-control"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        <label className="rate-label">RATE</label>
        <div className="rate-knob-container">
          <input
            type="range"
            min={0.01}
            max={20}
            step={0.01}
            value={parameters.frequency}
            onChange={(e) => updateParameter('frequency', parseFloat(e.target.value))}
            className="rate-knob"
          />
          <div className="rate-value">{formatFrequency(parameters.frequency)}</div>
        </div>
      </div>

      {/* CV Input for Rate */}
      <Handle
        type="target"
        position={Position.Left}
        id="frequency_cv"
        style={{ top: '35%', background: '#3498db' }}
        className="cv-input"
      />
      <div className="cv-label" style={{ top: '32%' }}>RATE</div>

      {/* Waveform Selector */}
      <div 
        className="waveform-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        <label className="section-label">WAVE</label>
        <div className="lfo-waveform-grid">
          {waveformSymbols.map((symbol, index) => (
            <button
              key={index}
              className={`lfo-wave-btn ${parameters.waveform === index ? 'active' : ''}`}
              style={{ 
                backgroundColor: parameters.waveform === index ? waveformColors[index] : 'transparent',
                borderColor: waveformColors[index],
                color: parameters.waveform === index ? '#fff' : waveformColors[index]
              }}
              onClick={() => updateParameter('waveform', index)}
              title={waveformNames[index]}
            >
              <span className="wave-symbol">{symbol}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Secondary Controls */}
      <div 
        className="secondary-controls"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Amplitude */}
        <div className="small-knob-group">
          <label className="small-label">AMP</label>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            value={parameters.amplitude}
            onChange={(e) => updateParameter('amplitude', parseFloat(e.target.value))}
            className="small-knob"
          />
          <div className="small-value">{Math.round(parameters.amplitude * 100)}%</div>
        </div>

        {/* Offset */}
        <div className="small-knob-group">
          <label className="small-label">OFFS</label>
          <input
            type="range"
            min={-5}
            max={5}
            step={0.1}
            value={parameters.offset}
            onChange={(e) => updateParameter('offset', parseFloat(e.target.value))}
            className="small-knob"
          />
          <div className="small-value">{parameters.offset.toFixed(1)}V</div>
        </div>
      </div>

      {/* CV Outputs */}
      <Handle
        type="source"
        position={Position.Right}
        id="cv_out"
        style={{ top: '40%', background: '#e74c3c' }}
        className="cv-output"
      />
      <div className="output-label" style={{ top: '37%' }}>CV</div>

      <Handle
        type="source"
        position={Position.Right}
        id="bipolar_out"
        style={{ top: '55%', background: '#9b59b6' }}
        className="cv-output"
      />
      <div className="output-label" style={{ top: '52%' }}>BI</div>

      {/* Phase LED indicator */}
      <div className="phase-led" style={{
        backgroundColor: parameters.active ? waveformColors[parameters.waveform] : '#666',
        opacity: parameters.active ? 0.8 : 0.3
      }}></div>

      {/* Module Footer */}
      <div className="module-footer">
        <div className="hp-marking">6HP</div>
      </div>
    </div>
  );
};

export default EurorackLFONode;