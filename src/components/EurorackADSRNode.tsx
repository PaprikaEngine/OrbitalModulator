import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface EurorackADSRNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const EurorackADSRNode: React.FC<EurorackADSRNodeProps> = ({ id, data, selected }) => {
  const [parameters, setParameters] = useState({
    attack: data.parameters.attack || 0.1,
    decay: data.parameters.decay || 0.3,
    sustain: data.parameters.sustain || 0.7,
    release: data.parameters.release || 0.5,
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

  const triggerGate = useCallback(async () => {
    try {
      await invoke('trigger_gate', { request: { node_id: id } });
    } catch (error) {
      console.error('Failed to trigger gate:', error);
    }
  }, [id]);

  const formatTime = (timeSeconds: number) => {
    if (timeSeconds < 0.001) return `${(timeSeconds * 1000000).toFixed(0)}μs`;
    if (timeSeconds < 1) return `${(timeSeconds * 1000).toFixed(0)}ms`;
    return `${timeSeconds.toFixed(2)}s`;
  };

  return (
    <div className={`eurorack-module adsr-module ${selected ? 'selected' : ''}`}>
      {/* Module Header */}
      <div className="module-header">
        <div className="module-brand">ORBITAL</div>
        <div className="module-name">ADSR</div>
        <div className="power-led active"></div>
      </div>

      {/* Gate Input */}
      <Handle
        type="target"
        position={Position.Left}
        id="gate_in"
        style={{ top: '15%', background: '#e67e22', width: '10px', height: '10px' }}
        className="cv-input"
      />
      <div className="input-label" style={{ top: '12%' }}>GATE</div>

      {/* CV Inputs */}
      <Handle
        type="target"
        position={Position.Left}
        id="velocity_in"
        style={{ top: '30%', background: '#9b59b6' }}
        className="cv-input"
      />

      {/* ADSR Controls in vertical layout */}
      <div className="adsr-controls">
        {/* Attack */}
        <div className="adsr-knob-group">
          <label className="adsr-label">A</label>
          <div className="adsr-knob-container">
            <input
              type="range"
              min={0.001}
              max={10}
              step={0.001}
              value={parameters.attack}
              onChange={(e) => updateParameter('attack', parseFloat(e.target.value))}
              className="adsr-knob attack-knob"
            />
            <div className="adsr-value">{formatTime(parameters.attack)}</div>
          </div>
        </div>

        {/* Decay */}
        <div className="adsr-knob-group">
          <label className="adsr-label">D</label>
          <div className="adsr-knob-container">
            <input
              type="range"
              min={0.001}
              max={10}
              step={0.001}
              value={parameters.decay}
              onChange={(e) => updateParameter('decay', parseFloat(e.target.value))}
              className="adsr-knob decay-knob"
            />
            <div className="adsr-value">{formatTime(parameters.decay)}</div>
          </div>
        </div>

        {/* Sustain */}
        <div className="adsr-knob-group">
          <label className="adsr-label">S</label>
          <div className="adsr-knob-container">
            <input
              type="range"
              min={0}
              max={1}
              step={0.01}
              value={parameters.sustain}
              onChange={(e) => updateParameter('sustain', parseFloat(e.target.value))}
              className="adsr-knob sustain-knob"
            />
            <div className="adsr-value">{Math.round(parameters.sustain * 100)}%</div>
          </div>
        </div>

        {/* Release */}
        <div className="adsr-knob-group">
          <label className="adsr-label">R</label>
          <div className="adsr-knob-container">
            <input
              type="range"
              min={0.001}
              max={10}
              step={0.001}
              value={parameters.release}
              onChange={(e) => updateParameter('release', parseFloat(e.target.value))}
              className="adsr-knob release-knob"
            />
            <div className="adsr-value">{formatTime(parameters.release)}</div>
          </div>
        </div>
      </div>

      {/* Manual Trigger Button */}
      <div className="trigger-section">
        <button 
          className="trigger-button"
          onClick={triggerGate}
        >
          TRIG
        </button>
      </div>

      {/* CV Input Labels */}
      <div className="cv-labels">
        <div className="cv-label" style={{ top: '27%' }}>VEL</div>
      </div>

      {/* CV Output */}
      <Handle
        type="source"
        position={Position.Right}
        id="cv_out"
        style={{ top: '50%', background: '#e74c3c' }}
        className="cv-output"
      />
      <div className="output-label" style={{ top: '47%' }}>CV</div>

      {/* Gate Output */}
      <Handle
        type="source"
        position={Position.Right}
        id="gate_out"
        style={{ top: '65%', background: '#e67e22' }}
        className="cv-output"
      />
      <div className="output-label" style={{ top: '62%' }}>GATE</div>

      {/* Module Footer */}
      <div className="module-footer">
        <div className="hp-marking">8HP</div>
      </div>
    </div>
  );
};

export default EurorackADSRNode;