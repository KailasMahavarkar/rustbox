# types.rs - Core Types and Data Structures

## Overview
Contains all fundamental data structures, enums, and type definitions used throughout the rustbox system. This module defines the core configuration, execution results, error handling, and resource management types.

## File Location
`src/types.rs`

## Purpose
- Define core data structures for process isolation
- Provide configuration management types
- Implement execution result and status tracking
- Define error types and handling mechanisms

## Dependencies
- `serde`: For serialization/deserialization support
- `std::path::PathBuf`: For file system path handling
- `std::time::Duration`: For time-based configuration
- `thiserror::Error`: For structured error handling

## Core Data Structures

### `struct IsolateConfig`
**Location**: `src/types.rs:8-40`

Process isolation configuration structure that defines all parameters for creating an isolated execution environment.

#### Fields
- **`instance_id: String`** - Unique identifier for the isolation instance
- **`workdir: PathBuf`** - Working directory for the isolated process
- **`chroot_dir: Option<PathBuf>`** - Optional root directory for chroot isolation
- **`uid: Option<u32>`** - User ID to run the process as
- **`gid: Option<u32>`** - Group ID to run the process as
- **`memory_limit: Option<u64>`** - Memory limit in bytes
- **`time_limit: Option<Duration>`** - General time limit for execution
- **`cpu_time_limit: Option<Duration>`** - CPU time limit
- **`wall_time_limit: Option<Duration>`** - Wall clock time limit
- **`process_limit: Option<u32>`** - Maximum number of processes
- **`file_size_limit: Option<u64>`** - Maximum file size in bytes
- **`enable_network: bool`** - Whether to enable network access
- **`environment: Vec<(String, String)>`** - Custom environment variables
- **`allowed_syscalls: Option<Vec<String>>`** - Allowed syscalls for seccomp filtering
- **`strict_mode: bool`** - Whether to fail hard if cgroups are unavailable

#### Traits
- `Clone, Debug, Serialize, Deserialize` - For copying, debugging, and persistence

### `impl Default for IsolateConfig`
**Location**: `src/types.rs:42-62`

Default configuration implementation providing sensible defaults for process isolation.

#### Default Values
- **Memory Limit**: 128MB (128 * 1024 * 1024 bytes)
- **Time Limits**: 10 seconds execution, 20 seconds wall time
- **Process Limit**: 1 process
- **File Size Limit**: 64MB
- **Network**: Disabled by default
- **Working Directory**: `/tmp/rustbox/{uuid}`
- **Strict Mode**: Disabled

### `struct ExecutionResult`
**Location**: `src/types.rs:65-87`

Comprehensive execution result containing all information about a completed process execution.

#### Fields
- **`exit_code: Option<i32>`** - Process exit code (None if killed)
- **`status: ExecutionStatus`** - Execution status enum
- **`stdout: String`** - Standard output captured from process
- **`stderr: String`** - Standard error captured from process
- **`cpu_time: f64`** - CPU time used in seconds
- **`wall_time: f64`** - Wall clock time used in seconds
- **`memory_peak: u64`** - Peak memory usage in bytes
- **`signal: Option<i32>`** - Signal that terminated the process (if any)
- **`success: bool`** - Overall success flag
- **`error_message: Option<String>`** - Additional error details

#### Traits
- `Clone, Debug, Serialize, Deserialize` - For copying, debugging, and persistence

### `enum ExecutionStatus`
**Location**: `src/types.rs:90-110`

Enumeration of possible execution statuses for categorizing process execution outcomes.

#### Variants
- **`Success`** - Process completed successfully
- **`TimeLimit`** - Process killed due to time limit
- **`MemoryLimit`** - Process killed due to memory limit
- **`RuntimeError`** - Process exited with non-zero code
- **`InternalError`** - Internal error in isolate system
- **`Signaled`** - Process was killed by signal
- **`SecurityViolation`** - Security violation (forbidden syscall, etc.)
- **`ProcessLimit`** - Process limit exceeded
- **`FileSizeLimit`** - File size limit exceeded

#### Traits
- `Clone, Debug, Serialize, Deserialize, PartialEq` - For copying, debugging, persistence, and comparison

### `struct ResourceUsage`
**Location**: `src/types.rs:113-125`

Resource usage statistics for monitoring process resource consumption.

#### Fields
- **`cpu_time: f64`** - CPU time consumed in seconds
- **`wall_time: f64`** - Wall clock time elapsed in seconds
- **`memory_peak: u64`** - Peak memory usage in bytes
- **`context_switches: u64`** - Number of context switches
- **`page_faults: u64`** - Number of page faults

#### Traits
- `Clone, Debug, Serialize, Deserialize` - For copying, debugging, and persistence

## Error Handling

### `enum IsolateError`
**Location**: `src/types.rs:128-143`

Structured error type for handling various failure modes in the isolation system.

#### Variants
- **`Io(std::io::Error)`** - I/O operation errors
- **`Cgroup(String)`** - Cgroup management errors
- **`Config(String)`** - Configuration validation errors
- **`Process(String)`** - Process execution errors

#### Traits
- `Error, Debug` - For structured error handling with the `thiserror` crate

### `type Result<T>`
**Location**: `src/types.rs:146`

Type alias for convenient error handling throughout the codebase.

#### Definition
```rust
pub type Result<T> = std::result::Result<T, IsolateError>;
```

## Design Patterns

### Configuration Management
- **Builder Pattern**: Default implementation allows for easy configuration building
- **Optional Fields**: Extensive use of `Option<T>` for flexible configuration
- **Sensible Defaults**: Default implementation provides production-ready defaults

### Resource Tracking
- **Comprehensive Monitoring**: ExecutionResult captures all relevant execution metrics
- **Structured Status**: ExecutionStatus enum provides clear categorization of outcomes
- **Resource Limits**: Multiple limit types (time, memory, processes, file size)

### Error Handling
- **Structured Errors**: IsolateError enum provides clear error categorization
- **Error Context**: String-based error messages provide detailed context
- **Error Propagation**: Result type alias simplifies error handling throughout codebase

## Usage Patterns

### Configuration Creation
```rust
let config = IsolateConfig {
    memory_limit: Some(256 * 1024 * 1024), // 256MB
    time_limit: Some(Duration::from_secs(30)),
    strict_mode: true,
    ..Default::default()
};
```

### Result Processing
```rust
match result.status {
    ExecutionStatus::Success => println!("Process completed successfully"),
    ExecutionStatus::TimeLimit => println!("Process timed out"),
    ExecutionStatus::MemoryLimit => println!("Process exceeded memory limit"),
    _ => println!("Process failed: {:?}", result.error_message),
}
```

### Error Handling
```rust
match isolate_operation() {
    Ok(result) => process_result(result),
    Err(IsolateError::Cgroup(msg)) => handle_cgroup_error(msg),
    Err(IsolateError::Config(msg)) => handle_config_error(msg),
    Err(e) => handle_generic_error(e),
}
```

## Serialization Support
All major types implement `Serialize` and `Deserialize` traits, enabling:
- Configuration persistence to JSON/YAML files
- Result output in structured formats
- Integration with external systems requiring structured data

## Security Considerations
- **Resource Limits**: All limits are configurable and enforced at runtime
- **Strict Mode**: Provides fail-safe behavior when security requirements cannot be met
- **Sandboxing**: Configuration supports various isolation mechanisms (chroot, uid/gid changes)
- **Network Isolation**: Network access is disabled by default