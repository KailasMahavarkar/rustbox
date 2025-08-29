/// Core types and structures for the rustbox system
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    // New enhanced lock errors
    #[error("Advanced lock error: {0}")]
    AdvancedLock(#[from] LockError),
}

/// Rich error types for the new locking system
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("Box {box_id} is busy (owned by PID {owner_pid:?})")]
    Busy { 
        box_id: u32, 
        owner_pid: Option<u32> 
    },

    #[error("Timeout waiting for box {box_id} after {waited:?} (current owner: {current_owner:?})")]
    Timeout {
        box_id: u32,
        waited: std::time::Duration,
        current_owner: Option<String>
    },

    #[error("Lock directory permission denied: {details}")]
    PermissionDenied { 
        details: String 
    },

    #[error("Filesystem error: {source}")]
    FilesystemError { 
        #[from] source: std::io::Error 
    },

    #[error("Lock corruption detected for box {box_id}: {details}")]
    CorruptedLock { 
        box_id: u32, 
        details: String 
    },

    #[error("System error: {message}")]
    SystemError { 
        message: String 
    },

    #[error("Heartbeat failed for box {box_id}: {reason}")]
    HeartbeatFailed {
        box_id: u32,
        reason: String,
    },

    #[error("Lock manager not initialized")]
    NotInitialized,
}

/// Convert to user-friendly exit codes and messages
impl From<LockError> for i32 {
    fn from(err: LockError) -> i32 {
        match err {
            LockError::Busy { .. } => 2,           // Temporary failure
            LockError::Timeout { .. } => 3,        // Timeout
            LockError::PermissionDenied { .. } => 77,  // Permission error
            LockError::FilesystemError { .. } => 74,   // IO error
            LockError::CorruptedLock { .. } => 75,     // Data error
            LockError::SystemError { .. } => 76,      // Service unavailable
            LockError::HeartbeatFailed { .. } => 78,  // Config error
            LockError::NotInitialized => 1,           // General error
        }
    }
}

/// Result type alias for rustbox operations
pub type Result<T> = std::result::Result<T, IsolateError>;

/// Result type for new lock operations
pub type LockResult<T> = std::result::Result<T, LockError>;

/// Lock information stored in lock files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub pid: u32,
    pub box_id: u32,
    pub created_at: std::time::SystemTime,
    pub rustbox_version: String,
}

/// Health status for the lock manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Lock manager health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockManagerHealth {
    pub status: HealthStatus,
    pub active_locks: u32,
    pub stale_locks_cleaned: u64,
    pub lock_directory_writable: bool,
    pub cleanup_thread_alive: bool,
    pub metrics: LockMetrics,
}

/// Metrics for lock operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockMetrics {
    pub total_acquisitions: u64,
    pub average_acquisition_time_ms: f64,
    pub lock_contentions: u64,
    pub cleanup_operations: u64,
    pub errors_by_type: HashMap<String, u64>,
}
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

/// Language-specific configuration for code execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanguageConfig {
    #[serde(default)]
    pub description: Option<String>,
    pub memory: Option<MemoryLimits>,
    pub time: Option<TimeLimits>,
    pub processes: Option<ProcessLimits>,
    pub filesystem: Option<FilesystemLimits>,
    pub network: Option<NetworkLimits>,
    pub syscalls: Option<SyscallLimits>,
    pub security: Option<SecurityConfig>,
    pub environment: Option<HashMap<String, String>>,
    pub compilation: Option<CompilationConfig>,
}

/// Memory limit configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryLimits {
    pub limit_mb: Option<u64>,
    pub limit_kb: Option<u64>,
    pub swap_limit_mb: Option<u64>,
}

/// Time limit configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeLimits {
    pub cpu_time_seconds: Option<u64>,
    pub wall_time_seconds: Option<u64>,
    pub idle_timeout_seconds: Option<u64>,
    pub compilation_time_seconds: Option<u64>,
}

/// Process limit configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessLimits {
    pub max_processes: Option<u32>,
    pub max_threads: Option<u32>,
    pub max_forks: Option<u32>,
}

/// Filesystem limit configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilesystemLimits {
    pub max_file_size_kb: Option<u64>,
    pub max_open_files: Option<u64>,
    pub max_directories: Option<u32>,
    pub read_only_paths: Option<Vec<String>>,
    pub writable_paths: Option<Vec<String>>,
    pub additional_read_only_paths: Option<Vec<String>>,
    pub required_binaries: Option<Vec<String>>,
    pub compilation_output_limit_kb: Option<u64>,
}

/// Network configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkLimits {
    pub enabled: Option<bool>,
    pub allow_localhost: Option<bool>,
    pub blocked_ports: Option<Vec<u16>>,
}

/// System call limitations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyscallLimits {
    pub allow_fork: Option<bool>,
    pub allow_exec: Option<bool>,
    pub allow_clone: Option<bool>,
    pub allow_network: Option<bool>,
    pub allow_filesystem_write: Option<bool>,
    pub allow_ptrace: Option<bool>,
    pub allow_mount: Option<bool>,
    pub blocked_syscalls: Option<Vec<String>>,
    pub additional_blocked_syscalls: Option<Vec<String>>,
    pub additional_allowed_syscalls: Option<Vec<String>>,
    pub compilation_syscalls: Option<Box<SyscallLimits>>,
}

/// Security configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub drop_capabilities: Option<bool>,
    pub use_seccomp: Option<bool>,
    pub use_namespaces: Option<bool>,
    pub use_cgroups: Option<bool>,
    pub no_new_privileges: Option<bool>,
    pub chroot_jail: Option<bool>,
}

/// Compilation configuration for compiled languages
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationConfig {
    pub enabled: Option<bool>,
    pub compiler: String,
    pub compiler_args: Vec<String>,
    pub max_compilation_time: Option<u64>,
    pub max_compilation_memory_mb: Option<u64>,
}

/// Security profile configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityProfile {
    pub description: String,
    pub apply_to_all_languages: Option<bool>,
    pub apply_to_languages: Option<Vec<String>>,
    pub overrides: LanguageConfig,
}

/// Complete language limits configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanguageLimitsConfig {
    pub default_limits: LanguageConfig,
    pub language_overrides: HashMap<String, LanguageConfig>,
    pub security_profiles: Option<HashMap<String, SecurityProfile>>,
}

/// New unified configuration structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnifiedConfig {
    pub isolate: IsolateSettings,
    pub syscalls: SyscallConfig,
    pub security: UnifiedSecurityConfig,
    pub languages: HashMap<String, LanguageSettings>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IsolateSettings {
    pub box_dir: String,
    pub run_dir: String,
    pub user: String,
    pub group: String,
    pub preserve_env: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyscallConfig {
    pub allow_fork: bool,
    pub allow_exec: bool,
    pub allow_clone: bool,
    pub allow_network: bool,
    pub allow_filesystem_write: bool,
    pub allow_ptrace: bool,
    pub allow_mount: bool,
    pub blocked_syscalls: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnifiedSecurityConfig {
    pub drop_capabilities: bool,
    pub use_seccomp: bool,
    pub use_namespaces: bool,
    pub use_cgroups: bool,
    pub no_new_privileges: bool,
    pub chroot_jail: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LanguageSettings {
    pub memory: UnifiedMemoryLimits,
    pub time: UnifiedTimeLimits,
    pub processes: UnifiedProcessLimits,
    pub filesystem: UnifiedFilesystemLimits,
    pub syscalls: Option<SyscallOverrides>,
    pub environment: HashMap<String, String>,
    pub compilation: CompilationSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnifiedMemoryLimits {
    pub limit_mb: u64,
    pub limit_kb: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnifiedTimeLimits {
    pub cpu_time_seconds: u64,
    pub wall_time_seconds: u64,
    pub compilation_time_seconds: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnifiedProcessLimits {
    pub max_processes: u32,
    pub max_threads: Option<u32>,
    pub max_forks: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnifiedFilesystemLimits {
    pub max_file_size_kb: u64,
    pub max_open_files: u32,
    pub additional_read_only_paths: Option<Vec<String>>,
    pub required_binaries: Option<Vec<String>>,
    pub compilation_output_limit_kb: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyscallOverrides {
    pub allow_exec: Option<bool>,
    pub allow_clone: Option<bool>,
    pub additional_blocked_syscalls: Option<Vec<String>>,
    pub additional_allowed_syscalls: Option<Vec<String>>,
    pub compilation_syscalls: Option<CompilationSyscalls>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationSyscalls {
    pub allow_fork: bool,
    pub allow_exec: bool,
    pub additional_allowed_syscalls: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationSettings {
    pub enabled: Option<bool>,
    pub compiler: String,
    pub compiler_args: Vec<String>,
    pub max_compilation_time: u64,
    pub max_compilation_memory_mb: u64,
}

impl UnifiedConfig {
    /// Load unified configuration from JSON file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| IsolateError::Config(format!("Failed to read config file: {}", e)))?;
        Self::from_json(&content)
    }

    /// Parse unified configuration from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| IsolateError::Config(format!("Failed to parse JSON config: {}", e)))
    }

    /// Convert unified config to IsolateConfig for a specific language
    pub fn to_isolate_config(&self, language: &str, base_config: &mut IsolateConfig) -> Result<()> {
        if let Some(lang_settings) = self.languages.get(language) {
            // Apply memory limits (convert MB to bytes)
            base_config.memory_limit = Some((lang_settings.memory.limit_mb * 1024 * 1024) as u64);
            
            // Apply time limits
            base_config.cpu_time_limit = Some(std::time::Duration::from_secs(lang_settings.time.cpu_time_seconds));
            base_config.wall_time_limit = Some(std::time::Duration::from_secs(lang_settings.time.wall_time_seconds));
            
            // Apply compilation time limits if present
            if let Some(compilation_time) = lang_settings.time.compilation_time_seconds {
                base_config.time_limit = Some(std::time::Duration::from_secs(compilation_time));
            }
            
            // Apply process limits
            base_config.process_limit = Some(lang_settings.processes.max_processes);
            
            // Apply filesystem limits
            base_config.file_size_limit = Some(lang_settings.filesystem.max_file_size_kb * 1024);
            base_config.fd_limit = Some(lang_settings.filesystem.max_open_files as u64);
            
            // Apply compilation output limit if present
            if let Some(output_limit) = lang_settings.filesystem.compilation_output_limit_kb {
                // This could be used for restricting compilation output size
                // For now, we'll apply it as additional file size limit during compilation
                base_config.file_size_limit = Some(output_limit * 1024);
            }
            
            // Apply environment variables
            base_config.environment.clear();
            for (key, value) in &lang_settings.environment {
                base_config.environment.push((key.clone(), value.clone()));
            }
            
            // Apply isolate settings
            base_config.chroot_dir = Some(std::path::PathBuf::from(&self.isolate.box_dir));
            base_config.workdir = std::path::PathBuf::from(&self.isolate.run_dir);
            
            // Apply syscall settings from global config
            base_config.enable_network = self.syscalls.allow_network;
            
            // Override with language-specific syscall settings if present
            if let Some(syscall_overrides) = &lang_settings.syscalls {
                if let Some(_allow_exec) = syscall_overrides.allow_exec {
                    // Store this information for later use by the isolation engine
                    // The actual syscall filtering would be done at the seccomp level
                }
                if let Some(_allow_clone) = syscall_overrides.allow_clone {
                    // Similarly, this would be handled by the seccomp filter
                }
            }
            
            // Apply security settings
            // These are mainly informational for the isolation engine
            // The actual enforcement happens in the isolation implementation
            
            Ok(())
        } else {
            Err(IsolateError::Config(format!("Language '{}' not supported", language)))
        }
    }

    /// Get supported languages
    pub fn get_supported_languages(&self) -> Vec<String> {
        self.languages.keys().cloned().collect()
    }

    /// Check if compilation is enabled for a language
    pub fn is_compilation_enabled(&self, language: &str) -> bool {
        if let Some(lang_settings) = self.languages.get(language) {
            lang_settings.compilation.enabled.unwrap_or(false)
        } else {
            false
        }
    }

    /// Get compilation settings for a language
    pub fn get_compilation_settings(&self, language: &str) -> Option<&CompilationSettings> {
        self.languages.get(language).map(|settings| &settings.compilation)
    }
}

impl LanguageLimitsConfig {
    /// Load language limits configuration from JSON file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| IsolateError::Config(format!("Failed to read config file: {}", e)))?;
        Self::from_json(&content)
    }

    /// Parse language limits configuration from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| IsolateError::Config(format!("Failed to parse JSON config: {}", e)))
    }

    /// Get configuration for a specific language, merging with defaults
    pub fn get_language_config(&self, language: &str) -> LanguageConfig {
        let mut config = self.default_limits.clone();
        
        if let Some(lang_config) = self.language_overrides.get(language) {
            merge_language_config(&mut config, lang_config);
        }
        
        config
    }

    /// Apply security profile to configuration
    pub fn apply_security_profile(&self, config: &mut LanguageConfig, profile_name: &str, language: &str) -> Result<()> {
        if let Some(profiles) = &self.security_profiles {
            if let Some(profile) = profiles.get(profile_name) {
                let should_apply = profile.apply_to_all_languages.unwrap_or(false) ||
                    profile.apply_to_languages.as_ref()
                        .map(|langs| langs.contains(&language.to_string()))
                        .unwrap_or(false);
                
                if should_apply {
                    merge_language_config(config, &profile.overrides);
                }
            } else {
                return Err(IsolateError::Config(format!("Security profile '{}' not found", profile_name)));
            }
        }
        Ok(())
    }

    /// Convert language config to IsolateConfig
    pub fn to_isolate_config(&self, language: &str, base_config: &mut IsolateConfig) -> Result<()> {
        let lang_config = self.get_language_config(language);
        
        // Apply memory limits
        if let Some(memory) = &lang_config.memory {
            if let Some(limit_mb) = memory.limit_mb {
                base_config.memory_limit = Some(limit_mb * 1024 * 1024);
            }
        }
        
        // Apply time limits
        if let Some(time) = &lang_config.time {
            if let Some(cpu_time) = time.cpu_time_seconds {
                base_config.cpu_time_limit = Some(Duration::from_secs(cpu_time));
            }
            if let Some(wall_time) = time.wall_time_seconds {
                base_config.wall_time_limit = Some(Duration::from_secs(wall_time));
            }
        }
        
        // Apply process limits
        if let Some(processes) = &lang_config.processes {
            if let Some(max_proc) = processes.max_processes {
                base_config.process_limit = Some(max_proc);
            }
        }
        
        // Apply filesystem limits
        if let Some(filesystem) = &lang_config.filesystem {
            if let Some(file_size) = filesystem.max_file_size_kb {
                base_config.file_size_limit = Some(file_size * 1024);
            }
            if let Some(open_files) = filesystem.max_open_files {
                base_config.fd_limit = Some(open_files);
            }
        }
        
        // Apply network settings
        if let Some(network) = &lang_config.network {
            if let Some(enabled) = network.enabled {
                base_config.enable_network = enabled;
            }
        }
        
        // Apply environment variables
        if let Some(env_vars) = &lang_config.environment {
            for (key, value) in env_vars {
                base_config.environment.push((key.clone(), value.clone()));
            }
        }
        
        Ok(())
    }
}

/// Merge two language configurations, with override taking precedence
fn merge_language_config(base: &mut LanguageConfig, override_config: &LanguageConfig) {
    if override_config.memory.is_some() {
        base.memory = override_config.memory.clone();
    }
    if override_config.time.is_some() {
        base.time = override_config.time.clone();
    }
    if override_config.processes.is_some() {
        base.processes = override_config.processes.clone();
    }
    if override_config.filesystem.is_some() {
        base.filesystem = override_config.filesystem.clone();
    }
    if override_config.network.is_some() {
        base.network = override_config.network.clone();
    }
    if override_config.syscalls.is_some() {
        base.syscalls = override_config.syscalls.clone();
    }
    if override_config.security.is_some() {
        base.security = override_config.security.clone();
    }
    if let Some(override_env) = &override_config.environment {
        if let Some(base_env) = &mut base.environment {
            base_env.extend(override_env.clone());
        } else {
            base.environment = Some(override_env.clone());
        }
    }
    if override_config.compilation.is_some() {
        base.compilation = override_config.compilation.clone();
    }
}