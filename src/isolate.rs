/// Main isolate management interface
use crate::executor::ProcessExecutor;
use crate::types::{ExecutionResult, IsolateConfig, IsolateError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
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

const LOCK_PREFIX: &str = "mini-isolate-lock";

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
        base_path.push("mini-isolate");
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
        isolate.save()?;
        Ok(isolate)
    }

    /// Load an existing isolate instance
    pub fn load(instance_id: &str) -> Result<Option<Self>> {
        let mut config_file = std::env::temp_dir();
        config_file.push("mini-isolate");
        config_file.push("instances.json");

        if !config_file.exists() {
            return Ok(None);
        }

        let instances = Self::load_all_instances()?;
        if let Some(instance) = instances.get(instance_id) {
            let mut base_path = std::env::temp_dir();
            base_path.push("mini-isolate");
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

        self.execute_with_overrides(&command, stdin_data, max_cpu, max_memory, max_time)
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
                file_path.to_string_lossy().to_string(),
            ]),
            "js" => Ok(vec!["node".to_string(), filename.to_string()]),
            "sh" => Ok(vec![
                "/bin/bash".to_string(),
                file_path.to_string_lossy().to_string(),
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
        let lock_path = std::env::temp_dir()
            .join("mini-isolate")
            .join("locks")
            .join(&instance_id);
        if lock_path.exists() {
            fs::remove_file(&lock_path)?;
        }

        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &IsolateConfig {
        &self.instance.config
    }

    /// Save instance configuration with atomic operations
    fn save(&self) -> Result<()> {
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
        config_file.push("mini-isolate");

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
        let lock_dir = std::env::temp_dir().join("mini-isolate").join("locks");
        fs::create_dir_all(&lock_dir)?;

        let lock_path = lock_dir.join(&self.instance.config.instance_id);

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
        let instances_dir = std::env::temp_dir().join("mini-isolate");
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
