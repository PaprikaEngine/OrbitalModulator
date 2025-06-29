use std::collections::HashMap;
use std::fmt;

/// パラメーター記述子 - 各パラメーターの特性を定義
pub trait ParameterDescriptor: Send + Sync + fmt::Debug {
    /// パラメーター名
    fn name(&self) -> &'static str;
    
    /// 最小値
    fn min_value(&self) -> f32;
    
    /// 最大値
    fn max_value(&self) -> f32;
    
    /// デフォルト値
    fn default_value(&self) -> f32;
    
    /// 単位（Hz, dB, % など）
    fn unit(&self) -> &'static str { "" }
    
    /// 値の検証
    fn validate(&self, value: f32) -> Result<f32, ParameterError> {
        let clamped = value.clamp(self.min_value(), self.max_value());
        if clamped != value {
            Err(ParameterError::OutOfRange {
                value,
                min: self.min_value(),
                max: self.max_value(),
            })
        } else {
            Ok(clamped)
        }
    }
    
    /// 表示用の値フォーマット
    fn format_value(&self, value: f32) -> String {
        if self.unit().is_empty() {
            format!("{:.2}", value)
        } else {
            format!("{:.2} {}", value, self.unit())
        }
    }
}

/// パラメーターエラー型
#[derive(Debug, Clone)]
pub enum ParameterError {
    NotFound { name: String },
    OutOfRange { value: f32, min: f32, max: f32 },
    InvalidType { expected: String, found: String },
}

impl fmt::Display for ParameterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterError::NotFound { name } => {
                write!(f, "Parameter '{}' not found", name)
            }
            ParameterError::OutOfRange { value, min, max } => {
                write!(f, "Parameter value {} out of range [{}, {}]", value, min, max)
            }
            ParameterError::InvalidType { expected, found } => {
                write!(f, "Invalid parameter type: expected {}, found {}", expected, found)
            }
        }
    }
}

impl std::error::Error for ParameterError {}

/// パラメーター管理トレイト - ノードのパラメーター操作を統一
pub trait Parameterizable: Send + Sync {
    /// パラメーターを設定
    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), ParameterError>;
    
    /// パラメーターを取得
    fn get_parameter(&self, name: &str) -> Result<f32, ParameterError>;
    
    /// 全パラメーターを取得
    fn get_all_parameters(&self) -> HashMap<String, f32>;
    
    /// パラメーター記述子一覧を取得
    fn get_parameter_descriptors(&self) -> Vec<Box<dyn ParameterDescriptor>>;
    
    /// パラメーターが存在するかチェック
    fn has_parameter(&self, name: &str) -> bool {
        self.get_parameter(name).is_ok()
    }
}

/// 基本的なパラメーター記述子の実装
#[derive(Debug, Clone)]
pub struct BasicParameter {
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub unit: &'static str,
}

impl BasicParameter {
    pub fn new(name: &'static str, min: f32, max: f32, default: f32) -> Self {
        Self {
            name,
            min,
            max,
            default,
            unit: "",
        }
    }
    
    pub fn with_unit(mut self, unit: &'static str) -> Self {
        self.unit = unit;
        self
    }
}

impl ParameterDescriptor for BasicParameter {
    fn name(&self) -> &'static str {
        self.name
    }
    
    fn min_value(&self) -> f32 {
        self.min
    }
    
    fn max_value(&self) -> f32 {
        self.max
    }
    
    fn default_value(&self) -> f32 {
        self.default
    }
    
    fn unit(&self) -> &'static str {
        self.unit
    }
}

/// CV変調可能なパラメーター
#[derive(Debug, Clone)]
pub struct ModulatableParameter {
    pub base: BasicParameter,
    pub cv_amount: f32,  // CV変調の強度 (0.0 - 1.0)
    pub curve: ModulationCurve,
}

#[derive(Debug, Clone, Copy)]
pub enum ModulationCurve {
    Linear,
    Exponential,
    Logarithmic,
}

impl ModulatableParameter {
    pub fn new(base: BasicParameter, cv_amount: f32) -> Self {
        Self {
            base,
            cv_amount,
            curve: ModulationCurve::Linear,
        }
    }
    
    pub fn with_curve(mut self, curve: ModulationCurve) -> Self {
        self.curve = curve;
        self
    }
    
    /// CV入力を適用して最終値を計算
    pub fn modulate(&self, base_value: f32, cv_input: f32) -> f32 {
        let range = self.base.max - self.base.min;
        let modulation = cv_input * self.cv_amount * range;
        
        let modulated = match self.curve {
            ModulationCurve::Linear => base_value + modulation,
            ModulationCurve::Exponential => {
                let normalized = (base_value - self.base.min) / range;
                let exp_mod = normalized * (2.0_f32).powf(modulation);
                self.base.min + exp_mod * range
            }
            ModulationCurve::Logarithmic => {
                let normalized = (base_value - self.base.min) / range;
                let log_mod = normalized * (modulation + 1.0).ln();
                self.base.min + log_mod * range
            }
        };
        
        modulated.clamp(self.base.min, self.base.max)
    }
}

impl ParameterDescriptor for ModulatableParameter {
    fn name(&self) -> &'static str {
        self.base.name()
    }
    
    fn min_value(&self) -> f32 {
        self.base.min_value()
    }
    
    fn max_value(&self) -> f32 {
        self.base.max_value()
    }
    
    fn default_value(&self) -> f32 {
        self.base.default_value()
    }
    
    fn unit(&self) -> &'static str {
        self.base.unit()
    }
}

/// パラメーター管理のヘルパーマクロ
#[macro_export]
macro_rules! define_parameters {
    (
        $(
            $name:ident: $param_type:expr
        ),* $(,)?
    ) => {
        fn get_parameter_descriptors(&self) -> Vec<Box<dyn $crate::parameters::ParameterDescriptor>> {
            vec![
                $(
                    Box::new($param_type),
                )*
            ]
        }
        
        fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), $crate::parameters::ParameterError> {
            match name {
                $(
                    stringify!($name) => {
                        let validated = $param_type.validate(value)?;
                        self.$name = validated;
                        Ok(())
                    }
                )*
                _ => Err($crate::parameters::ParameterError::NotFound {
                    name: name.to_string(),
                }),
            }
        }
        
        fn get_parameter(&self, name: &str) -> Result<f32, $crate::parameters::ParameterError> {
            match name {
                $(
                    stringify!($name) => Ok(self.$name),
                )*
                _ => Err($crate::parameters::ParameterError::NotFound {
                    name: name.to_string(),
                }),
            }
        }
        
        fn get_all_parameters(&self) -> std::collections::HashMap<String, f32> {
            let mut params = std::collections::HashMap::new();
            $(
                params.insert(stringify!($name).to_string(), self.$name);
            )*
            params
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestNode {
        frequency: f32,
        amplitude: f32,
        active: f32,
    }

    impl TestNode {
        fn new() -> Self {
            Self {
                frequency: 440.0,
                amplitude: 0.5,
                active: 1.0,
            }
        }
    }

    impl Parameterizable for TestNode {
        define_parameters! {
            frequency: BasicParameter::new("frequency", 20.0, 20000.0, 440.0).with_unit("Hz"),
            amplitude: BasicParameter::new("amplitude", 0.0, 1.0, 0.5),
            active: BasicParameter::new("active", 0.0, 1.0, 1.0)
        }
    }

    #[test]
    fn test_parameter_setting() {
        let mut node = TestNode::new();
        
        // 正常な値の設定
        assert!(node.set_parameter("frequency", 880.0).is_ok());
        assert_eq!(node.get_parameter("frequency").unwrap(), 880.0);
        
        // 範囲外の値（クランプされる）
        assert!(node.set_parameter("amplitude", -0.5).is_err());
        assert!(node.set_parameter("amplitude", 1.5).is_err());
        
        // 存在しないパラメーター
        assert!(node.set_parameter("nonexistent", 1.0).is_err());
    }

    #[test]
    fn test_modulation() {
        let param = ModulatableParameter::new(
            BasicParameter::new("test", 0.0, 100.0, 50.0),
            0.5, // 50% CV modulation
        );
        
        // CV入力なし
        assert_eq!(param.modulate(50.0, 0.0), 50.0);
        
        // 正のCV入力
        assert!(param.modulate(50.0, 0.5) > 50.0);
        
        // 負のCV入力
        assert!(param.modulate(50.0, -0.5) < 50.0);
        
        // 範囲外はクランプされる
        assert!(param.modulate(50.0, 2.0) <= 100.0);
        assert!(param.modulate(50.0, -2.0) >= 0.0);
    }
}