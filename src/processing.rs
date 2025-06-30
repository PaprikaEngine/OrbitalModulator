/*
 * OrbitalModulator - Professional Modular Synthesizer
 * Copyright (c) 2025 MACHIKO LAB
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use std::collections::HashMap;
use crate::parameters::{Parameterizable, ParameterError};
use crate::graph::{Node, Port, PortType};
use uuid::Uuid;

/// オーディオ処理のコンテキスト - すべての処理情報を統一
#[derive(Debug)]
pub struct ProcessContext {
    /// 入力バッファ群
    pub inputs: InputBuffers,
    /// 出力バッファ群
    pub outputs: OutputBuffers,
    /// サンプリングレート
    pub sample_rate: f32,
    /// バッファサイズ
    pub buffer_size: usize,
    /// 処理タイムスタンプ（サンプル数）
    pub timestamp: u64,
    /// BPM（シーケンサー等で使用）
    pub bpm: f32,
}

impl ProcessContext {
    /// Create a new processing context
    pub fn new(inputs: InputBuffers, outputs: OutputBuffers, sample_rate: f32, buffer_size: usize) -> Self {
        Self {
            inputs,
            outputs,
            sample_rate,
            buffer_size,
            timestamp: 0,
            bpm: 120.0,
        }
    }
    
    /// Get mutable reference to outputs
    pub fn outputs_mut(&mut self) -> &mut OutputBuffers {
        &mut self.outputs
    }
    
    /// Get reference to inputs
    pub fn inputs(&self) -> &InputBuffers {
        &self.inputs
    }
}

/// 入力ポート（バッファ）の管理
pub type InputPorts = InputBuffers;

/// 出力ポート（バッファ）の管理  
pub type OutputPorts = OutputBuffers;

/// 入力バッファの管理
#[derive(Debug, Default)]
pub struct InputBuffers {
    audio_buffers: HashMap<String, Vec<f32>>,
    cv_buffers: HashMap<String, Vec<f32>>,
}

impl InputBuffers {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// オーディオバッファを追加
    pub fn add_audio(&mut self, port_name: String, buffer: Vec<f32>) {
        self.audio_buffers.insert(port_name, buffer);
    }
    
    /// CVバッファを追加
    pub fn add_cv(&mut self, port_name: String, buffer: Vec<f32>) {
        self.cv_buffers.insert(port_name, buffer);
    }
    
    /// オーディオバッファを取得
    pub fn get_audio(&self, port_name: &str) -> Option<&[f32]> {
        self.audio_buffers.get(port_name).map(|v| v.as_slice())
    }
    
    /// CVバッファを取得
    pub fn get_cv(&self, port_name: &str) -> Option<&[f32]> {
        self.cv_buffers.get(port_name).map(|v| v.as_slice())
    }
    
    /// CVの最初の値を取得（単一値として扱う場合）
    pub fn get_cv_value(&self, port_name: &str) -> f32 {
        self.get_cv(port_name)
            .and_then(|buf| buf.first())
            .copied()
            .unwrap_or(0.0)
    }
    
    /// 空のバッファを作成（デフォルト値で）
    pub fn get_or_default_audio(&self, port_name: &str, size: usize) -> Vec<f32> {
        self.get_audio(port_name)
            .map(|buf| buf.to_vec())
            .unwrap_or_else(|| vec![0.0; size])
    }
    
    pub fn get_or_default_cv(&self, port_name: &str, size: usize) -> Vec<f32> {
        self.get_cv(port_name)
            .map(|buf| buf.to_vec())
            .unwrap_or_else(|| vec![0.0; size])
    }
}

/// 出力バッファの管理
#[derive(Debug, Default)]
pub struct OutputBuffers {
    audio_buffers: HashMap<String, Vec<f32>>,
    cv_buffers: HashMap<String, Vec<f32>>,
}

impl OutputBuffers {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// オーディオ出力バッファを確保
    pub fn allocate_audio(&mut self, port_name: String, size: usize) {
        self.audio_buffers.insert(port_name, vec![0.0; size]);
    }
    
    /// CV出力バッファを確保
    pub fn allocate_cv(&mut self, port_name: String, size: usize) {
        self.cv_buffers.insert(port_name, vec![0.0; size]);
    }
    
    /// オーディオ出力バッファを取得（可変）
    pub fn get_audio_mut(&mut self, port_name: &str) -> Option<&mut [f32]> {
        self.audio_buffers.get_mut(port_name).map(|v| v.as_mut_slice())
    }
    
    /// CV出力バッファを取得（可変）
    pub fn get_cv_mut(&mut self, port_name: &str) -> Option<&mut [f32]> {
        self.cv_buffers.get_mut(port_name).map(|v| v.as_mut_slice())
    }
    
    /// オーディオ出力バッファを取得（読み取り専用）
    pub fn get_audio(&self, port_name: &str) -> Option<&[f32]> {
        self.audio_buffers.get(port_name).map(|v| v.as_slice())
    }
    
    /// CV出力バッファを取得（読み取り専用）
    pub fn get_cv(&self, port_name: &str) -> Option<&[f32]> {
        self.cv_buffers.get(port_name).map(|v| v.as_slice())
    }
    
    /// CV出力に単一値を設定
    pub fn set_cv_value(&mut self, port_name: &str, value: f32) {
        if let Some(buffer) = self.cv_buffers.get_mut(port_name) {
            for sample in buffer.iter_mut() {
                *sample = value;
            }
        }
    }
    
    /// オーディオ出力をクリア
    pub fn clear_audio(&mut self, port_name: &str) {
        if let Some(buffer) = self.audio_buffers.get_mut(port_name) {
            buffer.fill(0.0);
        }
    }
    
    /// CV出力をクリア
    pub fn clear_cv(&mut self, port_name: &str) {
        if let Some(buffer) = self.cv_buffers.get_mut(port_name) {
            buffer.fill(0.0);
        }
    }
}

/// ノード情報の詳細版
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: Uuid,
    pub name: String,
    pub node_type: String,
    pub category: NodeCategory,
    pub description: String,
    pub input_ports: Vec<PortInfo>,
    pub output_ports: Vec<PortInfo>,
    pub latency_samples: u32,
    pub supports_bypass: bool,
}

/// ノードカテゴリ
#[derive(Debug, Clone, PartialEq)]
pub enum NodeCategory {
    Generator,
    Processor,
    Controller,
    Utility,
    Mixing,
    Analyzer,
}

/// ポート情報の詳細版
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub name: String,
    pub port_type: PortType,
    pub description: String,
    pub is_optional: bool,
}

impl PortInfo {
    pub fn new(name: &str, port_type: PortType) -> Self {
        Self {
            name: name.to_string(),
            port_type,
            description: String::new(),
            is_optional: false,
        }
    }
    
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }
    
    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }
}

/// 改善されたAudioNodeトレイト
pub trait AudioNode: Send + Sync + Parameterizable {
    /// オーディオ処理を実行
    fn process(&mut self, ctx: &mut ProcessContext) -> Result<(), ProcessingError>;
    
    /// ノード情報を取得
    fn node_info(&self) -> &NodeInfo;
    
    /// ノードをリセット（内部状態をクリア）
    fn reset(&mut self);
    
    /// レイテンシーをサンプル数で返す
    fn latency(&self) -> u32 {
        0
    }
    
    /// バイパス状態を設定
    fn set_bypass(&mut self, bypass: bool) -> Result<(), ParameterError> {
        self.set_parameter("bypass", if bypass { 1.0 } else { 0.0 })
    }
    
    /// アクティブ状態を設定
    fn set_active(&mut self, active: bool) -> Result<(), ParameterError> {
        self.set_parameter("active", if active { 1.0 } else { 0.0 })
    }
    
    /// アクティブ状態を取得
    fn is_active(&self) -> bool {
        self.get_parameter("active").unwrap_or(1.0) > 0.5
    }
    
    /// ダウンキャスト用のas_anyメソッド
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    
    /// 互換性のための古いNodeを生成（Tauri統合用）
    fn create_legacy_node(&self, name: String) -> Node {
        let info = self.node_info();
        Node {
            id: info.id,
            name,
            node_type: info.node_type.clone(),
            parameters: self.get_all_parameters(),
            input_ports: info.input_ports.iter().map(|p| Port {
                name: p.name.clone(),
                port_type: p.port_type,
            }).collect(),
            output_ports: info.output_ports.iter().map(|p| Port {
                name: p.name.clone(),
                port_type: p.port_type,
            }).collect(),
        }
    }
}

/// オーディオ処理エラー
#[derive(Debug, Clone)]
pub enum ProcessingError {
    /// 必須入力ポートが見つからない
    MissingRequiredInput { port_name: String },
    /// 出力バッファの準備エラー
    OutputBufferError { port_name: String },
    /// パラメーターエラー
    Parameter(ParameterError),
    /// 内部処理エラー
    Internal { message: String },
}

impl std::fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessingError::MissingRequiredInput { port_name } => {
                write!(f, "Missing required input port: {}", port_name)
            }
            ProcessingError::OutputBufferError { port_name } => {
                write!(f, "Output buffer error for port: {}", port_name)
            }
            ProcessingError::Parameter(err) => {
                write!(f, "Parameter error: {}", err)
            }
            ProcessingError::Internal { message } => {
                write!(f, "Internal processing error: {}", message)
            }
        }
    }
}

impl std::error::Error for ProcessingError {}

impl From<ParameterError> for ProcessingError {
    fn from(err: ParameterError) -> Self {
        ProcessingError::Parameter(err)
    }
}

/// オーディオ処理のヘルパーマクロ
#[macro_export]
macro_rules! process_audio_samples {
    ($inputs:expr, $outputs:expr, $input_port:expr, $output_port:expr, $process_fn:expr) => {
        let input_buffer = $inputs.get_or_default_audio($input_port, $outputs.get_audio($output_port).map(|b| b.len()).unwrap_or(512));
        
        if let Some(output_buffer) = $outputs.get_audio_mut($output_port) {
            for (i, (input_sample, output_sample)) in input_buffer.iter().zip(output_buffer.iter_mut()).enumerate() {
                *output_sample = $process_fn(*input_sample, i);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_buffers() {
        let mut inputs = InputBuffers::new();
        inputs.add_audio("test".to_string(), vec![1.0, 2.0, 3.0]);
        inputs.add_cv("cv_test".to_string(), vec![0.5]);
        
        assert_eq!(inputs.get_audio("test"), Some([1.0, 2.0, 3.0].as_slice()));
        assert_eq!(inputs.get_cv_value("cv_test"), 0.5);
        assert_eq!(inputs.get_cv_value("nonexistent"), 0.0);
    }

    #[test]
    fn test_output_buffers() {
        let mut outputs = OutputBuffers::new();
        outputs.allocate_audio("test".to_string(), 3);
        outputs.allocate_cv("cv_test".to_string(), 1);
        
        outputs.set_cv_value("cv_test", 1.0);
        assert_eq!(outputs.get_cv("cv_test"), Some([1.0].as_slice()));
        
        if let Some(buffer) = outputs.get_audio_mut("test") {
            buffer[0] = 2.0;
        }
        assert_eq!(outputs.get_audio("test").unwrap()[0], 2.0);
    }
}