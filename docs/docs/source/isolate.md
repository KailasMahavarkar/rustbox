# isolate.rs - Main Isolate Management Interface

## Overview
High-level isolate management interface that provides the primary API for creating, managing, and executing processes within isolated environments. Implements instance persistence, lifecycle management, and convenient execution methods.

## File Location
`src/isolate.rs`

## Purpose
- Provide high-level API for process isolation
- Manage persistent isolate instances
- Handle multi-language execution support
- Implement instance lifecycle management

## Dependencies
- `crate::executor::ProcessExecutor`: For low-level process execution
- `crate::types`: For configuration and result types
- `serde`: For instance serialization/deserialization
- `std::collections::HashMap`: For instance storage
- `chrono`: For timestamp management

## Core Structures

### `struct IsolateInstance`
**Location**: `src/isolate.rs:11-16`

Internal structure for persistent isolate instance storage.

#### Fields
- **`config: IsolateConfig`** - Complete isolate configuration
- **`created_at: chrono::DateTime<chrono::Utc>`** - Instance creation timestamp
- **`last_used: chrono::DateTime<chrono::Utc>`** - Last usage timestamp

#### Traits
- `Clone, Debug, Serialize, Deserialize` - For persistence and debugging

### `struct Isolate`
**Location**: `src/isolate.rs:19-22`

Main isolate manager for handling isolated execution environments.

#### Fields
- **`instance: IsolateInstance`** - Instance metadata and configuration
- **`base_path: PathBuf`** - Base directory for this isolate instance

#### Responsibilities
- Instance lifecycle management
- Process execution coordination
- Configuration persistence
- Multi-language execution support

## Instance Management

### `pub fn new(config: IsolateConfig) -> Result<Self>`
**Location**: `src/isolate.rs:25-48`

Creates a new isolate instance with the given configuration.

#### Functionality
1. **Directory Setup**
   - Creates base directory under `/tmp/mini-isolate/{instance_id}`
   - Ensures directory structure exists before proceeding

2. **Instance Creation**
   - Creates `IsolateInstance` with current timestamps
   - Sets up internal state tracking

3. **Configuration Persistence**
   - Automatically saves instance configuration to disk
   - Enables instance recovery across application restarts

#### Parameters
- **`config: IsolateConfig`** - Complete isolate configuration

#### Returns
- `Result<Self>` - New isolate instance or I/O error

### `pub fn load(instance_id: &str) -> Result<Option<Self>>`
**Location**: `src/isolate.rs:50-77`

Loads an existing isolate instance from persistent storage.

#### Functionality
1. **Configuration Loading**
   - Reads instances from `instances.json` file
   - Deserializes stored instance configurations

2. **Instance Validation**
   - Verifies instance exists in configuration
   - Checks that base directory still exists on disk

3. **Instance Reconstruction**
   - Rebuilds isolate instance from stored metadata
   - Restores complete state for continued use

#### Parameters
- **`instance_id: &str`** - Unique identifier for the instance

#### Returns
- `Result<Option<Self>>` - Loaded instance, None if not found, or I/O error

### `pub fn list_all() -> Result<Vec<String>>`
**Location**: `src/isolate.rs:79-83`

Lists all available isolate instances.

#### Functionality
- Loads all instance configurations from storage
- Returns list of instance IDs for enumeration
- Used by CLI for listing available instances

#### Returns
- `Result<Vec<String>>` - List of instance IDs or I/O error

## Process Execution

### `pub fn execute(&mut self, command: &[String], stdin_data: Option<&str>) -> Result<ExecutionResult>`
**Location**: `src/isolate.rs:85-96`

Executes a command within this isolate instance.

#### Functionality
1. **Usage Tracking**
   - Updates `last_used` timestamp
   - Persists updated metadata to disk

2. **Executor Creation**
   - Creates `ProcessExecutor` with current configuration
   - Delegates to low-level execution engine

3. **Result Processing**
   - Returns comprehensive execution results
   - Includes resource usage and output capture

#### Parameters
- **`command: &[String]`** - Command and arguments to execute
- **`stdin_data: Option<&str>`** - Optional stdin data

#### Returns
- `Result<ExecutionResult>` - Complete execution results

### `pub fn execute_with_overrides(...) -> Result<ExecutionResult>`
**Location**: `src/isolate.rs:98-132`

Executes a command with runtime resource limit overrides.

#### Functionality
1. **Configuration Cloning**
   - Creates modified copy of instance configuration
   - Applies runtime overrides to limits

2. **Override Processing**
   - **CPU Limits**: Converts seconds to Duration for both CPU and general time limits
   - **Memory Limits**: Converts MB to bytes for internal storage
   - **Time Limits**: Sets wall clock time limit

3. **Execution**
   - Creates new executor with modified configuration
   - Executes command with temporary overrides

#### Parameters
- **`command: &[String]`** - Command and arguments
- **`stdin_data: Option<&str>`** - Optional stdin data
- **`max_cpu: Option<u64>`** - CPU time override in seconds
- **`max_memory: Option<u64>`** - Memory override in MB
- **`max_time: Option<u64>`** - Wall time override in seconds

#### Returns
- `Result<ExecutionResult>` - Execution results with override limits applied

## File Execution Support

### `pub fn execute_file(&mut self, file_path: &Path, stdin_data: Option<&str>) -> Result<ExecutionResult>`
**Location**: `src/isolate.rs:134-151`

Executes a source file with automatic language detection and compilation.

#### Functionality
1. **File Validation**
   - Checks that source file exists
   - Validates file path structure

2. **File Staging**
   - Copies source file to isolate working directory
   - Preserves original filename for execution

3. **Command Generation**
   - Determines execution strategy based on file extension
   - Generates appropriate compilation and execution commands

4. **Execution**
   - Delegates to standard execute method
   - Returns standard execution results

#### Parameters
- **`file_path: &Path`** - Path to source file
- **`stdin_data: Option<&str>`** - Optional stdin data

#### Returns
- `Result<ExecutionResult>` - Execution results

### `pub fn execute_file_with_overrides(...) -> Result<ExecutionResult>`
**Location**: `src/isolate.rs:153-177`

Executes a source file with runtime resource overrides.

#### Functionality
- Combines file execution with resource override capabilities
- Same file staging and command generation as `execute_file`
- Applies resource overrides during execution

#### Parameters
- **`file_path: &Path`** - Path to source file
- **`stdin_data: Option<&str>`** - Optional stdin data
- **`max_cpu: Option<u64>`** - CPU time override
- **`max_memory: Option<u64>`** - Memory override
- **`max_time: Option<u64>`** - Wall time override

#### Returns
- `Result<ExecutionResult>` - Execution results with overrides

## Language Support

### `fn get_execution_command(&self, file_path: &Path) -> Result<Vec<String>>`
**Location**: `src/isolate.rs:179-237`

Determines execution command based on file extension with comprehensive language support.

#### Supported Languages

##### Python (.py)
- **Command**: `/usr/bin/python3 -u {filename}`
- **Features**: Unbuffered output for real-time monitoring
- **Execution**: Direct interpretation

##### JavaScript (.js)
- **Command**: `node {filename}`
- **Features**: Node.js runtime execution
- **Execution**: Direct interpretation

##### Shell Scripts (.sh)
- **Command**: `/bin/bash {filepath}`
- **Features**: Full bash shell capabilities
- **Execution**: Direct interpretation

##### C (.c)
- **Command**: `sh -c "gcc -o {executable} {filename} && ./{executable}"`
- **Features**: Compilation with gcc followed by execution
- **Execution**: Compile-and-run pattern

##### C++ (.cpp, .cc, .cxx)
- **Command**: `sh -c "g++ -o {executable} {filename} && ./{executable}"`
- **Features**: Compilation with g++ followed by execution
- **Execution**: Compile-and-run pattern

##### Rust (.rs)
- **Command**: `sh -c "rustc -o {executable} {filename} && ./{executable}"`
- **Features**: Compilation with rustc followed by execution
- **Execution**: Compile-and-run pattern

##### Go (.go)
- **Command**: `sh -c "go run {filename}"`
- **Features**: Go runtime compilation and execution
- **Execution**: Runtime compilation

##### Java (.java)
- **Command**: `sh -c "javac {filename} && java {classname}"`
- **Features**: Compilation with javac followed by JVM execution
- **Execution**: Compile-and-run with class name detection

##### Generic Files
- **Command**: `./{filename}`
- **Features**: Direct execution with shebang support
- **Execution**: Assumes executable with proper shebang

#### Error Handling
- **Invalid Paths**: Returns configuration error for malformed paths
- **Missing Extensions**: Falls back to direct execution
- **Filename Extraction**: Handles complex path structures

## Instance Persistence

### `fn save(&self) -> Result<()>`
**Location**: `src/isolate.rs:262-267`

Saves current instance configuration to persistent storage.

#### Functionality
- Loads existing instances from storage
- Updates current instance in collection
- Saves complete instance collection back to disk

### `fn load_all_instances() -> Result<HashMap<String, IsolateInstance>>`
**Location**: `src/isolate.rs:269-294`

Loads all instance configurations from persistent storage.

#### Functionality
1. **Directory Management**
   - Creates storage directory if not exists
   - Handles missing configuration files gracefully

2. **File Processing**
   - Reads JSON configuration file
   - Handles empty files appropriately
   - Deserializes stored instances

3. **Error Handling**
   - Returns empty collection for missing files
   - Provides detailed error messages for parsing failures

#### Storage Location
- **Directory**: `/tmp/mini-isolate/`
- **File**: `instances.json`
- **Format**: Pretty-printed JSON

### `fn save_all_instances(instances: &HashMap<String, IsolateInstance>) -> Result<()>`
**Location**: `src/isolate.rs:296-308`

Saves all instance configurations to persistent storage.

#### Functionality
- Ensures storage directory exists
- Serializes instances to pretty-printed JSON
- Writes atomically to storage file

## Cleanup and Lifecycle

### `pub fn cleanup(&self) -> Result<()>`
**Location**: `src/isolate.rs:239-253`

Removes isolate instance and all associated resources.

#### Functionality
1. **Directory Cleanup**
   - Recursively removes working directory and all contents
   - Handles missing directories gracefully

2. **Configuration Cleanup**
   - Removes instance from persistent storage
   - Updates configuration file immediately

3. **Resource Cleanup**
   - Ensures all associated resources are freed
   - Prepares instance for garbage collection

### `pub fn config(&self) -> &IsolateConfig`
**Location**: `src/isolate.rs:257-260`

Provides read-only access to the current instance configuration.

#### Returns
- `&IsolateConfig` - Reference to current configuration

## Design Patterns

### Builder Pattern Integration
- **Configuration**: Integrates seamlessly with `IsolateConfig::default()`
- **Override Support**: Runtime overrides without modifying base configuration
- **Fluent Interface**: Chainable configuration methods

### Persistence Strategy
- **JSON Storage**: Human-readable configuration format
- **Lazy Loading**: Instances loaded on-demand
- **Atomic Updates**: Safe concurrent access to configuration
- **Directory Structure**: Organized storage with predictable paths

### Error Handling Strategy
- **Graceful Degradation**: Missing files handled appropriately
- **Detailed Context**: Error messages provide actionable information
- **Recovery Support**: Transient errors don't corrupt persistent state
- **Resource Cleanup**: Ensures resources are freed even on errors

## Security Considerations

### File System Access
- **Controlled Paths**: All file operations within designated directories
- **Path Validation**: Input validation for file paths
- **Permission Handling**: Appropriate file permissions for working directories
- **Cleanup Guarantee**: Complete cleanup of temporary files

### Process Isolation
- **Working Directory**: Each instance has isolated working directory
- **Resource Limits**: Configurable limits enforced through executor
- **Environment Control**: Controlled environment variable exposure
- **Command Validation**: Input validation for executed commands

### Data Protection
- **Configuration Security**: Safe storage of instance configurations
- **Output Handling**: Secure capture and handling of process output
- **Error Information**: Careful handling of error details to prevent information leakage
- **Resource Tracking**: Accurate tracking of resource usage for monitoring

## Performance Optimization

### Efficiency Features
- **Lazy Configuration Loading**: Instances loaded only when needed
- **Minimal Memory Footprint**: Efficient storage of instance metadata
- **Fast Lookup**: HashMap-based instance lookup
- **Streaming Output**: Efficient handling of process output

### Scalability Support
- **Multiple Instances**: Support for numerous concurrent instances
- **Resource Isolation**: Per-instance resource tracking
- **Configuration Caching**: Efficient configuration management
- **Cleanup Automation**: Automatic resource cleanup prevents resource leaks