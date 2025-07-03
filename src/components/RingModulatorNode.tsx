import React, { useState, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface RingModulatorNodeProps {
  id: string;
  data: {
    label: string;
    parameters: {
      mix: number;
      carrier_gain: number;
      modulator_gain: number;
      active: number;
    };
  };
}

const RingModulatorNode: React.FC<RingModulatorNodeProps> = ({ id, data }) => {
  const [mix, setMix] = useState(data.parameters?.mix || 1.0);
  const [carrierGain, setCarrierGain] = useState(data.parameters?.carrier_gain || 1.0);
  const [modulatorGain, setModulatorGain] = useState(data.parameters?.modulator_gain || 1.0);
  const [active, setActive] = useState((data.parameters?.active || 1) !== 0);

  useEffect(() => {
    setMix(data.parameters?.mix || 1.0);
    setCarrierGain(data.parameters?.carrier_gain || 1.0);
    setModulatorGain(data.parameters?.modulator_gain || 1.0);
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

  const handleMixChange = (value: number) => {
    setMix(value);
    updateParameter('mix', value);
  };

  const handleCarrierGainChange = (value: number) => {
    setCarrierGain(value);
    updateParameter('carrier_gain', value);
  };

  const handleModulatorGainChange = (value: number) => {
    setModulatorGain(value);
    updateParameter('modulator_gain', value);
  };

  const toggleActive = () => {
    const newActive = !active;
    setActive(newActive);
    updateParameter('active', newActive ? 1 : 0);
  };

  // Calculate modulation visualization effect
  const modulationEffect = mix * 100;
  const carrierLevel = carrierGain * 50;
  const modulatorLevel = modulatorGain * 50;

  return (
    <div className="ring-modulator-node">
      {/* Input Handles */}
      <Handle
        type="target"
        position={Position.Left}
        id="carrier_in"
        style={{ background: '#e74c3c', top: '30%' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="modulator_in"
        style={{ background: '#3498db', top: '70%' }}
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

      {/* Ring Modulation Visualization */}
      <div className="ring-mod-indicator">
        <div className="modulation-display">
          <div className="signal-bars">
            <div className="signal-bar carrier">
              <div className="bar-label">Carrier</div>
              <div className="level-bar">
                <div 
                  className="level-fill carrier"
                  style={{ 
                    height: `${carrierLevel}%`,
                    background: '#e74c3c'
                  }}
                />
              </div>
            </div>
            <div className="multiplication-symbol">×</div>
            <div className="signal-bar modulator">
              <div className="bar-label">Modulator</div>
              <div className="level-bar">
                <div 
                  className="level-fill modulator"
                  style={{ 
                    height: `${modulatorLevel}%`,
                    background: '#3498db'
                  }}
                />
              </div>
            </div>
            <div className="equals-symbol">=</div>
            <div className="signal-bar output">
              <div className="bar-label">Output</div>
              <div className="level-bar">
                <div 
                  className="level-fill output"
                  style={{ 
                    height: `${modulationEffect}%`,
                    background: active ? '#27ae60' : '#7f8c8d'
                  }}
                />
              </div>
            </div>
          </div>
          <div className="modulation-formula">
            Ring Mod: A × B
          </div>
        </div>
      </div>

      {/* Controls */}
      <div className="ring-mod-controls">
        {/* Mix Control */}
        <div className="control-group">
          <label className="control-label">Mix</label>
          <div className="slider-control">
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={mix}
              onChange={(e) => handleMixChange(Number(e.target.value))}
              className="mix-slider"
            />
            <div className="control-value">{(mix * 100).toFixed(0)}%</div>
          </div>
        </div>

        {/* Carrier Gain Control */}
        <div className="control-group">
          <label className="control-label">Carrier</label>
          <div className="slider-control">
            <input
              type="range"
              min="0"
              max="2"
              step="0.01"
              value={carrierGain}
              onChange={(e) => handleCarrierGainChange(Number(e.target.value))}
              className="carrier-gain-slider"
            />
            <div className="control-value">{carrierGain.toFixed(2)}x</div>
          </div>
        </div>

        {/* Modulator Gain Control */}
        <div className="control-group">
          <label className="control-label">Modulator</label>
          <div className="slider-control">
            <input
              type="range"
              min="0"
              max="2"
              step="0.01"
              value={modulatorGain}
              onChange={(e) => handleModulatorGainChange(Number(e.target.value))}
              className="modulator-gain-slider"
            />
            <div className="control-value">{modulatorGain.toFixed(2)}x</div>
          </div>
        </div>
      </div>

      {/* Output Handle */}
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out"
        style={{ background: '#27ae60' }}
      />

      {/* Port Labels */}
      <div className="port-labels">
        <div className="input-labels">
          <div style={{ top: '30%' }}>Carrier</div>
          <div style={{ top: '70%' }}>Modulator</div>
        </div>
        <div className="output-labels">
          <div>Output</div>
        </div>
      </div>
    </div>
  );
};

export default RingModulatorNode;