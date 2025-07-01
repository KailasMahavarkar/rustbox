/// Process execution and monitoring
use crate::cgroup::CgroupController;
use crate::filesystem::FilesystemSecurity;
use crate::seccomp::SeccompFilter;
use crate::seccomp_native::NativeSeccompFilter;
use crate::types::{ExecutionResult, ExecutionStatus, IsolateConfig, IsolateError, Result};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

/// Process executor that handles isolation and monitoring
pub struct ProcessExecutor {
    config: IsolateConfig,
    cgroup: Option<CgroupController>,
    filesystem: FilesystemSecurity,
}

impl ProcessExecutor {
    /// Create a new process executor
    pub fn new(config: IsolateConfig) -> Result<Self> {
        // Check strict mode requirements
        if config.strict_mode {
            // Check if running as root (Unix only)
            #[cfg(unix)]
            {
                use nix::unistd::getuid;
                if !getuid().is_root() {
                    return Err(IsolateError::Config(
                        "Strict mode requires root privileges. Run with sudo or remove --strict flag.".to_string()
                    ));
                }
            }

            // Ensure cgroups are available in strict mode
            if !crate::cgroup::cgroups_available() {
                return Err(IsolateError::Config(
                    "Strict mode requires cgroups to be available on this system.".to_string(),
                ));
            }
        }

        let cgroup = if crate::cgroup::cgroups_available() {
            Some(CgroupController::new(
                &config.instance_id,
                config.strict_mode,
            )?)
        } else {
            if config.strict_mode {
                return Err(IsolateError::Config(
                    "Strict mode requires cgroups to be available on this system.".to_string(),
                ));
            }
            None
        };

        // Initialize filesystem security
        let filesystem = FilesystemSecurity::new(
            config.chroot_dir.clone(),
            config.workdir.clone(),
            config.strict_mode,
        );

        Ok(Self { config, cgroup, filesystem })
    }

    /// Setup resource limits using cgroups
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

            // Set CPU shares (relative weight)
            cgroup.set_cpu_limit(1024)?; // Standard CPU shares
        }
        Ok(())
    }

    /// Execute a command with isolation
    pub fn execute(
        &mut self,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        if command.is_empty() {
            return Err(IsolateError::Config("Empty command provided".to_string()));
        }

        let start_time = Instant::now();

        // Setup working directory
        self.setup_workdir()?;

        // Setup resource limits
        self.setup_resource_limits()?;

        // Create the command
        let mut cmd = Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }

        // Configure command
        cmd.current_dir(&self.filesystem.get_effective_workdir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &self.config.environment {
            cmd.env(key, value);
        }

        // Ensure PATH is set - inherit from parent process if not explicitly set
        if !self.config.environment.iter().any(|(k, _)| k == "PATH") {
            if let Ok(path) = std::env::var("PATH") {
                cmd.env("PATH", path);
            }
        }

        // Set user/group if specified (Unix only)
        #[cfg(unix)]
        if let Some(uid) = self.config.uid {
            let enable_seccomp = self.config.enable_seccomp;
            let seccomp_profile = self.config.seccomp_profile.clone();
            let chroot_dir = self.config.chroot_dir.clone();
            
            unsafe {
                cmd.pre_exec(move || {
                    // Apply chroot first if configured
                    if let Some(ref chroot_path) = chroot_dir {
                        let fs_security = FilesystemSecurity::new(Some(chroot_path.clone()), PathBuf::from("/"), false);
                        fs_security.apply_chroot().map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::PermissionDenied, e)
                        })?;
                    }
                    
                    // Set UID/GID after chroot
                    nix::unistd::setuid(nix::unistd::Uid::from_raw(uid)).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::PermissionDenied, e)
                    })?;
                    
                    // Apply seccomp filter after privilege drop
                    if enable_seccomp {
                        Self::apply_seccomp_filter_in_child(seccomp_profile.as_deref())?;
                    }
                    
                    Ok(())
                });
            }
        } else if self.config.enable_seccomp || self.config.chroot_dir.is_some() {
            // Apply seccomp and/or chroot even without UID change
            let seccomp_profile = self.config.seccomp_profile.clone();
            let enable_seccomp = self.config.enable_seccomp;
            let chroot_dir = self.config.chroot_dir.clone();
            
            unsafe {
                cmd.pre_exec(move || {
                    // Apply chroot first if configured
                    if let Some(ref chroot_path) = chroot_dir {
                        let fs_security = FilesystemSecurity::new(Some(chroot_path.clone()), PathBuf::from("/"), false);
                        fs_security.apply_chroot().map_err(|e| {
                            std::io::Error::new(std::io::ErrorKind::PermissionDenied, e)
                        })?;
                    }
                    
                    // Apply seccomp filter
                    if enable_seccomp {
                        Self::apply_seccomp_filter_in_child(seccomp_profile.as_deref())?;
                    }
                    
                    Ok(())
                });
            }
        }

        // Start the process
        let mut child = cmd
            .spawn()
            .map_err(|e| IsolateError::Process(format!("Failed to start process: {}", e)))?;

        let pid = child.id();

        // Add process to cgroup
        if let Some(ref cgroup) = self.cgroup {
            cgroup.add_process(pid)?;
        }

        // Handle stdin
        if let Some(data) = stdin_data {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(data.as_bytes());
            }
        }

        // Wait for process with timeout
        let wall_time_limit = self
            .config
            .wall_time_limit
            .unwrap_or(Duration::from_secs(30));
        let execution_result = self.wait_with_timeout(child, wall_time_limit, start_time)?;

        Ok(execution_result)
    }

    /// Wait for process with timeout and resource monitoring
    fn wait_with_timeout(
        &self,
        mut child: std::process::Child,
        timeout: Duration,
        start_time: Instant,
    ) -> Result<ExecutionResult> {
        let child_id = child.id();

        // Spawn monitoring thread to collect output and wait for process
        let monitor_handle = thread::spawn(move || {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();

            // Collect output
            if let Some(mut stdout_handle) = child.stdout.take() {
                let _ = stdout_handle.read_to_end(&mut stdout);
            }
            if let Some(mut stderr_handle) = child.stderr.take() {
                let _ = stderr_handle.read_to_end(&mut stderr);
            }

            // Wait for process
            let wait_result = child.wait();

            (wait_result, stdout, stderr)
        });

        // Wait for the monitoring thread with timeout
        let start = Instant::now();
        let result = loop {
            if monitor_handle.is_finished() {
                // Process completed, get the result
                match monitor_handle.join() {
                    Ok(result) => break Some(result),
                    Err(_) => return Err(IsolateError::Process("Thread join failed".to_string())),
                }
            }

            // Check if we've exceeded the timeout
            if start.elapsed() >= timeout {
                // Kill the process and wait a bit for cleanup
                #[cfg(unix)]
                unsafe {
                    libc::kill(child_id as i32, libc::SIGKILL);
                }

                #[cfg(not(unix))]
                {
                    // On Windows, forcefully terminate the process
                    let _ = std::process::Command::new("taskkill")
                        .args(&["/F", "/PID", &child_id.to_string()])
                        .output();
                }

                // Give the process a moment to die, then collect results
                thread::sleep(Duration::from_millis(100));

                match monitor_handle.join() {
                    Ok(result) => break Some(result),
                    Err(_) => break None,
                }
            }

            // Sleep briefly to avoid busy waiting
            thread::sleep(Duration::from_millis(10));
        };

        let (wait_result, stdout, stderr) = match result {
            Some((wait_result, stdout, stderr)) => (wait_result, stdout, stderr),
            None => {
                return Err(IsolateError::Process(
                    "Process monitoring failed".to_string(),
                ))
            }
        };

        // Check wall time after process completion
        let wall_time = start_time.elapsed().as_secs_f64();

        match wait_result {
            Ok(exit_status) => {
                let exit_code = exit_status.code();

                // Get signal information (Unix only)
                #[cfg(unix)]
                let signal = exit_status.signal();
                #[cfg(not(unix))]
                let signal = None;

                // Determine if process was killed due to timeout
                let timed_out = wall_time >= timeout.as_secs_f64() || signal == Some(9);

                let status = if timed_out {
                    ExecutionStatus::TimeLimit
                } else if exit_status.success() {
                    ExecutionStatus::Success
                } else if signal.is_some() {
                    ExecutionStatus::Signaled
                } else {
                    ExecutionStatus::RuntimeError
                };

                let (cpu_time, memory_peak) = self.get_resource_usage();

                Ok(ExecutionResult {
                    exit_code,
                    status,
                    stdout: String::from_utf8_lossy(&stdout).to_string(),
                    stderr: String::from_utf8_lossy(&stderr).to_string(),
                    cpu_time,
                    wall_time,
                    memory_peak,
                    signal,
                    success: exit_status.success() && !timed_out,
                    error_message: if timed_out {
                        Some("Wall time limit exceeded".to_string())
                    } else {
                        None
                    },
                })
            }
            Err(e) => Ok(ExecutionResult {
                exit_code: None,
                status: ExecutionStatus::InternalError,
                stdout: String::from_utf8_lossy(&stdout).to_string(),
                stderr: format!("Process error: {}", e),
                cpu_time: 0.0,
                wall_time,
                memory_peak: 0,
                signal: None,
                success: false,
                error_message: Some(e.to_string()),
            }),
        }
    }

    /// Get resource usage from cgroup
    fn get_resource_usage(&self) -> (f64, u64) {
        if let Some(ref cgroup) = self.cgroup {
            let cpu_time = cgroup.get_cpu_usage().unwrap_or(0.0);
            let memory_peak = cgroup.get_peak_memory_usage().unwrap_or(0);
            (cpu_time, memory_peak)
        } else {
            (0.0, 0)
        }
    }

    /// Setup working directory and filesystem isolation
    fn setup_workdir(&self) -> Result<()> {
        // Setup filesystem isolation (including chroot if configured)
        self.filesystem.setup_isolation()?;
        Ok(())
    }

    /// Cleanup resources
    pub fn cleanup(&mut self) -> Result<()> {
        if let Some(cgroup) = self.cgroup.take() {
            cgroup.cleanup()?;
        }
        
        // Cleanup filesystem isolation
        self.filesystem.cleanup()?;
        
        Ok(())
    }

    /// Apply seccomp filter in child process (called via pre_exec)
    fn apply_seccomp_filter_in_child(_profile: Option<&str>) -> std::io::Result<()> {
        // Try libseccomp first (if available and compiled with seccomp feature)
        #[cfg(feature = "seccomp")]
        {
            if crate::seccomp::is_seccomp_supported() {
                let filter = if let Some(profile) = _profile {
                    SeccompFilter::new_for_language(profile)
                } else {
                    SeccompFilter::new_for_anonymous_code()
                };

                return filter.apply().map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!("Failed to apply libseccomp filter: {}", e),
                    )
                });
            }
        }
        
        // Fallback to native seccomp implementation
        if NativeSeccompFilter::is_supported() {
            let native_filter = NativeSeccompFilter::new_for_anonymous_code();
            
            // Apply no-new-privs first (prevents privilege escalation)
            native_filter.apply_no_new_privs().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("Failed to apply no-new-privs: {}", e),
                )
            })?;
            
            // Apply basic seccomp protection
            native_filter.apply_basic_protection().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("Failed to apply native seccomp: {}", e),
                )
            })
        } else {
            // No seccomp available - log warning but don't fail
            eprintln!("Warning: No seccomp support available");
            Ok(())
        }
    }
}

impl Drop for ProcessExecutor {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
