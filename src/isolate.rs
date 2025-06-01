/// Main isolate management interface
use crate::executor::ProcessExecutor;
use crate::types::{ExecutionResult, IsolateConfig, IsolateError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
}

impl Isolate {
    /// Create a new isolate instance
    pub fn new(config: IsolateConfig) -> Result<Self> {
        let mut base_path = std::env::temp_dir();
        base_path.push("mini-isolate");
        base_path.push(&config.instance_id);

        // Create base directory
        fs::create_dir_all(&base_path)
            .map_err(IsolateError::Io)?;

        let instance = IsolateInstance {
            config,
            created_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
        };

        let isolate = Self {
            instance,
            base_path,
        };

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
                Ok(Some(Self {
                    instance: instance.clone(),
                    base_path,
                }))
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
    pub fn execute(&mut self, command: &[String], stdin_data: Option<&str>) -> Result<ExecutionResult> {
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
    pub fn execute_file(&mut self, file_path: &Path, stdin_data: Option<&str>) -> Result<ExecutionResult> {
        if !file_path.exists() {
            return Err(IsolateError::Config(format!("File not found: {}", file_path.display())));
        }

        // Copy file to working directory
        let filename = file_path.file_name()
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
            return Err(IsolateError::Config(format!("File not found: {}", file_path.display())));
        }

        // Copy file to working directory
        let filename = file_path.file_name()
            .ok_or_else(|| IsolateError::Config("Invalid file path".to_string()))?;
        
        let dest_path = self.instance.config.workdir.join(filename);
        fs::copy(file_path, &dest_path)?;

        // Determine execution command based on file extension
        let command = self.get_execution_command(&dest_path)?;
        
        self.execute_with_overrides(&command, stdin_data, max_cpu, max_memory, max_time)
    }

    /// Get execution command based on file extension
    fn get_execution_command(&self, file_path: &Path) -> Result<Vec<String>> {
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let filename = file_path.file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| IsolateError::Config("Invalid filename".to_string()))?;

        match extension.to_lowercase().as_str() {
            "py" => Ok(vec!["/usr/bin/python3".to_string(), "-u".to_string(), file_path.to_string_lossy().to_string()]),
            "js" => Ok(vec!["node".to_string(), filename.to_string()]),
            "sh" => Ok(vec!["/bin/bash".to_string(), file_path.to_string_lossy().to_string()]),
            "c" => {
                let executable = filename.strip_suffix(".c").unwrap_or("main");
                Ok(vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!("gcc -o {} {} && ./{}", executable, filename, executable),
                ])
            }
            "cpp" | "cc" | "cxx" => {
                let executable = filename.strip_suffix(&format!(".{}", extension)).unwrap_or("main");
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
            "go" => {
                Ok(vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!("go run {}", filename),
                ])
            }
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
    pub fn cleanup(&self) -> Result<()> {
        // Remove working directory
        if self.base_path.exists() {
            fs::remove_dir_all(&self.base_path)
                .map_err(IsolateError::Io)?;
        }

        // Remove from instances file
        let mut instances = Self::load_all_instances()?;
        instances.remove(&self.instance.config.instance_id);
        Self::save_all_instances(&instances)?;

        Ok(())
    }



    /// Get configuration
    pub fn config(&self) -> &IsolateConfig {
        &self.instance.config
    }

    /// Save instance configuration
    fn save(&self) -> Result<()> {
        let mut instances = Self::load_all_instances()?;
        instances.insert(self.instance.config.instance_id.clone(), self.instance.clone());
        Self::save_all_instances(&instances)
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

    /// Save all instances to storage
    fn save_all_instances(instances: &HashMap<String, IsolateInstance>) -> Result<()> {
        let mut config_file = std::env::temp_dir();
        config_file.push("mini-isolate");
        fs::create_dir_all(&config_file)?;
        config_file.push("instances.json");

        let content = serde_json::to_string_pretty(instances)
            .map_err(|e| IsolateError::Config(format!("Failed to serialize instances: {}", e)))?;

        fs::write(config_file, content)?;
        Ok(())
    }
}