/// Core types and structures for the mini-isolate system
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Process isolation configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IsolateConfig {
    /// Unique identifier for this isolation instance
    pub instance_id: String,
    /// Working directory for the isolated process
    pub workdir: PathBuf,
    /// Root directory for chroot (optional)
    pub chroot_dir: Option<PathBuf>,
    /// User ID to run as
    pub uid: Option<u32>,
    /// Group ID to run as
    pub gid: Option<u32>,
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
    /// Time limit for execution
    pub time_limit: Option<Duration>,
    /// CPU time limit
    pub cpu_time_limit: Option<Duration>,
    /// Wall clock time limit
    pub wall_time_limit: Option<Duration>,
    /// Maximum number of processes
    pub process_limit: Option<u32>,
    /// Maximum file size
    pub file_size_limit: Option<u64>,
    /// Enable networking
    pub enable_network: bool,
    /// Custom environment variables
    pub environment: Vec<(String, String)>,
    /// Allowed syscalls (seccomp filter)
    pub allowed_syscalls: Option<Vec<String>>,
    /// Strict mode: fail hard if cgroups unavailable or permission denied
    pub strict_mode: bool,
}

impl Default for IsolateConfig {
    fn default() -> Self {
        Self {
            instance_id: uuid::Uuid::new_v4().to_string(),
            workdir: std::env::temp_dir().join("mini-isolate"),
            chroot_dir: None,
            uid: None,
            gid: None,
            memory_limit: Some(128 * 1024 * 1024), // 128MB default
            time_limit: Some(Duration::from_secs(10)),
            cpu_time_limit: Some(Duration::from_secs(10)),
            wall_time_limit: Some(Duration::from_secs(20)),
            process_limit: Some(1),
            file_size_limit: Some(64 * 1024 * 1024), // 64MB
            enable_network: false,
            environment: Vec::new(),
            allowed_syscalls: None,
            strict_mode: false,
        }
    }
}

/// Execution result from an isolated process
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Exit code of the process
    pub exit_code: Option<i32>,
    /// Execution status
    pub status: ExecutionStatus,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// CPU time used (in seconds)
    pub cpu_time: f64,
    /// Wall clock time used (in seconds)
    pub wall_time: f64,
    /// Peak memory usage (in bytes)
    pub memory_peak: u64,
    /// Signal that terminated the process (if any)
    pub signal: Option<i32>,
    /// Success flag
    pub success: bool,
    /// Additional error message
    pub error_message: Option<String>,
}

/// Status of process execution
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    /// Process completed successfully
    Success,
    /// Process was killed due to time limit
    TimeLimit,
    /// Process was killed due to memory limit
    MemoryLimit,
    /// Process exited with non-zero code
    RuntimeError,
    /// Internal error in isolate system
    InternalError,
    /// Process was killed by signal
    Signaled,
    /// Security violation (forbidden syscall, etc.)
    SecurityViolation,
    /// Process limit exceeded
    ProcessLimit,
    /// File size limit exceeded
    FileSizeLimit,
}

/// Resource usage statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU time in seconds
    pub cpu_time: f64,
    /// Wall clock time in seconds
    pub wall_time: f64,
    /// Peak memory usage in bytes
    pub memory_peak: u64,
    /// Number of context switches
    pub context_switches: u64,
    /// Number of page faults
    pub page_faults: u64,
}

/// Custom error types for mini-isolate
#[derive(Error, Debug)]
pub enum IsolateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Cgroup error: {0}")]
    Cgroup(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Process error: {0}")]
    Process(String),
    

}

/// Result type alias for mini-isolate operations
pub type Result<T> = std::result::Result<T, IsolateError>;