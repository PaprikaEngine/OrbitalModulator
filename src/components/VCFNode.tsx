import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface VCFNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const VCFNode: React.FC<VCFNodeProps> = ({ id, data, selected }) => {
  const [parameters, setParameters] = useState({
    cutoff_frequency: data.parameters.cutoff_frequency || 1000,
    resonance: data.parameters.resonance || 1.0,
    filter_type: data.parameters.filter_type || 0, // 0=LP, 1=HP, 2=BP
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

  const filterTypes = ['LP', 'HP', 'BP'];
  const filterColors = ['#e74c3c', '#f39c12', '#9b59b6'];

  return (
    <div className={`eurorack-module vcf-module ${selected ? 'selected' : ''}`}>
      {/* Module Header - ドラッグハンドル */}
      <div className="module-header drag-handle">
        <div className="module-brand">ORBITAL</div>
        <div className="module-name">VCF</div>
        <div className="power-led active"></div>
      </div>

      {/* Audio Input */}
      <Handle
        type="target"
        position={Position.Left}
        id="audio_in"
        style={{ top: '20%', background: '#e74c3c', width: '12px', height: '12px' }}
        className="audio-input"
      />
      <div className="input-label" style={{ top: '17%' }}>IN</div>

      {/* CV Inputs */}
      <Handle
        type="target"
        position={Position.Left}
        id="cutoff_cv"
        style={{ top: '35%', background: '#3498db' }}
        className="cv-input"
      />
      <Handle
        type="target"
        position={Position.Left}
        id="resonance_cv"
        style={{ top: '50%', background: '#2ecc71' }}
        className="cv-input"
      />

      {/* Main Controls */}
      <div 
        className="control-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Cutoff Frequency */}
        <div className="knob-group large-knob">
          <label className="knob-label">CUTOFF</label>
          <div className="knob-container">
            <input
              type="range"
              min={20}
              max={20000}
              step={10}
              value={parameters.cutoff_frequency}
              onChange={(e) => updateParameter('cutoff_frequency', parseFloat(e.target.value))}
              className="cutoff-knob"
            />
            <div className="knob-value">
              {parameters.cutoff_frequency >= 1000 
                ? `${(parameters.cutoff_frequency / 1000).toFixed(1)}k` 
                : `${Math.round(parameters.cutoff_frequency)}`}Hz
            </div>
          </div>
        </div>

        {/* Resonance */}
        <div className="knob-group">
          <label className="knob-label">RES</label>
          <div className="knob-container">
            <input
              type="range"
              min={0.1}
              max={10}
              step={0.1}
              value={parameters.resonance}
              onChange={(e) => updateParameter('resonance', parseFloat(e.target.value))}
              className="resonance-knob"
            />
            <div className="knob-value">{parameters.resonance.toFixed(1)}</div>
          </div>
        </div>
      </div>

      {/* Filter Type Selector */}
      <div 
        className="filter-type-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        <label className="section-label">MODE</label>
        <div className="filter-type-buttons">
          {filterTypes.map((type, index) => (
            <button
              key={index}
              className={`filter-btn ${parameters.filter_type === index ? 'active' : ''}`}
              style={{ 
                backgroundColor: parameters.filter_type === index ? filterColors[index] : 'transparent',
                borderColor: filterColors[index],
                color: parameters.filter_type === index ? '#fff' : filterColors[index]
              }}
              onClick={() => updateParameter('filter_type', index)}
            >
              {type}
            </button>
          ))}
        </div>
      </div>

      {/* CV Input Labels */}
      <div className="cv-labels">
        <div className="cv-label" style={{ top: '32%' }}>1V/OCT</div>
        <div className="cv-label" style={{ top: '47%' }}>RES CV</div>
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
        <div className="hp-marking">8HP</div>
      </div>
    </div>
  );
};

export default VCFNode;