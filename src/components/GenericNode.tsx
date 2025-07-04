import React, { useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface GenericNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

// ノードタイプに基づく色とアイコンの決定
const getNodeStyle = (nodeType: string) => {
  // _refactored suffixを除去して基本タイプを取得
  const baseType = nodeType.replace('_refactored', '');
  
  const styles = {
    oscillator: {
      gradient: 'linear-gradient(135deg, #f8f9fa 0%, #f1f3f4 100%)',
      borderColor: '#4285f4',
      icon: '🎵'
    },
    sine_oscillator: {
      gradient: 'linear-gradient(135deg, #e0f2fe 0%, #b3e5fc 100%)',
      borderColor: '#0288d1',
      icon: '〜'
    },
    output: {
      gradient: 'linear-gradient(135deg, #fef7e0 0%, #fef3c7 100%)',
      borderColor: '#f59e0b',
      icon: '🔊'
    },
    filter: {
      gradient: 'linear-gradient(135deg, #f3e8ff 0%, #e9d5ff 100%)',
      borderColor: '#8b5cf6',
      icon: '🔧'
    },
    vcf: {
      gradient: 'linear-gradient(135deg, #f3e8ff 0%, #e9d5ff 100%)',
      borderColor: '#8b5cf6',
      icon: '🔧'
    },
    lfo: {
      gradient: 'linear-gradient(135deg, #ecfdf5 0%, #d1fae5 100%)',
      borderColor: '#10b981',
      icon: '🌊'
    },
    adsr: {
      gradient: 'linear-gradient(135deg, #fef2f2 0%, #fecaca 100%)',
      borderColor: '#ef4444',
      icon: '📈'
    },
    mixer: {
      gradient: 'linear-gradient(135deg, #f0f9ff 0%, #dbeafe 100%)',
      borderColor: '#3b82f6',
      icon: '🎚️'
    },
    delay: {
      gradient: 'linear-gradient(135deg, #fffbeb 0%, #fef3c7 100%)',
      borderColor: '#f59e0b',
      icon: '⏱️'
    },
    noise: {
      gradient: 'linear-gradient(135deg, #f9fafb 0%, #f3f4f6 100%)',
      borderColor: '#6b7280',
      icon: '📺'
    },
    vca: {
      gradient: 'linear-gradient(135deg, #f0fdf4 0%, #dcfce7 100%)',
      borderColor: '#22c55e',
      icon: '🔊'
    },
    sequencer: {
      gradient: 'linear-gradient(135deg, #fef2f2 0%, #fecaca 100%)',
      borderColor: '#ef4444',
      icon: '🎹'
    },
    spectrum_analyzer: {
      gradient: 'linear-gradient(135deg, #f1f5f9 0%, #e2e8f0 100%)',
      borderColor: '#64748b',
      icon: '📊'
    },
    oscilloscope: {
      gradient: 'linear-gradient(135deg, #f1f5f9 0%, #e2e8f0 100%)',
      borderColor: '#64748b',
      icon: '📈'
    },
    ring_modulator: {
      gradient: 'linear-gradient(135deg, #fdf4ff 0%, #fae8ff 100%)',
      borderColor: '#a855f7',
      icon: '💍'
    },
    sample_hold: {
      gradient: 'linear-gradient(135deg, #fffbeb 0%, #fef3c7 100%)',
      borderColor: '#f59e0b',
      icon: '📦'
    },
    attenuverter: {
      gradient: 'linear-gradient(135deg, #f0fdf4 0%, #dcfce7 100%)',
      borderColor: '#22c55e',
      icon: '⚡'
    },
    multiple: {
      gradient: 'linear-gradient(135deg, #eff6ff 0%, #dbeafe 100%)',
      borderColor: '#3b82f6',
      icon: '🔗'
    },
    quantizer: {
      gradient: 'linear-gradient(135deg, #fefce8 0%, #fef3c7 100%)',
      borderColor: '#eab308',
      icon: '🎯'
    },
    compressor: {
      gradient: 'linear-gradient(135deg, #fef2f2 0%, #fecaca 100%)',
      borderColor: '#ef4444',
      icon: '🗜️'
    },
    waveshaper: {
      gradient: 'linear-gradient(135deg, #fdf4ff 0%, #fae8ff 100%)',
      borderColor: '#a855f7',
      icon: '🌊'
    },
    clock_divider: {
      gradient: 'linear-gradient(135deg, #f1f5f9 0%, #e2e8f0 100%)',
      borderColor: '#64748b',
      icon: '⏰'
    },
    default: {
      gradient: 'linear-gradient(135deg, #ffffff 0%, #f8f9fa 100%)',
      borderColor: '#9ca3af',
      icon: '⚙️'
    }
  };
  return styles[baseType as keyof typeof styles] || styles.default;
};

// ポートタイプに基づく色決定（ケーブルカラーコードと同じ）
const getPortColor = (portName: string, portType: string) => {
  const name = portName.toLowerCase();
  const type = portType.toLowerCase();
  
  let color = '#888888'; // Default color
  let reason = 'default';
  
  // 具体的なポート名に基づく色分け
  if (name === 'frequency_cv' || name.startsWith('frequency')) {
    color = '#8844ff'; // Purple for frequency
    reason = 'frequency';
  }
  else if (name === 'amplitude_cv' || name.includes('amplitude')) {
    color = '#4444ff'; // Blue for CV
    reason = 'amplitude CV';
  }
  else if (name === 'phase_offset_cv' || name.includes('phase')) {
    color = '#4444ff'; // Blue for CV
    reason = 'phase CV';
  }
  else if (name === 'sync_in' || name.includes('sync')) {
    color = '#ff8844'; // Orange for sync
    reason = 'sync';
  }
  else if (name === 'waveform_cv' || name.includes('waveform')) {
    color = '#4444ff'; // Blue for CV
    reason = 'waveform CV';
  }
  else if (name === 'pulse_width_cv' || name.includes('pulse_width')) {
    color = '#4444ff'; // Blue for CV
    reason = 'pulse width CV';
  }
  else if (name.includes('audio') || name.endsWith('_out') || name === 'out') {
    color = '#ff4444'; // Red for audio
    reason = 'audio';
  }
  else if (name.endsWith('_cv') || name.includes('cv')) {
    color = '#4444ff'; // Blue for CV
    reason = 'general CV';
  }
  else if (name.includes('gate') || name.includes('trigger')) {
    color = '#44ff44'; // Green for gates
    reason = 'gate/trigger';
  }
  else if (name.includes('clock')) {
    color = '#ff8844'; // Orange for clock
    reason = 'clock';
  }
  
  // デバッグログ（必要に応じて有効化）
  // console.log(`Port color: "${portName}" (${portType}) -> ${color} (${reason})`);
  return color;
};

const GenericNode: React.FC<GenericNodeProps> = ({ id, data, selected }) => {
  const [isActive, setIsActive] = useState(true);
  const nodeStyle = getNodeStyle(data.nodeType);
  
  // デバッグ用ログ
  console.log('GenericNode rendering:', { 
    id, 
    nodeType: data.nodeType, 
    label: data.label,
    inputPorts: data.inputPorts,
    outputPorts: data.outputPorts
  });

  // ノードの高さを動的に計算（ポート間隔を24pxに変更）
  const maxPorts = Math.max(data.inputPorts.length, data.outputPorts.length);
  const calculatedHeight = Math.max(120, 80 + (maxPorts * 24));

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

  const toggleActive = useCallback(async () => {
    try {
      const newActiveState = !isActive;
      await updateParameter('active', newActiveState ? 1.0 : 0.0);
      setIsActive(newActiveState);
    } catch (error) {
      console.error('Failed to toggle active state:', error);
    }
  }, [isActive, updateParameter]);

  return (
    <div 
      className="orbital-node" 
      style={{
        background: nodeStyle.gradient,
        borderLeft: `4px solid ${nodeStyle.borderColor}`,
        border: `1px solid ${nodeStyle.borderColor}`,
        borderRadius: '8px',
        width: '280px',
        minHeight: `${calculatedHeight}px`,
        position: 'relative',
        padding: '12px',
        boxShadow: selected ? `0 0 0 2px ${nodeStyle.borderColor}` : '0 2px 4px rgba(0,0,0,0.1)'
      }}
    >
      {/* Input handles - 左側 */}
      {data.inputPorts.map((port, index) => (
        <Handle
          key={`input-${port.name}`}
          type="target"
          position={Position.Left}
          id={port.name}
          className="custom-port-handle"
          style={{ 
            top: `${40 + (index * 24)}px`,
            left: '-8px',
            width: '16px',
            height: '16px',
            borderRadius: '50%',
            '--port-bg-color': getPortColor(port.name, port.port_type),
            backgroundColor: getPortColor(port.name, port.port_type),
            border: '2px solid #fff',
            zIndex: 10
          } as React.CSSProperties}
          title={`${port.name} (${port.port_type})`}
        />
      ))}

      {/* Output handles - 右側 */}
      {data.outputPorts.map((port, index) => (
        <Handle
          key={`output-${port.name}`}
          type="source"
          position={Position.Right}
          id={port.name}
          className="custom-port-handle"
          style={{ 
            top: `${40 + (index * 24)}px`,
            right: '-8px',
            width: '16px',
            height: '16px',
            borderRadius: '50%',
            '--port-bg-color': getPortColor(port.name, port.port_type),
            backgroundColor: getPortColor(port.name, port.port_type),
            border: '2px solid #fff',
            zIndex: 10
          } as React.CSSProperties}
          title={`${port.name} (${port.port_type})`}
        />
      ))}

      {/* ヘッダー - ドラッグハンドル */}
      <div className="node-header drag-handle">
        <div className="node-title">
          {nodeStyle.icon} {data.label}
        </div>
        <button 
          className={`power-button ${isActive ? 'active' : 'inactive'}`}
          onClick={toggleActive}
          title={isActive ? 'Deactivate' : 'Activate'}
        >
          {isActive ? '●' : '○'}
        </button>
      </div>
      
      {/* パラメーター表示 */}
      <div className="node-controls">
        {Object.entries(data.parameters).map(([key, value]) => (
          <div key={key} className="control-group">
            <div className="parameter-display">
              <span className="control-label">{key}</span>
              <span className="control-value">
                {typeof value === 'number' ? value.toFixed(2) : value}
              </span>
            </div>
          </div>
        )).slice(0, 4)} {/* 最大4つのパラメーターを表示 */}
      </div>

    </div>
  );
};

export default GenericNode;