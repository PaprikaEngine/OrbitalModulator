import React, { useState, useEffect, useRef } from 'react';
import { Handle, Position } from 'reactflow';
import { invoke } from '@tauri-apps/api/core';

interface SpectrumAnalyzerNodeProps {
  id: string;
  data: {
    label: string;
    parameters: {
      window_type: number;
      smoothing: number;
      gain: number;
      active: number;
    };
  };
}

const SpectrumAnalyzerNode: React.FC<SpectrumAnalyzerNodeProps> = ({ id, data }) => {
  const [windowType, setWindowType] = useState(data.parameters?.window_type || 0);
  const [smoothing, setSmoothing] = useState(data.parameters?.smoothing || 0.8);
  const [gain, setGain] = useState(data.parameters?.gain || 1.0);
  const [active, setActive] = useState((data.parameters?.active || 1) !== 0);
  
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>();
  const [spectrumData, setSpectrumData] = useState<number[]>([]);
  const [frequencies, setFrequencies] = useState<number[]>([]);

  useEffect(() => {
    setWindowType(data.parameters?.window_type || 0);
    setSmoothing(data.parameters?.smoothing || 0.8);
    setGain(data.parameters?.gain || 1.0);
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

  const handleWindowTypeChange = (value: number) => {
    setWindowType(value);
    updateParameter('window_type', value);
  };

  const handleSmoothingChange = (value: number) => {
    setSmoothing(value);
    updateParameter('smoothing', value);
  };

  const handleGainChange = (value: number) => {
    setGain(value);
    updateParameter('gain', value);
  };

  const toggleActive = () => {
    const newActive = !active;
    setActive(newActive);
    updateParameter('active', newActive ? 1 : 0);
  };

  const windowTypes = [
    { value: 0, label: 'Hanning' },
    { value: 1, label: 'Hamming' },
    { value: 2, label: 'Blackman' },
    { value: 3, label: 'Rectangular' },
  ];

  // Fetch spectrum data periodically
  useEffect(() => {
    const fetchSpectrumData = async () => {
      if (!active) return;
      
      try {
        const [magnitudes, freqs] = await Promise.all([
          invoke<number[]>('get_spectrum_data', { node_id: id }),
          invoke<number[]>('get_spectrum_frequencies', { node_id: id })
        ]);
        
        setSpectrumData(magnitudes);
        if (frequencies.length === 0) {
          setFrequencies(freqs);
        }
      } catch (error) {
        // Silently handle errors (node might not be ready yet)
      }
    };

    const animate = () => {
      fetchSpectrumData();
      drawSpectrum();
      animationRef.current = requestAnimationFrame(animate);
    };

    if (active) {
      animate();
    }

    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [active, id]);

  const drawSpectrum = () => {
    const canvas = canvasRef.current;
    if (!canvas || spectrumData.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas
    ctx.fillStyle = '#000';
    ctx.fillRect(0, 0, width, height);

    // Draw grid
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 1;
    
    // Horizontal grid lines (dB scale)
    for (let i = 0; i <= 10; i++) {
      const y = (i / 10) * height;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    // Vertical grid lines (frequency scale)
    for (let i = 0; i <= 10; i++) {
      const x = (i / 10) * width;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }

    // Draw spectrum
    if (spectrumData.length > 0) {
      ctx.strokeStyle = '#00ff00';
      ctx.lineWidth = 2;
      ctx.beginPath();

      const binWidth = width / spectrumData.length;
      
      for (let i = 0; i < spectrumData.length; i++) {
        // Convert magnitude to dB and normalize
        const magnitude = spectrumData[i];
        const dB = magnitude > 0 ? 20 * Math.log10(magnitude) : -100;
        const normalizedY = Math.max(0, Math.min(1, (dB + 100) / 100)); // -100dB to 0dB range
        
        const x = i * binWidth;
        const y = height - (normalizedY * height);
        
        if (i === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }
      
      ctx.stroke();

      // Fill area under curve
      ctx.fillStyle = 'rgba(0, 255, 0, 0.1)';
      ctx.lineTo(width, height);
      ctx.lineTo(0, height);
      ctx.closePath();
      ctx.fill();
    }

    // Draw frequency labels
    ctx.fillStyle = '#fff';
    ctx.font = '10px monospace';
    ctx.textAlign = 'center';
    
    const freqLabels = ['20', '100', '1k', '5k', '10k', '20k'];
    for (let i = 0; i < freqLabels.length; i++) {
      const x = (i / (freqLabels.length - 1)) * width;
      ctx.fillText(freqLabels[i], x, height - 5);
    }

    // Draw dB labels
    ctx.textAlign = 'left';
    const dbLabels = ['0', '-20', '-40', '-60', '-80', '-100'];
    for (let i = 0; i < dbLabels.length; i++) {
      const y = (i / (dbLabels.length - 1)) * height + 12;
      ctx.fillText(dbLabels[i], 5, y);
    }
  };

  return (
    <div className="spectrum-analyzer-node">
      {/* Input Handle */}
      <Handle
        type="target"
        position={Position.Left}
        id="audio_in"
        style={{ background: '#e74c3c' }}
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

      {/* Spectrum Display */}
      <div className="spectrum-display">
        <canvas
          ref={canvasRef}
          width={280}
          height={150}
          className="spectrum-canvas"
        />
      </div>

      {/* Controls */}
      <div className="spectrum-controls">
        {/* Window Type */}
        <div className="control-group">
          <label className="control-label">Window</label>
          <select
            value={windowType}
            onChange={(e) => handleWindowTypeChange(Number(e.target.value))}
            className="window-select"
          >
            {windowTypes.map((type) => (
              <option key={type.value} value={type.value}>
                {type.label}
              </option>
            ))}
          </select>
        </div>

        {/* Smoothing */}
        <div className="control-group">
          <label className="control-label">Smooth</label>
          <div className="slider-control">
            <input
              type="range"
              min="0"
              max="0.99"
              step="0.01"
              value={smoothing}
              onChange={(e) => handleSmoothingChange(Number(e.target.value))}
              className="smoothing-slider"
            />
            <div className="control-value">{(smoothing * 100).toFixed(0)}%</div>
          </div>
        </div>

        {/* Gain */}
        <div className="control-group">
          <label className="control-label">Gain</label>
          <div className="slider-control">
            <input
              type="range"
              min="0.1"
              max="10"
              step="0.1"
              value={gain}
              onChange={(e) => handleGainChange(Number(e.target.value))}
              className="gain-slider"
            />
            <div className="control-value">{gain.toFixed(1)}x</div>
          </div>
        </div>
      </div>

      {/* Output Handle */}
      <Handle
        type="source"
        position={Position.Right}
        id="audio_out"
        style={{ background: '#e74c3c' }}
      />

      {/* Port Labels */}
      <div className="port-labels">
        <div className="input-labels">
          <div>Audio</div>
        </div>
        <div className="output-labels">
          <div>Audio</div>
        </div>
      </div>
    </div>
  );
};

export default SpectrumAnalyzerNode;