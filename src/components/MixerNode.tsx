import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface MixerNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

interface ChannelState {
  gain: number;
  pan: number;
}

const MixerNode: React.FC<MixerNodeProps> = ({ id, data }) => {
  // チャンネル数を入力ポートから判定
  const audioInputs = data.inputPorts?.filter(port => port.name.startsWith('audio_in_')) || [];
  const channelCount = audioInputs.length;
  
  // 各チャンネルの状態
  const [channels, setChannels] = useState<ChannelState[]>(() => {
    return Array.from({ length: channelCount }, (_, i) => ({
      gain: data.parameters?.[`gain_${i + 1}`] || 0.7,
      pan: data.parameters?.[`pan_${i + 1}`] || 0.0,
    }));
  });
  
  const [masterGain, setMasterGain] = useState(data.parameters?.master_gain || 0.8);
  const [isActive, setIsActive] = useState((data.parameters?.active || 1.0) !== 0.0);

  const updateParameter = useCallback(async (param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        node_id: id,
        param,
        value,
      });
    } catch (error) {
      console.error('Failed to update parameter:', error);
    }
  }, [id]);

  const handleChannelGainChange = useCallback(async (channelIndex: number, gain: number) => {
    const newChannels = [...channels];
    newChannels[channelIndex].gain = gain;
    setChannels(newChannels);
    await updateParameter(`gain_${channelIndex + 1}`, gain);
  }, [channels, updateParameter]);

  const handleChannelPanChange = useCallback(async (channelIndex: number, pan: number) => {
    const newChannels = [...channels];
    newChannels[channelIndex].pan = pan;
    setChannels(newChannels);
    await updateParameter(`pan_${channelIndex + 1}`, pan);
  }, [channels, updateParameter]);

  const handleMasterGainChange = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseFloat(e.target.value);
    setMasterGain(value);
    await updateParameter('master_gain', value);
  }, [updateParameter]);

  const toggleActive = useCallback(async () => {
    const newActiveState = !isActive;
    await updateParameter('active', newActiveState ? 1.0 : 0.0);
    setIsActive(newActiveState);
  }, [isActive, updateParameter]);

  // VUメーター風の表示
  const getVUMeterStyle = (level: number) => ({
    height: `${level * 100}%`,
    background: level > 0.8 ? '#ff4444' : level > 0.6 ? '#ffaa00' : '#44ff44',
    transition: 'height 0.1s ease',
  });

  return (
    <div className="mixer-node">
      {/* Input Handles */}
      {channels.map((_, index) => (
        <Handle
          key={`audio_in_${index + 1}`}
          type="target"
          position={Position.Left}
          id={`audio_in_${index + 1}`}
          style={{ 
            top: `${15 + (index * 70 / channelCount)}%`, 
            background: '#2ecc71' 
          }}
          title={`Audio Input ${index + 1}`}
        />
      ))}

      {/* CV Handles for gain and pan */}
      {channels.map((_, index) => (
        <React.Fragment key={`cv_${index}`}>
          <Handle
            type="target"
            position={Position.Top}
            id={`gain_cv_${index + 1}`}
            style={{ 
              left: `${20 + (index * 60 / channelCount)}%`, 
              background: '#f39c12' 
            }}
            title={`Gain CV ${index + 1}`}
          />
          <Handle
            type="target"
            position={Position.Top}
            id={`pan_cv_${index + 1}`}
            style={{ 
              left: `${30 + (index * 60 / channelCount)}%`, 
              background: '#f39c12' 
            }}
            title={`Pan CV ${index + 1}`}
          />
        </React.Fragment>
      ))}

      <Handle
        type="target"
        position={Position.Top}
        id="master_gain_cv"
        style={{ left: '85%', background: '#f39c12' }}
        title="Master Gain CV"
      />

      {/* Node Content */}
      <div className="node-header">
        <span className="node-title">MIXER {channelCount}CH</span>
        <button 
          className={`active-button ${isActive ? 'active' : 'inactive'}`}
          onClick={toggleActive}
          title={isActive ? 'Click to deactivate' : 'Click to activate'}
        >
          {isActive ? '●' : '○'}
        </button>
      </div>

      <div className="mixer-content">
        {/* Channel Strips */}
        <div className="channel-strips">
          {channels.map((channel, index) => (
            <div key={index} className="channel-strip">
              <div className="channel-label">CH{index + 1}</div>
              
              {/* VU Meter */}
              <div className="vu-meter">
                <div className="vu-meter-bar">
                  <div 
                    className="vu-meter-fill"
                    style={getVUMeterStyle(channel.gain)}
                  />
                </div>
              </div>
              
              {/* Gain Control */}
              <div className="gain-control">
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={channel.gain}
                  onChange={(e) => handleChannelGainChange(index, parseFloat(e.target.value))}
                  className="vertical-slider"
                  title={`Gain: ${(channel.gain * 100).toFixed(0)}%`}
                />
                <span className="gain-value">{(channel.gain * 100).toFixed(0)}</span>
              </div>
              
              {/* Pan Control */}
              <div className="pan-control">
                <div className="pan-knob">
                  <input
                    type="range"
                    min="-1"
                    max="1"
                    step="0.01"
                    value={channel.pan}
                    onChange={(e) => handleChannelPanChange(index, parseFloat(e.target.value))}
                    className="pan-slider"
                    title={`Pan: ${channel.pan > 0 ? 'R' : channel.pan < 0 ? 'L' : 'C'}${Math.abs(channel.pan * 100).toFixed(0)}`}
                  />
                </div>
                <span className="pan-value">
                  {channel.pan === 0 ? 'C' : 
                   channel.pan > 0 ? `R${(channel.pan * 100).toFixed(0)}` : 
                   `L${(Math.abs(channel.pan) * 100).toFixed(0)}`}
                </span>
              </div>
            </div>
          ))}
        </div>

        {/* Master Section */}
        <div className="master-section">
          <div className="master-label">MASTER</div>
          <div className="master-vu">
            <div className="vu-meter-bar">
              <div 
                className="vu-meter-fill"
                style={getVUMeterStyle(masterGain)}
              />
            </div>
          </div>
          <div className="master-gain-control">
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={masterGain}
              onChange={handleMasterGainChange}
              className="vertical-slider master-slider"
              title={`Master: ${(masterGain * 100).toFixed(0)}%`}
            />
            <span className="gain-value">{(masterGain * 100).toFixed(0)}</span>
          </div>
        </div>
      </div>

      {/* Output Handles */}
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out_l"
        style={{ top: '40%', background: '#2ecc71' }}
        title="Audio Output L"
      />
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out_r"
        style={{ top: '60%', background: '#2ecc71' }}
        title="Audio Output R"
      />
    </div>
  );
};

export default MixerNode;