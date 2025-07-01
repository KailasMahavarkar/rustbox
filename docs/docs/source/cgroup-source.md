# cgroup.rs - Cgroup Management for Resource Control

## Overview
Simplified cgroup (Control Groups) implementation for managing system resources and enforcing limits on isolated processes. Provides Linux kernel-level resource control through the cgroup v1 interface.

## File Location
`src/cgroup.rs`

## Purpose
- Manage Linux cgroups for process resource control
- Enforce memory, CPU, and process limits
- Provide resource usage monitoring
- Handle cgroup lifecycle management

## Dependencies
- `crate::types`: For error handling types
- `std::path::Path`: For file system path operations
- `std::fs`: For cgroup file system operations

## Core Structure

### `struct CgroupController`
**Location**: `src/cgroup.rs:7-10`

Simplified cgroup controller for managing process resources through the Linux cgroup interface.

#### Fields
- **`name: String`** - Unique name for this cgroup instance
- **`cgroup_path: std::path::PathBuf`** - Path to the cgroup directory in sysfs

#### Responsibilities
- Cgroup creation and management
- Resource limit enforcement
- Process assignment to cgroups
- Resource usage monitoring

## Constructor and Setup

### `pub fn new(name: &str, strict_mode: bool) -> Result<Self>`
**Location**: `src/cgroup.rs:13-45`

Creates a new cgroup controller with the specified name and behavior mode.

#### Functionality
1. **Cgroup Path Setup**
   - Creates path under `/sys/fs/cgroup/memory/{name}`
   - Uses memory cgroup as primary controller

2. **Directory Creation**
   - Attempts to create cgroup directory structure
   - Handles permission denied errors based on strict mode

3. **Error Handling Strategy**
   - **Strict Mode**: Returns error if cgroup creation fails
   - **Non-Strict Mode**: Issues warnings but continues execution
   - **Permission Errors**: Provides clear guidance about sudo requirements

4. **Graceful Degradation**
   - Allows operation without cgroups in non-strict mode
   - Provides clear warnings about missing functionality

#### Parameters
- **`name: &str`** - Unique identifier for this cgroup
- **`strict_mode: bool`** - Whether to enforce cgroup requirements

#### Returns
- `Result<Self>` - New cgroup controller or configuration error

#### Error Messages
- Permission denied: Suggests running with sudo or removing strict mode
- General failures: Detailed error context for troubleshooting

## Resource Limit Management

### `pub fn set_memory_limit(&self, limit_bytes: u64) -> Result<()>`
**Location**: `src/cgroup.rs:47-53`

Sets memory limit for processes in this cgroup.

#### Functionality
1. **Memory Limit Enforcement**
   - Writes to `memory.limit_in_bytes` file
   - Sets hard limit on memory usage

2. **Swap Control**
   - Also sets `memory.memsw.limit_in_bytes`
   - Prevents swap usage to enforce true memory limits

3. **Error Handling**
   - Gracefully handles file write failures
   - Continues operation if limit setting fails

#### Parameters
- **`limit_bytes: u64`** - Memory limit in bytes

#### Implementation Details
- Uses direct file writes to cgroup interface
- Handles both memory and memory+swap limits
- Provides hard limits enforced by kernel

### `pub fn set_cpu_limit(&self, cpu_shares: u64) -> Result<()>`
**Location**: `src/cgroup.rs:55-64`

Sets CPU resource allocation for processes in this cgroup.

#### Functionality
1. **CPU Cgroup Setup**
   - Creates directory under `/sys/fs/cgroup/cpu/{name}`
   - Ensures CPU controller is available

2. **CPU Shares Configuration**
   - Writes to `cpu.shares` file
   - Sets relative CPU allocation weight

3. **Resource Allocation**
   - Uses proportional CPU sharing
   - Default value of 1024 represents normal priority

#### Parameters
- **`cpu_shares: u64`** - CPU shares allocation (relative weight)

#### Implementation Details
- Separate cgroup hierarchy for CPU control
- Relative allocation rather than absolute limits
- Standard Linux CPU shares mechanism

### `pub fn set_process_limit(&self, limit: u64) -> Result<()>`
**Location**: `src/cgroup.rs:66-75`

Sets maximum number of processes/tasks allowed in this cgroup.

#### Functionality
1. **PIDs Cgroup Setup**
   - Creates directory under `/sys/fs/cgroup/pids/{name}`
   - Ensures PIDs controller is available

2. **Process Limit Configuration**
   - Writes to `pids.max` file
   - Sets hard limit on process count

3. **Fork Bomb Protection**
   - Prevents runaway process creation
   - Enforces maximum concurrent processes

#### Parameters
- **`limit: u64`** - Maximum number of processes allowed

#### Implementation Details
- Uses PIDs cgroup controller
- Hard limit enforced by kernel
- Prevents process explosion attacks

## Process Management

### `pub fn add_process(&self, pid: u32) -> Result<()>`
**Location**: `src/cgroup.rs:77-95`

Adds a process to this cgroup for resource control.

#### Functionality
1. **Memory Cgroup Assignment**
   - Writes PID to memory cgroup `tasks` file
   - Enrolls process in memory limit enforcement

2. **CPU Cgroup Assignment**
   - Adds process to CPU cgroup if available
   - Enables CPU resource control

3. **PIDs Cgroup Assignment**
   - Adds process to PIDs cgroup if available
   - Enrolls in process limit enforcement

4. **Multi-Controller Support**
   - Handles multiple cgroup controllers simultaneously
   - Gracefully handles missing controllers

#### Parameters
- **`pid: u32`** - Process ID to add to cgroup

#### Implementation Details
- Writes to multiple cgroup hierarchies
- Handles missing controllers gracefully
- Ensures process is controlled by all available limits

## Resource Monitoring

### `pub fn get_peak_memory_usage(&self) -> Result<u64>`
**Location**: `src/cgroup.rs:99-104`

Retrieves peak memory usage for processes in this cgroup.

#### Functionality
1. **Memory Statistics Reading**
   - Reads from `memory.max_usage_in_bytes` file
   - Gets peak memory usage since cgroup creation

2. **Data Parsing**
   - Converts string data to numeric bytes
   - Handles parsing errors with detailed context

#### Returns
- `Result<u64>` - Peak memory usage in bytes or parsing error

#### Implementation Details
- Uses cgroup memory statistics
- Provides peak usage rather than current usage
- Accurate tracking of maximum memory footprint

### `pub fn get_cpu_usage(&self) -> Result<f64>`
**Location**: `src/cgroup.rs:106-122`

Retrieves CPU usage statistics for processes in this cgroup.

#### Functionality
1. **CPU Statistics Reading**
   - Reads from `cpuacct.usage` file under CPU cgroup
   - Gets cumulative CPU time in nanoseconds

2. **Unit Conversion**
   - Converts nanoseconds to seconds
   - Provides floating-point seconds for easy use

3. **Availability Checking**
   - Checks if CPU accounting is available
   - Returns 0.0 if CPU statistics unavailable

#### Returns
- `Result<f64>` - CPU time used in seconds or parsing error

#### Implementation Details
- Uses CPU accounting cgroup controller
- Provides cumulative CPU time
- Handles missing CPU accounting gracefully

## Cleanup and Resource Management

### `pub fn cleanup(&self) -> Result<()>`
**Location**: `src/cgroup.rs:126-146`

Removes cgroup directories and cleans up resources.

#### Functionality
1. **Multi-Hierarchy Cleanup**
   - Removes directories from all cgroup hierarchies
   - Handles memory, CPU, PIDs, and cpuacct controllers

2. **Directory Removal**
   - Attempts to remove each cgroup directory
   - Handles missing directories gracefully

3. **Resource Cleanup**
   - Ensures kernel resources are freed
   - Prevents resource leaks

#### Cleanup Targets
- Memory cgroup directory
- CPU cgroup directory  
- PIDs cgroup directory
- CPU accounting directories

#### Implementation Details
- Must be called when cgroup is no longer needed
- Automatically called on `Drop`
- Handles cleanup failures gracefully

### `impl Drop for CgroupController`
**Location**: `src/cgroup.rs:165-169`

Automatic cleanup when cgroup controller is dropped.

#### Functionality
- Calls cleanup method automatically
- Ensures resources are freed even if manual cleanup is forgotten
- Prevents resource leaks through RAII pattern

## Utility Functions

### `fn write_cgroup_file(&self, filename: &str, content: &str) -> Result<()>`
**Location**: `src/cgroup.rs:150-155`

Helper function for writing to cgroup control files.

#### Functionality
- Constructs full path to cgroup file
- Writes content to cgroup control file
- Provides error context for debugging

#### Parameters
- **`filename: &str`** - Name of cgroup control file
- **`content: &str`** - Content to write to file

### `fn read_cgroup_file(&self, filename: &str) -> Result<String>`
**Location**: `src/cgroup.rs:157-162`

Helper function for reading from cgroup control files.

#### Functionality
- Constructs full path to cgroup file
- Reads content from cgroup control file
- Provides error context for debugging

#### Parameters
- **`filename: &str`** - Name of cgroup control file

#### Returns
- `Result<String>` - File content or I/O error

## System Integration Functions

### `pub fn cgroups_available() -> bool`
**Location**: `src/cgroup.rs:172-176`

Checks if cgroups are available on the current system.

#### Functionality
1. **Kernel Support Check**
   - Verifies `/proc/cgroups` exists
   - Confirms kernel cgroup support

2. **Mount Point Check**
   - Verifies `/sys/fs/cgroup` exists
   - Confirms cgroup filesystem is mounted

#### Returns
- `bool` - True if cgroups are available and mounted

#### Usage
- Called during system initialization
- Used for feature detection and capability checking
- Enables graceful degradation when cgroups unavailable

### `pub fn get_cgroup_mount() -> Result<String>`
**Location**: `src/cgroup.rs:178-186`

Gets the cgroup mount point path.

#### Functionality
1. **Availability Check**
   - Calls `cgroups_available()` first
   - Returns error if cgroups not available

2. **Mount Point Detection**
   - Returns standard cgroup v1 mount point
   - Currently hardcoded to `/sys/fs/cgroup`

#### Returns
- `Result<String>` - Cgroup mount point path or error

#### Implementation Details
- Assumes standard cgroup v1 layout
- Could be extended for cgroup v2 support
- Provides consistent mount point detection

## Error Handling Strategy

### Permission Handling
- **Strict Mode**: Hard failures when permissions insufficient
- **Non-Strict Mode**: Warnings with continued execution
- **User Guidance**: Clear messages about privilege requirements

### Resource Availability
- **Missing Controllers**: Graceful handling of unavailable cgroup controllers
- **File Operations**: Detailed error context for cgroup file operations
- **System Compatibility**: Clear detection of cgroup support

### Cleanup Robustness
- **Automatic Cleanup**: RAII pattern ensures cleanup on drop
- **Error Tolerance**: Cleanup continues even with individual failures
- **Resource Leak Prevention**: Multiple cleanup opportunities

## Security Considerations

### Privilege Requirements
- **Root Access**: Most cgroup operations require root privileges
- **Permission Checking**: Clear error messages for permission issues
- **Security Boundaries**: Proper isolation between different cgroup instances

### Resource Isolation
- **Process Containment**: Proper assignment of processes to cgroups
- **Resource Limits**: Enforced at kernel level for security
- **Namespace Isolation**: Each cgroup instance has unique namespace

### Attack Prevention
- **Resource Exhaustion**: Memory and process limits prevent resource attacks
- **Fork Bombs**: Process limits prevent process explosion attacks
- **CPU Starvation**: CPU shares prevent CPU monopolization

## Performance Characteristics

### Efficiency Features
- **Direct File I/O**: Minimal overhead for cgroup operations
- **Batch Operations**: Efficient setup of multiple limits
- **Lazy Evaluation**: Operations performed only when needed

### Scalability Considerations
- **Multiple Controllers**: Support for multiple cgroup hierarchies
- **Resource Monitoring**: Efficient access to kernel statistics
- **Cleanup Automation**: Prevents resource accumulation over time

## Linux Cgroup Integration

### Cgroup v1 Support
- **Multiple Hierarchies**: Separate hierarchies for different resource types
- **Standard Interface**: Uses standard Linux cgroup v1 interface
- **Kernel Integration**: Direct integration with kernel resource management

### Controller Support
- **Memory Controller**: Memory limit and usage tracking
- **CPU Controller**: CPU shares and usage tracking  
- **PIDs Controller**: Process limit and tracking
- **CPU Accounting**: CPU time accounting and statistics

### File System Interface
- **Sysfs Integration**: Uses `/sys/fs/cgroup` interface
- **Control Files**: Standard cgroup control file interface
- **Statistics Files**: Access to kernel-maintained statistics