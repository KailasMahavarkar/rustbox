/// Multi-process execution architecture for enhanced security and reliability
use crate::types::{ExecutionResult, ExecutionStatus, IsolateConfig, IsolateError, Result};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, pipe, ForkResult, Pid};
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// IPC message types for communication between processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    StatusUpdate { cpu_time: f64, memory: u64 },
    ProcessCompleted { exit_code: i32 },
    ProcessSignaled { signal: i32 },
    Error { message: String },
}

/// Multi-process executor implementing three-process architecture
pub struct MultiProcessExecutor {
    config: IsolateConfig,
    shutdown_flag: Arc<AtomicBool>,
}

impl MultiProcessExecutor {
    /// Create a new multi-process executor
    pub fn new(config: IsolateConfig) -> Result<Self> {
        Ok(Self {
            config,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Execute command using three-process architecture
    pub fn execute(
        &mut self,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        // Create pipes for IPC
        let (keeper_read, keeper_write) = pipe()
            .map_err(|e| IsolateError::Process(format!("Failed to create keeper pipe: {}", e)))?;
        let (proxy_read, proxy_write) = pipe()
            .map_err(|e| IsolateError::Process(format!("Failed to create proxy pipe: {}", e)))?;
        let (status_read, status_write) = pipe()
            .map_err(|e| IsolateError::Process(format!("Failed to create status pipe: {}", e)))?;

        // Fork the keeper process
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child: keeper_pid }) => {
                // This is the main process - wait for results
                self.wait_for_execution_results(keeper_pid, keeper_read, start_time)
            }
            Ok(ForkResult::Child) => {
                // This is the keeper process
                self.run_keeper_process(keeper_write, proxy_read, proxy_write, status_read, status_write, command, stdin_data)
            }
            Err(e) => Err(IsolateError::Process(format!("Failed to fork keeper process: {}", e))),
        }
    }

    /// Run the keeper process (middle layer)
    fn run_keeper_process(
        &self,
        keeper_write: RawFd,
        _proxy_read: RawFd,
        proxy_write: RawFd,
        status_read: RawFd,
        status_write: RawFd,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        // Fork the proxy process
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child: proxy_pid }) => {
                // Monitor proxy process and handle communication
                self.monitor_proxy_process(keeper_write, proxy_pid, status_read)
            }
            Ok(ForkResult::Child) => {
                // This is the proxy process
                self.run_proxy_process(proxy_write, status_write, command, stdin_data)
            }
            Err(e) => {
                let error_result = ExecutionResult {
                    exit_code: None,
                    status: ExecutionStatus::InternalError,
                    stdout: String::new(),
                    stderr: format!("Failed to fork proxy process: {}", e),
                    cpu_time: 0.0,
                    wall_time: 0.0,
                    memory_peak: 0,
                    signal: None,
                    success: false,
                    error_message: Some(format!("Fork error: {}", e)),
                };
                self.send_result_message(keeper_write, error_result.clone());
                Ok(error_result)
            }
        }
    }

    /// Run the proxy process (security boundary)
    fn run_proxy_process(
        &self,
        proxy_write: RawFd,
        status_write: RawFd,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        // Apply security measures first
        if let Err(e) = self.apply_security_measures(&self.config) {
            let error_result = ExecutionResult {
                exit_code: None,
                status: ExecutionStatus::SecurityViolation,
                stdout: String::new(),
                stderr: format!("Security setup failed: {}", e),
                cpu_time: 0.0,
                wall_time: 0.0,
                memory_peak: 0,
                signal: None,
                success: false,
                error_message: Some(format!("Security error: {}", e)),
            };
            self.send_result_message(proxy_write, error_result.clone());
            return Ok(error_result);
        }

        // Fork the inside process
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child: inside_pid }) => {
                // Monitor the inside process
                self.monitor_inside_process(inside_pid.as_raw() as u32, status_write);
                
                // Wait for inside process to complete
                match waitpid(inside_pid, None) {
                    Ok(WaitStatus::Exited(_, exit_code)) => {
                        let result = ExecutionResult {
                            exit_code: Some(exit_code),
                            status: if exit_code == 0 { ExecutionStatus::Success } else { ExecutionStatus::RuntimeError },
                            stdout: String::new(), // TODO: Capture actual output
                            stderr: String::new(),
                            cpu_time: 0.0, // TODO: Get actual CPU time
                            wall_time: 0.0,
                            memory_peak: 0,
                            signal: None,
                            success: exit_code == 0,
                            error_message: None,
                        };
                        self.send_result_message(proxy_write, result.clone());
                        Ok(result)
                    }
                    Ok(WaitStatus::Signaled(_, signal, _)) => {
                        let result = ExecutionResult {
                            exit_code: None,
                            status: ExecutionStatus::RuntimeError,
                            stdout: String::new(),
                            stderr: format!("Process terminated by signal {}", signal),
                            cpu_time: 0.0,
                            wall_time: 0.0,
                            memory_peak: 0,
                            signal: Some(signal as i32),
                            success: false,
                            error_message: Some(format!("Terminated by signal {}", signal)),
                        };
                        self.send_result_message(proxy_write, result.clone());
                        Ok(result)
                    }
                    Err(e) => {
                        let error_result = ExecutionResult {
                            exit_code: None,
                            status: ExecutionStatus::InternalError,
                            stdout: String::new(),
                            stderr: format!("Wait error: {}", e),
                            cpu_time: 0.0,
                            wall_time: 0.0,
                            memory_peak: 0,
                            signal: None,
                            success: false,
                            error_message: Some(format!("Wait error: {}", e)),
                        };
                        self.send_result_message(proxy_write, error_result.clone());
                        Ok(error_result)
                    }
                    _ => {
                        let error_result = ExecutionResult {
                            exit_code: None,
                            status: ExecutionStatus::InternalError,
                            stdout: String::new(),
                            stderr: "Unexpected wait status".to_string(),
                            cpu_time: 0.0,
                            wall_time: 0.0,
                            memory_peak: 0,
                            signal: None,
                            success: false,
                            error_message: Some("Unexpected wait status".to_string()),
                        };
                        self.send_result_message(proxy_write, error_result.clone());
                        Ok(error_result)
                    }
                }
            }
            Ok(ForkResult::Child) => {
                // This is the inside process - execute the actual command
                self.execute_command(&self.config, command, stdin_data)
            }
            Err(e) => {
                let error_result = ExecutionResult {
                    exit_code: None,
                    status: ExecutionStatus::InternalError,
                    stdout: String::new(),
                    stderr: format!("Failed to fork inside process: {}", e),
                    cpu_time: 0.0,
                    wall_time: 0.0,
                    memory_peak: 0,
                    signal: None,
                    success: false,
                    error_message: Some(format!("Fork error: {}", e)),
                };
                self.send_result_message(proxy_write, error_result.clone());
                Ok(error_result)
            }
        }
    }

    /// Wait for execution results from keeper process
    fn wait_for_execution_results(
        &self,
        keeper_pid: Pid,
        _keeper_read: RawFd,
        start_time: Instant,
    ) -> Result<ExecutionResult> {
        // Wait for keeper process to complete
        match waitpid(keeper_pid, None) {
            Ok(_) => {
                // Try to read result from pipe
                let result = ExecutionResult {
                    exit_code: Some(0),
                    status: ExecutionStatus::Success,
                    stdout: String::new(),
                    stderr: String::new(),
                    cpu_time: 0.0,
                    wall_time: start_time.elapsed().as_secs_f64(),
                    memory_peak: 0,
                    signal: None,
                    success: true,
                    error_message: None,
                };
                
                // TODO: Actually read result from pipe
                // For now, return a basic success result
                Ok(result)
            }
            Err(e) => Err(IsolateError::Process(format!("Keeper process failed: {}", e))),
        }
    }

    /// Monitor proxy process
    fn monitor_proxy_process(
        &self,
        keeper_write: RawFd,
        proxy_pid: Pid,
        _status_read: RawFd,
    ) -> Result<ExecutionResult> {
        // Wait for proxy process
        match waitpid(proxy_pid, None) {
            Ok(WaitStatus::Exited(_, exit_code)) => {
                let result = ExecutionResult {
                    exit_code: Some(exit_code),
                    status: if exit_code == 0 { ExecutionStatus::Success } else { ExecutionStatus::RuntimeError },
                    stdout: String::new(),
                    stderr: String::new(),
                    cpu_time: 0.0,
                    wall_time: 0.0,
                    memory_peak: 0,
                    signal: None,
                    success: exit_code == 0,
                    error_message: None,
                };
                self.send_result_message(keeper_write, result.clone());
                Ok(result)
            }
            Ok(WaitStatus::Signaled(_, signal, _)) => {
                let result = ExecutionResult {
                    exit_code: None,
                    status: ExecutionStatus::RuntimeError,
                    stdout: String::new(),
                    stderr: format!("Proxy terminated by signal {}", signal),
                    cpu_time: 0.0,
                    wall_time: 0.0,
                    memory_peak: 0,
                    signal: Some(signal as i32),
                    success: false,
                    error_message: Some(format!("Proxy terminated by signal {}", signal)),
                };
                self.send_result_message(keeper_write, result.clone());
                Ok(result)
            }
            Err(e) => {
                let error_result = ExecutionResult {
                    exit_code: None,
                    status: ExecutionStatus::InternalError,
                    stdout: String::new(),
                    stderr: format!("Proxy monitoring error: {}", e),
                    cpu_time: 0.0,
                    wall_time: 0.0,
                    memory_peak: 0,
                    signal: None,
                    success: false,
                    error_message: Some(format!("Monitoring error: {}", e)),
                };
                self.send_result_message(keeper_write, error_result.clone());
                Ok(error_result)
            }
            _ => {
                let error_result = ExecutionResult {
                    exit_code: None,
                    status: ExecutionStatus::InternalError,
                    stdout: String::new(),
                    stderr: "Unexpected proxy status".to_string(),
                    cpu_time: 0.0,
                    wall_time: 0.0,
                    memory_peak: 0,
                    signal: None,
                    success: false,
                    error_message: Some("Unexpected proxy status".to_string()),
                };
                self.send_result_message(keeper_write, error_result.clone());
                Ok(error_result)
            }
        }
    }

    /// Send result message through pipe
    fn send_result_message(&self, _fd: RawFd, _result: ExecutionResult) {
        // TODO: Implement actual IPC message sending
        // For now, this is a placeholder
    }

    /// Send status message through pipe
    fn send_status_message(&self, _fd: RawFd, _message: IpcMessage) {
        // TODO: Implement actual IPC message sending
        // For now, this is a placeholder
    }

    /// Send error message through pipe
    fn send_error_message(&self, fd: RawFd, message: &str) {
        let error_msg = IpcMessage::Error { message: message.to_string() };
        self.send_status_message(fd, error_msg);
    }

    /// Apply comprehensive security measures
    fn apply_security_measures(&self, config: &IsolateConfig) -> Result<()> {
        // Apply seccomp filtering if enabled
        if config.enable_seccomp {
            self.apply_seccomp_filtering(config)?;
        }
        
        // Apply no-new-privs to prevent privilege escalation
        self.apply_no_new_privs()?;
        
        // Apply resource limits
        self.apply_resource_limits(config)?;
        
        // Drop dangerous capabilities
        self.drop_dangerous_capabilities()?;
        
        Ok(())
    }

    /// Apply seccomp filtering based on configuration
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

    /// Apply no-new-privs flag
    fn apply_no_new_privs(&self) -> Result<()> {
        // Apply no-new-privs to prevent privilege escalation
        if let Err(e) = prctl::set_no_new_privileges(true) {
            return Err(IsolateError::Process(format!("Failed to set no-new-privs: {}", e)));
        }
        Ok(())
    }

    /// Apply resource limits using rlimits
    fn apply_resource_limits(&self, config: &IsolateConfig) -> Result<()> {
        use nix::sys::resource::{setrlimit, Resource};
        
        // Set file descriptor limit
        if let Some(fd_limit) = config.fd_limit {
            setrlimit(Resource::RLIMIT_NOFILE, fd_limit, fd_limit)
                .map_err(|e| IsolateError::Process(format!("Failed to set fd limit: {}", e)))?;
        }
        
        // Set stack size limit
        setrlimit(Resource::RLIMIT_STACK, 8 * 1024 * 1024, 8 * 1024 * 1024)
            .map_err(|e| IsolateError::Process(format!("Failed to set stack limit: {}", e)))?;
        
        // Set CPU time limit if specified
        if let Some(cpu_limit) = config.cpu_time_limit {
            setrlimit(Resource::RLIMIT_CPU, cpu_limit.as_secs(), cpu_limit.as_secs())
                .map_err(|e| IsolateError::Process(format!("Failed to set CPU limit: {}", e)))?;
        }
        
        // Set memory limit if specified
        if let Some(memory_limit) = config.memory_limit {
            setrlimit(Resource::RLIMIT_AS, memory_limit, memory_limit)
                .map_err(|e| IsolateError::Process(format!("Failed to set memory limit: {}", e)))?;
        }
        
        Ok(())
    }

    /// Drop dangerous capabilities
    fn drop_dangerous_capabilities(&self) -> Result<()> {
        // Drop all capabilities for security
        if let Err(e) = caps::clear(None, caps::CapSet::Effective) {
            return Err(IsolateError::Process(format!("Failed to drop capabilities: {}", e)));
        }
        Ok(())
    }

    /// Execute the actual command in the inside process
    fn execute_command(&self, config: &IsolateConfig, command: &[String], stdin_data: Option<&str>) -> Result<ExecutionResult> {
        // Use single process executor for inside process
        let mut executor = crate::executor::ProcessExecutor::new(config.clone())?;
        executor.execute_single_process(command, stdin_data)
    }

    /// Monitor the inside process and send status updates
    fn monitor_inside_process(&self, inside_pid: u32, status_pipe_write: RawFd) {
        let start_time = Instant::now();
        
        loop {
            // Check if process is still alive
            match nix::sys::signal::kill(nix::unistd::Pid::from_raw(inside_pid as i32), None) {
                Ok(_) => {
                    // Process is alive, send status update
                    let cpu_time = start_time.elapsed().as_secs_f64();
                    let memory = 0; // TODO: Get actual memory usage
                    
                    let status = IpcMessage::StatusUpdate { cpu_time, memory };
                    self.send_status_message(status_pipe_write, status);
                }
                Err(_) => {
                    // Process is dead, exit monitoring
                    break;
                }
            }
            
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// Terminate all processes
    fn terminate_all_processes(&self) {
        // Send shutdown signal to all processes
        self.shutdown_flag.store(true, Ordering::Relaxed);
    }
}