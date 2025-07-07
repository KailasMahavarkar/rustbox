# lib.rs - Library Root Module

## Overview
The root library module for rustbox, a process isolation and resource control system inspired by IOI Isolate. This module serves as the main entry point for organizing the codebase and defines the public API structure.

## File Location
`src/lib.rs`

## Purpose
- Defines the module structure for the entire library
- Provides organizational structure for the codebase
- Serves as the main library entry point for external crates
- Focuses on secure process execution with cgroup-v1 support

## Module Declarations

### Public Modules

#### `pub mod types`
- **Purpose**: Core data structures and type definitions
- **Contains**: Configuration structs, execution results, error types
- **Exports**: All fundamental types used throughout the system

#### `pub mod cgroup`
- **Purpose**: Cgroup management and resource control
- **Contains**: CgroupController implementation, resource limit management
- **Exports**: Functions and types for managing Linux control groups

#### `pub mod executor`  
- **Purpose**: Process execution and monitoring
- **Contains**: ProcessExecutor implementation, process isolation logic
- **Exports**: Core execution functionality with resource monitoring

#### `pub mod isolate`
- **Purpose**: Main isolation management interface
- **Contains**: Isolate struct, instance management, high-level API
- **Exports**: Primary user-facing API for creating and managing isolated environments

#### `pub mod cli`
- **Purpose**: Command-line interface implementation
- **Contains**: CLI argument parsing, command handling, user interaction
- **Exports**: CLI functionality for the standalone executable

## Architecture Notes

### Design Philosophy
- **Security First**: Designed with process isolation as the primary concern
- **Resource Control**: Built around Linux cgroups for reliable resource management  
- **IOI Compatibility**: Inspired by and designed to be compatible with IOI Isolate workflows
- **Modular Design**: Clean separation of concerns across modules

### Module Dependencies
```
lib.rs (root)
├── types (foundational types)
├── cgroup (depends on types)
├── executor (depends on types, cgroup)
├── isolate (depends on types, executor)
└── cli (depends on types, isolate)
```

### Key Features Supported
- Process isolation with configurable resource limits
- Memory, CPU, and time constraints
- File system isolation options
- Multi-language execution support
- Persistent instance management
- Comprehensive error handling

## Usage Context
This module is typically used by:
- The main binary (`main.rs`) for CLI operations
- External crates that want to embed rustbox functionality
- Test suites that need access to the core library components

## Security Considerations
- All modules are designed with security isolation in mind
- Resource limits are enforced at the cgroup level when possible
- Process execution is sandboxed by default
- Strict mode requires elevated privileges for full functionality