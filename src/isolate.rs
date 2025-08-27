/// Main isolate management interface
use crate::executor::ProcessExecutor;
use crate::types::{ExecutionResult, IsolateConfig, IsolateError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(unix)]
extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

#[cfg(unix)]
const LOCK_EX: i32 = 2;
#[cfg(unix)]
const LOCK_NB: i32 = 4;

/// Lock file record structure (simplified text format)
#[derive(Debug, Clone)]
struct LockRecord {
    owner_uid: u32,
    is_initialized: bool,
}

const LOCK_PREFIX: &str = "rustbox-lock";

/// Persistent isolate instance configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
struct IsolateInstance {
    config: IsolateConfig,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used: chrono::DateTime<chrono::Utc>,
}

/// Main isolate manager for handling multiple isolated environments
pub struct Isolate {
    instance: IsolateInstance,
    base_path: PathBuf,
    lock_file: Option<File>,
}

impl Isolate {
    /// Create a new isolate instance
    pub fn new(config: IsolateConfig) -> Result<Self> {
        let mut base_path = std::env::temp_dir();
        base_path.push("rustbox");
        base_path.push(&config.instance_id);

        // Create base directory
        fs::create_dir_all(&base_path).map_err(IsolateError::Io)?;

        let instance = IsolateInstance {
            config,
            created_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
        };

        let mut isolate = Self {
            instance,
            base_path,
            lock_file: None,
        };

        // Acquire lock before any operations
        isolate.acquire_lock(true)?;

        // Save the new instance
        isolate.atomic_instances_update(|instances| {
            instances.insert(isolate.instance.config.instance_id.clone(), isolate.instance.clone());
        })?;

        Ok(isolate)
    }

    /// Load an existing isolate instance
    pub fn load(instance_id: &str) -> Result<Option<Self>> {
        let mut config_file = std::env::temp_dir();
        config_file.push("rustbox");
        config_file.push("instances.json");

        if !config_file.exists() {
            return Ok(None);
        }

        let instances = Self::load_all_instances()?;
        if let Some(instance) = instances.get(instance_id) {
            let mut base_path = std::env::temp_dir();
            base_path.push("rustbox");
            base_path.push(instance_id);

            if base_path.exists() {
                let isolate = Self {
                    instance: instance.clone(),
                    base_path,
                    lock_file: None,
                };
                // Don't acquire lock for load - only for exclusive operations
                Ok(Some(isolate))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// List all isolate instances
    pub fn list_all() -> Result<Vec<String>> {
        let instances = Self::load_all_instances()?;
        Ok(instances.keys().cloned().collect())
    }

    /// Execute a command in this isolate
    pub fn execute(
        &mut self,
        command: &[String],
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        // Acquire lock for execution to prevent conflicts
        if self.lock_file.is_none() {
            self.acquire_lock(false)?;
        }

        // Update last used timestamp
        self.instance.last_used = chrono::Utc::now();
        self.save()?;

        // Create executor with current config
        let mut executor = ProcessExecutor::new(self.instance.config.clone())?;

        // Execute the command
        executor.execute(command, stdin_data)
    }

    /// Execute a command in this isolate with runtime resource overrides
    pub fn execute_with_overrides(
        &mut self,
        command: &[String],
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        // Update last used timestamp
        self.instance.last_used = chrono::Utc::now();
        self.save()?;

        // Clone config and apply overrides
        let mut config = self.instance.config.clone();

        if let Some(cpu_seconds) = max_cpu {
            config.cpu_time_limit = Some(Duration::from_secs(cpu_seconds));
            config.time_limit = Some(Duration::from_secs(cpu_seconds));
        }

        if let Some(memory_mb) = max_memory {
            config.memory_limit = Some(memory_mb * 1024 * 1024); // Convert MB to bytes
        }

        if let Some(time_seconds) = max_time {
            config.wall_time_limit = Some(Duration::from_secs(time_seconds));
        }

        if let Some(fd_limit_val) = fd_limit {
            config.fd_limit = Some(fd_limit_val);
        }

        // Create executor with modified config
        let mut executor = ProcessExecutor::new(config)?;

        // Execute the command
        executor.execute(command, stdin_data)
    }


    /// Execute a single file
    pub fn execute_file(
        &mut self,
        file_path: &Path,
        stdin_data: Option<&str>,
    ) -> Result<ExecutionResult> {
        if !file_path.exists() {
            return Err(IsolateError::Config(format!(
                "File not found: {}",
                file_path.display()
            )));
        }

        // Copy file to working directory
        let filename = file_path
            .file_name()
            .ok_or_else(|| IsolateError::Config("Invalid file path".to_string()))?;

        let dest_path = self.instance.config.workdir.join(filename);
        fs::copy(file_path, &dest_path)?;

        // Determine execution command based on file extension
        let command = self.get_execution_command(&dest_path)?;

        self.execute(&command, stdin_data)
    }

    /// Execute a single file with runtime resource overrides
    pub fn execute_file_with_overrides(
        &mut self,
        file_path: &Path,
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        if !file_path.exists() {
            return Err(IsolateError::Config(format!(
                "File not found: {}",
                file_path.display()
            )));
        }

        // Copy file to working directory
        let filename = file_path
            .file_name()
            .ok_or_else(|| IsolateError::Config("Invalid file path".to_string()))?;

        let dest_path = self.instance.config.workdir.join(filename);
        fs::copy(file_path, &dest_path)?;

        // Determine execution command based on file extension
        let command = self.get_execution_command(&dest_path)?;

        self.execute_with_overrides(&command, stdin_data, max_cpu, max_memory, max_time, fd_limit)
    }

    /// Execute code directly from string input (Judge0-style)
    pub fn execute_code_string(
        &mut self,
        language: &str,
        code: &str,
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        match language.to_lowercase().as_str() {
            "python" | "py" => self.execute_python_string(code, stdin_data, max_cpu, max_memory, max_time, fd_limit),
            "cpp" | "c++" | "cxx" => self.compile_and_execute_cpp(code, stdin_data, max_cpu, max_memory, max_time, fd_limit),
            "c" => self.compile_and_execute_c(code, stdin_data, max_cpu, max_memory, max_time, fd_limit),
            "java" => self.compile_and_execute_java(code, stdin_data, max_cpu, max_memory, max_time, fd_limit),
            "javascript" | "js" | "node" => self.execute_javascript_string(code, stdin_data, max_cpu, max_memory, max_time, fd_limit),
            "rust" | "rs" => self.compile_and_execute_rust(code, stdin_data, max_cpu, max_memory, max_time, fd_limit),
            "go" | "golang" => self.compile_and_execute_go(code, stdin_data, max_cpu, max_memory, max_time, fd_limit),
            _ => Err(IsolateError::Config(format!("Unsupported language: {}", language)))
        }
    }

    /// Execute Python code directly from string
    fn execute_python_string(
        &mut self,
        code: &str,
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        let command = vec![
            "/usr/bin/python3".to_string(),
            "-u".to_string(),
            "-c".to_string(),
            code.to_string()
        ];
        self.execute_with_overrides(&command, stdin_data, max_cpu, max_memory, max_time, fd_limit)
    }

    /// Execute JavaScript code directly from string
    fn execute_javascript_string(
        &mut self,
        code: &str,
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        let command = vec![
            "node".to_string(),
            "-e".to_string(),
            code.to_string()
        ];
        self.execute_with_overrides(&command, stdin_data, max_cpu, max_memory, max_time, fd_limit)
    }

    /// Compile and execute C++ code from string
    fn compile_and_execute_cpp(
        &mut self,
        code: &str,
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        // Write source code to file in sandbox
        let source_file = self.instance.config.workdir.join("solution.cpp");
        fs::write(&source_file, code)?;

        // Compile the code
        let compile_command = vec![
            "g++".to_string(),
            "-o".to_string(),
            "solution".to_string(),
            "solution.cpp".to_string(),
            "-std=c++17".to_string(),
            "-O2".to_string()
        ];

        let compile_result = self.execute(&compile_command, None)?;
        
        if !compile_result.success {
            return Ok(ExecutionResult {
                status: crate::types::ExecutionStatus::RuntimeError,
                exit_code: compile_result.exit_code,
                stdout: "".to_string(),
                stderr: format!("Compilation Error:\n{}", compile_result.stderr),
                wall_time: compile_result.wall_time,
                cpu_time: compile_result.cpu_time,
                memory_peak: compile_result.memory_peak,
                success: false,
                signal: None,
                error_message: Some("Compilation failed".to_string()),
            });
        }

        // Execute the compiled binary
        let execute_command = vec!["./solution".to_string()];
        self.execute_with_overrides(&execute_command, stdin_data, max_cpu, max_memory, max_time, fd_limit)
    }

    /// Compile and execute C code from string
    fn compile_and_execute_c(
        &mut self,
        code: &str,
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        // Write source code to file in sandbox
        let source_file = self.instance.config.workdir.join("solution.c");
        fs::write(&source_file, code)?;

        // Compile the code
        let compile_command = vec![
            "gcc".to_string(),
            "-o".to_string(),
            "solution".to_string(),
            "solution.c".to_string(),
            "-std=c11".to_string(),
            "-O2".to_string()
        ];

        let compile_result = self.execute(&compile_command, None)?;
        
        if !compile_result.success {
            return Ok(ExecutionResult {
                status: crate::types::ExecutionStatus::RuntimeError,
                exit_code: compile_result.exit_code,
                stdout: "".to_string(),
                stderr: format!("Compilation Error:\n{}", compile_result.stderr),
                wall_time: compile_result.wall_time,
                cpu_time: compile_result.cpu_time,
                memory_peak: compile_result.memory_peak,
                success: false,
                signal: None,
                error_message: Some("Compilation failed".to_string()),
            });
        }

        // Execute the compiled binary
        let execute_command = vec!["./solution".to_string()];
        self.execute_with_overrides(&execute_command, stdin_data, max_cpu, max_memory, max_time, fd_limit)
    }

    /// Compile and execute Java code from string
    fn compile_and_execute_java(
        &mut self,
        code: &str,
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        // Extract class name from code (simple heuristic)
        let class_name = self.extract_java_class_name(code).unwrap_or("Main".to_string());
        let source_file = self.instance.config.workdir.join(format!("{}.java", class_name));
        fs::write(&source_file, code)?;

        // Java needs relaxed isolation settings due to JVM threading requirements
        // Temporarily modify config for Java compilation and execution
        let original_config = self.instance.config.clone();
        
        // Relax isolation for Java (JVM requires more system access)
        self.instance.config.enable_pid_namespace = false;
        self.instance.config.enable_network_namespace = false;
        
        // Increase resource limits for JVM
        if max_memory.is_some() {
            self.instance.config.memory_limit = Some(max_memory.unwrap() * 1024 * 1024);
        } else {
            self.instance.config.memory_limit = Some(512 * 1024 * 1024); // 512MB default for Java
        }
        
        // Increase process limit for JVM threads
        self.instance.config.process_limit = Some(50);

        // Compile the code with relaxed settings
        let compile_command = vec![
            "javac".to_string(),
            "-cp".to_string(),
            ".".to_string(),
            format!("{}.java", class_name)
        ];

        let compile_result = self.execute(&compile_command, None)?;
        
        if !compile_result.success {
            // Restore original config
            self.instance.config = original_config;
            return Ok(ExecutionResult {
                status: crate::types::ExecutionStatus::RuntimeError,
                exit_code: compile_result.exit_code,
                stdout: "".to_string(),
                stderr: format!("Java Compilation Error:\n{}", compile_result.stderr),
                wall_time: compile_result.wall_time,
                cpu_time: compile_result.cpu_time,
                memory_peak: compile_result.memory_peak,
                success: false,
                signal: None,
                error_message: Some("Java compilation failed".to_string()),
            });
        }

        // Execute the compiled class with relaxed settings
        let execute_command = vec![
            "java".to_string(),
            "-cp".to_string(),
            ".".to_string(),
            class_name
        ];
        
        let result = self.execute_with_overrides(&execute_command, stdin_data, max_cpu, max_memory, max_time, fd_limit);
        
        // Restore original config
        self.instance.config = original_config;
        
        result
    }

    /// Compile and execute Rust code from string
    fn compile_and_execute_rust(
        &mut self,
        code: &str,
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        // Write source code to file in sandbox
        let source_file = self.instance.config.workdir.join("solution.rs");
        fs::write(&source_file, code)?;

        // Compile the code
        let compile_command = vec![
            "rustc".to_string(),
            "-o".to_string(),
            "solution".to_string(),
            "solution.rs".to_string(),
            "-O".to_string()
        ];

        let compile_result = self.execute(&compile_command, None)?;
        
        if !compile_result.success {
            return Ok(ExecutionResult {
                status: crate::types::ExecutionStatus::RuntimeError,
                exit_code: compile_result.exit_code,
                stdout: "".to_string(),
                stderr: format!("Compilation Error:\n{}", compile_result.stderr),
                wall_time: compile_result.wall_time,
                cpu_time: compile_result.cpu_time,
                memory_peak: compile_result.memory_peak,
                success: false,
                signal: None,
                error_message: Some("Compilation failed".to_string()),
            });
        }

        // Execute the compiled binary
        let execute_command = vec!["./solution".to_string()];
        self.execute_with_overrides(&execute_command, stdin_data, max_cpu, max_memory, max_time, fd_limit)
    }

    /// Compile and execute Go code from string
    fn compile_and_execute_go(
        &mut self,
        code: &str,
        stdin_data: Option<&str>,
        max_cpu: Option<u64>,
        max_memory: Option<u64>,
        max_time: Option<u64>,
        fd_limit: Option<u64>,
    ) -> Result<ExecutionResult> {
        // Write source code to file in sandbox
        let source_file = self.instance.config.workdir.join("solution.go");
        fs::write(&source_file, code)?;

        // Go needs extremely relaxed settings due to its toolchain and runtime requirements
        let original_config = self.instance.config.clone();
        
        // Disable namespace isolation for Go (like Java) - Go runtime conflicts with PID namespace
        self.instance.config.enable_pid_namespace = false;
        self.instance.config.enable_network_namespace = false;
        
        // Increase resource limits significantly for Go compilation
        if max_memory.is_some() {
            self.instance.config.memory_limit = Some(max_memory.unwrap() * 1024 * 1024);
        } else {
            self.instance.config.memory_limit = Some(256 * 1024 * 1024); // Test 128MB - realistic for 10^5 benchmark
        }
        
        // Go toolchain needs processes - will be optimized via binary search
        self.instance.config.process_limit = Some(60);
        
        // Go file descriptor limit - optimized for realistic workloads
        self.instance.config.fd_limit = Some(128);

        // Compile the code using go build
        let compile_command = vec![
            "go".to_string(),
            "build".to_string(),
            "-o".to_string(),
            "solution".to_string(),
            "solution.go".to_string()
        ];

        let compile_result = self.execute(&compile_command, None)?;
        
        if !compile_result.success {
            // Restore original config
            self.instance.config = original_config;
            return Ok(ExecutionResult {
                status: crate::types::ExecutionStatus::RuntimeError,
                exit_code: compile_result.exit_code,
                stdout: "".to_string(),
                stderr: format!("Go Compilation Error:\n{}", compile_result.stderr),
                wall_time: compile_result.wall_time,
                cpu_time: compile_result.cpu_time,
                memory_peak: compile_result.memory_peak,
                success: false,
                signal: None,
                error_message: Some("Go compilation failed".to_string()),
            });
        }

        // Execute the compiled binary
        let execute_command = vec!["./solution".to_string()];
        let result = self.execute_with_overrides(&execute_command, stdin_data, max_cpu, max_memory, max_time, fd_limit);
        
        // Restore original config
        self.instance.config = original_config;
        
        result
    }

    /// Extract Java class name from source code (simple regex-based extraction)
    fn extract_java_class_name(&self, code: &str) -> Option<String> {
        // Look for "public class ClassName" pattern
        for line in code.lines() {
            let line = line.trim();
            if line.starts_with("public class ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let class_name = parts[2].trim_end_matches('{').trim();
                    return Some(class_name.to_string());
                }
            }
        }
        None
    }

    /// Get execution command based on file extension
    fn get_execution_command(&self, file_path: &Path) -> Result<Vec<String>> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let filename = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| IsolateError::Config("Invalid filename".to_string()))?;

        match extension.to_lowercase().as_str() {
            "py" => Ok(vec![
                "/usr/bin/python3".to_string(),
                "-u".to_string(),
                filename.to_string(),
            ]),
            "js" => Ok(vec!["node".to_string(), filename.to_string()]),
            "sh" => Ok(vec![
                "/bin/bash".to_string(),
                filename.to_string(),
            ]),
            "c" => {
                let executable = filename.strip_suffix(".c").unwrap_or("main");
                Ok(vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!("gcc -o {} {} && ./{}", executable, filename, executable),
                ])
            }
            "cpp" | "cc" | "cxx" => {
                let executable = filename
                    .strip_suffix(&format!(".{}", extension))
                    .unwrap_or("main");
                Ok(vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!("g++ -o {} {} && ./{}", executable, filename, executable),
                ])
            }
            "rs" => {
                let executable = filename.strip_suffix(".rs").unwrap_or("main");
                Ok(vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!("rustc -o {} {} && ./{}", executable, filename, executable),
                ])
            }
            "go" => Ok(vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("go run {}", filename),
            ]),
            "java" => {
                let classname = filename.strip_suffix(".java").unwrap_or("Main");
                Ok(vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!("javac {} && java {}", filename, classname),
                ])
            }
            _ => {
                // Try to execute directly (assume it's a script with shebang)
                Ok(vec![format!("./{}", filename)])
            }
        }
    }

    /// Clean up this isolate instance
    pub fn cleanup(mut self) -> Result<()> {
        let instance_id = self.instance.config.instance_id.clone();

        // Acquire lock for cleanup to prevent conflicts
        if self.lock_file.is_none() {
            self.acquire_lock(false)?;
        }

        // Remove from storage atomically
        self.atomic_instances_update(|instances| {
            instances.remove(&instance_id);
        })?;

        // Clean up filesystem
        if self.base_path.exists() {
            fs::remove_dir_all(&self.base_path).map_err(IsolateError::Io)?;
        }

        // Release lock before removing lock file
        self.release_lock();

        // Remove lock file
        let lock_filename = instance_id.replace("/", "_");
        let lock_path = std::env::temp_dir()
            .join("rustbox")
            .join("locks")
            .join(&lock_filename);
        if lock_path.exists() {
            fs::remove_file(&lock_path)?;
        }

        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &IsolateConfig {
        &self.instance.config
    }

    /// Add directory bindings to the isolate configuration
    pub fn add_directory_bindings(&mut self, bindings: Vec<crate::types::DirectoryBinding>) -> Result<()> {
        // Validate all bindings before applying any
        for binding in &bindings {
            // Check if source exists (unless maybe flag is set)
            if !binding.maybe && !binding.source.exists() {
                return Err(IsolateError::Config(format!(
                    "Source directory does not exist: {}",
                    binding.source.display()
                )));
            }

            // Validate source is actually a directory
            if binding.source.exists() && !binding.source.is_dir() {
                return Err(IsolateError::Config(format!(
                    "Source path is not a directory: {}",
                    binding.source.display()
                )));
            }

            // Validate target path format
            if binding.target.is_absolute() && binding.target.starts_with("/") {
                // This is good - absolute path in sandbox
            } else {
                return Err(IsolateError::Config(format!(
                    "Target path must be absolute (start with /): {}",
                    binding.target.display()
                )));
            }
        }

        // Add bindings to configuration
        self.instance.config.directory_bindings.extend(bindings);
        
        // Update last_used timestamp
        self.instance.last_used = chrono::Utc::now();
        
        // Save updated configuration
        self.save()?;
        
        Ok(())
    }

    /// Save instance configuration with atomic operations
    pub fn save(&self) -> Result<()> {
        self.atomic_instances_update(|instances| {
            instances.insert(
                self.instance.config.instance_id.clone(),
                self.instance.clone(),
            );
        })
    }

    /// Load all instances from storage
    fn load_all_instances() -> Result<HashMap<String, IsolateInstance>> {
        let mut config_file = std::env::temp_dir();
        config_file.push("rustbox");

        // Create directory if it doesn't exist
        if !config_file.exists() {
            fs::create_dir_all(&config_file)?;
        }

        config_file.push("instances.json");

        if !config_file.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(config_file)?;
        if content.trim().is_empty() {
            return Ok(HashMap::new());
        }

        let instances: HashMap<String, IsolateInstance> = serde_json::from_str(&content)
            .map_err(|e| IsolateError::Config(format!("Failed to parse instances: {}", e)))?;

        Ok(instances)
    }

    /// Acquire exclusive lock for this isolate instance (based on isolate-reference)
    fn acquire_lock(&mut self, is_init: bool) -> Result<()> {
        let lock_dir = std::env::temp_dir().join("rustbox").join("locks");
        fs::create_dir_all(&lock_dir)?;

        let lock_filename = self.instance.config.instance_id.replace("/", "_");
        let lock_path = lock_dir.join(&lock_filename);

        // Open lock file with read/write/create permissions, but without truncating.
        let mut lock_file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(is_init)
            .open(&lock_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    IsolateError::Lock("Instance not initialized".to_string())
                } else {
                    IsolateError::Io(e)
                }
            })?;

        // Acquire exclusive lock (non-blocking to detect busy state)
        #[cfg(unix)]
        {
            let result = unsafe { flock(lock_file.as_raw_fd(), LOCK_EX | LOCK_NB) };
            if result != 0 {
                let errno = std::io::Error::last_os_error();
                return match errno.raw_os_error() {
                    Some(libc::EWOULDBLOCK) => Err(IsolateError::LockBusy),
                    _ => Err(IsolateError::Lock(format!(
                        "Cannot acquire lock: {}",
                        errno
                    ))),
                };
            }
        }

        let mut content = String::new();
        lock_file.read_to_string(&mut content)?;

        let mut lock_record = if !content.is_empty() {
            let lines: Vec<&str> = content.trim().lines().collect();
            if lines.len() >= 3 && lines[0] == LOCK_PREFIX {
                let owner_uid: u32 = lines[1].parse().map_err(|_| IsolateError::LockCorrupted)?;
                let is_initialized: bool =
                    lines[2].parse().map_err(|_| IsolateError::LockCorrupted)?;

                // Check ownership (allow root to take over)
                let current_uid = unsafe { libc::getuid() };
                if is_initialized && owner_uid != current_uid && current_uid != 0 {
                    return Err(IsolateError::Lock(format!(
                        "Instance owned by uid {}",
                        owner_uid
                    )));
                }
                LockRecord {
                    owner_uid,
                    is_initialized,
                }
            } else {
                return Err(IsolateError::LockCorrupted);
            }
        } else {
            // New or truncated lock file
            LockRecord {
                owner_uid: unsafe { libc::getuid() },
                is_initialized: false,
            }
        };

        if is_init {
            // Now that we have the lock, we can safely write to the file.
            lock_record.is_initialized = true;
            let new_content = format!(
                "{}
{}
{}
",
                LOCK_PREFIX, lock_record.owner_uid, lock_record.is_initialized
            );

            // Truncate and write from the beginning
            lock_file.seek(SeekFrom::Start(0))?;
            lock_file.set_len(new_content.len() as u64)?;
            lock_file.write_all(new_content.as_bytes())?;
        } else if !lock_record.is_initialized {
            return Err(IsolateError::Lock(
                "Instance not properly initialized".to_string(),
            ));
        }

        self.lock_file = Some(lock_file);
        Ok(())
    }

    /// Atomic update of instances.json with file locking
    fn atomic_instances_update<F>(&self, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut HashMap<String, IsolateInstance>),
    {
        let instances_dir = std::env::temp_dir().join("rustbox");
        fs::create_dir_all(&instances_dir)?;

        let instances_file = instances_dir.join("instances.json");
        let instances_lock = instances_dir.join("instances.lock");

        // Acquire global instances file lock
        let lock_file = File::create(&instances_lock)?;

        #[cfg(unix)]
        {
            let result = unsafe { flock(lock_file.as_raw_fd(), LOCK_EX) };
            if result != 0 {
                let errno = std::io::Error::last_os_error();
                return Err(IsolateError::Lock(format!(
                    "Cannot lock instances file: {}",
                    errno
                )));
            }
        }

        // Load current instances
        let mut instances = if instances_file.exists() {
            let content = fs::read_to_string(&instances_file)?;
            if content.trim().is_empty() {
                HashMap::new()
            } else {
                serde_json::from_str(&content).map_err(|e| {
                    IsolateError::Config(format!("Failed to parse instances: {}", e))
                })?
            }
        } else {
            HashMap::new()
        };

        // Apply update
        update_fn(&mut instances);

        // Write atomically (write to temp file, then rename)
        let temp_file = instances_dir.join("instances.json.tmp");
        let content = serde_json::to_string_pretty(&instances)
            .map_err(|e| IsolateError::Config(format!("Failed to serialize instances: {}", e)))?;

        fs::write(&temp_file, content)?;
        fs::rename(&temp_file, &instances_file)?;

        // Lock is automatically released when lock_file goes out of scope
        Ok(())
    }

    /// Release the lock (happens automatically on drop)
    fn release_lock(&mut self) {
        self.lock_file = None;
    }
}

impl Drop for Isolate {
    fn drop(&mut self) {
        // Lock is automatically released when file descriptor is closed
        self.release_lock();
    }
}
