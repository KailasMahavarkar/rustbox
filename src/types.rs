/// Core types and structures for the rustbox system
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;


/// Directory binding configuration for filesystem access
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectoryBinding {
    /// Source directory on host system
    pub source: PathBuf,
    /// Target directory within sandbox
    pub target: PathBuf,
    /// Access permissions
    pub permissions: DirectoryPermissions,
    /// Ignore if source doesn't exist
    pub maybe: bool,
    /// Create as temporary directory
    pub is_tmp: bool,
}

/// Directory access permissions
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DirectoryPermissions {
    /// Read-only access
    ReadOnly,
    /// Read-write access
    ReadWrite,
    /// No execution allowed
    NoExec,
}

impl DirectoryBinding {
    /// Parse directory binding from string format like "source=target:options"
    pub fn parse(binding_str: &str) -> std::result::Result<Self, String> {
        let parts: Vec<&str> = binding_str.split(':').collect();
        let path_part = parts[0];
        let options = if parts.len() > 1 { parts[1] } else { "" };

        let (source, target) = if path_part.contains('=') {
            let path_parts: Vec<&str> = path_part.split('=').collect();
            if path_parts.len() != 2 {
                return Err("Invalid directory binding format. Use: source=target or source=target:options".to_string());
            }
            (PathBuf::from(path_parts[0]), PathBuf::from(path_parts[1]))
        } else {
            // If no target specified, use same path in sandbox
            (PathBuf::from(path_part), PathBuf::from(path_part))
        };

        let mut permissions = DirectoryPermissions::ReadOnly;
        let mut maybe = false;
        let mut is_tmp = false;

        for option in options.split(',') {
            match option.trim() {
                "rw" => permissions = DirectoryPermissions::ReadWrite,
                "ro" => permissions = DirectoryPermissions::ReadOnly,
                "noexec" => permissions = DirectoryPermissions::NoExec,
                "maybe" => maybe = true,
                "tmp" => is_tmp = true,
                "" => {}, // Empty option
                _ => return Err(format!("Unknown directory binding option: {}", option)),
            }
        }

        Ok(DirectoryBinding {
            source,
            target,
            permissions,
            maybe,
            is_tmp,
        })
    }
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
    /// File descriptor limit (max open files)
    pub fd_limit: Option<u64>,
    /// Disk quota limit in bytes (filesystem-dependent)
    pub disk_quota: Option<u64>,
    /// Enable networking
    pub enable_network: bool,
    /// Custom environment variables
    pub environment: Vec<(String, String)>,
    /// Strict mode: fail hard if cgroups unavailable or permission denied
    pub strict_mode: bool,
    /// Inherit file descriptors from parent process
    #[serde(default)]
    pub inherit_fds: bool,
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
    /// Directory bindings for filesystem access
    pub directory_bindings: Vec<DirectoryBinding>,
}

impl Default for IsolateConfig {
    fn default() -> Self {
        Self {
            instance_id: uuid::Uuid::new_v4().to_string(),
            workdir: std::env::temp_dir().join("rustbox"),
            chroot_dir: None,
            uid: None,
            gid: None,
            memory_limit: Some(256 * 1024 * 1024), // 128MB default
            time_limit: Some(Duration::from_secs(10)),
            cpu_time_limit: Some(Duration::from_secs(10)),
            wall_time_limit: Some(Duration::from_secs(20)),
            process_limit: Some(1),
            file_size_limit: Some(64 * 1024 * 1024), // 64MB
            stack_limit: Some(8 * 1024 * 1024), // 8MB default stack
            core_limit: Some(0), // Disable core dumps by default
            fd_limit: Some(64), // Default file descriptor limit (like isolate-reference)
            disk_quota: None, // No disk quota by default
            enable_network: false,
            environment: Vec::new(),
            strict_mode: false,
            inherit_fds: false,
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
            directory_bindings: Vec::new(),
        }
    }
}

/// Execution result from an isolated process
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
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

/// Custom error types for rustbox
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

/// Result type alias for rustbox operations
pub type Result<T> = std::result::Result<T, IsolateError>;
impl From<std::process::Output> for ExecutionResult {
    fn from(output: std::process::Output) -> Self {
        let status = if output.status.success() {
            ExecutionStatus::Success
        } else {
            ExecutionStatus::RuntimeError
        };

        Self {
            exit_code: output.status.code(),
            status,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            cpu_time: 0.0, // Not available from std::process::Output
            wall_time: 0.0, // Not available from std::process::Output
            memory_peak: 0, // Not available from std::process::Output
            signal: {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    output.status.signal()
                }
                #[cfg(not(unix))]
                {
                    None
                }
            },
            success: output.status.success(),
            error_message: None,
        }
    }
}
impl From<nix::errno::Errno> for IsolateError {
    fn from(err: nix::errno::Errno) -> Self {
        IsolateError::Process(err.to_string())
    }
}impl Default for ExecutionStatus {
    fn default() -> Self {
        ExecutionStatus::Success
    }
}