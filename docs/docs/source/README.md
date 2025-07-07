# rustbox Source Code Documentation

## Overview
This directory contains comprehensive technical documentation for each source file in the rustbox project. Each markdown file provides detailed explanations of the code structure, functions, and implementation details.

> **Note**: This documentation is automatically generated from source code analysis and provides developer-level technical details. For user documentation, see the [main docs](../README.md).

## Documentation Structure

### Core Files Documentation

#### [lib.md](./lib.md) - Library Root Module
- **File**: `src/lib.rs`
- **Purpose**: Root library module defining the project structure
- **Key Content**: Module organization, public API structure, architecture overview

#### [main.md](./main.md) - Application Entry Point  
- **File**: `src/main.rs`
- **Purpose**: Main application bootstrap and initialization
- **Key Content**: System checks, platform validation, CLI launching

#### [types-source.md](./types-source.md) - Core Types and Data Structures
- **File**: `src/types.rs`
- **Purpose**: Fundamental data structures and type definitions
- **Key Content**: `IsolateConfig`, `ExecutionResult`, `ExecutionStatus`, error types

#### [cli-source.md](./cli-source.md) - Command Line Interface
- **File**: `src/cli.rs`
- **Purpose**: User-facing command-line interface implementation
- **Key Content**: Command parsing, argument handling, result presentation

#### [executor.md](./executor.md) - Process Execution Engine
- **File**: `src/executor.rs`
- **Purpose**: Low-level process execution and monitoring
- **Key Content**: `ProcessExecutor`, resource monitoring, timeout handling

#### [isolate.md](./isolate.md) - High-Level Isolation API
- **File**: `src/isolate.rs`
- **Purpose**: Main isolation management interface
- **Key Content**: `Isolate` struct, instance management, multi-language support

#### [cgroup-source.md](./cgroup-source.md) - Linux Cgroup Integration
- **File**: `src/cgroup.rs`
- **Purpose**: Linux cgroup resource control implementation
- **Key Content**: `CgroupController`, resource limits, monitoring

## Project Architecture

### Module Dependencies
```
main.rs (entry point)
├── cli.rs (depends on isolate, types)
├── isolate.rs (depends on executor, types)
├── executor.rs (depends on cgroup, types)
├── cgroup.rs (depends on types)
└── types.rs (foundational)
```

### Data Flow
1. **User Input**: CLI parses user commands and arguments
2. **Configuration**: Commands create or load `IsolateConfig`
3. **Instance Management**: `Isolate` manages persistent instances
4. **Process Execution**: `ProcessExecutor` handles low-level execution
5. **Resource Control**: `CgroupController` enforces system limits
6. **Result Collection**: `ExecutionResult` captures comprehensive outcomes

## Key Concepts

### Process Isolation
- **Working Directory Isolation**: Each instance has dedicated workspace
- **Resource Limits**: Memory, CPU, time, and process count limits
- **User/Group Isolation**: Optional privilege dropping
- **Environment Control**: Custom environment variables

### Resource Management
- **Cgroup Integration**: Linux kernel-level resource control
- **Multi-Controller Support**: Memory, CPU, PIDs controllers
- **Resource Monitoring**: Real-time usage tracking
- **Graceful Degradation**: Continues operation without cgroups

### Multi-Language Support
- **Automatic Detection**: File extension-based language detection
- **Compilation Support**: C, C++, Rust, Java compilation
- **Runtime Execution**: Python, JavaScript, Go runtime execution
- **Shell Integration**: Bash script execution support

### Instance Persistence
- **Configuration Storage**: JSON-based instance persistence
- **Lifecycle Management**: Create, load, list, cleanup operations
- **Usage Tracking**: Creation and last-used timestamps
- **State Recovery**: Instances survive application restarts

## Security Features

### Process Containment
- **Filesystem Isolation**: Confined to working directory
- **Network Isolation**: Disabled by default
- **Resource Limits**: Hard limits enforced by kernel
- **Privilege Dropping**: Optional user/group ID changes

### Attack Prevention
- **Resource Exhaustion**: Memory and CPU limits prevent resource attacks
- **Fork Bombs**: Process limits prevent process explosion
- **Time Limits**: Wall clock and CPU time limits prevent infinite loops
- **File Size Limits**: Prevent disk space exhaustion

### Strict Mode
- **Security Enforcement**: Requires root privileges and cgroups
- **Fail-Safe Operation**: Hard failures when security requirements not met
- **Clear Feedback**: Detailed error messages for security violations

## Error Handling Strategy

### Structured Errors
- **`IsolateError` Enum**: Categorized error types (IO, Cgroup, Config, Process)
- **Error Context**: Detailed error messages with actionable information
- **Error Propagation**: Consistent error handling across modules

### Graceful Degradation
- **Non-Strict Mode**: Continues operation with warnings when possible
- **Feature Detection**: Automatically detects system capabilities
- **Fallback Behavior**: Provides reduced functionality when features unavailable

### User-Friendly Messages
- **Clear Guidance**: Error messages provide troubleshooting steps
- **Permission Errors**: Specific guidance about privilege requirements
- **System Compatibility**: Clear messages about platform requirements

## Performance Considerations

### Efficiency Optimizations
- **Concurrent Monitoring**: Parallel output collection and timeout monitoring
- **Resource Polling**: Efficient cgroup resource monitoring
- **Lazy Loading**: Instances and configurations loaded on-demand
- **Minimal Overhead**: Direct system call interface usage

### Scalability Features
- **Multiple Instances**: Support for numerous concurrent isolates
- **Resource Isolation**: Per-instance resource tracking
- **Cleanup Automation**: Automatic resource cleanup prevents leaks
- **Efficient Storage**: Minimal memory footprint for instance metadata

## Usage Patterns

### Basic Workflow
1. **Initialize**: `rustbox init --box-id mybox`
2. **Execute**: `rustbox run --box-id mybox program args`
3. **Monitor**: Results include resource usage and timing
4. **Cleanup**: `rustbox cleanup --box-id mybox`

### Advanced Features
- **Resource Overrides**: Runtime limit adjustments
- **File Execution**: Direct source file execution with compilation
- **JSON Output**: Machine-readable result format
- **Verbose Modes**: Detailed debugging information

### Integration Points
- **External Systems**: JSON output for automation
- **CI/CD Pipelines**: Exit codes for build integration
- **Educational Platforms**: Safe code execution environment
- **Contest Systems**: IOI-compatible execution model

## Development Guidelines

### Code Organization
- **Separation of Concerns**: Each module has distinct responsibilities
- **Error Handling**: Consistent error propagation and handling
- **Documentation**: Comprehensive inline and external documentation
- **Testing**: Modular design enables comprehensive testing

### Security Practices
- **Least Privilege**: Operations use minimal required privileges
- **Input Validation**: All user inputs validated before processing
- **Resource Limits**: All operations subject to configurable limits
- **Safe Defaults**: Secure defaults with opt-in for relaxed security

### Maintenance Considerations
- **Platform Compatibility**: Clear platform-specific code separation
- **Version Compatibility**: Stable API and configuration formats
- **Logging**: Comprehensive logging for debugging and monitoring
- **Cleanup**: Automatic resource cleanup prevents system pollution

## Future Enhancements

### Potential Improvements
- **Cgroup v2 Support**: Modern cgroup interface support
- **Container Integration**: Docker/Podman integration
- **Network Isolation**: Advanced network sandboxing
- **Seccomp Filters**: System call filtering for enhanced security

### Extensibility Points
- **Language Plugins**: Pluggable language execution support
- **Custom Limits**: Additional resource limit types
- **Monitoring Hooks**: Real-time monitoring callbacks
- **Result Processors**: Custom result processing pipelines

This knowledge base provides comprehensive technical documentation for understanding, maintaining, and extending the rustbox codebase. Each file contains detailed function-level documentation with implementation details, security considerations, and usage patterns.