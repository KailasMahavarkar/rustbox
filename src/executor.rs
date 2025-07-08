/// Process execution and monitoring with reliable resource limits
use crate::cgroup::Cgroup;
use crate::filesystem::FilesystemSecurity;
use crate::types::{ExecutionResult, ExecutionStatus, IsolateConfig, IsolateError, Result};
use crate::multiprocess::MultiProcessExecutor;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

/// Process executor that handles isolation and monitoring with focus on reliability
pub struct ProcessExecutor {
    config: IsolateConfig,
    cgroup: Option<Cgroup>,
    filesystem_security: FilesystemSecurity,
}

impl ProcessExecutor {
    /// Create a new process executor
    pub fn new(config: IsolateConfig) -> Result<Self> {
        let cgroup = if crate::cgroup::cgroups_available() {
            match Cgroup::new(&config.instance_id, config.strict_mode) {
                Ok(cgroup) => Some(cgroup),
                Err(e) => {
                    eprintln!("Failed to create cgroup controller: {:?}", e);
                    if config.strict_mode {
                        return Err(e);
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        };

        // Create filesystem security controller
        let filesystem_security = FilesystemSecurity::new(
            config.chroot_dir.clone(),
            config.workdir.clone(),
            config.strict_mode
        );

        // Set up filesystem isolation if chroot is specified
        if config.chroot_dir.is_some() {
            filesystem_security.setup_isolation()?;
        }

        Ok(Self { 
            config, 
            cgroup,
            filesystem_security,
        })
    }

    /// Setup resource limits using cgroups only
    fn setup_resource_limits(&self) -> Result<()> {
        if let Some(ref cgroup) = self.cgroup {
            // Set memory limit
            if let Some(memory_limit) = self.config.memory_limit {
                cgroup.set_memory_limit(memory_limit)?;
            }

            // Set process limit
            if let Some(process_limit) = self.config.process_limit {
                cgroup.set_process_limit(process_limit as u64)?;
            }

            // Set CPU shares
            cgroup.set_cpu_limit(1024)?;
        }

        Ok(())
    }

    /// Execute command with multi-process architecture for production reliability
    pub fn execute_multiprocess(
        &mut self,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        let mut multiprocess_executor = MultiProcessExecutor::new(self.config.clone())?;
        multiprocess_executor.execute(command, stdin_data)
    }

    /// Execute a command with appropriate isolation method based on configuration
    pub fn execute(
        &mut self,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        if self.config.use_multiprocess {
            self.execute_multiprocess(command, stdin_data)
        } else {
            self.execute_single_process(command, stdin_data)
        }
    }

    /// Execute a command with minimal isolation for maximum reliability
    pub fn execute_single_process(
        &mut self,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        if command.is_empty() {
            return Err(IsolateError::Config("Empty command provided".to_string()));
        }

        let start_time = Instant::now();

        // Setup resource limits
        self.setup_resource_limits()?;

        // Create the command with minimal configuration
        let mut cmd = Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }

        // Configure basic I/O
        cmd.current_dir(&self.config.workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set basic environment
        cmd.env_clear();
        cmd.env("PATH", "/usr/local/bin:/usr/bin:/bin");
        
        // Add custom environment variables
        for (key, value) in &self.config.environment {
            cmd.env(key, value);
        }

        // Setup resource limits using rlimits in pre_exec hook
        use std::os::unix::process::CommandExt;
        let config_clone = self.config.clone();
        let filesystem_security = self.filesystem_security.clone();
        unsafe {
            cmd.pre_exec(move || {
                // Apply filesystem isolation (chroot) first if configured
                if config_clone.chroot_dir.is_some() {
                    if let Err(e) = filesystem_security.apply_chroot() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            format!("Failed to apply chroot: {}", e)
                        ));
                    }
                }

                // Set file descriptor limit if specified
                if let Some(fd_limit) = config_clone.fd_limit {
                    use nix::sys::resource::{setrlimit, Resource};
                    setrlimit(Resource::RLIMIT_NOFILE, fd_limit, fd_limit)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("setrlimit failed: {}", e)))?;
                }

                // Apply seccomp filtering if enabled (before dropping privileges)
                if config_clone.enable_seccomp {
                    let filter = if let Some(ref profile) = config_clone.seccomp_profile {
                        crate::seccomp::SeccompFilter::new_for_language(profile)
                    } else {
                        crate::seccomp::SeccompFilter::new_for_anonymous_code()
                    };
                    
                    if let Err(e) = crate::seccomp::apply_seccomp_with_fallback(&filter, config_clone.strict_mode) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            format!("Failed to apply seccomp filter: {}", e)
                        ));
                    }
                }

                // Drop privileges if uid/gid specified (requires root to start)
                if let Some(gid) = config_clone.gid {
                    if libc::setgid(gid) != 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied, 
                            format!("Failed to setgid to {}", gid)
                        ));
                    }
                }
                
                if let Some(uid) = config_clone.uid {
                    if libc::setuid(uid) != 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied, 
                            format!("Failed to setuid to {}", uid)
                        ));
                    }
                }

                Ok(())
            });
        }

        // Start the process
        let mut child = cmd
            .spawn()
            .map_err(|e| IsolateError::Process(format!("Failed to start process: {}", e)))?;

        let pid = child.id();

        // Add process to cgroup after spawning
        if let Some(ref cgroup) = self.cgroup {
            cgroup.add_process(pid)?;
        }

        // Handle stdin
        if let Some(data) = stdin_data {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(data.as_bytes());
                drop(stdin); // Close stdin
            }
        }

        // Wait for process with timeout
        let wall_time_limit = self
            .config
            .wall_time_limit
            .unwrap_or(Duration::from_secs(30));
        
        self.wait_with_timeout(child, wall_time_limit, start_time, pid)
    }

    /// Simple and reliable timeout implementation with proper CPU time monitoring
    fn wait_with_timeout(
        &self,
        mut child: std::process::Child,
        timeout: Duration,
        start_time: Instant,
        pid: u32,
    ) -> Result<ExecutionResult> {
        let child_id = child.id();
        let timeout_start = Instant::now();
        
        // Check if we have a CPU time limit
        let cpu_time_limit = self.config.cpu_time_limit;
        
        // Simple polling loop
        loop {
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    // Process completed - collect output
                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();

                    if let Some(mut stdout_handle) = child.stdout.take() {
                        let _ = stdout_handle.read_to_end(&mut stdout);
                    }
                    if let Some(mut stderr_handle) = child.stderr.take() {
                        let _ = stderr_handle.read_to_end(&mut stderr);
                    }
                    
                    let wall_time = start_time.elapsed().as_secs_f64();
                    let (cpu_time, memory_peak) = self.get_resource_usage(pid);
                    
                    return Ok(ExecutionResult {
                        exit_code: exit_status.code(),
                        status: if exit_status.success() {
                            ExecutionStatus::Success
                        } else {
                            ExecutionStatus::RuntimeError
                        },
                        stdout: String::from_utf8_lossy(&stdout).to_string(),
                        stderr: String::from_utf8_lossy(&stderr).to_string(),
                        cpu_time,
                        wall_time,
                        memory_peak,
                        signal: {
                            #[cfg(unix)]
                            { exit_status.signal() }
                            #[cfg(not(unix))]
                            { None }
                        },
                        success: exit_status.success(),
                        error_message: None,
                    });
                }
                Ok(None) => {
                    // Process still running - check limits
                    let elapsed = timeout_start.elapsed();
                    let (cpu_time, memory_peak) = self.get_resource_usage(pid);
                    
                    // Check CPU time limit first if set
                    if let Some(cpu_limit) = cpu_time_limit {
                        if cpu_time >= cpu_limit.as_secs_f64() {
                            // CPU time limit exceeded
                            self.terminate_process(child_id);
                            let _ = child.wait();
                            
                            let mut stdout = Vec::new();
                            let mut stderr = Vec::new();

                            if let Some(mut stdout_handle) = child.stdout.take() {
                                let _ = stdout_handle.read_to_end(&mut stdout);
                            }
                            if let Some(mut stderr_handle) = child.stderr.take() {
                                let _ = stderr_handle.read_to_end(&mut stderr);
                            }
                            
                            let wall_time = start_time.elapsed().as_secs_f64();

                            return Ok(ExecutionResult {
                                exit_code: None,
                                status: ExecutionStatus::TimeLimit,
                                stdout: String::from_utf8_lossy(&stdout).to_string(),
                                stderr: String::from_utf8_lossy(&stderr).to_string(),
                                cpu_time,
                                wall_time,
                                memory_peak,
                                signal: Some(9), // SIGKILL
                                success: false,
                                error_message: Some("CPU time limit exceeded".to_string()),
                            });
                        }
                    }
                    
                    // Check wall time limit
                    if elapsed >= timeout {
                        // Wall time limit exceeded
                        self.terminate_process(child_id);
                        let _ = child.wait();
                        
                        let mut stdout = Vec::new();
                        let mut stderr = Vec::new();

                        if let Some(mut stdout_handle) = child.stdout.take() {
                            let _ = stdout_handle.read_to_end(&mut stdout);
                        }
                        if let Some(mut stderr_handle) = child.stderr.take() {
                            let _ = stderr_handle.read_to_end(&mut stderr);
                        }
                        
                        let wall_time = start_time.elapsed().as_secs_f64();
                        let (cpu_time, memory_peak) = self.get_resource_usage(pid);

                        return Ok(ExecutionResult {
                            exit_code: None,
                            status: ExecutionStatus::TimeLimit,
                            stdout: String::from_utf8_lossy(&stdout).to_string(),
                            stderr: String::from_utf8_lossy(&stderr).to_string(),
                            cpu_time,
                            wall_time,
                            memory_peak,
                            signal: Some(9), // SIGKILL
                            success: false,
                            error_message: Some("Wall time limit exceeded".to_string()),
                        });
                    }
                }
                Err(e) => {
                    return Err(IsolateError::Process(format!("Process monitoring error: {}", e)));
                }
            }
            
            // Brief sleep to avoid busy waiting
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Terminate a process gracefully then forcefully
    fn terminate_process(&self, pid: u32) {
        #[cfg(unix)]
        unsafe {
            // Send SIGTERM first
            libc::kill(pid as i32, libc::SIGTERM);
        }

        // Wait a bit for graceful shutdown
        thread::sleep(Duration::from_millis(100));
        
        #[cfg(unix)]
        unsafe {
            // Send SIGKILL if still running
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }

    /// Get resource usage - try multiple methods for reliability
    fn get_resource_usage(&self, pid: u32) -> (f64, u64) {
        // Method 1: Try cgroup if available
        if let Some(ref cgroup) = self.cgroup {
            if let Ok(cpu_time) = cgroup.get_cpu_usage() {
                if cpu_time > 0.0 {
                    let memory_peak = cgroup.get_peak_memory_usage().unwrap_or(0);
                    return (cpu_time, memory_peak);
                }
            }
        }

        // Method 2: Try /proc/pid/stat for CPU time
        let cpu_time = self.get_proc_cpu_time(pid).unwrap_or(0.0);
        
        // Method 3: Try /proc/pid/status for memory
        let memory_peak = self.get_proc_memory_peak(pid).unwrap_or(0);

        (cpu_time, memory_peak)
    }

    /// Get CPU time from /proc/pid/stat
    fn get_proc_cpu_time(&self, pid: u32) -> Option<f64> {
        let stat_path = format!("/proc/{}/stat", pid);
        let stat_content = std::fs::read_to_string(stat_path).ok()?;
        
        let fields: Vec<&str> = stat_content.split_whitespace().collect();
        if fields.len() >= 17 {
            // Fields 13 and 14 are utime and stime (user and system time in clock ticks)
            let utime: u64 = fields[13].parse().ok()?;
            let stime: u64 = fields[14].parse().ok()?;
            
            // Convert clock ticks to seconds (usually 100 ticks per second)
            let clock_ticks_per_sec = 100.0; // sysconf(_SC_CLK_TCK) is usually 100
            let total_time = (utime + stime) as f64 / clock_ticks_per_sec;
            
            Some(total_time)
        } else {
            None
        }
    }

    /// Get memory usage from /proc/pid/status
    fn get_proc_memory_peak(&self, pid: u32) -> Option<u64> {
        let status_path = format!("/proc/{}/status", pid);
        let status_content = std::fs::read_to_string(status_path).ok()?;
        
        for line in status_content.lines() {
            if line.starts_with("VmPeak:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return Some(kb * 1024); // Convert KB to bytes
                    }
                }
            }
        }
        
        None
    }

    /// Cleanup resources
    pub fn cleanup(&mut self) -> Result<()> {
        if let Some(cgroup) = self.cgroup.take() {
            cgroup.cleanup()?;
        }
        Ok(())
    }
}