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
        node_id: id,
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
    <div className={`eurorack-module delay-module ${active ? 'active' : 'inactive'}`}>
      {/* Module Header - ドラッグハンドル */}
      <div className="module-header drag-handle">
        <div className="module-brand">ORBITAL</div>
        <div className="module-name">DELAY</div>
        <div className={`power-led ${active ? 'active' : ''}`}></div>
      </div>

      {/* Audio Input */}
      <Handle
        type="target"
        position={Position.Left}
        id="audio_in"
        style={{ top: '15%', background: '#e74c3c', width: '12px', height: '12px' }}
        className="audio-input"
      />
      <div className="input-label" style={{ top: '12%' }}>IN</div>

      {/* CV Inputs */}
      <Handle
        type="target"
        position={Position.Left}
        id="delay_time_cv"
        style={{ top: '30%', background: '#3498db' }}
        className="cv-input"
      />
      <Handle
        type="target"
        position={Position.Left}
        id="feedback_cv"
        style={{ top: '45%', background: '#2ecc71' }}
        className="cv-input"
      />
      <Handle
        type="target"
        position={Position.Left}
        id="mix_cv"
        style={{ top: '60%', background: '#f39c12' }}
        className="cv-input"
      />

      {/* Main Controls */}
      <div 
        className="control-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Delay Time Control (Large Knob) */}
        <div className="knob-group large-knob">
          <label className="knob-label">TIME</label>
          <div className="knob-container">
            <input
              type="range"
              min="1"
              max="2000"
              step="1"
              value={delayTime}
              onChange={(e) => handleDelayTimeChange(Number(e.target.value))}
              className="delay-time-knob"
            />
            <div className="knob-value">{formatDelayTime(delayTime)}</div>
          </div>
        </div>

        {/* Feedback Control */}
        <div className="knob-group">
          <label className="knob-label">FEEDBACK</label>
          <div className="knob-container">
            <input
              type="range"
              min="0"
              max="0.95"
              step="0.01"
              value={feedback}
              onChange={(e) => handleFeedbackChange(Number(e.target.value))}
              className="feedback-knob"
            />
            <div className="knob-value">{Math.round(feedback * 100)}%</div>
          </div>
        </div>

        {/* Mix Control */}
        <div className="knob-group">
          <label className="knob-label">MIX</label>
          <div className="knob-container">
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={mix}
              onChange={(e) => handleMixChange(Number(e.target.value))}
              className="mix-knob"
            />
            <div className="knob-value">{Math.round(mix * 100)}%</div>
          </div>
        </div>
      </div>

      {/* Effect Display */}
      <div className="delay-display">
        <div className="time-readout">
          {formatDelayTime(delayTime)}
        </div>
        <div className="effect-meter">
          <div 
            className="meter-fill"
            style={{
              width: `${effectIntensity * 100}%`,
              backgroundColor: getEffectColor(),
            }}
          />
        </div>
      </div>

      {/* CV Input Labels */}
      <div className="cv-labels">
        <div className="cv-label" style={{ top: '27%' }}>TIME</div>
        <div className="cv-label" style={{ top: '42%' }}>FB</div>
        <div className="cv-label" style={{ top: '57%' }}>MIX</div>
      </div>

      {/* Audio Output */}
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out"
        style={{ top: '75%', background: '#e74c3c', width: '12px', height: '12px' }}
        className="audio-output"
      />
      <div className="output-label">OUT</div>

      {/* Module Footer */}
      <div className="module-footer">
        <div className="hp-marking">10HP</div>
      </div>
    </div>
  );
};

export default DelayNode;