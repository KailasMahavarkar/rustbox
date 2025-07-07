# Cgroup Module Documentation

The `cgroup.rs` module provides Linux cgroup-v1 integration for resource control in the rustbox system.

## Overview

The cgroup module uses the `cgroups-rs` crate to interface with Linux cgroup subsystems for enforcing resource limits on isolated processes. It provides fine-grained control over memory, CPU, and process limits.

## Core Structure

### `CgroupController`

Main structure for managing cgroup resources.

```rust
pub struct CgroupController {
    cgroup: Cgroup,
    name: String,
}
```

The controller wraps the `cgroups-rs::Cgroup` and provides a simplified interface for common operations.

## Public Methods

### Constructor

#### `new(name: &str) -> Result<Self>`

Creates a new cgroup controller with the specified name.

```rust
use mini_isolate::cgroup::CgroupController;

let controller = CgroupController::new("isolate-01")?;
```

**Parameters:**
- `name`: Unique identifier for the cgroup

**Returns:**
- `Result<CgroupController>` - Success or cgroup creation error

### Resource Limits

#### `set_memory_limit(limit_bytes: u64) -> Result<()>`

Sets memory limit in bytes for the cgroup.

```rust
// Set 128MB memory limit
controller.set_memory_limit(128 * 1024 * 1024)?;
```

**Features:**
- Sets both memory and memory+swap limits
- Prevents swap usage to ensure accurate memory accounting
- Uses the `MemController` from cgroups-rs

#### `set_cpu_limit(cpu_shares: u64) -> Result<()>`

Sets CPU shares for the cgroup (relative CPU weight).

```rust
// Standard CPU shares (1024 is normal priority)
controller.set_cpu_limit(1024)?;
```

**Parameters:**
- `cpu_shares`: Relative CPU weight (default: 1024)

#### `set_process_limit(limit: u64) -> Result<()>`

Sets maximum number of processes/tasks in the cgroup.

```rust
// Limit to 5 processes
controller.set_process_limit(5)?;
```

### Process Management

#### `add_process(pid: u32) -> Result<()>`

Adds a process to the cgroup for resource tracking.

```rust
let child = Command::new("./program").spawn()?;
controller.add_process(child.id())?;
```

**Important:** This must be called immediately after process creation for effective resource control.

### Resource Monitoring



#### `get_peak_memory_usage() -> Result<u64>`

Get peak memory usage since cgroup creation.

```rust
let peak_memory = controller.get_peak_memory_usage()?;
println!("Peak memory: {} KB", peak_memory / 1024);
```

#### `get_cpu_usage() -> Result<f64>`

Get cumulative CPU time usage in seconds.

```rust
let cpu_time = controller.get_cpu_usage()?;
println!("CPU time: {:.3} seconds", cpu_time);
```



### Cleanup

#### `cleanup() -> Result<()>`

Remove all tasks and delete the cgroup.

```rust
controller.cleanup()?;
```

**Note:** This is automatically called when the controller is dropped.

## Utility Functions

### `cgroups_available() -> bool`

Check if cgroups are available on the system.

```rust
if !cgroups_available() {
    eprintln!("Cgroups not supported on this system");
    return;
}
```

### `get_cgroup_mount() -> Result<String>`

Get the cgroup mount point (typically `/sys/fs/cgroup`).

```rust
let mount_point = get_cgroup_mount()?;
println!("Cgroups mounted at: {}", mount_point);
```

## Error Handling

The module uses the `IsolateError::Cgroup` variant for all cgroup-related errors:

```rust
use mini_isolate::types::{IsolateError, Result};

match controller.set_memory_limit(limit) {
    Ok(()) => println!("Memory limit set successfully"),
    Err(IsolateError::Cgroup(msg)) => eprintln!("Cgroup error: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Usage Examples

### Basic Resource Control

```rust
use mini_isolate::cgroup::CgroupController;
use std::process::Command;

// Create controller
let controller = CgroupController::new("my-isolate")?;

// Set limits
controller.set_memory_limit(64 * 1024 * 1024)?; // 64MB
controller.set_process_limit(1)?;
controller.set_cpu_limit(512)?; // Half normal priority

// Start process
let mut child = Command::new("./my-program").spawn()?;
controller.add_process(child.id())?;

// Wait and monitor
let status = child.wait()?;
let peak_memory = controller.get_peak_memory_usage()?;
let cpu_time = controller.get_cpu_usage()?;

println!("Exit status: {:?}", status);
println!("Peak memory: {} KB", peak_memory / 1024);
println!("CPU time: {:.3}s", cpu_time);

// Cleanup
controller.cleanup()?;
```

### Memory Limit Monitoring

```rust
use std::time::Duration;
use std::thread;

let controller = CgroupController::new("memory-test")?;
controller.set_memory_limit(32 * 1024 * 1024)?; // 32MB

let mut child = Command::new("memory-hungry-program").spawn()?;
controller.add_process(child.id())?;

// Monitor memory usage
loop {
    match child.try_wait() {
        Ok(Some(status)) => {
            let peak = controller.get_peak_memory_usage()?;
            println!("Process finished. Peak memory: {} KB", peak / 1024);
            break;
        },
        Ok(None) => {
            // Process still running
            let current = controller.get_peak_memory_usage()?;
            println!("Current memory: {} KB", current / 1024);
            
            let peak = controller.get_peak_memory_usage()?;
            println!("Peak memory: {} KB", peak / 1024);
            
            thread::sleep(Duration::from_millis(100));
        },
        Err(e) => {
            eprintln!("Error waiting for process: {}", e);
            break;
        }
    }
}
```

## System Requirements

### Cgroup v1 Support

The module requires cgroup v1 to be available:

```bash
# Check if cgroups are mounted
mount | grep cgroup

# Typical cgroup v1 mount
/sys/fs/cgroup/memory on /sys/fs/cgroup/memory type cgroup (rw,nosuid,nodev,noexec,relatime,memory)
```

### Required Controllers

The following cgroup controllers should be available:

- **memory**: Memory usage control
- **cpu**: CPU shares control  
- **pids**: Process/task limit control

Check availability:

```bash
cat /proc/cgroups
# Should show: memory, cpu, pids subsystems
```

### Permissions

Cgroup operations typically require root privileges or proper permissions:

```bash
# Run with elevated privileges
sudo ./rustbox init --box-id 0

# Or configure cgroup permissions for user
sudo chown -R user:user /sys/fs/cgroup/memory/user/
```

## Implementation Notes

### Drop Behavior

The `CgroupController` implements `Drop` to automatically clean up:

```rust
impl Drop for CgroupController {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
```

### Thread Safety

The controller is not thread-safe. Use separate controllers for concurrent operations or implement proper synchronization.

### Performance Considerations

- Cgroup operations involve filesystem I/O
- Frequent monitoring can impact performance
- Batch operations when possible

## Future Enhancements

Planned improvements for the cgroup module:

1. **Cgroup v2 Support**: When rust ecosystem adds support
2. **Network Control**: Bandwidth and connection limits
3. **I/O Limits**: Disk I/O bandwidth and IOPS limits
4. **CPU Quota**: Absolute CPU time limits vs. relative shares
5. **Freezer Support**: Pause/resume process execution