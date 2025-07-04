import React, { useRef, useEffect, useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface EurorackSpectrumAnalyzerNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

interface SpectrumData {
  magnitudes: number[];
  frequencies: number[];
  peak_frequency: number;
  peak_magnitude: number;
  total_energy: number;
  centroid: number;
}

const EurorackSpectrumAnalyzerNode: React.FC<EurorackSpectrumAnalyzerNodeProps> = ({ id, data, selected }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>();
  
  const [spectrumData, setSpectrumData] = useState<SpectrumData>({
    magnitudes: new Array(512).fill(0),
    frequencies: new Array(512).fill(0),
    peak_frequency: 0,
    peak_magnitude: 0,
    total_energy: 0,
    centroid: 0
  });
  
  const [parameters, setParameters] = useState({
    window_type: data.parameters.window_type || 1, // 0=Rectangular, 1=Hanning, 2=Hamming, 3=Blackman
    smoothing: data.parameters.smoothing || 0.8,
    gain: data.parameters.gain || 1.0,
    active: data.parameters.active || 1,
    ...data.parameters
  });

  const [displayMode, setDisplayMode] = useState<'magnitude' | 'log' | 'db'>('db');
  const [freezeDisplay, setFreezeDisplay] = useState(false);

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

  // Canvas描画関数
  const drawSpectrum = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // 背景をクリア（ダークテーマ）
    ctx.fillStyle = '#0a0a0a';
    ctx.fillRect(0, 0, width, height);

    // グリッド描画
    ctx.strokeStyle = '#333333';
    ctx.lineWidth = 0.5;
    
    // 周波数グリッド（対数スケール）
    const freqLines = [100, 200, 500, 1000, 2000, 5000, 10000, 20000];
    freqLines.forEach(freq => {
      if (freq <= 20000) {
        const x = (Math.log10(freq) - Math.log10(20)) / (Math.log10(20000) - Math.log10(20)) * width;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
        
        // 周波数ラベル
        ctx.fillStyle = '#666';
        ctx.font = '10px Monaco';
        ctx.textAlign = 'center';
        const label = freq >= 1000 ? `${freq/1000}k` : `${freq}`;
        ctx.fillText(label, x, height - 5);
      }
    });

    // dBグリッド
    const dbLines = [-60, -40, -20, -10, -6, -3, 0];
    dbLines.forEach(db => {
      const y = height - ((db + 60) / 60) * height;
      if (y >= 0 && y <= height) {
        ctx.strokeStyle = db === 0 ? '#666' : '#333';
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();
        
        // dBラベル
        ctx.fillStyle = '#666';
        ctx.font = '10px Monaco';
        ctx.textAlign = 'left';
        ctx.fillText(`${db}dB`, 2, y - 2);
      }
    });

    // スペクトラム描画
    const { magnitudes, frequencies } = spectrumData;
    if (magnitudes.length > 1 && !freezeDisplay) {
      ctx.strokeStyle = '#00ff41';
      ctx.lineWidth = 1.5;
      ctx.shadowColor = '#00ff41';
      ctx.shadowBlur = 2;
      
      ctx.beginPath();
      
      for (let i = 1; i < magnitudes.length; i++) {
        const freq = frequencies[i];
        if (freq < 20 || freq > 20000) continue;
        
        // 対数周波数スケール
        const x = (Math.log10(freq) - Math.log10(20)) / (Math.log10(20000) - Math.log10(20)) * width;
        
        // dBスケール変換
        let magnitude = magnitudes[i] * parameters.gain;
        let y: number;
        
        switch (displayMode) {
          case 'db':
            const db = 20 * Math.log10(Math.max(magnitude, 0.000001));
            y = height - ((db + 60) / 60) * height;
            break;
          case 'log':
            y = height - (Math.log10(Math.max(magnitude, 0.000001) + 1) / 6) * height;
            break;
          case 'magnitude':
          default:
            y = height - magnitude * height;
            break;
        }
        
        y = Math.max(0, Math.min(height, y));
        
        if (i === 1) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }
      
      ctx.stroke();
      ctx.shadowBlur = 0;

      // ピーク周波数マーカー
      if (spectrumData.peak_frequency > 0) {
        const peakX = (Math.log10(spectrumData.peak_frequency) - Math.log10(20)) / (Math.log10(20000) - Math.log10(20)) * width;
        ctx.strokeStyle = '#ff4444';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(peakX, 0);
        ctx.lineTo(peakX, height);
        ctx.stroke();
        
        // ピーク周波数ラベル
        ctx.fillStyle = '#ff4444';
        ctx.font = 'bold 12px Monaco';
        ctx.textAlign = 'center';
        ctx.fillText(`${Math.round(spectrumData.peak_frequency)}Hz`, peakX, 20);
      }
    }

    // フリーズ表示時のオーバーレイ
    if (freezeDisplay) {
      ctx.fillStyle = 'rgba(255, 255, 0, 0.1)';
      ctx.fillRect(0, 0, width, height);
      
      ctx.fillStyle = '#ffff00';
      ctx.font = 'bold 14px Monaco';
      ctx.textAlign = 'center';
      ctx.fillText('FREEZE', width / 2, 30);
    }

  }, [spectrumData, parameters, displayMode, freezeDisplay]);

  // データ取得関数
  const fetchSpectrumData = useCallback(async () => {
    if (freezeDisplay || !parameters.active) return;
    
    try {
      const [magnitudes, frequencies] = await Promise.all([
        invoke<number[]>('get_spectrum_data', { node_id: id }),
        invoke<number[]>('get_spectrum_frequencies', { node_id: id })
      ]);
      
      if (magnitudes.length > 0) {
        // ピーク検出
        let peakMagnitude = 0;
        let peakFrequency = 0;
        let totalEnergy = 0;
        let weightedSum = 0;
        
        for (let i = 0; i < magnitudes.length; i++) {
          const magnitude = magnitudes[i];
          const frequency = frequencies[i] || i * 44100 / 2 / magnitudes.length;
          
          if (magnitude > peakMagnitude && frequency >= 20 && frequency <= 20000) {
            peakMagnitude = magnitude;
            peakFrequency = frequency;
          }
          
          totalEnergy += magnitude * magnitude;
          weightedSum += magnitude * frequency;
        }
        
        const centroid = totalEnergy > 0 ? weightedSum / totalEnergy : 0;
        
        setSpectrumData({
          magnitudes,
          frequencies,
          peak_frequency: peakFrequency,
          peak_magnitude: peakMagnitude,
          total_energy: Math.sqrt(totalEnergy),
          centroid
        });
      }
    } catch (error) {
      // API エラーは静かに処理（ノードがまだ準備できていない可能性）
    }
  }, [id, freezeDisplay, parameters.active]);

  // アニメーションループ
  const animate = useCallback(() => {
    drawSpectrum();
    fetchSpectrumData();
    animationRef.current = requestAnimationFrame(animate);
  }, [drawSpectrum, fetchSpectrumData]);

  // Canvas初期化とアニメーション開始
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    // Canvas高DPI対応
    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    
    const ctx = canvas.getContext('2d');
    if (ctx) {
      ctx.scale(dpr, dpr);
    }

    // アニメーション開始
    animate();

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [animate]);

  const windowTypes = ['RECT', 'HANN', 'HAMM', 'BLCK'];

  return (
    <div className={`eurorack-module spectrum-analyzer-module ${selected ? 'selected' : ''}`}>
      {/* Module Header - ドラッグハンドル */}
      <div className="module-header drag-handle">
        <div className="module-brand">ORBITAL</div>
        <div className="module-name">SPECTRUM</div>
        <div className="power-led active"></div>
      </div>

      {/* Audio Input */}
      <Handle
        type="target"
        position={Position.Left}
        id="audio_in"
        style={{ top: '15%', background: '#00ff41', width: '12px', height: '12px' }}
        className="audio-input"
      />
      <div className="input-label" style={{ top: '12%' }}>IN</div>

      {/* Control Panel */}
      <div 
        className="spectrum-controls"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        {/* Window Type */}
        <div className="control-group">
          <label className="control-label">WINDOW</label>
          <select 
            value={parameters.window_type}
            onChange={(e) => updateParameter('window_type', parseInt(e.target.value))}
            className="spectrum-select"
          >
            {windowTypes.map((type, index) => (
              <option key={index} value={index}>{type}</option>
            ))}
          </select>
        </div>

        {/* Smoothing */}
        <div className="control-group">
          <label className="control-label">SMOOTH</label>
          <input
            type="range"
            min={0}
            max={0.99}
            step={0.01}
            value={parameters.smoothing}
            onChange={(e) => updateParameter('smoothing', parseFloat(e.target.value))}
            className="spectrum-slider"
          />
          <span className="control-value">{Math.round(parameters.smoothing * 100)}%</span>
        </div>

        {/* Gain */}
        <div className="control-group">
          <label className="control-label">GAIN</label>
          <input
            type="range"
            min={0.1}
            max={10}
            step={0.1}
            value={parameters.gain}
            onChange={(e) => updateParameter('gain', parseFloat(e.target.value))}
            className="spectrum-slider"
          />
          <span className="control-value">{parameters.gain.toFixed(1)}x</span>
        </div>
      </div>

      {/* Display Mode Buttons */}
      <div 
        className="display-mode-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        <label className="section-label">MODE</label>
        <div className="mode-buttons">
          {(['db', 'log', 'magnitude'] as const).map((mode) => (
            <button
              key={mode}
              className={`mode-btn ${displayMode === mode ? 'active' : ''}`}
              onClick={() => setDisplayMode(mode)}
            >
              {mode.toUpperCase()}
            </button>
          ))}
        </div>
      </div>

      {/* Spectrum Display */}
      <div className="spectrum-display">
        <canvas
          ref={canvasRef}
          style={{ 
            width: '100%', 
            height: '200px',
            border: '2px solid #333',
            borderRadius: '4px',
            background: '#0a0a0a'
          }}
        />
      </div>

      {/* Freeze Button */}
      <div 
        className="freeze-section"
        onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
      >
        <button
          className={`freeze-button ${freezeDisplay ? 'active' : ''}`}
          onClick={() => setFreezeDisplay(!freezeDisplay)}
        >
          {freezeDisplay ? 'UNFREEZE' : 'FREEZE'}
        </button>
      </div>

      {/* Measurements Display */}
      <div className="spectrum-measurements">
        <div className="measurement-row">
          <span className="measure-label">Peak:</span>
          <span className="measure-value">
            {spectrumData.peak_frequency > 0 
              ? `${Math.round(spectrumData.peak_frequency)}Hz` 
              : '--'}
          </span>
        </div>
        <div className="measurement-row">
          <span className="measure-label">Energy:</span>
          <span className="measure-value">
            {(spectrumData.total_energy * 100).toFixed(1)}%
          </span>
        </div>
        <div className="measurement-row">
          <span className="measure-label">Centroid:</span>
          <span className="measure-value">
            {spectrumData.centroid > 0 
              ? `${Math.round(spectrumData.centroid)}Hz` 
              : '--'}
          </span>
        </div>
      </div>

      {/* Audio Output (pass-through) */}
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out"
        style={{ top: '15%', background: '#00ff41', width: '12px', height: '12px' }}
        className="audio-output"
      />
      <div className="output-label" style={{ top: '12%' }}>OUT</div>

      {/* Module Footer */}
      <div className="module-footer">
        <div className="hp-marking">14HP</div>
      </div>
    </div>
  );
};

export default EurorackSpectrumAnalyzerNode;