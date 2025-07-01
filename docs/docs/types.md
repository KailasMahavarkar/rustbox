# Types Module Documentation

The `types.rs` module defines the core data structures and error types used throughout the Mini-Isolate system.

## Core Data Structures

### `IsolateConfig`

The main configuration structure for isolation instances.

```rust
pub struct IsolateConfig {
    pub instance_id: String,
    pub workdir: PathBuf,
    pub chroot_dir: Option<PathBuf>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub memory_limit: Option<u64>,
    pub time_limit: Option<Duration>,
    pub cpu_time_limit: Option<Duration>,
    pub wall_time_limit: Option<Duration>,
    pub process_limit: Option<u32>,
    pub file_size_limit: Option<u64>,
    pub enable_network: bool,
    pub environment: Vec<(String, String)>,
    pub allowed_syscalls: Option<Vec<String>>,
}
```

#### Fields

- **instance_id**: Unique identifier for the isolation instance
- **workdir**: Working directory for isolated processes
- **chroot_dir**: Optional chroot directory for filesystem isolation
- **uid/gid**: User/group IDs to run processes as
- **memory_limit**: Maximum memory usage in bytes
- **time_limit**: CPU execution time limit
- **cpu_time_limit**: CPU time limit (alternative naming)
- **wall_time_limit**: Wall clock time limit
- **process_limit**: Maximum number of processes
- **file_size_limit**: Maximum file size in bytes
- **enable_network**: Whether to allow network access
- **environment**: Custom environment variables
- **allowed_syscalls**: Seccomp filter syscalls (future feature)

#### Default Values

```rust
impl Default for IsolateConfig {
    fn default() -> Self {
        Self {
            instance_id: uuid::Uuid::new_v4().to_string(),
            workdir: std::env::temp_dir().join("mini-isolate"),
            chroot_dir: None,
            uid: None,
            gid: None,
            memory_limit: Some(128 * 1024 * 1024), // 128MB
            time_limit: Some(Duration::from_secs(10)),
            cpu_time_limit: Some(Duration::from_secs(10)),
            wall_time_limit: Some(Duration::from_secs(20)),
            process_limit: Some(1),
            file_size_limit: Some(64 * 1024 * 1024), // 64MB
            enable_network: false,
            environment: Vec::new(),
            allowed_syscalls: None,
        }
    }
}
```

### `ExecutionResult`

Represents the result of executing a process in the isolate.

```rust
pub struct ExecutionResult {
    pub exit_code: Option<i32>,
    pub status: ExecutionStatus,
    pub stdout: String,
    pub stderr: String,
    pub cpu_time: f64,
    pub wall_time: f64,
    pub memory_peak: u64,
    pub signal: Option<i32>,
    pub success: bool,
    pub error_message: Option<String>,
}
```

#### Fields

- **exit_code**: Process exit code (None if killed by signal)
- **status**: Execution status category
- **stdout/stderr**: Process output streams
- **cpu_time**: CPU time used in seconds
- **wall_time**: Wall clock time in seconds
- **memory_peak**: Peak memory usage in bytes
- **signal**: Signal that terminated the process (if any)
- **success**: Boolean indicating successful execution
- **error_message**: Error description (if any)

### `ExecutionStatus`

Enumeration of possible execution outcomes.

```rust
pub enum ExecutionStatus {
    Success,
    TimeLimit,
    MemoryLimit,
    RuntimeError,
    InternalError,
    Signaled,
    SecurityViolation,
    ProcessLimit,
    FileSizeLimit,
}
```

#### Variants

- **Success**: Process completed successfully
- **TimeLimit**: Killed due to time limit
- **MemoryLimit**: Killed due to memory limit
- **RuntimeError**: Process exited with non-zero code
- **InternalError**: Internal isolate system error
- **Signaled**: Process terminated by signal
- **SecurityViolation**: Security policy violation
- **ProcessLimit**: Too many processes spawned
- **FileSizeLimit**: File size limit exceeded

### `ResourceUsage`

Resource usage statistics structure.

```rust
pub struct ResourceUsage {
    pub cpu_time: f64,
    pub wall_time: f64,
    pub memory_peak: u64,
    pub context_switches: u64,
    pub page_faults: u64,
}
```

## Error Types

### `IsolateError`

Main error type for the isolate system using `thiserror`.

```rust
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
```

### `Result<T>`

Type alias for convenient error handling:

```rust
pub type Result<T> = std::result::Result<T, IsolateError>;
```

## Usage Examples

### Creating a Configuration

```rust
use mini_isolate::types::{IsolateConfig, Duration};

let mut config = IsolateConfig::default();
config.memory_limit = Some(256 * 1024 * 1024); // 256MB
config.time_limit = Some(Duration::from_secs(30));
config.process_limit = Some(5);
```

### Handling Results

```rust
use mini_isolate::types::{ExecutionStatus, Result};

fn check_execution_result(result: &ExecutionResult) -> bool {
    match result.status {
        ExecutionStatus::Success => true,
        ExecutionStatus::TimeLimit => {
            eprintln!("Time limit exceeded: {} seconds", result.wall_time);
            false
        },
        ExecutionStatus::MemoryLimit => {
            eprintln!("Memory limit exceeded: {} bytes", result.memory_peak);
            false
        },
        _ => false,
    }
}
```

## Serialization

All public types implement `Serialize` and `Deserialize` for JSON persistence:

```rust
use serde_json;

// Serialize configuration
let config = IsolateConfig::default();
let json = serde_json::to_string_pretty(&config)?;

// Serialize execution result
let result = ExecutionResult { /* ... */ };
let json = serde_json::to_string(&result)?;
```

## Best Practices

1. Always use the `Default` implementation as a starting point
2. Set appropriate resource limits based on your use case
3. Handle all error cases explicitly
4. Use the structured error types for proper error reporting
5. Serialize results for persistent logging and analysis