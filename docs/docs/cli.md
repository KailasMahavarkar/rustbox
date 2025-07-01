# CLI Module Documentation

The `cli.rs` module provides the command-line interface for Mini-Isolate, designed to be compatible with IOI Isolate command patterns.

## Overview

The CLI module implements a comprehensive command-line interface that closely follows the IOI Isolate command structure while adding modern features like JSON output and verbose logging.

## Command Structure

```
mini-isolate <COMMAND> [OPTIONS]
```

## Commands

### `init` - Initialize Isolate Instance

Creates a new isolate instance with specified resource limits.

```bash
mini-isolate init [OPTIONS]
```

#### Options

- `--box-id <ID>` - Instance identifier (default: "0")
- `--dir <PATH>` - Working directory for the isolate
- `--mem <MB>` - Memory limit in megabytes (default: 128)
- `--time <SECONDS>` - CPU time limit in seconds (default: 10)
- `--wall-time <SECONDS>` - Wall clock time limit (default: 2x time limit)
- `--processes <NUM>` - Process limit (default: 1)
- `--fsize <MB>` - File size limit in megabytes (default: 64)

#### Examples

```bash
# Basic initialization
mini-isolate init --box-id 0

# Custom limits
mini-isolate init --box-id contest01 --mem 256 --time 30 --processes 5

# Specific working directory
mini-isolate init --box-id test --dir /tmp/my-isolate --mem 64 --time 5
```

### `run` - Execute Command

Runs a command or program in the specified isolate instance.

```bash
mini-isolate run [OPTIONS] <PROGRAM> [ARGS...]
```

#### Options

- `--box-id <ID>` - Instance identifier (default: "0")
- `--input <FILE>` - Input file (stdin redirection)
- `--output <FILE>` - Output JSON results to file
- `--verbose` - Verbose output including stdout/stderr

#### Arguments

- `<PROGRAM>` - Program or command to execute
- `[ARGS...]` - Arguments for the program

#### Examples

```bash
# Simple command execution
mini-isolate run --box-id 0 "echo Hello World"

# Execute with arguments
mini-isolate run --box-id 0 "/usr/bin/python3" "-c" "print('Hello')"

# With input file
mini-isolate run --box-id 0 --input input.txt "./my-program"

# Save results to JSON
mini-isolate run --box-id 0 --output result.json "./my-program"

# Verbose output
mini-isolate run --box-id 0 --verbose "./my-program"
```

### `execute` - Execute Source File

Executes a source file directly with automatic language detection and compilation.

```bash
mini-isolate execute [OPTIONS] --source <FILE>
```

#### Options

- `--box-id <ID>` - Instance identifier (default: "0")
- `--source <FILE>` - Source file to execute
- `--input <FILE>` - Input file (stdin redirection)
- `--output <FILE>` - Output JSON results to file
- `--verbose` - Verbose output including stdout/stderr

#### Supported Languages

The system automatically detects language based on file extension:

- **Python**: `.py` → `python3 filename.py`
- **JavaScript**: `.js` → `node filename.js`
- **C**: `.c` → `gcc -o main filename.c && ./main`
- **C++**: `.cpp`, `.cc`, `.cxx` → `g++ -o main filename.cpp && ./main`
- **Rust**: `.rs` → `rustc -o main filename.rs && ./main`
- **Go**: `.go` → `go run filename.go`
- **Java**: `.java` → `javac filename.java && java classname`

#### Examples

```bash
# Execute Python script
mini-isolate execute --box-id 0 --source hello.py

# Execute with input
mini-isolate execute --box-id 0 --source solution.cpp --input test.txt

# Save detailed results
mini-isolate execute --box-id 0 --source program.c --output result.json --verbose
```

### `list` - List Instances

Lists all available isolate instances.

```bash
mini-isolate list
```

#### Output

```
Available isolate instances:
  0 - /tmp/mini-isolate/0
  contest01 - /tmp/mini-isolate/contest01
  test - /tmp/my-isolate
```

### `cleanup` - Clean Up Resources

Removes isolate instances and cleans up resources.

```bash
mini-isolate cleanup [OPTIONS]
```

#### Options

- `--box-id <ID>` - Specific instance to clean up
- `--all` - Clean up all instances

#### Examples

```bash
# Clean specific instance
mini-isolate cleanup --box-id 0

# Clean all instances
mini-isolate cleanup --all
```

### `info` - System Information

Displays system information and capabilities.

```bash
mini-isolate info [OPTIONS]
```

#### Options

- `--cgroups` - Show detailed cgroup information

#### Example Output

```
Mini-Isolate System Information
==============================
Cgroups: Available
Cgroup mount: /sys/fs/cgroup

System Information:
Platform: linux
Architecture: x86_64
Active instances: 2
```

## Exit Codes

Mini-Isolate uses specific exit codes to indicate execution results:

- **0**: Success
- **1**: Runtime error or general failure
- **2**: Time limit exceeded
- **3**: Memory limit exceeded
- **4**: Security violation
- **5**: Internal error

## Output Formats

### Standard Output

Default human-readable output:

```
Status: Success
Exit code: 0
Time: 1.234s (wall), 0.567s (CPU)
Memory peak: 2048 KB

--- STDOUT ---
Hello, World!
```

### JSON Output

Structured output when using `--output`:

```json
{
  "exit_code": 0,
  "status": "Success",
  "stdout": "Hello, World!\n",
  "stderr": "",
  "cpu_time": 0.567,
  "wall_time": 1.234,
  "memory_peak": 2097152,
  "signal": null,
  "success": true,
  "error_message": null
}
```

### Verbose Mode

When `--verbose` is enabled, always shows stdout/stderr even on success:

```
Status: Success
Exit code: 0
Time: 1.234s (wall), 0.567s (CPU)
Memory peak: 2048 KB

--- STDOUT ---
Hello, World!

--- STDERR ---
Debug: Starting execution
```

## Implementation Details

### CLI Structure

The CLI is built using the `clap` crate with derive macros:

```rust
#[derive(Parser)]
#[command(name = "mini-isolate")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init { /* ... */ },
    Run { /* ... */ },
    Execute { /* ... */ },
    List,
    Cleanup { /* ... */ },
    Info { /* ... */ },
}
```

### Error Handling

The CLI uses `anyhow` for error handling and provides user-friendly error messages:

```rust
pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // Command processing...
}
```

### Integration with Core Modules

The CLI interfaces with core modules:

- **Isolate**: For instance management
- **Types**: For configuration and results
- **Cgroup**: For system capability checking

## Usage Patterns

### Contest Environment Setup

```bash
# Setup multiple isolate instances for contest
for i in {0..9}; do
    mini-isolate init --box-id $i --mem 256 --time 60 --processes 10
done

# Execute solutions
mini-isolate execute --box-id 0 --source solution1.cpp --input test1.txt
mini-isolate execute --box-id 1 --source solution2.py --input test2.txt

# Cleanup after contest
mini-isolate cleanup --all
```

### Automated Testing

```bash
#!/bin/bash
# Test script with JSON output

mini-isolate init --box-id test --mem 128 --time 10

for test_file in tests/*.txt; do
    result_file="results/$(basename "$test_file" .txt).json"
    mini-isolate execute --box-id test --source program.cpp \
        --input "$test_file" --output "$result_file"
    
    # Check if successful
    if [ $? -eq 0 ]; then
        echo "Test $(basename "$test_file") passed"
    else
        echo "Test $(basename "$test_file") failed with exit code $?"
    fi
done

mini-isolate cleanup --box-id test
```

### Development Workflow

```bash
# Development cycle
mini-isolate init --box-id dev --mem 512 --time 30

# Test during development
mini-isolate execute --box-id dev --source main.rs --verbose

# Quick execution
mini-isolate run --box-id dev "./target/release/my-program"

# Cleanup
mini-isolate cleanup --box-id dev
```

## Comparison with IOI Isolate

| Feature | IOI Isolate | Mini-Isolate |
|---------|-------------|--------------|
| Box ID | `--box-id` | `--box-id` |
| Memory Limit | `--mem` | `--mem` |
| Time Limit | `--time` | `--time` |
| Wall Time | `--wall-time` | `--wall-time` |
| Process Limit | `--processes` | `--processes` |
| File Size | `--fsize` | `--fsize` |
| Cleanup | `--cleanup` | `cleanup` subcommand |
| Init | `--init` | `init` subcommand |
| Run | `--run` | `run` / `execute` subcommands |
| JSON Meta | `--meta` file | `--output` option |
| Language Detection | Manual | Automatic |

## Best Practices

1. **Always initialize** before running programs
2. **Use specific box-ids** for different use cases
3. **Set appropriate limits** based on expected resource usage
4. **Use JSON output** for automated processing
5. **Clean up** instances when done
6. **Check system info** before deployment
7. **Use verbose mode** for debugging