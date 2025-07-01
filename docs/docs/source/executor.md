# executor.rs - Process Execution and Monitoring

## Overview
Core process execution module responsible for managing isolated process execution, resource monitoring, and result collection. Implements the low-level process management and cgroup integration.

## File Location
`src/executor.rs`

## Purpose
- Execute processes within isolated environments
- Monitor resource usage and enforce limits
- Handle process lifecycle management
- Integrate with cgroup controllers for resource control

## Dependencies
- `crate::cgroup::CgroupController`: For resource limit enforcement
- `crate::types`: For configuration and result types
- `std::process`: For process spawning and management
- `std::thread`: For concurrent monitoring
- `std::time`: For timeout handling
- `nix` (Unix): For Unix-specific process operations

## Core Structure

### `struct ProcessExecutor`
**Location**: `src/executor.rs:14-17`

Main process execution engine that handles isolation and monitoring.

#### Fields
- **`config: IsolateConfig`** - Configuration for the isolated execution
- **`cgroup: Option<CgroupController>`** - Optional cgroup controller for resource limits

#### Responsibilities
- Process spawning with configured isolation
- Resource limit enforcement through cgroups
- Process monitoring and timeout handling
- Result collection and formatting

## Core Functions

### `impl ProcessExecutor`

#### `pub fn new(config: IsolateConfig) -> Result<Self>`
**Location**: `src/executor.rs:20-55`

Creates a new process executor with the given configuration.

##### Functionality
1. **Strict Mode Validation**
   - Checks if running as root when strict mode is enabled
   - Validates cgroup availability in strict mode
   - Returns configuration errors if requirements not met

2. **Cgroup Setup**
   - Creates `CgroupController` if cgroups are available
   - Handles permission errors gracefully in non-strict mode
   - Issues warnings when cgroups cannot be created

3. **Error Handling**
   - Returns `IsolateError::Config` for strict mode violations
   - Continues execution with warnings in non-strict mode

##### Parameters
- **`config: IsolateConfig`** - Execution configuration

##### Returns
- `Result<Self>` - New executor instance or configuration error

#### `fn setup_resource_limits(&self) -> Result<()>`
**Location**: `src/executor.rs:57-74`

Configures resource limits using the cgroup controller.

##### Functionality
- **Memory Limits**: Sets memory.limit_in_bytes if configured
- **Process Limits**: Sets pids.max if configured  
- **CPU Limits**: Sets cpu.shares for relative CPU allocation
- **Graceful Degradation**: Continues if cgroup not available

##### Resource Limit Types
- Memory limit in bytes (hard limit)
- Process/task limit (maximum number of processes)
- CPU shares (relative CPU weight, default 1024)

#### `pub fn execute(&mut self, command: &[String], stdin_data: Option<&str>) -> Result<ExecutionResult>`
**Location**: `src/executor.rs:76-149`

Main execution function that runs a command with full isolation and monitoring.

##### Functionality
1. **Input Validation**
   - Ensures command array is not empty
   - Returns configuration error for invalid input

2. **Environment Setup**
   - Creates working directory if needed
   - Configures resource limits via cgroups
   - Sets up process execution environment

3. **Command Construction**
   - Creates `std::process::Command` with first element as program
   - Adds remaining elements as arguments
   - Configures stdio pipes for capturing output

4. **Process Configuration**
   - Sets working directory to configured path
   - Configures environment variables from config
   - Ensures PATH is available for command execution
   - Sets user/group ID if specified (Unix only)

5. **Process Execution**
   - Spawns the child process
   - Adds process to cgroup for resource control
   - Handles stdin data if provided
   - Initiates monitoring with timeout

##### Parameters
- **`command: &[String]`** - Command and arguments to execute
- **`stdin_data: Option<&str>`** - Optional stdin data

##### Returns
- `Result<ExecutionResult>` - Comprehensive execution results

#### `fn wait_with_timeout(...) -> Result<ExecutionResult>`
**Location**: `src/executor.rs:151-282`

Advanced process monitoring with timeout handling and resource tracking.

##### Functionality
1. **Concurrent Monitoring**
   - Spawns monitoring thread for output collection
   - Collects stdout and stderr asynchronously
   - Waits for process completion in separate thread

2. **Timeout Management**
   - Monitors wall clock time against configured limit
   - Sends SIGKILL to process when timeout exceeded
   - Handles cross-platform process termination

3. **Output Collection**
   - Captures stdout and stderr streams
   - Converts binary output to UTF-8 strings
   - Preserves output even during timeout scenarios

4. **Status Determination**
   - Maps process exit status to `ExecutionStatus` enum
   - Handles signal-based termination (Unix)
   - Detects timeout conditions accurately

5. **Resource Usage Tracking**
   - Retrieves CPU time and memory usage from cgroup
   - Calculates wall clock time from execution start
   - Provides comprehensive resource statistics

##### Parameters
- **`child: std::process::Child`** - Spawned child process
- **`timeout: Duration`** - Maximum execution time
- **`start_time: Instant`** - Execution start timestamp

##### Returns
- `Result<ExecutionResult>` - Complete execution results with resource usage

#### `fn get_resource_usage(&self) -> (f64, u64)`
**Location**: `src/executor.rs:284-293`

Retrieves resource usage statistics from the cgroup controller.

##### Functionality
- **CPU Time**: Gets CPU usage in seconds from cgroup
- **Memory Usage**: Gets peak memory usage in bytes
- **Fallback**: Returns (0.0, 0) if cgroup unavailable

##### Returns
- `(f64, u64)` - CPU time in seconds, peak memory in bytes

#### `fn setup_workdir(&self) -> Result<()>`
**Location**: `src/executor.rs:295-313`

Creates and configures the working directory for process execution.

##### Functionality
- **Directory Creation**: Creates directory tree if not exists
- **Permission Setting**: Sets 755 permissions (rwxr-xr-x) on Unix
- **Error Handling**: Propagates I/O errors appropriately

##### Security Considerations
- Uses standard permissions for directory access
- Ensures directory exists before process execution
- Handles permission errors gracefully

#### `pub fn cleanup(&mut self) -> Result<()>`
**Location**: `src/executor.rs:315-321`

Cleans up executor resources and cgroup controllers.

##### Functionality
- **Cgroup Cleanup**: Removes cgroup directories and files
- **Resource Deallocation**: Frees allocated system resources
- **Error Handling**: Ensures cleanup proceeds even with errors

## Implementation Details

### Platform-Specific Code

#### Unix-Specific Features
**Location**: `src/executor.rs:10-11, 114-124, 232-236`

- **Command Extensions**: Uses `CommandExt` for pre_exec hooks
- **User/Group Setting**: Implements setuid/setgid for privilege dropping
- **Signal Handling**: Implements SIGKILL for process termination
- **Exit Status**: Uses `ExitStatusExt` for signal information

#### Cross-Platform Support
**Location**: `src/executor.rs:198-204`

- **Process Termination**: Uses taskkill on Windows
- **Fallback Handling**: Provides alternatives for Unix-specific features
- **Conditional Compilation**: Uses `#[cfg(unix)]` for platform-specific code

### Thread Safety and Concurrency

#### Monitoring Thread
**Location**: `src/executor.rs:160-177`

- **Concurrent Execution**: Monitoring runs in separate thread
- **Output Collection**: Asynchronous stdout/stderr capture
- **Thread Synchronization**: Uses thread join for result collection
- **Timeout Handling**: Main thread monitors timeout while monitoring thread collects output

#### Resource Synchronization
- **Cgroup Operations**: Thread-safe cgroup file operations
- **Process Management**: Safe process ID handling across threads
- **Error Propagation**: Proper error handling across thread boundaries

### Error Handling Patterns

#### Configuration Errors
- **Strict Mode**: Hard failures when security requirements not met
- **Validation**: Input validation with descriptive error messages
- **Resource Availability**: Clear messages about missing system features

#### Runtime Errors
- **Process Failures**: Detailed error context for process spawn failures
- **Timeout Handling**: Graceful handling of process termination
- **Resource Errors**: Proper handling of cgroup operation failures

## Security Features

### Process Isolation
- **Working Directory**: Confined execution environment
- **User/Group Isolation**: Privilege dropping when configured
- **Resource Limits**: Enforced through cgroups when available
- **Environment Control**: Controlled environment variable exposure

### Resource Protection
- **Memory Limits**: Hard memory limits via cgroups
- **CPU Limits**: CPU time and share limits
- **Process Limits**: Maximum process count enforcement
- **Timeout Protection**: Wall clock and CPU time limits

### Attack Surface Reduction
- **Minimal Privileges**: Drops privileges when possible
- **Controlled Execution**: Limited file system access
- **Resource Exhaustion**: Protection against resource-based attacks
- **Process Containment**: Isolated process execution environment

## Integration Points

### Cgroup Integration
- **Controller Management**: Lifecycle management of cgroup controllers
- **Resource Monitoring**: Real-time resource usage tracking
- **Limit Enforcement**: Hard limits through kernel mechanisms
- **Cleanup**: Proper cleanup of cgroup resources

### Configuration System
- **Flexible Limits**: All limits configurable through `IsolateConfig`
- **Default Handling**: Sensible defaults with override capabilities
- **Validation**: Input validation with user-friendly error messages
- **Environment Variables**: Custom environment variable support

## Performance Considerations

### Efficiency Optimizations
- **Concurrent Monitoring**: Parallel output collection and timeout monitoring
- **Resource Polling**: Efficient cgroup resource monitoring
- **Memory Management**: Careful handling of output buffer sizes
- **Thread Management**: Minimal thread creation and cleanup overhead

### Scalability Features
- **Resource Isolation**: Per-instance resource tracking
- **Cleanup Automation**: Automatic resource cleanup on drop
- **Error Recovery**: Graceful degradation when resources unavailable
- **System Integration**: Efficient use of kernel features when available