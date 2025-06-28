import React, { useRef, useEffect, useState, useCallback } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface OscilloscopeData {
  nodeId: string;
  parameters: {
    time_div: number;
    volt_div: number;
    position_h: number;
    position_v: number;
    trigger_level: number;
  };
}

interface Measurements {
  vpp: number;
  vrms: number;
  frequency: number;
  period: number;
  duty_cycle: number;
}

interface OscilloscopeNodeProps extends NodeProps {
  data: {
    label: string;
    nodeType: string;
    parameters: Record<string, number>;
    inputPorts: Array<{ name: string; port_type: string }>;
    outputPorts: Array<{ name: string; port_type: string }>;
  };
}

const OscilloscopeNode: React.FC<OscilloscopeNodeProps> = ({ id, data }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>();
  const [waveformData, setWaveformData] = useState<Float32Array>(new Float32Array(2048));
  const [measurements, setMeasurements] = useState<Measurements>({
    vpp: 0,
    vrms: 0,
    frequency: 0,
    period: 0,
    duty_cycle: 0,
  });
  
  const [parameters, setParameters] = useState({
    time_div: data.parameters.time_div || 0.01,
    volt_div: data.parameters.volt_div || 1.0,
    position_h: data.parameters.position_h || 0.0,
    position_v: data.parameters.position_v || 0.0,
    trigger_level: data.parameters.trigger_level || 0.0,
  });

  // Canvas描画関数
  const drawOscilloscope = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // CRT風背景
    ctx.fillStyle = '#001100';
    ctx.fillRect(0, 0, width, height);

    // グリッド描画
    ctx.strokeStyle = '#003300';
    ctx.lineWidth = 0.5;
    
    // 垂直グリッド線
    const gridDivisions = 10;
    for (let i = 0; i <= gridDivisions; i++) {
      const x = (i / gridDivisions) * width;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }
    
    // 水平グリッド線
    for (let i = 0; i <= gridDivisions; i++) {
      const y = (i / gridDivisions) * height;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    // 中央線（より明るく）
    ctx.strokeStyle = '#005500';
    ctx.lineWidth = 1;
    
    // 中央垂直線
    ctx.beginPath();
    ctx.moveTo(width / 2, 0);
    ctx.lineTo(width / 2, height);
    ctx.stroke();
    
    // 中央水平線
    ctx.beginPath();
    ctx.moveTo(0, height / 2);
    ctx.lineTo(width, height / 2);
    ctx.stroke();

    // トリガーレベル線
    const triggerY = height / 2 - (parameters.trigger_level / parameters.volt_div) * (height / 8);
    if (triggerY >= 0 && triggerY <= height) {
      ctx.strokeStyle = '#ffff00';
      ctx.lineWidth = 1;
      ctx.setLineDash([5, 5]);
      ctx.beginPath();
      ctx.moveTo(0, triggerY);
      ctx.lineTo(width, triggerY);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // 波形描画
    const hasSignal = waveformData.some(sample => Math.abs(sample) > 0.001);
    
    if (hasSignal && waveformData.length > 1) {
      ctx.strokeStyle = '#00ff00';
      ctx.lineWidth = 2;
      ctx.shadowColor = '#66ff66';
      ctx.shadowBlur = 3;
      
      ctx.beginPath();
      
      const centerY = height / 2;
      const scaleY = (height / 8) / parameters.volt_div; // 8 divisions vertically
      
      for (let i = 0; i < waveformData.length; i++) {
        const x = (i / (waveformData.length - 1)) * width;
        const y = centerY - (waveformData[i] * scaleY) + (parameters.position_v * height * 0.1);
        
        if (i === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }
      
      ctx.stroke();
      ctx.shadowBlur = 0;
    } else {
      // 信号がない場合のメッセージ表示
      ctx.fillStyle = '#666666';
      ctx.font = '16px -apple-system, BlinkMacSystemFont, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText('No Signal - Connect audio input', width / 2, height / 2);
    }

  }, [waveformData, parameters]);

  // データ取得関数
  const fetchWaveformData = useCallback(async () => {
    try {
      // TauriのAPIを呼び出して実際の波形データを取得
      const data = await invoke('get_oscilloscope_data', { nodeId: id }) as {
        waveform: number[];
        measurements: {
          vpp: number;
          vrms: number;
          frequency: number;
          period: number;
          duty_cycle: number;
        };
      };
      
      // 実際のデータがある場合のみ使用
      if (data.waveform && data.waveform.length > 0) {
        setWaveformData(new Float32Array(data.waveform));
        setMeasurements(data.measurements);
      } else {
        // データがない場合は空の波形を表示
        setWaveformData(new Float32Array(512)); // 全て0の配列
        setMeasurements({
          vpp: 0,
          vrms: 0,
          frequency: 0,
          period: 0,
          duty_cycle: 0,
        });
      }
    } catch (error) {
      // API呼び出しが失敗した場合は空の波形を表示
      setWaveformData(new Float32Array(512)); // 全て0の配列
      setMeasurements({
        vpp: 0,
        vrms: 0,
        frequency: 0,
        period: 0,
        duty_cycle: 0,
      });
    }
  }, [id]);

  // アニメーションループ
  const animate = useCallback(() => {
    drawOscilloscope();
    fetchWaveformData();
    animationRef.current = requestAnimationFrame(animate);
  }, [drawOscilloscope, fetchWaveformData]);

  // パラメーター更新
  const updateParameter = useCallback(async (param: string, value: number) => {
    try {
      await invoke('set_node_parameter', {
        nodeId: id,
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

  return (
    <div className="oscilloscope-node">
      {/* 入力ハンドル */}
      <Handle
        type="target"
        position={Position.Left}
        id="audio_in"
        style={{ top: '50%', background: '#00ff00' }}
      />

      {/* オシロスコープ本体 */}
      <div className="oscilloscope-container">
        {/* コントロールパネル */}
        <div 
          className="control-panel"
          onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
        >
          <div className="control-section">
            <label>VOLT/DIV</label>
            <select 
              value={parameters.volt_div}
              onChange={(e) => updateParameter('volt_div', parseFloat(e.target.value))}
            >
              <option value={0.1}>0.1V</option>
              <option value={0.2}>0.2V</option>
              <option value={0.5}>0.5V</option>
              <option value={1.0}>1.0V</option>
              <option value={2.0}>2.0V</option>
              <option value={5.0}>5.0V</option>
            </select>
          </div>
          
          <div className="control-section">
            <label>TIME/DIV</label>
            <select 
              value={parameters.time_div}
              onChange={(e) => updateParameter('time_div', parseFloat(e.target.value))}
            >
              <option value={0.0001}>0.1ms</option>
              <option value={0.0002}>0.2ms</option>
              <option value={0.0005}>0.5ms</option>
              <option value={0.001}>1ms</option>
              <option value={0.002}>2ms</option>
              <option value={0.005}>5ms</option>
              <option value={0.01}>10ms</option>
              <option value={0.02}>20ms</option>
              <option value={0.05}>50ms</option>
            </select>
          </div>

          <div className="control-section">
            <label>TRIGGER</label>
            <input
              type="range"
              min={-2}
              max={2}
              step={0.1}
              value={parameters.trigger_level}
              onChange={(e) => updateParameter('trigger_level', parseFloat(e.target.value))}
            />
            <span>{parameters.trigger_level.toFixed(1)}V</span>
          </div>
        </div>

        {/* オシロスコープ画面 */}
        <div className="oscilloscope-screen">
          <canvas
            ref={canvasRef}
            style={{ 
              width: '400px', 
              height: '300px',
              pointerEvents: 'none' // Canvas がドラッグを妨げないようにする
            }}
          />
        </div>

        {/* 測定値表示 */}
        <div 
          className="measurements"
          onMouseDown={(e) => e.stopPropagation()} // ドラッグ開始を防ぐ
        >
          <div className="measurement">
            <span className="label">Vpp:</span>
            <span className="value">{measurements.vpp.toFixed(2)}V</span>
          </div>
          <div className="measurement">
            <span className="label">Vrms:</span>
            <span className="value">{measurements.vrms.toFixed(3)}V</span>
          </div>
          <div className="measurement">
            <span className="label">Freq:</span>
            <span className="value">{measurements.frequency.toFixed(1)}Hz</span>
          </div>
          <div className="measurement">
            <span className="label">Period:</span>
            <span className="value">{(measurements.period * 1000).toFixed(2)}ms</span>
          </div>
        </div>
      </div>

      {/* 出力ハンドル */}
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out"
        style={{ top: '50%', background: '#00ff00' }}
      />
    </div>
  );
};

export default OscilloscopeNode;