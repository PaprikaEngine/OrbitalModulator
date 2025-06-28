import React, { useState, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface DelayNodeProps {
  id: string;
  data: {
    label: string;
    parameters: {
      delay_time: number;
      feedback: number;
      mix: number;
      active: number;
    };
  };
}

const DelayNode: React.FC<DelayNodeProps> = ({ id, data }) => {
  const [delayTime, setDelayTime] = useState(data.parameters?.delay_time || 250);
  const [feedback, setFeedback] = useState(data.parameters?.feedback || 0.3);
  const [mix, setMix] = useState(data.parameters?.mix || 0.5);
  const [active, setActive] = useState((data.parameters?.active || 1) !== 0);

  useEffect(() => {
    setDelayTime(data.parameters?.delay_time || 250);
    setFeedback(data.parameters?.feedback || 0.3);
    setMix(data.parameters?.mix || 0.5);
    setActive((data.parameters?.active || 1) !== 0);
  }, [data.parameters]);

  const updateParameter = async (param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        nodeId: id,
        param,
        value,
      });
    } catch (error) {
      console.error(`Failed to update ${param}:`, error);
    }
  };

  const handleDelayTimeChange = (value: number) => {
    setDelayTime(value);
    updateParameter('delay_time', value);
  };

  const handleFeedbackChange = (value: number) => {
    setFeedback(value);
    updateParameter('feedback', value);
  };

  const handleMixChange = (value: number) => {
    setMix(value);
    updateParameter('mix', value);
  };

  const toggleActive = () => {
    const newActive = !active;
    setActive(newActive);
    updateParameter('active', newActive ? 1 : 0);
  };

  // Format delay time for display
  const formatDelayTime = (ms: number) => {
    if (ms >= 1000) {
      return `${(ms / 1000).toFixed(2)}s`;
    }
    return `${ms.toFixed(0)}ms`;
  };

  // Calculate visual feedback level for effect indicator
  const effectIntensity = feedback * mix;
  const getEffectColor = () => {
    if (effectIntensity < 0.2) return '#27ae60';
    if (effectIntensity < 0.5) return '#f39c12';
    return '#e74c3c';
  };

  return (
    <div className="delay-node">
      {/* Input Handles */}
      <Handle
        type="target"
        position={Position.Left}
        id="audio_in"
        style={{ top: '30%', background: '#e74c3c' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="delay_time_cv"
        style={{ top: '50%', background: '#3498db' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="feedback_cv"
        style={{ top: '65%', background: '#3498db' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="mix_cv"
        style={{ top: '80%', background: '#3498db' }}
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

      {/* Effect Indicator */}
      <div className="delay-indicator">
        <div className="effect-visual">
          <div 
            className="effect-bar"
            style={{
              background: getEffectColor(),
              width: `${effectIntensity * 100}%`,
            }}
          />
        </div>
        <div className="delay-time-display">
          {formatDelayTime(delayTime)}
        </div>
      </div>

      {/* Controls */}
      <div className="delay-controls">
        {/* Delay Time */}
        <div className="control-group">
          <label className="control-label">Time</label>
          <div className="delay-time-control">
            <input
              type="range"
              min="1"
              max="2000"
              step="1"
              value={delayTime}
              onChange={(e) => handleDelayTimeChange(Number(e.target.value))}
              className="delay-slider"
            />
            <div className="control-value">{formatDelayTime(delayTime)}</div>
          </div>
        </div>

        {/* Feedback */}
        <div className="control-group">
          <label className="control-label">Feedback</label>
          <div className="knob-control">
            <input
              type="range"
              min="0"
              max="0.95"
              step="0.01"
              value={feedback}
              onChange={(e) => handleFeedbackChange(Number(e.target.value))}
              className="feedback-knob"
            />
            <div className="control-value">{(feedback * 100).toFixed(0)}%</div>
          </div>
        </div>

        {/* Mix (Dry/Wet) */}
        <div className="control-group">
          <label className="control-label">Mix</label>
          <div className="knob-control">
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={mix}
              onChange={(e) => handleMixChange(Number(e.target.value))}
              className="mix-knob"
            />
            <div className="control-value">{(mix * 100).toFixed(0)}%</div>
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
          <div style={{ top: '30%' }}>Audio</div>
          <div style={{ top: '50%' }}>Time CV</div>
          <div style={{ top: '65%' }}>FB CV</div>
          <div style={{ top: '80%' }}>Mix CV</div>
        </div>
        <div className="output-labels">
          <div>Audio</div>
        </div>
      </div>
    </div>
  );
};

export default DelayNode;