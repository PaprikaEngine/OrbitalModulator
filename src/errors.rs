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

use std::fmt;
use uuid::Uuid;
use crate::parameters::ParameterError;
use crate::processing::ProcessingError;

/// OrbitalModulator全体のエラー型
#[derive(Debug, Clone)]
pub enum AudioEngineError {
    /// ノードが見つからない
    NodeNotFound { id: Uuid },
    
    /// ノードの作成に失敗
    NodeCreationFailed { 
        node_type: String, 
        reason: String 
    },
    
    /// ノード間の接続エラー
    ConnectionError { 
        source: String, 
        target: String, 
        reason: String 
    },
    
    /// ポートが見つからない
    PortNotFound { 
        node_id: Uuid, 
        port_name: String 
    },
    
    /// ポートタイプの不一致
    PortTypeMismatch { 
        source_type: String, 
        target_type: String 
    },
    
    /// 循環参照の検出
    CircularDependency { 
        cycle: Vec<Uuid> 
    },
    
    /// パラメーターエラー
    Parameter { 
        node_id: Uuid, 
        error: ParameterError 
    },
    
    /// オーディオ処理エラー
    Processing { 
        node_id: Uuid, 
        error: ProcessingError 
    },
    
    /// ファイルI/Oエラー
    FileIo { 
        operation: String, 
        path: String, 
        reason: String 
    },
    
    /// 設定の解析エラー
    ConfigParsing { 
        file: String, 
        line: Option<u32>, 
        reason: String 
    },
    
    /// オーディオデバイスエラー
    AudioDevice { 
        device_name: Option<String>, 
        reason: String 
    },
    
    /// 内部エラー（予期しない状況）
    Internal { 
        message: String, 
        location: Option<String> 
    },
}

impl fmt::Display for AudioEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioEngineError::NodeNotFound { id } => {
                write!(f, "Node not found: {}", id)
            }
            AudioEngineError::NodeCreationFailed { node_type, reason } => {
                write!(f, "Failed to create {} node: {}", node_type, reason)
            }
            AudioEngineError::ConnectionError { source, target, reason } => {
                write!(f, "Connection error from {} to {}: {}", source, target, reason)
            }
            AudioEngineError::PortNotFound { node_id, port_name } => {
                write!(f, "Port '{}' not found on node {}", port_name, node_id)
            }
            AudioEngineError::PortTypeMismatch { source_type, target_type } => {
                write!(f, "Port type mismatch: cannot connect {} to {}", source_type, target_type)
            }
            AudioEngineError::CircularDependency { cycle } => {
                write!(f, "Circular dependency detected: {:?}", cycle)
            }
            AudioEngineError::Parameter { node_id, error } => {
                write!(f, "Parameter error on node {}: {}", node_id, error)
            }
            AudioEngineError::Processing { node_id, error } => {
                write!(f, "Processing error on node {}: {}", node_id, error)
            }
            AudioEngineError::FileIo { operation, path, reason } => {
                write!(f, "File I/O error during {}: {} - {}", operation, path, reason)
            }
            AudioEngineError::ConfigParsing { file, line, reason } => {
                if let Some(line_num) = line {
                    write!(f, "Config parsing error in {}:{}: {}", file, line_num, reason)
                } else {
                    write!(f, "Config parsing error in {}: {}", file, reason)
                }
            }
            AudioEngineError::AudioDevice { device_name, reason } => {
                if let Some(name) = device_name {
                    write!(f, "Audio device error on '{}': {}", name, reason)
                } else {
                    write!(f, "Audio device error: {}", reason)
                }
            }
            AudioEngineError::Internal { message, location } => {
                if let Some(loc) = location {
                    write!(f, "Internal error at {}: {}", loc, message)
                } else {
                    write!(f, "Internal error: {}", message)
                }
            }
        }
    }
}

impl std::error::Error for AudioEngineError {}

impl From<ParameterError> for AudioEngineError {
    fn from(error: ParameterError) -> Self {
        AudioEngineError::Internal {
            message: format!("Parameter error: {}", error),
            location: None,
        }
    }
}

impl From<ProcessingError> for AudioEngineError {
    fn from(error: ProcessingError) -> Self {
        AudioEngineError::Internal {
            message: format!("Processing error: {}", error),
            location: None,
        }
    }
}

impl From<std::io::Error> for AudioEngineError {
    fn from(error: std::io::Error) -> Self {
        AudioEngineError::FileIo {
            operation: "unknown".to_string(),
            path: "unknown".to_string(),
            reason: error.to_string(),
        }
    }
}

/// 結果型のエイリアス
pub type AudioEngineResult<T> = Result<T, AudioEngineError>;

/// エラーログのレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// ロギングトレイト
pub trait Logger: Send + Sync {
    fn log(&self, level: LogLevel, message: &str);
    
    fn trace(&self, message: &str) {
        self.log(LogLevel::Trace, message);
    }
    
    fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }
    
    fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }
    
    fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }
    
    fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
}

/// シンプルなコンソールロガー
pub struct ConsoleLogger {
    min_level: LogLevel,
}

impl ConsoleLogger {
    pub fn new(min_level: LogLevel) -> Self {
        Self { min_level }
    }
}

impl Logger for ConsoleLogger {
    fn log(&self, level: LogLevel, message: &str) {
        if level >= self.min_level {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            
            println!(
                "[{:.3}] [{}] {}",
                timestamp.as_secs_f64(),
                level,
                message
            );
        }
    }
}

/// エラーハンドリングのヘルパーマクロ
#[macro_export]
macro_rules! log_error {
    ($logger:expr, $error:expr) => {
        $logger.error(&format!("Error: {}", $error));
    };
    ($logger:expr, $error:expr, $context:expr) => {
        $logger.error(&format!("Error in {}: {}", $context, $error));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($logger:expr, $message:expr) => {
        $logger.warn($message);
    };
    ($logger:expr, $format:expr, $($args:expr),*) => {
        $logger.warn(&format!($format, $($args),*));
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $message:expr) => {
        $logger.info($message);
    };
    ($logger:expr, $format:expr, $($args:expr),*) => {
        $logger.info(&format!($format, $($args),*));
    };
}

/// カスタムエラー作成のヘルパー
impl AudioEngineError {
    pub fn node_not_found(id: Uuid) -> Self {
        AudioEngineError::NodeNotFound { id }
    }
    
    pub fn connection_failed(source: &str, target: &str, reason: &str) -> Self {
        AudioEngineError::ConnectionError {
            source: source.to_string(),
            target: target.to_string(),
            reason: reason.to_string(),
        }
    }
    
    pub fn port_not_found(node_id: Uuid, port_name: &str) -> Self {
        AudioEngineError::PortNotFound {
            node_id,
            port_name: port_name.to_string(),
        }
    }
    
    pub fn internal(message: &str) -> Self {
        AudioEngineError::Internal {
            message: message.to_string(),
            location: None,
        }
    }
    
    pub fn internal_at(message: &str, location: &str) -> Self {
        AudioEngineError::Internal {
            message: message.to_string(),
            location: Some(location.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let id = Uuid::new_v4();
        let error = AudioEngineError::node_not_found(id);
        assert!(error.to_string().contains(&id.to_string()));
    }

    #[test]
    fn test_error_conversion() {
        let param_error = ParameterError::NotFound {
            name: "test".to_string(),
        };
        let engine_error: AudioEngineError = param_error.into();
        
        match engine_error {
            AudioEngineError::Internal { .. } => (),
            _ => panic!("Expected Internal error variant"),
        }
    }

    #[test]
    fn test_logger() {
        let logger = ConsoleLogger::new(LogLevel::Warn);
        
        // These should not output (below min level)
        logger.debug("debug message");
        logger.info("info message");
        
        // These should output
        logger.warn("warn message");
        logger.error("error message");
    }
}