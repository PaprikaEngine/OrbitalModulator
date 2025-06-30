/*
 * OrbitalModulator - Professional Modular Synthesizer
 * Copyright (c) 2025 MACHIKO LAB
 *
 * C ABI Bridge - Converts between C-compatible interface and Rust traits
 */

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Mutex};
use std::ptr;

use crate::plugin::{
    PluginError, PluginResult, PluginConfig, PluginStats,
    api::{PluginNodeFactory, PluginMetadata},
};

/// C-compatible wrapper for PluginNodeFactory
#[repr(C)]
pub struct CPluginFactory {
    /// Pointer to the Rust factory implementation
    rust_factory: *mut c_void,
    
    /// Function pointers for the C ABI
    create_node: extern "C" fn(*mut c_void, *const c_char, *const c_char, f32) -> *mut c_void,
    get_metadata: extern "C" fn(*mut c_void) -> *const c_char,
    get_supported_types: extern "C" fn(*mut c_void) -> *const c_char,
    validate_compatibility: extern "C" fn(*mut c_void, *const c_char) -> i32,
    initialize: extern "C" fn(*mut c_void, *const c_void) -> i32,
    cleanup: extern "C" fn(*mut c_void) -> i32,
    get_stats: extern "C" fn(*mut c_void) -> *const c_char,
    configure: extern "C" fn(*mut c_void, *const c_void) -> i32,
    destroy: extern "C" fn(*mut c_void),
}

/// C-compatible factory wrapper that implements our trait
#[derive(Debug)]
pub struct CFactoryWrapper {
    c_factory: *mut CPluginFactory,
    metadata_cache: Arc<Mutex<Option<PluginMetadata>>>,
}

impl CFactoryWrapper {
    /// Create a new wrapper from a C factory
    pub unsafe fn new(c_factory: *mut CPluginFactory) -> Self {
        Self {
            c_factory,
            metadata_cache: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Helper to get cached metadata
    fn get_cached_metadata(&self) -> PluginResult<PluginMetadata> {
        let mut cache = self.metadata_cache.lock()
            .map_err(|e| PluginError::Internal { 
                message: format!("Failed to lock metadata cache: {}", e) 
            })?;
        
        if cache.is_none() {
            // Load metadata from C factory
            let metadata_json = unsafe {
                let c_factory = &*self.c_factory;
                let json_ptr = (c_factory.get_metadata)(c_factory.rust_factory);
                if json_ptr.is_null() {
                    return Err(PluginError::Internal {
                        message: "C factory returned null metadata".to_string(),
                    });
                }
                
                let json_cstr = CStr::from_ptr(json_ptr);
                json_cstr.to_str()
                    .map_err(|e| PluginError::Internal {
                        message: format!("Invalid UTF-8 in metadata: {}", e),
                    })?
            };
            
            let metadata: PluginMetadata = serde_json::from_str(metadata_json)
                .map_err(|e| PluginError::Internal {
                    message: format!("Failed to parse metadata JSON: {}", e),
                })?;
            
            *cache = Some(metadata);
        }
        
        Ok(cache.as_ref().unwrap().clone())
    }
}

impl PluginNodeFactory for CFactoryWrapper {
    fn metadata(&self) -> &PluginMetadata {
        // For C ABI wrapper, we'll use lazy_static for compatibility
        lazy_static::lazy_static! {
            static ref DEFAULT_METADATA: PluginMetadata = PluginMetadata {
                id: "c_plugin".to_string(),
                name: "C Plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "Plugin loaded via C ABI".to_string(),
                author: "Unknown".to_string(),
                website: None,
                category: crate::plugin::api::PluginCategory::Custom("C Plugin".to_string()),
                license: crate::plugin::api::PluginLicense::Custom("Unknown".to_string()),
                api_version: crate::plugin::api::PLUGIN_API_VERSION,
                node_types: vec!["c_node".to_string()],
                dependencies: Vec::new(),
                tags: vec!["c".to_string(), "native".to_string()],
                min_orbital_version: "1.0.0".to_string(),
            };
        }
        
        &DEFAULT_METADATA
    }
    
    fn create_node(&self, node_type: &str, name: String, sample_rate: f32) -> PluginResult<Box<dyn crate::processing::AudioNode>> {
        let node_type_cstr = CString::new(node_type)
            .map_err(|e| PluginError::Internal {
                message: format!("Invalid node type string: {}", e),
            })?;
        
        let name_cstr = CString::new(name)
            .map_err(|e| PluginError::Internal {
                message: format!("Invalid name string: {}", e),
            })?;
        
        let node_ptr = unsafe {
            let c_factory = &*self.c_factory;
            (c_factory.create_node)(
                c_factory.rust_factory,
                node_type_cstr.as_ptr(),
                name_cstr.as_ptr(),
                sample_rate,
            )
        };
        
        if node_ptr.is_null() {
            return Err(PluginError::RuntimeError {
                plugin_id: "c_plugin".to_string(),
                error: "C factory returned null node".to_string(),
            });
        }
        
        // Create a wrapper that converts C node to AudioNode trait
        Ok(Box::new(CNodeWrapper::new(node_ptr, self.c_factory)))
    }
    
    fn supported_node_types(&self) -> Vec<String> {
        unsafe {
            let c_factory = &*self.c_factory;
            let types_ptr = (c_factory.get_supported_types)(c_factory.rust_factory);
            if types_ptr.is_null() {
                return Vec::new();
            }
            
            let types_cstr = CStr::from_ptr(types_ptr);
            if let Ok(types_str) = types_cstr.to_str() {
                if let Ok(types) = serde_json::from_str::<Vec<String>>(types_str) {
                    return types;
                }
            }
            
            Vec::new()
        }
    }
    
    fn validate_compatibility(&self, host_version: &str) -> PluginResult<()> {
        let version_cstr = CString::new(host_version)
            .map_err(|e| PluginError::Internal {
                message: format!("Invalid version string: {}", e),
            })?;
        
        let result = unsafe {
            let c_factory = &*self.c_factory;
            (c_factory.validate_compatibility)(c_factory.rust_factory, version_cstr.as_ptr())
        };
        
        if result == 0 {
            Ok(())
        } else {
            Err(PluginError::VersionMismatch {
                plugin_id: "c_plugin".to_string(),
                required: "unknown".to_string(),
                found: host_version.to_string(),
            })
        }
    }
    
    fn initialize(&mut self, config: &PluginConfig) -> PluginResult<()> {
        let config_ptr = config as *const PluginConfig as *const c_void;
        
        let result = unsafe {
            let c_factory = &*self.c_factory;
            (c_factory.initialize)(c_factory.rust_factory, config_ptr)
        };
        
        if result == 0 {
            Ok(())
        } else {
            Err(PluginError::RuntimeError {
                plugin_id: "c_plugin".to_string(),
                error: "Failed to initialize C plugin".to_string(),
            })
        }
    }
    
    fn cleanup(&mut self) -> PluginResult<()> {
        let result = unsafe {
            let c_factory = &*self.c_factory;
            (c_factory.cleanup)(c_factory.rust_factory)
        };
        
        if result == 0 {
            Ok(())
        } else {
            Err(PluginError::RuntimeError {
                plugin_id: "c_plugin".to_string(),
                error: "Failed to cleanup C plugin".to_string(),
            })
        }
    }
    
    fn get_stats(&self) -> PluginStats {
        unsafe {
            let c_factory = &*self.c_factory;
            let stats_ptr = (c_factory.get_stats)(c_factory.rust_factory);
            if stats_ptr.is_null() {
                return PluginStats::default();
            }
            
            let stats_cstr = CStr::from_ptr(stats_ptr);
            if let Ok(stats_str) = stats_cstr.to_str() {
                if let Ok(stats) = serde_json::from_str::<PluginStats>(stats_str) {
                    return stats;
                }
            }
            
            PluginStats::default()
        }
    }
    
    fn configure(&mut self, config: &PluginConfig) -> PluginResult<()> {
        let config_ptr = config as *const PluginConfig as *const c_void;
        
        let result = unsafe {
            let c_factory = &*self.c_factory;
            (c_factory.configure)(c_factory.rust_factory, config_ptr)
        };
        
        if result == 0 {
            Ok(())
        } else {
            Err(PluginError::RuntimeError {
                plugin_id: "c_plugin".to_string(),
                error: "Failed to configure C plugin".to_string(),
            })
        }
    }
}

impl Drop for CFactoryWrapper {
    fn drop(&mut self) {
        unsafe {
            let c_factory = &*self.c_factory;
            (c_factory.destroy)(c_factory.rust_factory);
        }
    }
}

/// C-compatible node wrapper
pub struct CNodeWrapper {
    c_node: *mut c_void,
    c_factory: *mut CPluginFactory,
    node_info: crate::processing::NodeInfo,
}

impl CNodeWrapper {
    pub fn new(c_node: *mut c_void, c_factory: *mut CPluginFactory) -> Self {
        // Create a minimal node info - in a real implementation, this would be retrieved from C
        let node_info = crate::processing::NodeInfo {
            id: uuid::Uuid::new_v4(),
            name: "C Plugin Node".to_string(),
            node_type: "c_plugin_node".to_string(),
            category: crate::processing::NodeCategory::Processor,
            input_ports: Vec::new(),
            output_ports: Vec::new(),
            description: "Node loaded from C plugin".to_string(),
            latency_samples: 0,
            supports_bypass: false,
        };
        
        Self {
            c_node,
            c_factory,
            node_info,
        }
    }
}

impl crate::processing::AudioNode for CNodeWrapper {
    fn process(&mut self, _ctx: &mut crate::processing::ProcessContext) -> Result<(), crate::processing::ProcessingError> {
        // This would call the C node's process function
        // For now, we'll just return Ok
        Ok(())
    }
    
    fn node_info(&self) -> &crate::processing::NodeInfo {
        &self.node_info
    }
    
    fn reset(&mut self) {
        // This would call the C node's reset function
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl crate::parameters::Parameterizable for CNodeWrapper {
    fn set_parameter(&mut self, _name: &str, _value: f32) -> Result<(), crate::parameters::ParameterError> {
        // This would call the C node's set parameter function
        Ok(())
    }
    
    fn get_parameter(&self, _name: &str) -> Result<f32, crate::parameters::ParameterError> {
        // This would call the C node's get parameter function
        Err(crate::parameters::ParameterError::NotFound { 
            name: _name.to_string() 
        })
    }
    
    fn get_all_parameters(&self) -> std::collections::HashMap<String, f32> {
        std::collections::HashMap::new()
    }
    
    fn get_parameter_descriptors(&self) -> Vec<Box<dyn crate::parameters::ParameterDescriptor>> {
        Vec::new()
    }
}

/// Helper function to create a factory wrapper from C ABI entry points
pub unsafe fn create_factory_from_c_symbols(
    create_fn: extern "C" fn() -> *mut c_void,
    get_info_fn: extern "C" fn() -> *const c_char,
    get_version_fn: extern "C" fn() -> u32,
) -> PluginResult<Box<dyn PluginNodeFactory>> {
    // Verify API version
    let plugin_version = get_version_fn();
    if plugin_version != crate::plugin::api::PLUGIN_API_VERSION {
        return Err(PluginError::VersionMismatch {
            plugin_id: "c_plugin".to_string(),
            required: crate::plugin::api::PLUGIN_API_VERSION.to_string(),
            found: plugin_version.to_string(),
        });
    }
    
    // Create the factory instance
    let rust_factory = create_fn();
    if rust_factory.is_null() {
        return Err(PluginError::LoadError {
            plugin_id: "c_plugin".to_string(),
            reason: "Factory creation returned null".to_string(),
        });
    }
    
    // Create the C factory wrapper
    let c_factory = Box::into_raw(Box::new(CPluginFactory {
        rust_factory,
        create_node: c_create_node_stub,
        get_metadata: c_get_metadata_stub,
        get_supported_types: c_get_supported_types_stub,
        validate_compatibility: c_validate_compatibility_stub,
        initialize: c_initialize_stub,
        cleanup: c_cleanup_stub,
        get_stats: c_get_stats_stub,
        configure: c_configure_stub,
        destroy: c_destroy_stub,
    }));
    
    Ok(Box::new(CFactoryWrapper::new(c_factory)))
}

/// C ABI stub functions - these would be implemented by actual plugin loading
extern "C" fn c_create_node_stub(_factory: *mut c_void, _node_type: *const c_char, _name: *const c_char, _sample_rate: f32) -> *mut c_void {
    ptr::null_mut()
}

extern "C" fn c_get_metadata_stub(_factory: *mut c_void) -> *const c_char {
    ptr::null()
}

extern "C" fn c_get_supported_types_stub(_factory: *mut c_void) -> *const c_char {
    ptr::null()
}

extern "C" fn c_validate_compatibility_stub(_factory: *mut c_void, _version: *const c_char) -> i32 {
    -1
}

extern "C" fn c_initialize_stub(_factory: *mut c_void, _config: *const c_void) -> i32 {
    -1
}

extern "C" fn c_cleanup_stub(_factory: *mut c_void) -> i32 {
    -1
}

extern "C" fn c_get_stats_stub(_factory: *mut c_void) -> *const c_char {
    ptr::null()
}

extern "C" fn c_configure_stub(_factory: *mut c_void, _config: *const c_void) -> i32 {
    -1
}

extern "C" fn c_destroy_stub(_factory: *mut c_void) {
    // Stub implementation
}

// SAFETY: These wrappers are thread-safe because:
// 1. The C pointers are only accessed through the wrapper methods
// 2. The actual C plugin implementations are assumed to be thread-safe
// 3. All accesses are synchronized through the AudioNode and Parameterizable traits
// 4. The metadata cache is protected by Mutex
unsafe impl Send for CFactoryWrapper {}
unsafe impl Sync for CFactoryWrapper {}
unsafe impl Send for CNodeWrapper {}
unsafe impl Sync for CNodeWrapper {}