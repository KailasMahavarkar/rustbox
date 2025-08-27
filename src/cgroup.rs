/// Enhanced Cgroup management for resource control with improved reliability
use crate::types::{IsolateError, Result};
use std::collections::HashSet;
use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};

pub struct Cgroup {
    name: String,
    cgroup_path: PathBuf,
    available_controllers: HashSet<String>,
    has_cgroup_support: bool,
}

impl Cgroup {
    pub fn new(name: &str, strict_mode: bool) -> Result<Self> {
        // Sanitize the name by replacing forward slashes with underscores
        let sanitized_name = name.replace("/", "_");
        let cgroup_base = "/sys/fs/cgroup";
        let cgroup_path = Path::new(cgroup_base).join("memory").join(&sanitized_name);

        let cgroups_available = Self::cgroups_available();
        if !cgroups_available {
            if strict_mode {
                return Err(IsolateError::Cgroup(
                    "Cgroups not available on this system".to_string(),
                ));
            } else {
                eprintln!("Warning: Cgroups not available. Resource limits will not be enforced.");
                return Ok(Self {
                    name: name.replace("/", "_"),
                    cgroup_path: PathBuf::new(),
                    available_controllers: HashSet::new(),
                    has_cgroup_support: false,
                });
            }
        }

        let available_controllers = match Self::get_available_controllers() {
            Ok(controllers) => controllers,
            Err(e) => {
                if strict_mode {
                    return Err(IsolateError::Cgroup(format!(
                        "Failed to get available controllers: {}",
                        e
                    )));
                } else {
                    eprintln!("Warning: Failed to get available controllers: {}", e);
                    HashSet::new()
                }
            }
        };

        if strict_mode {
            let required_controllers = vec!["memory", "cpu", "cpuacct"];
            for controller in &required_controllers {
                if !available_controllers.contains(*controller) {
                    return Err(IsolateError::Cgroup(format!(
                        "Required controller '{}' not available. Available controllers: {:?}",
                        controller, available_controllers
                    )));
                }
            }
        }

        match fs::create_dir_all(&cgroup_path) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                if strict_mode {
                    return Err(IsolateError::Cgroup(
                        "Permission denied creating cgroup. Run with sudo for resource limits, or remove --strict flag.".to_string(),
                    ));
                } else {
                    eprintln!("Warning: Cannot create cgroup (permission denied). Resource limits will not be enforced.");
                    return Ok(Self {
                        name: name.replace("/", "_"),
                        cgroup_path: PathBuf::new(),
                        available_controllers,
                        has_cgroup_support: false,
                    });
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to create cgroup directory: {}", e);
                if strict_mode {
                    return Err(IsolateError::Cgroup(error_msg));
                } else {
                    eprintln!("Warning: {}", error_msg);
                    return Ok(Self {
                        name: name.replace("/", "_"),
                        cgroup_path: PathBuf::new(),
                        available_controllers,
                        has_cgroup_support: false,
                    });
                }
            }
        }

        Ok(Self {
            name: sanitized_name.clone(),
            cgroup_path,
            available_controllers,
            has_cgroup_support: true,
        })
    }

    pub fn set_memory_limit(&self, limit_bytes: u64) -> Result<()> {
        if !self.has_cgroup_support || !self.available_controllers.contains("memory") {
            return Ok(());
        }

        self.write_cgroup_file("memory.limit_in_bytes", &limit_bytes.to_string())?;

        let memsw_file = self.cgroup_path.join("memory.memsw.limit_in_bytes");
        if memsw_file.exists() {
            let _ = self.write_cgroup_file("memory.memsw.limit_in_bytes", &limit_bytes.to_string());
        }

        Ok(())
    }

    pub fn set_cpu_limit(&self, cpu_shares: u64) -> Result<()> {
        if !self.has_cgroup_support || !self.available_controllers.contains("cpu") {
            return Ok(());
        }

        let cpu_path = Path::new("/sys/fs/cgroup/cpu").join(&self.name);
        if let Err(e) = fs::create_dir_all(&cpu_path) {
            eprintln!("Warning: Failed to create CPU cgroup: {}", e);
            return Ok(());
        }

        let shares_file = cpu_path.join("cpu.shares");
        if let Err(e) = fs::write(shares_file, cpu_shares.to_string()) {
            eprintln!("Warning: Failed to set CPU shares: {}", e);
        }

        if self.available_controllers.contains("cpuacct") {
            let cpuacct_path = Path::new("/sys/fs/cgroup/cpuacct").join(&self.name);
            let _ = fs::create_dir_all(&cpuacct_path);
        }

        Ok(())
    }

    pub fn set_process_limit(&self, limit: u64) -> Result<()> {
        if !self.has_cgroup_support || !self.available_controllers.contains("pids") {
            return Ok(());
        }

        let pids_path = Path::new("/sys/fs/cgroup/pids").join(&self.name);
        if let Err(e) = fs::create_dir_all(&pids_path) {
            eprintln!("Warning: Failed to create PIDs cgroup: {}", e);
            return Ok(());
        }

        let max_file = pids_path.join("pids.max");
        if let Err(e) = fs::write(max_file, limit.to_string()) {
            eprintln!("Warning: Failed to set process limit: {}", e);
        }

        Ok(())
    }

    pub fn add_process(&self, pid: u32) -> Result<()> {
        if !self.has_cgroup_support {
            return Ok(());
        }

        let pid_str = pid.to_string();

        if self.available_controllers.contains("memory") {
            if let Err(e) = self.write_cgroup_file("tasks", &pid_str) {
                eprintln!("Warning: Failed to add process to memory cgroup: {}", e);
            }
        }

        if self.available_controllers.contains("cpu") {
            let cpu_tasks = Path::new("/sys/fs/cgroup/cpu")
                .join(&self.name)
                .join("tasks");
            if cpu_tasks.parent().unwrap().exists() {
                if let Err(e) = fs::write(cpu_tasks, &pid_str) {
                    eprintln!("Warning: Failed to add process to CPU cgroup: {}", e);
                }
            }
        }

        if self.available_controllers.contains("cpuacct") {
            let cpuacct_tasks = Path::new("/sys/fs/cgroup/cpuacct")
                .join(&self.name)
                .join("tasks");
            if cpuacct_tasks.parent().unwrap().exists() {
                if let Err(e) = fs::write(cpuacct_tasks, &pid_str) {
                    eprintln!("Warning: Failed to add process to cpuacct cgroup: {}", e);
                }
            }
        }

        if self.available_controllers.contains("pids") {
            let pids_tasks = Path::new("/sys/fs/cgroup/pids")
                .join(&self.name)
                .join("tasks");
            if pids_tasks.parent().unwrap().exists() {
                if let Err(e) = fs::write(pids_tasks, &pid_str) {
                    eprintln!("Warning: Failed to add process to PIDs cgroup: {}", e);
                }
            }
        }

        Ok(())
    }

    pub fn get_peak_memory_usage(&self) -> Result<u64> {
        if !self.has_cgroup_support || !self.available_controllers.contains("memory") {
            return Ok(0);
        }

        let usage = self.read_cgroup_file("memory.max_usage_in_bytes")?;
        usage
            .trim()
            .parse()
            .map_err(|e| IsolateError::Cgroup(format!("Failed to parse peak memory usage: {}", e)))
    }

    /// Get current memory usage (more reliable than peak for live monitoring)
    pub fn get_current_memory_usage(&self) -> Result<u64> {
        if !self.has_cgroup_support || !self.available_controllers.contains("memory") {
            return Ok(0);
        }

        let usage = self.read_cgroup_file("memory.usage_in_bytes")?;
        usage
            .trim()
            .parse()
            .map_err(|e| IsolateError::Cgroup(format!("Failed to parse current memory usage: {}", e)))
    }

    /// Get comprehensive memory statistics from cgroup
    pub fn get_memory_stats(&self) -> Result<(u64, u64, u64)> {
        if !self.has_cgroup_support || !self.available_controllers.contains("memory") {
            return Ok((0, 0, 0));
        }

        let current = self.get_current_memory_usage().unwrap_or(0);
        let peak = self.get_peak_memory_usage().unwrap_or(0);
        
        // Try to get memory limit
        let limit = self.read_cgroup_file("memory.limit_in_bytes")
            .and_then(|s| s.trim().parse::<u64>().map_err(|e| IsolateError::Cgroup(e.to_string())))
            .unwrap_or(u64::MAX);

        Ok((current, peak, limit))
    }

    /// Check if the process hit the memory limit (OOM condition)
    pub fn check_oom_killed(&self) -> bool {
        if !self.has_cgroup_support || !self.available_controllers.contains("memory") {
            return false;
        }

        // Check memory.oom_control for under_oom flag
        if let Ok(oom_control) = self.read_cgroup_file("memory.oom_control") {
            if oom_control.contains("under_oom 1") {
                return true;
            }
        }

        // Also check memory.stat for oom_kill events
        if let Ok(memory_stat) = self.read_cgroup_file("memory.stat") {
            for line in memory_stat.lines() {
                if line.starts_with("oom_kill ") || line.starts_with("oom ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(count) = parts[1].parse::<u64>() {
                            if count > 0 {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // Check if current memory usage equals the limit (potential OOM)
        if let (Ok(limit), Ok(usage)) = (
            self.read_cgroup_file("memory.limit_in_bytes").and_then(|s| {
                s.trim().parse::<u64>().map_err(|e| IsolateError::Cgroup(e.to_string()))
            }),
            self.read_cgroup_file("memory.usage_in_bytes").and_then(|s| {
                s.trim().parse::<u64>().map_err(|e| IsolateError::Cgroup(e.to_string()))
            })
        ) {
            // If usage is very close to limit (within 1MB), consider it OOM
            if limit > 0 && usage >= limit.saturating_sub(1024 * 1024) {
                return true;
            }
        }

        false
    }

    pub fn get_cpu_usage(&self) -> Result<f64> {
        if !self.has_cgroup_support || !self.available_controllers.contains("cpuacct") {
            return Ok(0.0);
        }

        // Method 1: Try cpuacct.usage (nanoseconds, most accurate)
        let cpuacct_usage_path = Path::new("/sys/fs/cgroup/cpuacct")
            .join(&self.name)
            .join("cpuacct.usage");

        if cpuacct_usage_path.exists() {
            if let Ok(usage_content) = fs::read_to_string(&cpuacct_usage_path) {
                if let Ok(usage_ns) = usage_content.trim().parse::<u64>() {
                    let cpu_time = usage_ns as f64 / 1_000_000_000.0;
                    return Ok(cpu_time);
                }
            }
        }

        // Method 2: Try cpuacct.stat (USER_HZ units, fallback)
        let cpuacct_stat_path = Path::new("/sys/fs/cgroup/cpuacct")
            .join(&self.name)
            .join("cpuacct.stat");

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
                    let total_time = user_time + sys_time;
                    // Convert USER_HZ to seconds (typically USER_HZ = 100)
                    let cpu_time = total_time as f64 / 100.0;
                    return Ok(cpu_time);
                }
            }
        }

        Ok(0.0)
    }

    /// Get comprehensive resource usage statistics from cgroups exclusively
    pub fn get_resource_stats(&self) -> (f64, u64, bool) {
        let cpu_time = self.get_cpu_usage().unwrap_or(0.0);
        let memory_peak = self.get_peak_memory_usage().unwrap_or(0);
        let oom_killed = self.check_oom_killed();
        
        (cpu_time, memory_peak, oom_killed)
    }

    /// Check if cgroup is in a resource limit violation state
    pub fn is_resource_limited(&self) -> (bool, bool) {
        let oom_killed = self.check_oom_killed();
        
        // Check if memory usage is at or near limit
        let memory_limited = if let (Ok(current), Ok(limit)) = (
            self.get_current_memory_usage(),
            self.read_cgroup_file("memory.limit_in_bytes")
                .and_then(|s| s.trim().parse::<u64>().map_err(|e| IsolateError::Cgroup(e.to_string())))
        ) {
            limit > 0 && current >= limit.saturating_sub(1024 * 1024) // Within 1MB of limit
        } else {
            false
        };
        
        (oom_killed || memory_limited, false) // (memory_limited, cpu_limited)
    }

    pub fn cleanup(&self) -> Result<()> {
        if !self.has_cgroup_support {
            return Ok(());
        }

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

    fn get_available_controllers() -> Result<HashSet<String>> {
        let content = fs::read_to_string("/proc/cgroups")
            .map_err(|e| IsolateError::Cgroup(format!("Failed to read /proc/cgroups: {}", e)))?;

        let mut controllers = HashSet::new();

        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let controller_name = parts[0];
                let enabled = parts[3] == "1";

                if enabled {
                    controllers.insert(controller_name.to_string());
                }
            }
        }

        Ok(controllers)
    }

    pub fn cgroups_available() -> bool {
        Path::new("/proc/cgroups").exists() && Path::new("/sys/fs/cgroup").exists()
    }

    fn write_cgroup_file(&self, filename: &str, content: &str) -> Result<()> {
        if !self.has_cgroup_support {
            return Ok(());
        }

        let file_path = self.cgroup_path.join(filename);
        fs::write(file_path, content)
            .map_err(|e| IsolateError::Cgroup(format!("Failed to write {}: {}", filename, e)))
    }

    fn read_cgroup_file(&self, filename: &str) -> Result<String> {
        if !self.has_cgroup_support {
            return Ok("0".to_string());
        }

        let file_path = self.cgroup_path.join(filename);
        fs::read_to_string(file_path)
            .map_err(|e| IsolateError::Cgroup(format!("Failed to read {}: {}", filename, e)))
    }

    pub fn get_info(&self) -> CgroupInfo {
        CgroupInfo {
            name: self.name.clone(),
            has_support: self.has_cgroup_support,
            available_controllers: self.available_controllers.clone(),
            memory_controller: self.available_controllers.contains("memory"),
            cpu_controller: self.available_controllers.contains("cpu"),
            cpuacct_controller: self.available_controllers.contains("cpuacct"),
            pids_controller: self.available_controllers.contains("pids"),
        }
    }
}

impl Drop for Cgroup {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[derive(Debug, Clone)]
pub struct CgroupInfo {
    pub name: String,
    pub has_support: bool,
    pub available_controllers: HashSet<String>,
    pub memory_controller: bool,
    pub cpu_controller: bool,
    pub cpuacct_controller: bool,
    pub pids_controller: bool,
}

pub fn cgroups_available() -> bool {
    Cgroup::cgroups_available()
}

pub fn get_cgroup_mount() -> Result<String> {
    if !cgroups_available() {
        return Err(IsolateError::Cgroup(
            "Cgroups not available on this system".to_string(),
        ));
    }

    Ok("/sys/fs/cgroup".to_string())
}

pub fn get_available_controllers() -> Result<HashSet<String>> {
    Cgroup::get_available_controllers()
}
