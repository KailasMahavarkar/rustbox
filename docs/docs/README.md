# rustbox Documentation

rustbox is a process isolation and resource control system inspired by the IOI Isolate project. It provides secure execution environments for running untrusted code with strict resource limits using cgroup-v1 support.

## Overview

rustbox focuses on:
- **Process Isolation**: Secure execution environments for untrusted programs
- **Resource Control**: Memory, CPU, time, and process limits using cgroups
- **Security**: Sandboxed execution with controlled system access
- **IOI Compatibility**: Commands and behavior similar to IOI Isolate

## Architecture

The system is designed with a modular architecture:

### Core Modules

1. **types.rs** - Core data structures and error types
2. **cgroup.rs** - Cgroup management for resource control
3. **executor.rs** - Process execution and monitoring
4. **isolate.rs** - Main isolate management interface
5. **cli.rs** - Command-line interface

### Key Features

- **Cgroup-v1 Support**: Full resource control using Linux cgroups
- **Memory Limits**: Strict memory usage enforcement
- **Time Limits**: CPU and wall-clock time restrictions
- **Process Limits**: Control over number of spawned processes
- **File Size Limits**: Restrict output file sizes
- **Multi-language Support**: Automatic detection and execution of various programming languages

## Getting Started

### Prerequisites

- Linux system with cgroup-v1 support
- Root privileges (recommended for full functionality)
- Rust toolchain

### Installation

```bash
cargo build --release
sudo cp target/release/rustbox /usr/local/bin/
```

### Basic Usage

1. **Initialize an isolate instance:**
```bash
rustbox init --box-id 0 --mem 128 --time 10 --processes 1
```

2. **Execute a program:**
```bash
rustbox run --box-id 0 "echo Hello World"
```

3. **Execute a source file:**
```bash
rustbox execute --box-id 0 --source hello.py
```

4. **Clean up:**
```bash
rustbox cleanup --box-id 0
```

## Documentation Structure

### User Documentation
- [Types](types.md) - Core data structures and types
- [Cgroup](cgroup.md) - Resource control implementation
- [CLI](cli.md) - Command-line interface reference
- [Examples](examples.md) - Usage examples and tutorials
- [Testing](testing.md) - Testing framework and procedures
- [IOI Compatibility](ioi-compatibility.md) - Compatibility notes with IOI Isolate
- [Resource Control Usage](resource-control-usage.md) - Resource control guide

### Source Code Documentation
- [Source Code Overview](source/README.md) - Comprehensive source code documentation
- [Library Root](source/lib.md) - `src/lib.rs` - Module organization and structure
- [Main Entry Point](source/main.md) - `src/main.rs` - Application bootstrap and initialization
- [Core Types Implementation](source/types-source.md) - `src/types.rs` - Data structures and type definitions
- [CLI Implementation](source/cli-source.md) - `src/cli.rs` - Command-line interface details
- [Process Executor](source/executor.md) - `src/executor.rs` - Process execution and monitoring
- [Isolate Manager](source/isolate.md) - `src/isolate.rs` - High-level isolation API
- [Cgroup Controller Implementation](source/cgroup-source.md) - `src/cgroup.rs` - Linux cgroup integration

## Differences from IOI Isolate

While inspired by IOI Isolate, rustbox has some differences:

1. **Cgroup Version**: Uses cgroup-v1 (Rust ecosystem limitation)
2. **Implementation**: Written in Rust for memory safety
3. **Configuration**: JSON-based persistent configuration
4. **API**: Modern CLI with better error reporting

## Contributing

See the individual module documentation for implementation details and contribution guidelines.

## License

This project is licensed under the MIT License.