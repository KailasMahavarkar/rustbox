/// Resource limit management using rlimit system calls
use crate::types::{IsolateError, Result};
use nix::sys::resource::{setrlimit, getrlimit, Resource};
use std::fs;
use std::path::Path;

/// Resource limit controller for managing process resource limits
pub struct ResourceLimitController {
    strict_mode: bool,
}

impl ResourceLimitController {
    /// Create a new resource limit controller
    pub fn new(strict_mode: bool) -> Self {
        Self { strict_mode }
    }

    /// Set stack size limit using rlimit
    pub fn set_stack_limit(&self, limit_bytes: u64) -> Result<()> {
        match setrlimit(Resource::RLIMIT_STACK, limit_bytes, limit_bytes) {
            Ok(_) => Ok(()),
            Err(e) => {
                let error_msg = format!("Failed to set stack limit: {}", e);
                if self.strict_mode {
                    Err(IsolateError::ResourceLimit(error_msg))
                } else {
                    eprintln!("Warning: {}", error_msg);
                    Ok(())
                }
            }
        }
    }

    /// Set core dump size limit using rlimit
    pub fn set_core_limit(&self, limit_bytes: u64) -> Result<()> {
        match setrlimit(Resource::RLIMIT_CORE, limit_bytes, limit_bytes) {
            Ok(_) => Ok(()),
            Err(e) => {
                let error_msg = format!("Failed to set core dump limit: {}", e);
                if self.strict_mode {
                    Err(IsolateError::ResourceLimit(error_msg))
                } else {
                    eprintln!("Warning: {}", error_msg);
                    Ok(())
                }
            }
        }
    }

    /// Set file size limit using rlimit (for individual files)
    pub fn set_file_size_limit(&self, limit_bytes: u64) -> Result<()> {
        match setrlimit(Resource::RLIMIT_FSIZE, limit_bytes, limit_bytes) {
            Ok(_) => Ok(()),
            Err(e) => {
                let error_msg = format!("Failed to set file size limit: {}", e);
                if self.strict_mode {
                    Err(IsolateError::ResourceLimit(error_msg))
                } else {
                    eprintln!("Warning: {}", error_msg);
                    Ok(())
                }
            }
        }
    }

    /// Set virtual memory limit using rlimit
    pub fn set_virtual_memory_limit(&self, limit_bytes: u64) -> Result<()> {
        match setrlimit(Resource::RLIMIT_AS, limit_bytes, limit_bytes) {
            Ok(_) => Ok(()),
            Err(e) => {
                let error_msg = format!("Failed to set virtual memory limit: {}", e);
                if self.strict_mode {
                    Err(IsolateError::ResourceLimit(error_msg))
                } else {
                    eprintln!("Warning: {}", error_msg);
                    Ok(())
                }
            }
        }
    }

    /// Set CPU time limit using rlimit
    pub fn set_cpu_time_limit(&self, limit_seconds: u64) -> Result<()> {
        match setrlimit(Resource::RLIMIT_CPU, limit_seconds, limit_seconds) {
            Ok(_) => Ok(()),
            Err(e) => {
                let error_msg = format!("Failed to set CPU time limit: {}", e);
                if self.strict_mode {
                    Err(IsolateError::ResourceLimit(error_msg))
                } else {
                    eprintln!("Warning: {}", error_msg);
                    Ok(())
                }
            }
        }
    }

    /// Set number of processes limit using rlimit
    pub fn set_process_limit(&self, limit: u64) -> Result<()> {
        match setrlimit(Resource::RLIMIT_NPROC, limit, limit) {
            Ok(_) => Ok(()),
            Err(e) => {
                let error_msg = format!("Failed to set process limit: {}", e);
                if self.strict_mode {
                    Err(IsolateError::ResourceLimit(error_msg))
                } else {
                    eprintln!("Warning: {}", error_msg);
                    Ok(())
                }
            }
        }
    }

    /// Set disk quota limit (simplified implementation using directory size monitoring)
    /// Note: Real disk quotas require filesystem-level support (ext2/3/4, XFS, etc.)
    pub fn set_disk_quota(&self, workdir: &Path, limit_bytes: u64) -> Result<()> {
        if !workdir.exists() {
            return Err(IsolateError::ResourceLimit(
                "Working directory does not exist for disk quota".to_string()
            ));
        }

        // For now, we'll implement a simple check rather than true filesystem quotas
        // Real implementation would require quota tools (setquota, quotactl syscall)
        let usage = self.get_directory_size(workdir)?;
        
        if usage > limit_bytes {
            let error_msg = format!(
                "Directory size {} exceeds disk quota limit {}",
                usage, limit_bytes
            );
            if self.strict_mode {
                return Err(IsolateError::ResourceLimit(error_msg));
            } else {
                eprintln!("Warning: {}", error_msg);
            }
        }

        Ok(())
    }

    /// Get current directory size (recursive)
    pub fn get_directory_size(&self, dir: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    total_size += self.get_directory_size(&path)?;
                } else {
                    let metadata = entry.metadata()?;
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// Check if current usage exceeds disk quota
    pub fn check_disk_quota(&self, workdir: &Path, limit_bytes: u64) -> Result<bool> {
        let usage = self.get_directory_size(workdir)?;
        Ok(usage <= limit_bytes)
    }

    /// Get current resource limits for monitoring
    pub fn get_current_limits(&self) -> Result<ResourceLimits> {
        let stack = getrlimit(Resource::RLIMIT_STACK)
            .map_err(|e| IsolateError::ResourceLimit(format!("Failed to get stack limit: {}", e)))?;
        
        let core = getrlimit(Resource::RLIMIT_CORE)
            .map_err(|e| IsolateError::ResourceLimit(format!("Failed to get core limit: {}", e)))?;
        
        let fsize = getrlimit(Resource::RLIMIT_FSIZE)
            .map_err(|e| IsolateError::ResourceLimit(format!("Failed to get file size limit: {}", e)))?;
        
        let vmem = getrlimit(Resource::RLIMIT_AS)
            .map_err(|e| IsolateError::ResourceLimit(format!("Failed to get virtual memory limit: {}", e)))?;
        
        let cpu_time = getrlimit(Resource::RLIMIT_CPU)
            .map_err(|e| IsolateError::ResourceLimit(format!("Failed to get CPU time limit: {}", e)))?;
        
        let nproc = getrlimit(Resource::RLIMIT_NPROC)
            .map_err(|e| IsolateError::ResourceLimit(format!("Failed to get process limit: {}", e)))?;

        Ok(ResourceLimits {
            stack_soft: Some(stack.0),
            stack_hard: Some(stack.1),
            core_soft: Some(core.0),
            core_hard: Some(core.1),
            fsize_soft: Some(fsize.0),
            fsize_hard: Some(fsize.1),
            vmem_soft: Some(vmem.0),
            vmem_hard: Some(vmem.1),
            cpu_time_soft: Some(cpu_time.0),
            cpu_time_hard: Some(cpu_time.1),
            nproc_soft: Some(nproc.0),
            nproc_hard: Some(nproc.1),
        })
    }
}

/// Current resource limits structure
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub stack_soft: Option<u64>,
    pub stack_hard: Option<u64>,
    pub core_soft: Option<u64>,
    pub core_hard: Option<u64>,
    pub fsize_soft: Option<u64>,
    pub fsize_hard: Option<u64>,
    pub vmem_soft: Option<u64>,
    pub vmem_hard: Option<u64>,
    pub cpu_time_soft: Option<u64>,
    pub cpu_time_hard: Option<u64>,
    pub nproc_soft: Option<u64>,
    pub nproc_hard: Option<u64>,
}

/// Check if resource limits are supported on this system
pub fn resource_limits_supported() -> bool {
    // Check if we can read /proc/self/limits
    Path::new("/proc/self/limits").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile;

    #[test]
    fn test_resource_limit_controller_creation() {
        let controller = ResourceLimitController::new(false);
        assert!(!controller.strict_mode);

        let strict_controller = ResourceLimitController::new(true);
        assert!(strict_controller.strict_mode);
    }

    #[test]
    fn test_get_current_limits() {
        let controller = ResourceLimitController::new(false);
        let limits = controller.get_current_limits();
        assert!(limits.is_ok(), "Should be able to get current limits");
    }

    #[test]
    fn test_directory_size_calculation() {
        let controller = ResourceLimitController::new(false);
        // Create a temporary directory for testing
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        
        // Create a small test file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "Hello, World!").expect("Failed to write test file");
        
        let size = controller.get_directory_size(temp_dir.path());
        assert!(size.is_ok(), "Should be able to calculate directory size: {:?}", size);
        
        let calculated_size = size.unwrap();
        assert!(calculated_size >= 13, "Directory size should be at least 13 bytes (Hello, World!)");
    }

    #[test]
    fn test_resource_limits_supported() {
        // This should work on most Linux systems
        assert!(resource_limits_supported());
    }

    #[test]
    fn test_stack_limit_setting() {
        let controller = ResourceLimitController::new(false);
        // Set a reasonable stack limit (8MB)
        let result = controller.set_stack_limit(8 * 1024 * 1024);
        assert!(result.is_ok(), "Should be able to set stack limit");
    }

    #[test]
    fn test_core_limit_setting() {
        let controller = ResourceLimitController::new(false);
        // Disable core dumps
        let result = controller.set_core_limit(0);
        assert!(result.is_ok(), "Should be able to disable core dumps");
    }
}