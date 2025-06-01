/// Cgroup management for resource control - simplified implementation
use crate::types::{IsolateError, Result};
use std::path::Path;
use std::fs;

/// Simplified cgroup controller for managing process resources
pub struct CgroupController {
    name: String,
    cgroup_path: std::path::PathBuf,
}

impl CgroupController {
    /// Create a new cgroup controller
    pub fn new(name: &str, strict_mode: bool) -> Result<Self> {
        let cgroup_base = "/sys/fs/cgroup";
        let cgroup_path = Path::new(cgroup_base).join("memory").join(name);
        
        // Try to create the cgroup directory
        match fs::create_dir_all(&cgroup_path) {
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                if strict_mode {
                    return Err(IsolateError::Cgroup(
                        "Permission denied creating cgroup. Run with sudo for resource limits, or remove --strict flag.".to_string()
                    ));
                } else {
                    // Continue without cgroup support
                    eprintln!("Warning: Cannot create cgroup (permission denied). Resource limits will not be enforced.");
                }
            },
            Err(e) => {
                let error_msg = format!("Failed to create cgroup directory: {}", e);
                if strict_mode {
                    return Err(IsolateError::Cgroup(error_msg));
                } else {
                    eprintln!("Warning: {}", error_msg);
                }
            }
        }

        Ok(Self {
            name: name.to_string(),
            cgroup_path,
        })
    }

    /// Set memory limit in bytes
    pub fn set_memory_limit(&self, limit_bytes: u64) -> Result<()> {
        let _ = self.write_cgroup_file("memory.limit_in_bytes", &limit_bytes.to_string());
        // Also set memory+swap limit to prevent swap usage
        let _ = self.write_cgroup_file("memory.memsw.limit_in_bytes", &limit_bytes.to_string());
        Ok(())
    }

    /// Set CPU time limit (using shares)
    pub fn set_cpu_limit(&self, cpu_shares: u64) -> Result<()> {
        let cpu_path = Path::new("/sys/fs/cgroup/cpu").join(&self.name);
        let _ = fs::create_dir_all(&cpu_path);
        
        let shares_file = cpu_path.join("cpu.shares");
        let _ = fs::write(shares_file, cpu_shares.to_string());
        
        Ok(())
    }

    /// Set process/task limit
    pub fn set_process_limit(&self, limit: u64) -> Result<()> {
        let pids_path = Path::new("/sys/fs/cgroup/pids").join(&self.name);
        let _ = fs::create_dir_all(&pids_path);
        
        let max_file = pids_path.join("pids.max");
        let _ = fs::write(max_file, limit.to_string());
        
        Ok(())
    }

    /// Add a process to this cgroup
    pub fn add_process(&self, pid: u32) -> Result<()> {
        // Add to memory cgroup
        let _ = self.write_cgroup_file("tasks", &pid.to_string());
        
        // Add to CPU cgroup if exists
        let cpu_tasks = Path::new("/sys/fs/cgroup/cpu").join(&self.name).join("tasks");
        if cpu_tasks.parent().unwrap().exists() {
            let _ = fs::write(cpu_tasks, pid.to_string());
        }
        
        // Add to PIDs cgroup if exists
        let pids_tasks = Path::new("/sys/fs/cgroup/pids").join(&self.name).join("tasks");
        if pids_tasks.parent().unwrap().exists() {
            let _ = fs::write(pids_tasks, pid.to_string());
        }
        
        Ok(())
    }



    /// Get peak memory usage
    pub fn get_peak_memory_usage(&self) -> Result<u64> {
        let usage = self.read_cgroup_file("memory.max_usage_in_bytes")?;
        usage.trim().parse()
            .map_err(|e| IsolateError::Cgroup(format!("Failed to parse peak memory usage: {}", e)))
    }

    /// Get CPU usage statistics (approximate)
    pub fn get_cpu_usage(&self) -> Result<f64> {
        let cpu_stat_path = Path::new("/sys/fs/cgroup/cpu").join(&self.name).join("cpuacct.usage");
        
        if cpu_stat_path.exists() {
            let usage_ns = fs::read_to_string(cpu_stat_path)
                .map_err(|e| IsolateError::Cgroup(format!("Failed to read CPU usage: {}", e)))?;
                
            let usage_ns: u64 = usage_ns.trim().parse()
                .map_err(|e| IsolateError::Cgroup(format!("Failed to parse CPU usage: {}", e)))?;
                
            // Convert nanoseconds to seconds
            Ok(usage_ns as f64 / 1_000_000_000.0)
        } else {
            Ok(0.0)
        }
    }



    /// Remove the cgroup when done
    pub fn cleanup(&self) -> Result<()> {
        // Remove cgroup directories (they must be empty first)
        let dirs = [
            &self.cgroup_path,
            &Path::new("/sys/fs/cgroup/cpu").join(&self.name),
            &Path::new("/sys/fs/cgroup/pids").join(&self.name),
        ];
        
        for dir in &dirs {
            if dir.exists() {
                let _ = fs::remove_dir(dir);
            }
        }
        
        Ok(())
    }


    
    /// Helper to write to cgroup files
    fn write_cgroup_file(&self, filename: &str, content: &str) -> Result<()> {
        let file_path = self.cgroup_path.join(filename);
        fs::write(file_path, content)
            .map_err(|e| IsolateError::Cgroup(format!("Failed to write {}: {}", filename, e)))
    }
    
    /// Helper to read from cgroup files
    fn read_cgroup_file(&self, filename: &str) -> Result<String> {
        let file_path = self.cgroup_path.join(filename);
        fs::read_to_string(file_path)
            .map_err(|e| IsolateError::Cgroup(format!("Failed to read {}: {}", filename, e)))
    }
}

impl Drop for CgroupController {
    fn drop(&mut self) {
        // Attempt cleanup on drop, but don't panic if it fails
        let _ = self.cleanup();
    }
}

/// Check if cgroups are available on the system
pub fn cgroups_available() -> bool {
    Path::new("/proc/cgroups").exists() && 
    Path::new("/sys/fs/cgroup").exists()
}

/// Get cgroup mount point
pub fn get_cgroup_mount() -> Result<String> {
    if !cgroups_available() {
        return Err(IsolateError::Cgroup("Cgroups not available on this system".to_string()));
    }
    
    // For cgroup v1, typically mounted at /sys/fs/cgroup
    Ok("/sys/fs/cgroup".to_string())
}