/// Enhanced Cgroup management for resource control with improved CPU time tracking
use crate::types::{IsolateError, Result};
use std::fs;
use std::path::Path;

/// Enhanced cgroup controller with better CPU time tracking
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
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                if strict_mode {
                    return Err(IsolateError::Cgroup(
                        "Permission denied creating cgroup. Run with sudo for resource limits, or remove --strict flag.".to_string()
                    ));
                } else {
                    // Continue without cgroup support
                    eprintln!("Warning: Cannot create cgroup (permission denied). Resource limits will not be enforced.");
                    // Return a "dummy" controller that won't actually enforce limits
                    return Ok(Self {
                        name: name.to_string(),
                        cgroup_path: std::path::PathBuf::new(), // Empty path indicates no cgroup support
                    });
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to create cgroup directory: {}", e);
                if strict_mode {
                    return Err(IsolateError::Cgroup(error_msg));
                } else {
                    eprintln!("Warning: {}", error_msg);
                    // Return a "dummy" controller
                    return Ok(Self {
                        name: name.to_string(),
                        cgroup_path: std::path::PathBuf::new(),
                    });
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
        // Skip if no cgroup support (empty path)
        if self.cgroup_path.as_os_str().is_empty() {
            return Ok(());
        }
        let _ = self.write_cgroup_file("memory.limit_in_bytes", &limit_bytes.to_string());
        // Also set memory+swap limit to prevent swap usage
        let _ = self.write_cgroup_file("memory.memsw.limit_in_bytes", &limit_bytes.to_string());
        Ok(())
    }

    /// Set CPU time limit (using shares)
    pub fn set_cpu_limit(&self, cpu_shares: u64) -> Result<()> {
        // Skip if no cgroup support (empty path)
        if self.cgroup_path.as_os_str().is_empty() {
            return Ok(());
        }
        
        let cpu_path = Path::new("/sys/fs/cgroup/cpu").join(&self.name);
        let _ = fs::create_dir_all(&cpu_path);

        let shares_file = cpu_path.join("cpu.shares");
        let _ = fs::write(shares_file, cpu_shares.to_string());

        // Also create cpuacct cgroup for better CPU time tracking
        let cpuacct_path = Path::new("/sys/fs/cgroup/cpuacct").join(&self.name);
        let _ = fs::create_dir_all(&cpuacct_path);

        Ok(())
    }

    /// Set process/task limit
    pub fn set_process_limit(&self, limit: u64) -> Result<()> {
        // Skip if no cgroup support (empty path)
        if self.cgroup_path.as_os_str().is_empty() {
            return Ok(());
        }
        
        let pids_path = Path::new("/sys/fs/cgroup/pids").join(&self.name);
        let _ = fs::create_dir_all(&pids_path);

        let max_file = pids_path.join("pids.max");
        let _ = fs::write(max_file, limit.to_string());

        Ok(())
    }

    /// Add a process to this cgroup
    pub fn add_process(&self, pid: u32) -> Result<()> {
        // Skip if no cgroup support (empty path)
        if self.cgroup_path.as_os_str().is_empty() {
            return Ok(());
        }
        
        // Add to memory cgroup
        let _ = self.write_cgroup_file("tasks", &pid.to_string());

        // Add to CPU cgroup if exists
        let cpu_tasks = Path::new("/sys/fs/cgroup/cpu")
            .join(&self.name)
            .join("tasks");
        if cpu_tasks.parent().unwrap().exists() {
            let _ = fs::write(cpu_tasks, pid.to_string());
        }

        // Add to cpuacct cgroup for better CPU time tracking
        let cpuacct_tasks = Path::new("/sys/fs/cgroup/cpuacct")
            .join(&self.name)
            .join("tasks");
        if cpuacct_tasks.parent().unwrap().exists() {
            let _ = fs::write(cpuacct_tasks, pid.to_string());
        }

        // Add to PIDs cgroup if exists
        let pids_tasks = Path::new("/sys/fs/cgroup/pids")
            .join(&self.name)
            .join("tasks");
        if pids_tasks.parent().unwrap().exists() {
            let _ = fs::write(pids_tasks, pid.to_string());
        }

        Ok(())
    }

    /// Get peak memory usage
    pub fn get_peak_memory_usage(&self) -> Result<u64> {
        let usage = self.read_cgroup_file("memory.max_usage_in_bytes")?;
        usage
            .trim()
            .parse()
            .map_err(|e| IsolateError::Cgroup(format!("Failed to parse peak memory usage: {}", e)))
    }

    /// Enhanced CPU usage tracking with multiple methods for reliability
    pub fn get_cpu_usage(&self) -> Result<f64> {
        // Method 1: Try cpuacct.usage (most accurate nanosecond precision)
        let cpuacct_usage_path = Path::new("/sys/fs/cgroup/cpuacct")
            .join(&self.name)
            .join("usage");

        if cpuacct_usage_path.exists() {
            if let Ok(usage_content) = fs::read_to_string(&cpuacct_usage_path) {
                if let Ok(usage_ns) = usage_content.trim().parse::<u64>() {
                    if usage_ns > 0 {
                        // Convert nanoseconds to seconds with high precision
                        let cpu_time = usage_ns as f64 / 1_000_000_000.0;
                        return Ok(cpu_time);
                    }
                }
            }
        }

        // Method 2: Try cpuacct.stat for user+system breakdown
        let cpuacct_stat_path = Path::new("/sys/fs/cgroup/cpuacct")
            .join(&self.name)
            .join("stat");

        if cpuacct_stat_path.exists() {
            if let Ok(stat_content) = fs::read_to_string(&cpuacct_stat_path) {
                let mut user_time = 0u64;
                let mut sys_time = 0u64;
                
                for line in stat_content.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        match parts[0] {
                            "user" => {
                                user_time = parts[1].parse().unwrap_or(0);
                            }
                            "system" => {
                                sys_time = parts[1].parse().unwrap_or(0);
                            }
                            _ => {}
                        }
                    }
                }
                
                if user_time > 0 || sys_time > 0 {
                    // cpuacct.stat values are in USER_HZ (usually 100Hz)
                    let total_time = user_time + sys_time;
                    let cpu_time = total_time as f64 / 100.0; // Convert USER_HZ to seconds
                    return Ok(cpu_time);
                }
            }
        }

        // Method 3: Try cpu.stat for throttling information
        let cpu_stat_path = Path::new("/sys/fs/cgroup/cpu")
            .join(&self.name)
            .join("stat");

        if cpu_stat_path.exists() {
            if let Ok(stat_content) = fs::read_to_string(&cpu_stat_path) {
                for line in stat_content.lines() {
                    if line.starts_with("throttled_time") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(throttled_ns) = parts[1].parse::<u64>() {
                                if throttled_ns > 0 {
                                    // This gives us throttled time in nanoseconds
                                    let cpu_time = throttled_ns as f64 / 1_000_000_000.0;
                                    return Ok(cpu_time);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Method 4: Fallback to 0.0 (will trigger /proc fallback in executor)
        Ok(0.0)
    }

    /// Remove the cgroup when done
    pub fn cleanup(&self) -> Result<()> {
        // Remove cgroup directories (they must be empty first)
        let dirs = [
            &self.cgroup_path,
            &Path::new("/sys/fs/cgroup/cpu").join(&self.name),
            &Path::new("/sys/fs/cgroup/pids").join(&self.name),
            &Path::new("/sys/fs/cgroup/memory").join(&self.name),
            &Path::new("/sys/fs/cgroup/cpuacct").join(&self.name),
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
        // Skip if no cgroup support (empty path)
        if self.cgroup_path.as_os_str().is_empty() {
            return Ok(());
        }
        
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
    Path::new("/proc/cgroups").exists() && Path::new("/sys/fs/cgroup").exists()
}

/// Get cgroup mount point
pub fn get_cgroup_mount() -> Result<String> {
    if !cgroups_available() {
        return Err(IsolateError::Cgroup(
            "Cgroups not available on this system".to_string(),
        ));
    }

    // For cgroup v1, typically mounted at /sys/fs/cgroup
    Ok("/sys/fs/cgroup".to_string())
}