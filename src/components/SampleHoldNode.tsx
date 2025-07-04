import React, { useState, useEffect } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface SampleHoldNodeProps {
  id: string;
  data: {
    label: string;
    parameters: {
      trigger_threshold: number;
      held_value: number;
      manual_trigger: number;
      active: number;
    };
  };
}

const SampleHoldNode: React.FC<SampleHoldNodeProps> = ({ id, data }) => {
  const [triggerThreshold, setTriggerThreshold] = useState(data.parameters?.trigger_threshold || 0.1);
  const [heldValue, setHeldValue] = useState(data.parameters?.held_value || 0.0);
  const [active, setActive] = useState((data.parameters?.active || 1) !== 0);
  const [manualTriggerPressed, setManualTriggerPressed] = useState(false);

  useEffect(() => {
    setTriggerThreshold(data.parameters?.trigger_threshold || 0.1);
    setHeldValue(data.parameters?.held_value || 0.0);
    setActive((data.parameters?.active || 1) !== 0);
  }, [data.parameters]);

  // Periodically update held value display
  useEffect(() => {
    const interval = setInterval(async () => {
      if (active) {
        try {
          const value = await invoke<number>('get_node_parameter', {
            node_id: id,
            param: 'held_value',
          });
          setHeldValue(value);
        } catch (error) {
          // Silently handle errors
        }
      }
    }, 100); // Update every 100ms

    return () => clearInterval(interval);
  }, [id, active]);

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

  const handleTriggerThresholdChange = (value: number) => {
    setTriggerThreshold(value);
    updateParameter('trigger_threshold', value);
  };

  const handleManualTrigger = () => {
    setManualTriggerPressed(true);
    updateParameter('manual_trigger', 1);
    
    // Release trigger after short delay
    setTimeout(() => {
      setManualTriggerPressed(false);
      updateParameter('manual_trigger', 0);
    }, 100);
  };

  const toggleActive = () => {
    const newActive = !active;
    setActive(newActive);
    updateParameter('active', newActive ? 1 : 0);
  };

  // Convert held value to display format
  const displayValue = heldValue.toFixed(3);
  const voltageValue = (heldValue * 5).toFixed(2); // Assuming ±5V range

  return (
    <div className="sample-hold-node">
      {/* Input Handles */}
      <Handle
        type="target"
        position={Position.Left}
        id="signal_in"
        style={{ background: '#e74c3c', top: '35%' }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="trigger_in"
        style={{ background: '#f39c12', top: '65%' }}
      />

      {/* Header - ドラッグハンドル */}
      <div className="node-header drag-handle">
        <div className="node-title">{data.label}</div>
        <button
          className={`active-button ${active ? 'active' : 'inactive'}`}
          onClick={toggleActive}
          title={active ? 'Click to deactivate' : 'Click to activate'}
        >
          {active ? 'ON' : 'OFF'}
        </button>
      </div>

      {/* Sample & Hold Display */}
      <div className="sample-hold-display">
        <div className="held-value-display">
          <div className="value-label">Held Value</div>
          <div className="value-main">{displayValue}</div>
          <div className="value-voltage">{voltageValue}V</div>
        </div>
        
        <div className="sample-hold-indicator">
          <div className="signal-flow">
            <div className="input-signal">
              <div className="signal-label">Signal In</div>
              <div className="signal-wave">
                {/* Simple wave visualization */}
                <svg width="40" height="20" viewBox="0 0 40 20">
                  <path
                    d="M0,10 Q10,5 20,10 T40,10"
                    stroke="#e74c3c"
                    strokeWidth="2"
                    fill="none"
                  />
                </svg>
              </div>
            </div>
            
            <div className="sample-symbol">S&H</div>
            
            <div className="output-signal">
              <div className="signal-label">Held Out</div>
              <div className="signal-flat">
                {/* Flat line representing held value */}
                <svg width="40" height="20" viewBox="0 0 40 20">
                  <line
                    x1="0"
                    y1={10 + heldValue * 5}
                    x2="40"
                    y2={10 + heldValue * 5}
                    stroke="#27ae60"
                    strokeWidth="3"
                  />
                </svg>
              </div>
            </div>
          </div>
          
          <div className="trigger-indicator">
            <div className="trigger-label">Trigger</div>
            <div className={`trigger-led ${manualTriggerPressed ? 'triggered' : ''}`}>
              ●
            </div>
          </div>
        </div>
      </div>

      {/* Controls */}
      <div 
        className="sample-hold-controls"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Trigger Threshold */}
        <div className="control-group">
          <label className="control-label">Threshold</label>
          <div className="slider-control">
            <input
              type="range"
              min="0.01"
              max="1.0"
              step="0.01"
              value={triggerThreshold}
              onChange={(e) => handleTriggerThresholdChange(Number(e.target.value))}
              className="threshold-slider"
            />
            <div className="control-value">{(triggerThreshold * 1000).toFixed(0)}mV</div>
          </div>
        </div>

        {/* Manual Trigger Button */}
        <div className="control-group">
          <label className="control-label">Manual</label>
          <button
            className={`trigger-button ${manualTriggerPressed ? 'pressed' : ''}`}
            onMouseDown={handleManualTrigger}
            disabled={!active}
          >
            TRIGGER
          </button>
        </div>
      </div>

      {/* Output Handles */}
      <Handle
        type="source"
        position={Position.Right}
        id="signal_out"
        style={{ background: '#27ae60', top: '35%' }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="trigger_out"
        style={{ background: '#f39c12', top: '65%' }}
      />

      {/* Port Labels */}
      <div className="port-labels">
        <div className="input-labels">
          <div style={{ top: '35%' }}>Signal</div>
          <div style={{ top: '65%' }}>Trigger</div>
        </div>
        <div className="output-labels">
          <div style={{ top: '35%' }}>S&H Out</div>
          <div style={{ top: '65%' }}>Trig Out</div>
        </div>
      </div>
    </div>
  );
};

export default SampleHoldNode;