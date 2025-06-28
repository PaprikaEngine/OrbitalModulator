# 🎵 Orbital Modulator

A powerful modular synthesizer application built with Tauri (Rust) and React+TypeScript, featuring a comprehensive collection of synthesis modules and real-time audio processing.

## ✨ Features

### Core Synthesis Modules
- **Multiple Oscillators**: Sine, Triangle, Sawtooth, Pulse waveforms with frequency and amplitude control
- **VCF Filter**: Voltage-controlled filter with resonance and cutoff frequency control
- **ADSR Envelope**: Attack, Decay, Sustain, Release envelope generator
- **LFO**: Low-frequency oscillator with multiple waveforms for modulation
- **VCA**: Voltage-controlled amplifier with CV sensitivity
- **Noise Generator**: White, pink, brown, and blue noise sources

### Advanced Processing Modules
- **FFT Spectrum Analyzer**: Real-time frequency domain visualization with multiple window functions
- **Ring Modulator**: Signal multiplication with carrier/modulator gain controls
- **Sample & Hold**: Edge-triggered sampling with manual trigger and threshold control
- **Attenuverter**: Signal attenuation/inversion with DC offset and dual outputs
- **Delay Effect**: Echo/delay with feedback and mix controls
- **Sequencer**: 16-step pattern sequencer with note, gate, and velocity programming

### Utilities & Monitoring
- **Oscilloscope**: Real-time waveform display with trigger controls and measurements
- **Mixer**: 4/8-channel audio mixer with individual gain and pan controls
- **Audio Output**: Final output stage with master volume and stereo support

### Visual Interface
- **Node-based Workflow**: Drag-and-drop visual programming interface
- **Real-time Visualization**: Live parameter updates and signal flow displays
- **Professional Styling**: Each module features unique color schemes and professional controls
- **Connection Management**: Visual cable connections with automatic routing validation

## 🛠️ Technology Stack

- **Backend**: Rust with Tauri v2 framework
- **Frontend**: React 18 + TypeScript + Vite
- **Audio Engine**: CPAL + FunDSP + DASP for cross-platform audio processing
- **UI Framework**: ReactFlow for node-based interface
- **Build System**: Cargo + npm with hot reload support

## 🚀 Getting Started

### Prerequisites
- Rust (latest stable version)
- Node.js (v16 or higher)
- npm or yarn package manager

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/your-username/OrbitalModulator.git
   cd OrbitalModulator
   ```

2. **Install dependencies**
   ```bash
   # Install Rust dependencies
   cargo build
   
   # Install Node.js dependencies
   npm install
   ```

3. **Run in development mode**
   ```bash
   # Start the development server with hot reload
   npm run tauri dev
   ```

4. **Build for production**
   ```bash
   # Build optimized release version
   npm run tauri build
   ```

## 📖 Usage Guide

### Creating Your First Patch

1. **Start the Audio Engine**: Click the "🔊 Start Engine" button in the toolbar
2. **Add Modules**: Select a module type from the dropdown and give it a name, then click "Create Node"
3. **Connect Modules**: Drag from output ports (right side) to input ports (left side) to create audio/CV connections
4. **Adjust Parameters**: Click on nodes to open the parameter panel and adjust settings
5. **Monitor Output**: Add an oscilloscope or spectrum analyzer to visualize your signals

### Basic Signal Chain Example

```
Sine Oscillator → VCF Filter → VCA → Audio Output
     ↑               ↑         ↑
   LFO (vibrato)  ADSR Env  ADSR Env
```

### Advanced Techniques

- **Ring Modulation**: Connect two oscillators to a ring modulator for complex harmonic content
- **Sample & Hold**: Use with noise generator and LFO for random voltage generation
- **Sequencing**: Program patterns with the sequencer and use CV outputs to control other modules
- **Spectral Analysis**: Monitor your signals with the FFT spectrum analyzer

## 🎛️ Module Reference

### Oscillators
- **Frequency Range**: 20Hz - 20kHz
- **CV Inputs**: Frequency modulation, amplitude modulation
- **Waveforms**: Sine, Triangle, Sawtooth, Pulse with PWM

### Filters
- **Types**: Low-pass, High-pass, Band-pass, Notch
- **Cutoff Range**: 20Hz - 20kHz with resonance control
- **CV Inputs**: Cutoff frequency, resonance modulation

### Envelopes & LFOs
- **ADSR**: Classic envelope with CV trigger input
- **LFO Rates**: 0.1Hz - 20Hz with multiple waveforms
- **Sync Options**: Retrigger and free-running modes

### Effects & Utilities
- **Delay**: Up to 2 seconds with feedback control
- **Ring Mod**: True analog-style ring modulation
- **S&H**: Precision edge-triggered sampling
- **Attenuverter**: ±100% gain with ±5V DC offset

## 💾 Patch Management

- **Save/Load**: Use the 💾 Save and 📂 Load buttons to store your creations
- **File Format**: JSON-based patch files with complete parameter state
- **Examples**: Check the `examples/` directory for sample patches
- **Auto-save**: Project state is preserved between sessions

## 🔧 Development

### Project Structure
```
src/
├── audio/           # Rust audio engine
├── nodes/           # Individual module implementations
├── components/      # React UI components
├── styles.css       # Global styling
└── main.rs         # Tauri main process

examples/           # Sample patch files
docs/              # Documentation
CLAUDE.md          # Development notes and specifications
```

### Adding New Modules

1. **Create Rust Implementation**
   ```rust
   // src/nodes/my_module.rs
   impl AudioNode for MyModule {
       fn process(&mut self, inputs: &HashMap<String, &[f32]>, outputs: &mut HashMap<String, &mut [f32]>) {
           // Audio processing logic
       }
   }
   ```

2. **Create React Component**
   ```tsx
   // src/components/MyModuleNode.tsx
   const MyModuleNode: React.FC<MyModuleNodeProps> = ({ id, data }) => {
       // UI component logic
   };
   ```

3. **Register Module**
   - Add to `src/nodes/mod.rs`
   - Add to `nodeTypes` in `App.tsx`
   - Add to toolbar options in `Toolbar.tsx`

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Guidelines
- Follow Rust best practices for audio processing code
- Use TypeScript for all React components
- Maintain consistent styling across modules
- Add appropriate documentation for new features
- Test audio functionality across different platforms

## 🐛 Known Issues

- Audio latency may vary depending on system configuration
- Some complex patches may require higher buffer sizes for stable operation
- WebGL acceleration recommended for optimal spectrum analyzer performance

## 🗺️ Roadmap

- [ ] MIDI input/output support
- [ ] Plugin system for third-party modules
- [ ] Multi-track recording and playback
- [ ] Advanced modulation matrix
- [ ] Preset bank management
- [ ] Live performance mode

## 📞 Support

If you encounter any issues or have questions:
- Check the [Issues](https://github.com/your-username/OrbitalModulator/issues) page
- Join our community discussions
- Read the documentation in the `docs/` directory

---

**Built with ❤️ using Rust and React**

*Orbital Modulator - Where sound meets code*