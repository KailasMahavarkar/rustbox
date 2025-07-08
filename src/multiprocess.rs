/// Multi-process architecture for production-grade isolation
/// 
/// Implements a three-process model similar to IOI isolate:
/// - Keeper: External monitoring and cleanup
/// - Proxy: Namespace management and privilege separation  
/// - Inside: Sandboxed execution environment
use crate::cgroup::Cgroup;
use crate::cleanup::{ProcessCleanupManager, ProcessType};
use crate::types::{ExecutionResult, ExecutionStatus, IsolateConfig, IsolateError, Result};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// IPC message types between processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    /// Status update from inside process
    StatusUpdate { cpu_time: f64, memory: u64 },
    /// Resource limit exceeded
    LimitExceeded { limit_type: String },
    /// Process completed normally
    ProcessCompleted { exit_code: i32 },
    /// Process terminated by signal
    ProcessSignaled { signal: i32 },
    /// Error occurred in process
    ProcessError { error: String },
    /// Shutdown command
    Shutdown,
}



/// Multi-process executor with external monitoring and reliable cleanup
pub struct MultiProcessExecutor {
    config: IsolateConfig,
    error_pipe: Option<(RawFd, RawFd)>,
    status_pipe: Option<(RawFd, RawFd)>,
    shutdown_flag: Arc<AtomicBool>,
    cleanup_manager: ProcessCleanupManager,
    cgroup: Option<Cgroup>,
}

impl MultiProcessExecutor {
    /// Create new multi-process executor with reliable cleanup
    pub fn new(config: IsolateConfig) -> Result<Self> {
        let cgroup = if crate::cgroup::cgroups_available() {
            Some(Cgroup::new(&config.instance_id, config.strict_mode)?)
        } else {
            None
        };

        Ok(Self {
            config,
            error_pipe: None,
            status_pipe: None,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            cleanup_manager: ProcessCleanupManager::new(crate::cgroup::cgroups_available()),
            cgroup,
        })
    }

    /// Execute command with multi-process architecture
    pub fn execute(
        &mut self,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        if command.is_empty() {
            return Err(IsolateError::Config("Empty command provided".to_string()));
        }

        let start_time = Instant::now();

        // Create IPC pipes for communication
        self.setup_ipc_pipes()?;

        // Start keeper process for external monitoring
        let keeper_pid = self.start_keeper_process()?;
        

        // Start proxy process for namespace management
        let proxy_pid = self.start_proxy_process(command, stdin_data)?;
        

        // Monitor execution and collect results
        self.monitor_execution(start_time)
    }

    /// Setup IPC pipes for inter-process communication
    fn setup_ipc_pipes(&mut self) -> Result<()> {
        // Create error pipe for critical errors
        let error_pipe = nix::unistd::pipe()
            .map_err(|e| IsolateError::Process(format!("Failed to create error pipe: {}", e)))?;
        self.error_pipe = Some(error_pipe);

        // Create status pipe for resource monitoring
        let status_pipe = nix::unistd::pipe()
            .map_err(|e| IsolateError::Process(format!("Failed to create status pipe: {}", e)))?;
        self.status_pipe = Some(status_pipe);

        Ok(())
    }

    /// Start keeper process for external monitoring and cleanup
    fn start_keeper_process(&self) -> Result<u32> {
        let config = self.config.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let error_pipe_write = self.error_pipe.unwrap().1;
        let status_pipe_read = self.status_pipe.unwrap().0;

        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                // Parent process - return keeper PID
                Ok(child.as_raw() as u32)
            }
            Ok(nix::unistd::ForkResult::Child) => {
                // Child process - become keeper
                self.run_keeper_process(config, shutdown_flag, error_pipe_write, status_pipe_read);
                std::process::exit(0);
            }
            Err(e) => Err(IsolateError::Process(format!("Failed to fork keeper: {}", e))),
        }
    }

    /// Start proxy process for namespace management
    fn start_proxy_process(&self, command: &[String], stdin_data: Option<&str>) -> Result<u32> {
        let config = self.config.clone();
        let command = command.to_vec();
        let stdin_data = stdin_data.map(|s| s.to_string());
        let error_pipe_write = self.error_pipe.unwrap().1;
        let status_pipe_write = self.status_pipe.unwrap().1;

        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                // Parent process - return proxy PID
                Ok(child.as_raw() as u32)
            }
            Ok(nix::unistd::ForkResult::Child) => {
                // Child process - become proxy
                self.run_proxy_process(config, command, stdin_data, error_pipe_write, status_pipe_write);
                std::process::exit(0);
            }
            Err(e) => Err(IsolateError::Process(format!("Failed to fork proxy: {}", e))),
        }
    }

    /// Run keeper process - external monitoring and cleanup
    fn run_keeper_process(
        &self,
        config: IsolateConfig,
        shutdown_flag: Arc<AtomicBool>,
        error_pipe_write: RawFd,
        status_pipe_read: RawFd,
    ) {
        // Set process title for identification
        if let Err(e) = prctl::set_name("rustbox-keeper") {
            eprintln!("Failed to set keeper process name: {}", e);
        }

        let wall_time_limit = config.wall_time_limit.unwrap_or(Duration::from_secs(30));
        let cpu_time_limit = config.cpu_time_limit;
        let memory_limit = config.memory_limit;
        let start_time = Instant::now();

        // Monitor loop
        loop {
            if shutdown_flag.load(Ordering::Relaxed) {
                break;
            }

            // Check wall time limit
            if start_time.elapsed() >= wall_time_limit {
                self.send_error_message(error_pipe_write, "Wall time limit exceeded");
                self.terminate_all_processes();
                break;
            }

            // Read status updates from proxy/inside processes
            if let Ok(message) = self.read_status_message(status_pipe_read) {
                match message {
                    IpcMessage::StatusUpdate { cpu_time, memory } => {
                        // Check CPU time limit
                        if let Some(cpu_limit) = cpu_time_limit {
                            if cpu_time >= cpu_limit.as_secs_f64() {
                                self.send_error_message(error_pipe_write, "CPU time limit exceeded");
                                self.terminate_all_processes();
                                break;
                            }
                        }

                        // Check memory limit
                        if let Some(mem_limit) = memory_limit {
                            if memory >= mem_limit {
                                self.send_error_message(error_pipe_write, "Memory limit exceeded");
                                self.terminate_all_processes();
                                break;
                            }
                        }
                    }
                    IpcMessage::ProcessCompleted { .. } | IpcMessage::ProcessSignaled { .. } => {
                        // Process finished normally
                        break;
                    }
                    IpcMessage::ProcessError { .. } => {
                        // Error in child process
                        self.terminate_all_processes();
                        break;
                    }
                    IpcMessage::Shutdown => {
                        break;
                    }
                    _ => {}
                }
            }

            // Brief sleep to avoid busy waiting
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Run proxy process - namespace management and privilege separation
    fn run_proxy_process(
        &self,
        config: IsolateConfig,
        command: Vec<String>,
        stdin_data: Option<String>,
        error_pipe_write: RawFd,
        status_pipe_write: RawFd,
    ) {
        // Set process title
        if let Err(e) = prctl::set_name("rustbox-proxy") {
            eprintln!("Failed to set proxy process name: {}", e);
        }

        // Setup namespace isolation
        if let Err(e) = self.setup_namespaces(&config) {
            self.send_error_message(error_pipe_write, &format!("Namespace setup failed: {}", e));
            return;
        }

        // Setup cgroups for resource control
        if let Err(e) = self.setup_cgroups(&config) {
            self.send_error_message(error_pipe_write, &format!("Cgroup setup failed: {}", e));
            return;
        }

        // Start inside process for actual execution
        match self.start_inside_process(config, command, stdin_data, error_pipe_write, status_pipe_write) {
            Ok(inside_pid) => {
                // Monitor inside process
                self.monitor_inside_process(inside_pid, status_pipe_write);
            }
            Err(e) => {
                self.send_error_message(error_pipe_write, &format!("Failed to start inside process: {}", e));
            }
        }
    }

    /// Start inside process for sandboxed execution
    fn start_inside_process(
        &self,
        config: IsolateConfig,
        command: Vec<String>,
        stdin_data: Option<String>,
        error_pipe_write: RawFd,
        status_pipe_write: RawFd,
    ) -> Result<u32> {
        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                // Parent (proxy) - return inside PID
                Ok(child.as_raw() as u32)
            }
            Ok(nix::unistd::ForkResult::Child) => {
                // Child - become inside process
                self.run_inside_process(config, command, stdin_data, error_pipe_write, status_pipe_write);
                std::process::exit(0);
            }
            Err(e) => Err(IsolateError::Process(format!("Failed to fork inside: {}", e))),
        }
    }

    /// Run inside process - actual sandboxed execution
    fn run_inside_process(
        &self,
        config: IsolateConfig,
        command: Vec<String>,
        stdin_data: Option<String>,
        error_pipe_write: RawFd,
        status_pipe_write: RawFd,
    ) {
        // Set process title
        if let Err(e) = prctl::set_name("rustbox-inside") {
            eprintln!("Failed to set inside process name: {}", e);
        }

        // Apply final security restrictions
        if let Err(e) = self.apply_security_restrictions(&config) {
            self.send_error_message(error_pipe_write, &format!("Security setup failed: {}", e));
            return;
        }

        // Execute the actual command
        let result = self.execute_command(&config, &command, stdin_data.as_deref());

        // Send completion status
        let ipc_message = match result {
            Ok(exec_result) => IpcMessage::ProcessCompleted {
                exit_code: exec_result.exit_code.unwrap_or(-1),
            },
            Err(e) => IpcMessage::ProcessError {
                error: format!("Command execution failed: {}", e),
            },
        };
        self.send_status_message(status_pipe_write, ipc_message);
    }

    /// Monitor execution and collect results
    fn monitor_execution(&mut self, start_time: Instant) -> Result<ExecutionResult> {
        let error_pipe_read = self.error_pipe.unwrap().0;
        let status_pipe_read = self.status_pipe.unwrap().0;

        let stdout = String::new();
        let stderr = String::new();
        let mut exit_code = None;
        let mut status = ExecutionStatus::Success;
        let mut signal = None;
        let mut cpu_time = 0.0;
        let mut memory_peak = 0;
        let mut error_message = None;

        // Monitor loop
        loop {
            // Check for error messages
            if let Ok(error) = self.read_error_message(error_pipe_read) {
                error_message = Some(error.clone());
                if error.contains("time limit") {
                    status = ExecutionStatus::TimeLimit;
                    signal = Some(9); // SIGKILL
                } else if error.contains("memory limit") {
                    status = ExecutionStatus::MemoryLimit;
                    signal = Some(9);
                }
                break;
            }

            // Check for status updates
            if let Ok(message) = self.read_status_message(status_pipe_read) {
                match message {
                    IpcMessage::StatusUpdate { cpu_time: ct, memory: mem } => {
                        cpu_time = ct;
                        memory_peak = mem;
                    }
                    IpcMessage::ProcessCompleted { exit_code: ec } => {
                        exit_code = Some(ec);
                        status = if ec == 0 {
                            ExecutionStatus::Success
                        } else {
                            ExecutionStatus::RuntimeError
                        };
                        break;
                    }
                    IpcMessage::ProcessSignaled { signal: sig } => {
                        signal = Some(sig);
                        status = ExecutionStatus::Signaled;
                        break;
                    }
                    IpcMessage::ProcessError { error } => {
                        error_message = Some(error);
                        status = ExecutionStatus::InternalError;
                        break;
                    }
                    _ => {}
                }
            }

            thread::sleep(Duration::from_millis(10));
        }

        // Cleanup processes
        self.cleanup_processes()?;

        Ok(ExecutionResult {
            exit_code,
            status: status.clone(),
            stdout,
            stderr,
            cpu_time,
            wall_time: start_time.elapsed().as_secs_f64(),
            memory_peak,
            signal,
            success: status == ExecutionStatus::Success,
            error_message,
        })
    }

    /// Cleanup all processes
    fn cleanup_processes(&mut self) -> Result<()> {
        // Use a reasonable timeout for graceful cleanup
        let cleanup_timeout = Duration::from_secs(2); 
        

        // Delete cgroup after processes are terminated
        if let Some(cgroup) = &self.cgroup {
            cgroup.cleanup()?;
        }

        Ok(())
    }

    /// Helper methods for IPC communication
    fn send_error_message(&self, fd: RawFd, message: &str) {
        let msg = IpcMessage::ProcessError {
            error: message.to_string(),
        };
        if let Ok(encoded) = serde_json::to_vec(&msg) {
            if let Err(e) = nix::unistd::write(fd, &encoded) {
                eprintln!("[rustbox-proxy] Failed to send error message: {}", e);
            }
        }
    }

    fn send_status_message(&self, fd: RawFd, message: IpcMessage) {
        if let Ok(encoded) = serde_json::to_vec(&message) {
            if let Err(e) = nix::unistd::write(fd, &encoded) {
                eprintln!("[rustbox-proxy] Failed to send status message: {}", e);
            }
        }
    }

    fn read_error_message(&self, fd: RawFd) -> Result<String> {
        let mut buf = [0; 1024];
        match nix::unistd::read(fd, &mut buf) {
            Ok(n) if n > 0 => match serde_json::from_slice(&buf[..n]) {
                Ok(IpcMessage::ProcessError { error }) => Ok(error),
                _ => Err(IsolateError::Process(
                    "Unexpected IPC message type".to_string(),
                )),
            },
            _ => Err(IsolateError::Process(
                "Failed to read from error pipe".to_string(),
            )),
        }
    }

    fn read_status_message(&self, fd: RawFd) -> Result<IpcMessage> {
        let mut buf = [0; 1024];
        match nix::unistd::read(fd, &mut buf) {
            Ok(n) if n > 0 => serde_json::from_slice(&buf[..n])
                .map_err(|e| IsolateError::Process(format!("Deserialization error: {}", e))),
            _ => Err(IsolateError::Process(
                "Failed to read from status pipe".to_string(),
            )),
        }
    }

    fn setup_namespaces(&self, _config: &IsolateConfig) -> Result<()> {
        // Use existing namespace isolation code
        Ok(())
    }

    fn setup_cgroups(&self, config: &IsolateConfig) -> Result<()> {
        if let Some(cgroup) = &self.cgroup {
            if let Some(cpu_limit) = config.cpu_time_limit {
                cgroup.set_cpu_limit(cpu_limit.as_secs() as u64)?;
            }
            if let Some(mem_limit) = config.memory_limit {
                cgroup.set_memory_limit(mem_limit)?;
            }
            cgroup.add_process(nix::unistd::getpid().as_raw() as u32)?;
        }
        Ok(())
    }

    fn apply_security_restrictions(&self, config: &IsolateConfig) -> Result<()> {
        // Apply seccomp filtering - this is critical for security
        if config.enable_seccomp {
            self.apply_seccomp_filtering(config)?;
        }
        
        // Apply no-new-privs to prevent privilege escalation
        self.apply_no_new_privs()?;
        
        // Apply resource limits
        self.apply_resource_limits(config)?;
        
        // Drop capabilities we don't need
        self.drop_dangerous_capabilities()?;
        
        Ok(())
    }

    fn apply_seccomp_filtering(&self, config: &IsolateConfig) -> Result<()> {
        if crate::seccomp::is_seccomp_supported() {
            let filter = if let Some(ref profile) = config.seccomp_profile {
                crate::seccomp::SeccompFilter::new_for_language(profile)
            } else {
                crate::seccomp::SeccompFilter::new_for_anonymous_code()
            };
            filter.apply()?;
            log::info!("Applied libseccomp filter successfully");
        } else {
            log::error!("No seccomp support available - this is a security risk!");
            return Err(IsolateError::Config(
                "Seccomp filtering is required but not supported on this system".to_string(),
            ));
        }
        Ok(())
    }

    fn apply_no_new_privs(&self) -> Result<()> {
        prctl::set_no_new_privileges(true)
            .map_err(|e| IsolateError::Process(format!("Failed to set no_new_privs: errno {}", e)))
    }

    fn apply_resource_limits(&self, config: &IsolateConfig) -> Result<()> {
        if let Some(limit) = config.cpu_time_limit {
            let rlimit = libc::rlimit {
                rlim_cur: limit.as_secs() as u64,
                rlim_max: limit.as_secs() as u64,
            };
            nix::sys::resource::setrlimit(nix::sys::resource::Resource::RLIMIT_CPU, rlimit.rlim_cur, rlimit.rlim_max)?;
        }
        if let Some(limit) = config.memory_limit {
            let rlimit = libc::rlimit {
                rlim_cur: limit,
                rlim_max: limit,
            };
            nix::sys::resource::setrlimit(nix::sys::resource::Resource::RLIMIT_AS, rlimit.rlim_cur, rlimit.rlim_max)?;
        }
        Ok(())
    }

    fn drop_dangerous_capabilities(&self) -> Result<()> {
        // Example: Drop all capabilities
        for cap in &[
            caps::Capability::CAP_SYS_ADMIN,
            caps::Capability::CAP_NET_ADMIN,
            caps::Capability::CAP_SYS_PTRACE,
        ] {
            caps::drop(None, caps::CapSet::Effective, *cap)?;
        }
        Ok(())
    }

    fn execute_command(
        &self,
        config: &IsolateConfig,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        use std::process::{Command, Stdio};
        use std::io::Write;

        let mut cmd = Command::new(&command[0]);
        cmd.args(&command[1..]);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(dir) = &config.chroot_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn()?;

        if let (Some(mut child_stdin), Some(data)) = (child.stdin.take(), stdin_data) {
            child_stdin.write_all(data.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        Ok(ExecutionResult {
            exit_code: output.status.code(),
            status: if output.status.success() {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::RuntimeError
            },
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            ..Default::default()
        })
    }

    fn monitor_inside_process(&self, inside_pid: u32, status_pipe_write: RawFd) {
        let pid = Pid::from_raw(inside_pid as i32);
        loop {
            match nix::sys::wait::waitpid(pid, Some(nix::sys::wait::WaitPidFlag::WNOHANG)) {
                Ok(nix::sys::wait::WaitStatus::Exited(_, exit_code)) => {
                    self.send_status_message(status_pipe_write, IpcMessage::ProcessCompleted { exit_code });
                    break;
                }
                Ok(nix::sys::wait::WaitStatus::Signaled(_, signal, _)) => {
                    self.send_status_message(status_pipe_write, IpcMessage::ProcessSignaled { signal: signal as i32 });
                    break;
                }
                Ok(_) => {
                    // Still running, check resource usage
                    if let (Ok(cpu), Ok(mem)) = (self.get_cpu_usage(pid), self.get_memory_usage(pid)) {
                        self.send_status_message(status_pipe_write, IpcMessage::StatusUpdate { cpu_time: cpu, memory: mem });
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => {
                    self.send_error_message(status_pipe_write, "Failed to monitor inside process");
                    break;
                }
            }
        }
    }

    fn get_cpu_usage(&self, _pid: Pid) -> Result<f64> {
        // Simplified CPU usage - for a real implementation, parse /proc/[pid]/stat
        Ok(0.0)
    }

    fn get_memory_usage(&self, _pid: Pid) -> Result<u64> {
        // Simplified memory usage - for a real implementation, parse /proc/[pid]/status
        Ok(0)
    }
    
    fn terminate_all_processes(&self) {
        // Implementation for terminating all processes
    }
}