# main.rs - Application Entry Point

## Overview
The main entry point for the Mini-Isolate application. This file contains the primary executable logic that initializes the system, performs platform and permission checks, and launches the CLI interface.

## File Location
`src/main.rs`

## Purpose
- Application bootstrapping and initialization
- System compatibility and permission verification
- Environment setup and validation
- CLI interface launching

## Functions

### `fn main() -> Result<()>`
**Location**: `src/main.rs:11`

The main application entry point that performs system initialization and launches the CLI.

#### Functionality
1. **Logging Initialization**
   - Initializes the `env_logger` for runtime logging
   - Enables debug and error tracking throughout the application

2. **Platform Compatibility Check**
   - Verifies the application is running on a Unix-like system
   - Exits with error code 1 if running on unsupported platforms (Windows, etc.)
   - Ensures compatibility with Linux-specific features (cgroups, process management)

3. **Permission Verification**
   - Checks if the application is running with root privileges using `libc::getuid()`
   - Issues warnings when running without root access
   - Explains that some features (cgroups, resource limits) may not work without proper permissions

4. **Cgroup Availability Check**
   - Calls `crate::cgroup::cgroups_available()` to verify cgroup support
   - Warns users when cgroups are not available
   - Provides guidance on ensuring `/proc/cgroups` and `/sys/fs/cgroup` availability

5. **CLI Launch**
   - Calls `cli::run()` to start the command-line interface
   - Passes control to the CLI module for user interaction

#### Return Value
- `anyhow::Result<()>` - Returns success or propagates errors from CLI execution

#### Error Handling
- Platform incompatibility: Immediate exit with descriptive error message
- Permission issues: Warning messages but continues execution
- Cgroup unavailability: Warning messages but continues execution
- CLI errors: Propagated up the call stack

## Dependencies

### External Crates
- `anyhow`: For error handling and result types
- `env_logger`: For runtime logging initialization
- `libc`: For low-level system calls (getuid check)

### Internal Modules
- `crate::cgroup`: For cgroup availability checking
- `crate::cli`: For command-line interface execution

## System Requirements Check

### Platform Requirements
- **Required**: Unix-like operating system (Linux preferred)
- **Reason**: Relies on Unix-specific process management and cgroup features
- **Fallback**: None - application exits on unsupported platforms

### Permission Requirements
- **Recommended**: Root privileges (UID 0)
- **Required Features**: Cgroup management, full resource limit enforcement
- **Fallback**: Limited functionality without root access

### Cgroup Requirements
- **Required Files**: `/proc/cgroups`, `/sys/fs/cgroup`
- **Purpose**: Resource limit enforcement (memory, CPU, process limits)
- **Fallback**: Resource limits disabled, execution continues

## Security Considerations

### Privilege Handling
- Application checks for root privileges but doesn't enforce them
- Provides clear warnings about functionality limitations
- Allows users to make informed decisions about execution context

### System Validation
- Validates system compatibility before proceeding
- Ensures required system features are available
- Provides clear error messages for troubleshooting

## Usage Flow
1. User executes the binary
2. System performs compatibility checks
3. Warnings are displayed for any limitations
4. CLI interface is launched for user commands
5. Application exits based on CLI command results

## Integration Points
- **CLI Module**: Primary integration point for user interaction
- **Cgroup Module**: System capability validation
- **Logging System**: Runtime debugging and error tracking