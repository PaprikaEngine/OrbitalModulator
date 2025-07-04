import React, { useState, useCallback } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface OutputNodeProps {
  id: string;
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const OutputNode: React.FC<OutputNodeProps> = ({ id, data }) => {
  const [masterVolume, setMasterVolume] = useState(data.parameters.master_volume || 0.7);
  const [isMuted, setIsMuted] = useState(data.parameters.mute || false);

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

  const handleVolumeChange = useCallback(async (value: number) => {
    setMasterVolume(value);
    await updateParameter('master_volume', value);
  }, [updateParameter]);

  const toggleMute = useCallback(async () => {
    const newMuteState = !isMuted;
    setIsMuted(newMuteState);
    await updateParameter('mute', newMuteState ? 1.0 : 0.0);
  }, [isMuted, updateParameter]);

  return (
    <div className="node-container output-node">
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

      {/* ヘッダー - ドラッグハンドル */}
      <div className="node-header drag-handle">
        <div className="node-title">🔊 {data.label}</div>
        <button 
          className={`mute-button ${isMuted ? 'muted' : 'unmuted'}`}
          onClick={toggleMute}
          title={isMuted ? 'Unmute' : 'Mute'}
        >
          {isMuted ? '🔇' : '🔊'}
        </button>
      </div>
      
      {/* パラメーター調整UI */}
      <div 
        className="node-controls"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* マスターボリューム */}
        <div className="control-group">
          <label className="control-label">Master Volume</label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={masterVolume}
            onChange={(e) => handleVolumeChange(Number(e.target.value))}
            className="control-slider"
          />
          <span className="control-value">{(masterVolume * 100).toFixed(0)}%</span>
        </div>

        {/* ミュート状態表示 */}
        <div className="control-group">
          <div className={`status-indicator ${isMuted ? 'muted' : 'active'}`}>
            {isMuted ? 'MUTED' : 'ACTIVE'}
          </div>
        </div>
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
      </div>
    </div>
  );
};

export default OutputNode;