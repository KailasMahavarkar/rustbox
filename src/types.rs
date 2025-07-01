/// Core types and structures for the mini-isolate system
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Helper function for serde default value
fn default_true() -> bool {
    true
}

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
    /// Stack size limit in bytes
    pub stack_limit: Option<u64>,
    /// Core dump size limit in bytes (0 to disable core dumps)
    pub core_limit: Option<u64>,
    /// Disk quota limit in bytes (filesystem-dependent)
    pub disk_quota: Option<u64>,
    /// Enable networking
    pub enable_network: bool,
    /// Custom environment variables
    pub environment: Vec<(String, String)>,
    /// Allowed syscalls (seccomp filter)
    pub allowed_syscalls: Option<Vec<String>>,
    /// Strict mode: fail hard if cgroups unavailable or permission denied
    pub strict_mode: bool,
    /// Inherit file descriptors from parent process
    #[serde(default)]
    pub inherit_fds: bool,
    /// Enable seccomp syscall filtering
    #[serde(default = "default_true")]
    pub enable_seccomp: bool,
    /// Language-specific seccomp profile
    pub seccomp_profile: Option<String>,
    /// Redirect stdout to file (optional)
    pub stdout_file: Option<PathBuf>,
    /// Redirect stderr to file (optional)
    pub stderr_file: Option<PathBuf>,    /// Enable TTY support for interactive programs
    pub enable_tty: bool,
    /// Use pipes for real-time I/O instead of files
    pub use_pipes: bool,
    /// Input data to send to stdin
    pub stdin_data: Option<String>,
    /// Redirect stdin from file (optional)
    pub stdin_file: Option<PathBuf>,
    /// Buffer size for I/O operations (bytes)
    pub io_buffer_size: usize,
    /// Text encoding for I/O operations
    pub text_encoding: String,    /// Namespace isolation configuration
    pub enable_pid_namespace: bool,
    pub enable_mount_namespace: bool,
    pub enable_network_namespace: bool,
    pub enable_user_namespace: bool,
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
            stack_limit: Some(8 * 1024 * 1024), // 8MB default stack
            core_limit: Some(0), // Disable core dumps by default
            disk_quota: None, // No disk quota by default
            enable_network: false,
            environment: Vec::new(),
            allowed_syscalls: None,
            strict_mode: false,
            inherit_fds: false,
            enable_seccomp: true,
            seccomp_profile: None,
            stdout_file: None,
            stderr_file: None,
            enable_tty: false,
            use_pipes: false,
            stdin_data: None,
            stdin_file: None,
            io_buffer_size: 8192, // 8KB default buffer
            text_encoding: "utf-8".to_string(),
            enable_pid_namespace: true,
            enable_mount_namespace: true,
            enable_network_namespace: true,
            enable_user_namespace: false, // User namespace can be complex, disabled by default
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
    /// Stack limit exceeded
    StackLimit,
    /// Core dump limit exceeded
    CoreLimit,
    /// Disk quota exceeded
    DiskQuotaExceeded,
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

    #[error("Lock error: {0}")]
    Lock(String),

    #[error("Lock already held by process")]
    LockBusy,

    #[error("Lock file corrupted or incompatible")]
    LockCorrupted,

    #[error("Namespace isolation error: {0}")]
    Namespace(String),

    #[error("Resource limit error: {0}")]
    ResourceLimit(String),
}

/// Result type alias for mini-isolate operations
pub type Result<T> = std::result::Result<T, IsolateError>;
