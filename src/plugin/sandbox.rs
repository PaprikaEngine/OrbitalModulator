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

//! Plugin Sandbox - Security and resource isolation for plugins
//! 
//! This module provides security features to isolate plugins and monitor
//! their resource usage to prevent malicious or buggy plugins from
//! affecting the host system.

use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};
use std::thread;

use crate::plugin::{
    PluginError, PluginResult, PluginConfig, PluginStats,
    manifest::{Permission, Requirements},
};

/// Plugin sandbox environment
pub struct PluginSandbox {
    plugin_id: String,
    config: PluginConfig,
    requirements: Requirements,
    stats: Arc<RwLock<PluginStats>>,
    start_time: Instant,
    resource_monitor: Arc<Mutex<ResourceMonitor>>,
    permissions: Vec<Permission>,
}

/// Resource monitoring for plugins
#[derive(Debug)]
struct ResourceMonitor {
    cpu_samples: Vec<f32>,
    memory_peak: usize,
    memory_current: usize,
    #[allow(dead_code)]
    network_requests: u64,
    #[allow(dead_code)]
    file_operations: u64,
    last_cpu_check: Instant,
    violations: Vec<SecurityViolation>,
}

/// Security violation record
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    pub timestamp: SystemTime,
    pub violation_type: ViolationType,
    pub description: String,
    pub severity: Severity,
}

/// Types of security violations
#[derive(Debug, Clone)]
pub enum ViolationType {
    CpuExceeded,
    MemoryExceeded,
    TimeoutExceeded,
    UnauthorizedFileAccess,
    UnauthorizedNetworkAccess,
    PermissionViolation,
    MaliciousBehavior,
}

/// Violation severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl PluginSandbox {
    /// Create a new plugin sandbox
    pub fn new(
        plugin_id: String,
        config: PluginConfig,
        requirements: Requirements,
        permissions: Vec<Permission>,
    ) -> Self {
        let stats = Arc::new(RwLock::new(PluginStats::default()));
        let resource_monitor = Arc::new(Mutex::new(ResourceMonitor {
            cpu_samples: Vec::with_capacity(100),
            memory_peak: 0,
            memory_current: 0,
            network_requests: 0,
            file_operations: 0,
            last_cpu_check: Instant::now(),
            violations: Vec::new(),
        }));

        Self {
            plugin_id,
            config,
            requirements,
            stats,
            start_time: Instant::now(),
            resource_monitor,
            permissions,
        }
    }

    /// Start monitoring plugin resources
    pub fn start_monitoring(&self) -> PluginResult<()> {
        if !self.config.enable_sandbox {
            return Ok(());
        }

        // Validate initial requirements
        self.validate_system_requirements()?;

        // Start resource monitoring thread
        self.start_resource_monitor_thread();

        Ok(())
    }

    /// Stop monitoring and cleanup
    pub fn stop_monitoring(&self) -> PluginResult<()> {
        // Generate final statistics
        let _uptime = self.start_time.elapsed();
        
        {
            let _stats = self.stats.write().unwrap();
            // Final CPU usage calculation would go here
        }

        Ok(())
    }

    /// Check if an operation is permitted
    pub fn check_permission(&self, operation: &OperationType) -> PluginResult<()> {
        if !self.config.enable_sandbox {
            return Ok(());
        }

        match operation {
            OperationType::FileRead(path) => {
                self.check_file_permission(path, false)
            }
            OperationType::FileWrite(path) => {
                self.check_file_permission(path, true)
            }
            OperationType::NetworkRequest(domain) => {
                self.check_network_permission(domain)
            }
            OperationType::SystemCall(call) => {
                self.check_system_permission(call)
            }
        }
    }

    /// Record resource usage
    pub fn record_cpu_usage(&self, usage: f32) -> PluginResult<()> {
        if !self.config.enable_sandbox {
            return Ok(());
        }

        {
            let mut monitor = self.resource_monitor.lock().unwrap();
            monitor.cpu_samples.push(usage);
            
            // Keep only last 100 samples
            if monitor.cpu_samples.len() > 100 {
                monitor.cpu_samples.remove(0);
            }
        }

        // Check CPU limit
        if usage > self.config.max_cpu_usage {
            self.record_violation(SecurityViolation {
                timestamp: SystemTime::now(),
                violation_type: ViolationType::CpuExceeded,
                description: format!("CPU usage {:.1}% exceeds limit {:.1}%", 
                                   usage * 100.0, self.config.max_cpu_usage * 100.0),
                severity: Severity::High,
            })?;
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.cpu_usage = usage;
        }

        Ok(())
    }

    /// Record memory usage
    pub fn record_memory_usage(&self, bytes: usize) -> PluginResult<()> {
        if !self.config.enable_sandbox {
            return Ok(());
        }

        {
            let mut monitor = self.resource_monitor.lock().unwrap();
            monitor.memory_current = bytes;
            monitor.memory_peak = monitor.memory_peak.max(bytes);
        }

        // Check memory limit
        if bytes > self.config.max_memory_usage {
            self.record_violation(SecurityViolation {
                timestamp: SystemTime::now(),
                violation_type: ViolationType::MemoryExceeded,
                description: format!("Memory usage {} bytes exceeds limit {} bytes", 
                                   bytes, self.config.max_memory_usage),
                severity: Severity::High,
            })?;
        }

        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.memory_usage = bytes;
        }

        Ok(())
    }

    /// Get current statistics
    pub fn get_stats(&self) -> PluginStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }

    /// Get security violations
    pub fn get_violations(&self) -> Vec<SecurityViolation> {
        let monitor = self.resource_monitor.lock().unwrap();
        monitor.violations.clone()
    }

    /// Check if plugin should be disabled due to violations
    pub fn should_disable(&self) -> bool {
        if !self.config.auto_disable_on_error {
            return false;
        }

        let monitor = self.resource_monitor.lock().unwrap();
        
        // Count critical violations in last 5 minutes
        let cutoff = SystemTime::now() - Duration::from_secs(300);
        let critical_violations = monitor.violations.iter()
            .filter(|v| v.timestamp >= cutoff && v.severity == Severity::Critical)
            .count();

        critical_violations >= 3
    }

    /// Validate system requirements
    fn validate_system_requirements(&self) -> PluginResult<()> {
        // Check minimum memory
        let available_memory = self.get_available_system_memory();
        if available_memory < self.requirements.min_memory {
            return Err(PluginError::ResourceLimit {
                plugin_id: self.plugin_id.clone(),
                resource: "memory".to_string(),
                limit: format!("Required: {} bytes, Available: {} bytes", 
                             self.requirements.min_memory, available_memory),
            });
        }

        // Check CPU cores
        let cpu_cores = num_cpus::get() as u32;
        if cpu_cores < self.requirements.min_cpu_cores {
            return Err(PluginError::ResourceLimit {
                plugin_id: self.plugin_id.clone(),
                resource: "cpu_cores".to_string(),
                limit: format!("Required: {}, Available: {}", 
                             self.requirements.min_cpu_cores, cpu_cores),
            });
        }

        Ok(())
    }

    /// Check file access permission
    fn check_file_permission(&self, path: &str, write: bool) -> PluginResult<()> {
        if !self.config.allow_file_system {
            return Err(PluginError::SecurityViolation {
                plugin_id: self.plugin_id.clone(),
                violation: "File system access not allowed".to_string(),
            });
        }

        for permission in &self.permissions {
            match permission {
                Permission::FileRead { path: allowed_path } if !write => {
                    if path.starts_with(allowed_path) {
                        return Ok(());
                    }
                }
                Permission::FileWrite { path: allowed_path } if write => {
                    if path.starts_with(allowed_path) {
                        return Ok(());
                    }
                }
                _ => continue,
            }
        }

        let operation = if write { "write" } else { "read" };
        Err(PluginError::SecurityViolation {
            plugin_id: self.plugin_id.clone(),
            violation: format!("Unauthorized file {} access: {}", operation, path),
        })
    }

    /// Check network access permission
    fn check_network_permission(&self, domain: &str) -> PluginResult<()> {
        if !self.config.allow_network {
            return Err(PluginError::SecurityViolation {
                plugin_id: self.plugin_id.clone(),
                violation: "Network access not allowed".to_string(),
            });
        }

        for permission in &self.permissions {
            if let Permission::Network { domains } = permission {
                if domains.iter().any(|d| domain.ends_with(d)) {
                    return Ok(());
                }
            }
        }

        Err(PluginError::SecurityViolation {
            plugin_id: self.plugin_id.clone(),
            violation: format!("Unauthorized network access: {}", domain),
        })
    }

    /// Check system call permission
    fn check_system_permission(&self, _call: &str) -> PluginResult<()> {
        // Most system calls are denied by default in sandbox mode
        Err(PluginError::SecurityViolation {
            plugin_id: self.plugin_id.clone(),
            violation: "System calls not allowed in sandbox".to_string(),
        })
    }

    /// Record a security violation
    fn record_violation(&self, violation: SecurityViolation) -> PluginResult<()> {
        {
            let mut monitor = self.resource_monitor.lock().unwrap();
            monitor.violations.push(violation.clone());
            
            // Keep only last 1000 violations
            if monitor.violations.len() > 1000 {
                monitor.violations.remove(0);
            }
        }

        // Log violation
        match violation.severity {
            Severity::Critical => {
                eprintln!("CRITICAL SECURITY VIOLATION in plugin {}: {}", 
                         self.plugin_id, violation.description);
            }
            Severity::High => {
                eprintln!("Security violation in plugin {}: {}", 
                         self.plugin_id, violation.description);
            }
            _ => {
                println!("Security warning in plugin {}: {}", 
                        self.plugin_id, violation.description);
            }
        }

        Ok(())
    }

    /// Start resource monitoring thread
    fn start_resource_monitor_thread(&self) {
        let _plugin_id = self.plugin_id.clone();
        let monitor = Arc::clone(&self.resource_monitor);
        let stats = Arc::clone(&self.stats);
        let _config = self.config.clone();

        thread::spawn(move || {
            let mut last_check = Instant::now();
            
            loop {
                thread::sleep(Duration::from_millis(100)); // Check every 100ms
                
                let now = Instant::now();
                if now.duration_since(last_check) >= Duration::from_secs(1) {
                    // Update CPU usage calculation
                    if let Ok(mut monitor_guard) = monitor.lock() {
                        monitor_guard.last_cpu_check = now;
                        
                        if !monitor_guard.cpu_samples.is_empty() {
                            let avg_cpu: f32 = monitor_guard.cpu_samples.iter().sum::<f32>() 
                                              / monitor_guard.cpu_samples.len() as f32;
                            
                            if let Ok(mut stats_guard) = stats.write() {
                                stats_guard.cpu_usage = avg_cpu;
                                stats_guard.memory_usage = monitor_guard.memory_current;
                            }
                        }
                    }
                    
                    last_check = now;
                }
            }
        });
    }

    /// Get available system memory (stub implementation)
    fn get_available_system_memory(&self) -> u64 {
        // In a real implementation, this would query actual system memory
        8 * 1024 * 1024 * 1024 // 8GB assumption
    }
}

/// Operations that plugins can perform
pub enum OperationType {
    FileRead(String),
    FileWrite(String),
    NetworkRequest(String),
    SystemCall(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::plugin::PluginCategory;

    #[test]
    fn test_sandbox_creation() {
        let requirements = Requirements {
            min_memory: 64 * 1024 * 1024,
            min_cpu_cores: 1,
            max_cpu_usage: 0.1,
            network_access: false,
            file_access: vec![],
            platforms: vec!["linux".to_string()],
            permissions: vec![],
        };

        let sandbox = PluginSandbox::new(
            "test_plugin".to_string(),
            PluginConfig::default(),
            requirements,
            vec![],
        );

        assert_eq!(sandbox.plugin_id, "test_plugin");
    }

    #[test]
    fn test_cpu_usage_monitoring() {
        let requirements = Requirements {
            min_memory: 64 * 1024 * 1024,
            min_cpu_cores: 1,
            max_cpu_usage: 0.1,
            network_access: false,
            file_access: vec![],
            platforms: vec!["linux".to_string()],
            permissions: vec![],
        };

        let sandbox = PluginSandbox::new(
            "test_plugin".to_string(),
            PluginConfig::default(),
            requirements,
            vec![],
        );

        // Record normal CPU usage
        assert!(sandbox.record_cpu_usage(0.05).is_ok());
        
        // Record excessive CPU usage (should create violation)
        assert!(sandbox.record_cpu_usage(0.8).is_ok());
        
        let violations = sandbox.get_violations();
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_permission_checking() {
        let requirements = Requirements {
            min_memory: 64 * 1024 * 1024,
            min_cpu_cores: 1,
            max_cpu_usage: 0.1,
            network_access: false,
            file_access: vec![],
            platforms: vec!["linux".to_string()],
            permissions: vec![],
        };

        let permissions = vec![
            Permission::FileRead { path: "/tmp".to_string() },
            Permission::Network { domains: vec!["example.com".to_string()] },
        ];

        let sandbox = PluginSandbox::new(
            "test_plugin".to_string(),
            PluginConfig {
                allow_file_system: true,
                allow_network: true,
                ..Default::default()
            },
            requirements,
            permissions,
        );

        // Allowed file access
        assert!(sandbox.check_permission(&OperationType::FileRead("/tmp/test.txt".to_string())).is_ok());
        
        // Denied file access
        assert!(sandbox.check_permission(&OperationType::FileRead("/etc/passwd".to_string())).is_err());
        
        // Allowed network access
        assert!(sandbox.check_permission(&OperationType::NetworkRequest("api.example.com".to_string())).is_ok());
        
        // Denied network access
        assert!(sandbox.check_permission(&OperationType::NetworkRequest("malicious.com".to_string())).is_err());
    }
}