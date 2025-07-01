# cli.rs - Command Line Interface Implementation

## Overview
Comprehensive command-line interface for the Mini-Isolate system, providing user-friendly access to all isolation and execution functionality. Implements argument parsing, command handling, and result presentation.

## File Location
`src/cli.rs`

## Purpose
- Provide intuitive command-line interface for process isolation
- Handle user input validation and processing
- Manage isolate instance lifecycle
- Present execution results in user-friendly formats

## Dependencies
- `clap`: For command-line argument parsing with derive macros
- `crate::isolate::Isolate`: For isolate instance management
- `crate::types`: For configuration and result types
- `std::path::PathBuf`: For file path handling
- `std::time::Duration`: For time-based parameters

## Core Structures

### `struct Cli`
**Location**: `src/cli.rs:8-14`

Main CLI structure using clap's derive interface for argument parsing.

#### Fields
- **`command: Commands`** - The subcommand to execute

#### Attributes
- `#[command(name = "mini-isolate")]` - Sets the program name
- `#[command(about = "...")]` - Provides help text description

### `enum Commands`
**Location**: `src/cli.rs:16-154`

Enumeration of all available CLI subcommands with their respective parameters.

## Commands

### `Init`
**Location**: `src/cli.rs:18-51`

Initialize a new isolate instance with specified resource limits and configuration.

#### Parameters
- **`box_id: String`** - Instance identifier (default: "0")
- **`dir: Option<PathBuf>`** - Working directory for the isolate
- **`mem: u64`** - Memory limit in MB (default: 128)
- **`time: u64`** - Time limit in seconds (default: 10)
- **`wall_time: Option<u64>`** - Wall clock time limit (default: 2x time limit)
- **`processes: u32`** - Process limit (default: 1)
- **`fsize: u64`** - File size limit in MB (default: 64)
- **`strict: bool`** - Strict mode flag

#### Functionality
- Creates a new `IsolateConfig` with specified parameters
- Sets up working directory (auto-generated if not specified)
- Converts user-friendly units (MB) to bytes for internal use
- Initializes and saves the isolate instance

### `Run`
**Location**: `src/cli.rs:53-93`

Execute a program within an existing isolate instance.

#### Parameters
- **`box_id: String`** - Instance identifier (default: "0")
- **`program: String`** - Program to execute (path to executable)
- **`args: Vec<String>`** - Arguments for the program
- **`input: Option<PathBuf>`** - Input file for stdin redirection
- **`output: Option<PathBuf>`** - Output file for JSON results
- **`verbose: bool`** - Verbose output flag
- **`max_cpu: Option<u64>`** - Override CPU time limit
- **`max_memory: Option<u64>`** - Override memory limit
- **`max_time: Option<u64>`** - Override execution time limit
- **`strict: bool`** - Strict mode flag

#### Functionality
- Loads existing isolate instance or exits with error
- Reads input data from file if specified
- Constructs command array from program and arguments
- Executes with optional resource overrides
- Handles result output (JSON file or console display)

### `Execute`
**Location**: `src/cli.rs:95-132`

Execute a source file directly with automatic language detection and compilation.

#### Parameters
- **`box_id: String`** - Instance identifier (default: "0")
- **`source: PathBuf`** - Source file to execute
- **`input: Option<PathBuf>`** - Input file for stdin
- **`output: Option<PathBuf>`** - Output file for JSON results
- **`verbose: bool`** - Verbose output flag
- **`max_cpu: Option<u64>`** - Override CPU time limit
- **`max_memory: Option<u64>`** - Override memory limit
- **`max_time: Option<u64>`** - Override execution time limit
- **`strict: bool`** - Strict mode flag

#### Functionality
- Similar to `Run` but handles source files with automatic compilation
- Detects language based on file extension
- Copies source file to isolate working directory
- Compiles and executes based on language type

### `List`
**Location**: `src/cli.rs:135`

List all available isolate instances.

#### Functionality
- Retrieves all configured isolate instances
- Displays instance IDs and working directories
- Shows "No instances found" when appropriate

### `Cleanup`
**Location**: `src/cli.rs:137-146`

Clean up isolate instance(s) and their associated resources.

#### Parameters
- **`box_id: Option<String>`** - Specific instance ID to clean
- **`all: bool`** - Clean all instances flag

#### Functionality
- Cleans specific instance if `box_id` provided
- Cleans all instances if `all` flag set
- Removes working directories and instance configurations
- Validates that either `box_id` or `all` is specified

### `Info`
**Location**: `src/cli.rs:148-154`

Display system information and capabilities.

#### Parameters
- **`cgroups: bool`** - Show detailed cgroup information

#### Functionality
- Shows cgroup availability and mount information
- Displays system platform and architecture
- Lists active isolate instances
- Shows detailed cgroup controller information if requested

## Core Functions

### `pub fn run() -> anyhow::Result<()>`
**Location**: `src/cli.rs:156-442`

Main CLI execution function that parses arguments and dispatches to appropriate handlers.

#### Command Handling

#### Init Command Handler
**Location**: `src/cli.rs:159-197`

- Creates `IsolateConfig` with user-specified parameters
- Sets default working directory if not provided
- Converts MB values to bytes for internal storage
- Creates and saves new isolate instance
- Displays confirmation message with instance location

#### Run Command Handler
**Location**: `src/cli.rs:199-285`

- Loads existing isolate instance or exits with error
- Applies strict mode override if specified
- Reads stdin data from input file if provided
- Constructs command array from program and arguments
- Executes with optional resource limit overrides
- Handles output formatting (JSON file or console)
- Sets appropriate exit codes based on execution status

#### Execute Command Handler
**Location**: `src/cli.rs:287-367`

- Similar flow to Run command
- Handles source file execution with language detection
- Copies source file to working directory
- Uses `execute_file_with_overrides` method
- Same output handling and exit code logic

#### List Command Handler
**Location**: `src/cli.rs:369-381`

- Retrieves all isolate instances using `Isolate::list_all()`
- Displays formatted list of instances with working directories
- Shows appropriate message when no instances exist

#### Cleanup Command Handler
**Location**: `src/cli.rs:383-405`

- Handles both single instance and bulk cleanup
- Validates input parameters (requires either `box_id` or `all`)
- Iterates through instances for bulk cleanup
- Provides feedback for each cleaned instance

#### Info Command Handler
**Location**: `src/cli.rs:407-438`

- Displays comprehensive system information
- Shows cgroup availability and mount points
- Presents platform and architecture details
- Lists count of active instances
- Shows detailed cgroup controller information if requested

## Result Processing and Output

### Console Output Format
**Location**: `src/cli.rs:252-273, 334-355`

Standard console output includes:
- **Execution Status**: Success, timeout, memory limit, etc.
- **Exit Code**: Process exit code if available
- **Timing Information**: Wall time and CPU time in seconds
- **Memory Usage**: Peak memory usage in KB
- **Output Streams**: STDOUT and STDERR (verbose mode or on failure)
- **Error Messages**: Additional error details when available

### JSON Output Format
**Location**: `src/cli.rs:246-249, 329-332`

- Complete `ExecutionResult` serialized as pretty-printed JSON
- Includes all execution metrics and output
- Written to user-specified output file
- Suitable for automated processing and integration

### Exit Code Mapping
**Location**: `src/cli.rs:275-284, 357-366`

- **0**: Success
- **1**: Runtime error (uses process exit code if available)
- **2**: Time limit exceeded
- **3**: Memory limit exceeded
- **4**: Security violation
- **5**: Internal error
- **Default**: 1 for other failures

## Input/Output Handling

### Stdin Redirection
**Location**: `src/cli.rs:227-232, 314-319`

- Reads entire input file into memory
- Passes as string to executor for stdin redirection
- Handles file read errors gracefully
- Optional feature (None if no input file specified)

### Error Handling Patterns
- **Instance Not Found**: Clear error message with suggestion to run `init`
- **Parameter Validation**: Clap handles most validation automatically
- **File Operations**: Proper error propagation with context
- **Resource Override Validation**: Applied at execution time

## Integration Points

### Isolate Instance Management
- **Creation**: `Isolate::new()` with configured parameters
- **Loading**: `Isolate::load()` with instance ID lookup
- **Listing**: `Isolate::list_all()` for instance enumeration
- **Execution**: `execute()` and `execute_with_overrides()` methods
- **Cleanup**: `cleanup()` method for resource cleanup

### Configuration Management
- **Parameter Conversion**: MB to bytes, seconds to Duration
- **Default Handling**: Sensible defaults with override capabilities
- **Validation**: Input validation through clap constraints
- **Persistence**: Automatic saving of instance configurations

## User Experience Features

### Help System
- Comprehensive help text for all commands and parameters
- Default values displayed in help output
- Clear parameter descriptions and constraints
- Command-specific help available via `--help`

### Error Messages
- Clear, actionable error messages
- Specific guidance for common issues (missing instances, permissions)
- Detailed system information for troubleshooting
- Appropriate exit codes for script integration

### Flexible Output Options
- Console output optimized for human readability
- JSON output for machine processing and integration
- Verbose mode for detailed debugging information
- Quiet operation when outputting to files

## Security Considerations
- **Strict Mode**: Configurable security enforcement
- **Resource Limits**: User-configurable with sensible defaults
- **File Access**: Controlled through working directory isolation
- **Privilege Requirements**: Clear warnings about permission needs