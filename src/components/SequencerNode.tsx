import React, { useState, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface SequencerNodeProps {
  id: string;
  data: {
    label: string;
    parameters: Record<string, number>;
  };
}

interface Step {
  note: number;
  gate: boolean;
  velocity: number;
}

const SequencerNode: React.FC<SequencerNodeProps> = ({ id, data }) => {
  const [bpm, setBpm] = useState(data.parameters?.bpm || 120);
  const [stepCount, setStepCount] = useState(data.parameters?.step_count || 8);
  const [currentStep, setCurrentStep] = useState(data.parameters?.current_step || 0);
  const [running, setRunning] = useState((data.parameters?.running || 0) !== 0);
  const [active, setActive] = useState((data.parameters?.active || 1) !== 0);
  
  const [steps, setSteps] = useState<Step[]>(() => {
    const initialSteps: Step[] = [];
    const notes = [261.63, 293.66, 329.63, 349.23, 392.00, 440.00, 493.88, 523.25]; // C4 to C5
    
    for (let i = 0; i < 16; i++) {
      initialSteps.push({
        note: notes[i % notes.length],
        gate: i < stepCount,
        velocity: 0.8,
      });
    }
    return initialSteps;
  });

  useEffect(() => {
    setBpm(data.parameters?.bpm || 120);
    setStepCount(data.parameters?.step_count || 8);
    setCurrentStep(data.parameters?.current_step || 0);
    setRunning((data.parameters?.running || 0) !== 0);
    setActive((data.parameters?.active || 1) !== 0);
    
    // Load step data from parameters
    const newSteps = [...steps];
    for (let i = 0; i < 16; i++) {
      if (data.parameters?.[`step_${i}_note`] !== undefined) {
        newSteps[i].note = data.parameters[`step_${i}_note`];
      }
      if (data.parameters?.[`step_${i}_gate`] !== undefined) {
        newSteps[i].gate = data.parameters[`step_${i}_gate`] !== 0;
      }
      if (data.parameters?.[`step_${i}_velocity`] !== undefined) {
        newSteps[i].velocity = data.parameters[`step_${i}_velocity`];
      }
    }
    setSteps(newSteps);
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

  const handleBpmChange = (value: number) => {
    setBpm(value);
    updateParameter('bpm', value);
  };

  const handleStepCountChange = (value: number) => {
    setStepCount(value);
    updateParameter('step_count', value);
  };

  const toggleRunning = () => {
    const newRunning = !running;
    setRunning(newRunning);
    updateParameter('running', newRunning ? 1 : 0);
  };

  const resetSequencer = () => {
    updateParameter('reset', 1);
    setCurrentStep(0);
  };

  const toggleActive = () => {
    const newActive = !active;
    setActive(newActive);
    updateParameter('active', newActive ? 1 : 0);
  };

  const toggleStepGate = (stepIndex: number) => {
    const newSteps = [...steps];
    newSteps[stepIndex].gate = !newSteps[stepIndex].gate;
    setSteps(newSteps);
    updateParameter(`step_${stepIndex}_gate`, newSteps[stepIndex].gate ? 1 : 0);
  };

  const updateStepNote = (stepIndex: number, note: number) => {
    const newSteps = [...steps];
    newSteps[stepIndex].note = note;
    setSteps(newSteps);
    updateParameter(`step_${stepIndex}_note`, note);
  };

  const updateStepVelocity = (stepIndex: number, velocity: number) => {
    const newSteps = [...steps];
    newSteps[stepIndex].velocity = velocity;
    setSteps(newSteps);
    updateParameter(`step_${stepIndex}_velocity`, velocity);
  };

  // Note names for display
  const getNoteDisplay = (freq: number) => {
    const notes = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
    const c4 = 261.63;
    const semitones = Math.round(12 * Math.log2(freq / c4));
    const octave = Math.floor(semitones / 12) + 4;
    const note = notes[((semitones % 12) + 12) % 12];
    return `${note}${octave}`;
  };

  const noteFrequencies = [
    261.63, 277.18, 293.66, 311.13, 329.63, 349.23, 369.99, 392.00, 415.30, 440.00, 466.16, 493.88, // C4-B4
    523.25, 554.37, 587.33, 622.25, 659.25, 698.46, 739.99, 783.99, 830.61, 880.00, 932.33, 987.77  // C5-B5
  ];

  return (
    <div className="sequencer-node">
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

      {/* Transport Controls */}
      <div className="sequencer-transport">
        <button 
          className={`transport-button ${running ? 'running' : 'stopped'}`}
          onClick={toggleRunning}
          title={running ? 'Stop sequencer' : 'Start sequencer'}
        >
          {running ? '⏸' : '▶'}
        </button>
        
        <button 
          className="transport-button reset"
          onClick={resetSequencer}
          title="Reset to first step"
        >
          ⏹
        </button>

        <div className="bpm-control">
          <label>BPM</label>
          <input
            type="number"
            min="60"
            max="200"
            value={bpm}
            onChange={(e) => handleBpmChange(Number(e.target.value))}
            className="bpm-input"
          />
        </div>

        <div className="step-count-control">
          <label>Steps</label>
          <input
            type="number"
            min="1"
            max="16"
            value={stepCount}
            onChange={(e) => handleStepCountChange(Number(e.target.value))}
            className="step-count-input"
          />
        </div>
      </div>

      {/* Step Display */}
      <div className="step-indicator">
        Current: {currentStep + 1}/{stepCount}
      </div>

      {/* Step Grid */}
      <div className="step-grid">
        {steps.slice(0, stepCount).map((step, index) => (
          <div
            key={index}
            className={`step ${index === currentStep ? 'current' : ''} ${step.gate ? 'active' : 'inactive'}`}
          >
            <div className="step-number">{index + 1}</div>
            
            <button
              className={`gate-button ${step.gate ? 'on' : 'off'}`}
              onClick={() => toggleStepGate(index)}
              title={`Toggle step ${index + 1} gate`}
            >
              {step.gate ? '●' : '○'}
            </button>

            <select
              value={step.note}
              onChange={(e) => updateStepNote(index, Number(e.target.value))}
              className="note-select"
              disabled={!step.gate}
            >
              {noteFrequencies.map((freq, noteIndex) => (
                <option key={noteIndex} value={freq}>
                  {getNoteDisplay(freq)}
                </option>
              ))}
            </select>

            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={step.velocity}
              onChange={(e) => updateStepVelocity(index, Number(e.target.value))}
              className="velocity-slider"
              disabled={!step.gate}
              title={`Velocity: ${Math.round(step.velocity * 100)}%`}
            />

            <div className="velocity-display">
              {Math.round(step.velocity * 100)}%
            </div>
          </div>
        ))}
      </div>

      {/* Output Handles */}
      <Handle
        type="source"
        position={Position.Right}
        id="note_cv"
        style={{ top: '30%', background: '#3498db' }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="gate_out"
        style={{ top: '50%', background: '#e74c3c' }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="velocity_cv"
        style={{ top: '70%', background: '#f39c12' }}
      />

      {/* Port Labels */}
      <div className="port-labels">
        <div className="output-labels">
          <div style={{ top: '30%' }}>Note CV</div>
          <div style={{ top: '50%' }}>Gate</div>
          <div style={{ top: '70%' }}>Velocity</div>
        </div>
      </div>
    </div>
  );
};

export default SequencerNode;