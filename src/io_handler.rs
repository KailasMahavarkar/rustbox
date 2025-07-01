/// Advanced I/O handling for code sandbox execution
use crate::types::{IsolateConfig, IsolateError, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// I/O handler for managing stdin, stdout, stderr with advanced features
pub struct IoHandler {
    config: IsolateConfig,
    stdin_handle: Option<Box<dyn Write + Send>>,
    stdout_handle: Option<Box<dyn Read + Send>>,
    stderr_handle: Option<Box<dyn Read + Send>>,
    output_threads: Vec<thread::JoinHandle<()>>,
    stdout_data: std::sync::Arc<std::sync::Mutex<String>>,
    stderr_data: std::sync::Arc<std::sync::Mutex<String>>,
}

impl IoHandler {
    /// Create a new I/O handler with the given configuration
    pub fn new(config: IsolateConfig) -> Result<Self> {
        let stdout_data = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let stderr_data = std::sync::Arc::new(std::sync::Mutex::new(String::new()));

        Ok(Self {
            config,
            stdin_handle: None,
            stdout_handle: None,
            stderr_handle: None,
            output_threads: Vec::new(),
            stdout_data,
            stderr_data,
        })
    }

    /// Configure command with appropriate I/O settings
    pub fn configure_command(&mut self, cmd: &mut Command) -> Result<()> {
        // Configure stdin
        if let Some(ref stdin_file) = self.config.stdin_file {
            let file = File::open(stdin_file)
                .map_err(|e| IsolateError::Process(format!("Failed to open stdin file: {}", e)))?;
            cmd.stdin(Stdio::from(file));
        } else if self.config.use_pipes || self.config.stdin_data.is_some() {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }

        // Configure stdout
        if self.config.use_pipes {
            cmd.stdout(Stdio::piped());
        } else if let Some(ref stdout_file) = self.config.stdout_file {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(stdout_file)
                .map_err(|e| IsolateError::Process(format!("Failed to create stdout file: {}", e)))?;
            cmd.stdout(Stdio::from(file));
        } else {
            cmd.stdout(Stdio::piped());
        }

        // Configure stderr
        if self.config.use_pipes {
            cmd.stderr(Stdio::piped());
        } else if let Some(ref stderr_file) = self.config.stderr_file {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(stderr_file)
                .map_err(|e| IsolateError::Process(format!("Failed to create stderr file: {}", e)))?;
            cmd.stderr(Stdio::from(file));
        } else {
            cmd.stderr(Stdio::piped());
        }

        // Configure TTY if enabled
        if self.config.enable_tty {
            self.configure_tty(cmd)?;
        }

        Ok(())
    }

    /// Configure TTY support for interactive programs
    fn configure_tty(&self, cmd: &mut Command) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            
            // Create a new session and set process group
            unsafe {
                cmd.pre_exec(|| {
                    // Create new session
                    if libc::setsid() == -1 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Failed to create new session"
                        ));
                    }
                    Ok(())
                });
            }
        }

        #[cfg(not(unix))]
        {
            return Err(IsolateError::Config(
                "TTY support is only available on Unix systems".to_string()
            ));
        }

        Ok(())
    }

    /// Start I/O handling for a child process
    pub fn start_io_handling(&mut self, child: &mut std::process::Child) -> Result<()> {
        // Handle stdin
        if let Some(stdin_data) = &self.config.stdin_data {
            if let Some(mut stdin) = child.stdin.take() {
                let data = stdin_data.clone();
                let buffer_size = self.config.io_buffer_size;
                
                thread::spawn(move || {
                    let mut writer = BufWriter::with_capacity(buffer_size, stdin);
                    if let Err(e) = writer.write_all(data.as_bytes()) {
                        eprintln!("Failed to write stdin data: {}", e);
                    }
                    if let Err(e) = writer.flush() {
                        eprintln!("Failed to flush stdin: {}", e);
                    }
                });
            }
        }

        // Handle stdout
        if self.config.use_pipes {
            if let Some(stdout) = child.stdout.take() {
                let stdout_data = self.stdout_data.clone();
                let buffer_size = self.config.io_buffer_size;
                let encoding = self.config.text_encoding.clone();
                
                let handle = thread::spawn(move || {
                    let mut reader = BufReader::with_capacity(buffer_size, stdout);
                    let mut buffer = String::new();
                    
                    loop {
                        match reader.read_line(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(_) => {
                                if let Ok(mut data) = stdout_data.lock() {
                                    data.push_str(&buffer);
                                }
                                buffer.clear();
                            }
                            Err(e) => {
                                eprintln!("Error reading stdout: {}", e);
                                break;
                            }
                        }
                    }
                });
                self.output_threads.push(handle);
            }
        }

        // Handle stderr
        if self.config.use_pipes {
            if let Some(stderr) = child.stderr.take() {
                let stderr_data = self.stderr_data.clone();
                let buffer_size = self.config.io_buffer_size;
                let encoding = self.config.text_encoding.clone();
                
                let handle = thread::spawn(move || {
                    let mut reader = BufReader::with_capacity(buffer_size, stderr);
                    let mut buffer = String::new();
                    
                    loop {
                        match reader.read_line(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(_) => {
                                if let Ok(mut data) = stderr_data.lock() {
                                    data.push_str(&buffer);
                                }
                                buffer.clear();
                            }
                            Err(e) => {
                                eprintln!("Error reading stderr: {}", e);
                                break;
                            }
                        }
                    }
                });
                self.output_threads.push(handle);
            }
        }

        Ok(())
    }

    /// Get captured output data
    pub fn get_output(&mut self) -> Result<(String, String)> {
        // Wait for output threads to complete
        for handle in self.output_threads.drain(..) {
            if let Err(e) = handle.join() {
                eprintln!("I/O thread panicked: {:?}", e);
            }
        }

        let stdout = if self.config.use_pipes {
            self.stdout_data.lock()
                .map_err(|e| IsolateError::Process(format!("Failed to lock stdout data: {}", e)))?
                .clone()
        } else if let Some(ref stdout_file) = self.config.stdout_file {
            std::fs::read_to_string(stdout_file)
                .unwrap_or_else(|_| String::new())
        } else {
            String::new()
        };

        let stderr = if self.config.use_pipes {
            self.stderr_data.lock()
                .map_err(|e| IsolateError::Process(format!("Failed to lock stderr data: {}", e)))?
                .clone()
        } else if let Some(ref stderr_file) = self.config.stderr_file {
            std::fs::read_to_string(stderr_file)
                .unwrap_or_else(|_| String::new())
        } else {
            String::new()
        };

        Ok((stdout, stderr))
    }

    /// Send data to stdin in real-time (for interactive programs)
    pub fn send_stdin(&mut self, data: &str) -> Result<()> {
        if let Some(ref mut stdin) = self.stdin_handle {
            stdin.write_all(data.as_bytes())
                .map_err(|e| IsolateError::Process(format!("Failed to write to stdin: {}", e)))?;
            stdin.flush()
                .map_err(|e| IsolateError::Process(format!("Failed to flush stdin: {}", e)))?;
        }
        Ok(())
    }

    /// Get real-time output (for streaming scenarios)
    pub fn get_realtime_output(&self) -> Result<(String, String)> {
        let stdout = self.stdout_data.lock()
            .map_err(|e| IsolateError::Process(format!("Failed to lock stdout data: {}", e)))?
            .clone();

        let stderr = self.stderr_data.lock()
            .map_err(|e| IsolateError::Process(format!("Failed to lock stderr data: {}", e)))?
            .clone();

        Ok((stdout, stderr))
    }

    /// Check if TTY is supported on this system
    pub fn is_tty_supported() -> bool {
        #[cfg(unix)]
        {
            true
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    /// Create a pseudo-terminal for interactive programs
    #[cfg(unix)]
    pub fn create_pty(&self) -> Result<(RawFd, RawFd)> {
        use std::ffi::CString;
        use std::ptr;

        unsafe {
            let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            if master == -1 {
                return Err(IsolateError::Process("Failed to create PTY master".to_string()));
            }

            if libc::grantpt(master) == -1 {
                libc::close(master);
                return Err(IsolateError::Process("Failed to grant PTY".to_string()));
            }

            if libc::unlockpt(master) == -1 {
                libc::close(master);
                return Err(IsolateError::Process("Failed to unlock PTY".to_string()));
            }

            let slave_name = libc::ptsname(master);
            if slave_name.is_null() {
                libc::close(master);
                return Err(IsolateError::Process("Failed to get PTY slave name".to_string()));
            }

            let slave = libc::open(slave_name, libc::O_RDWR | libc::O_NOCTTY);
            if slave == -1 {
                libc::close(master);
                return Err(IsolateError::Process("Failed to open PTY slave".to_string()));
            }

            Ok((master, slave))
        }
    }

    #[cfg(not(unix))]
    pub fn create_pty(&self) -> Result<(RawFd, RawFd)> {
        Err(IsolateError::Config(
            "PTY creation is only supported on Unix systems".to_string()
        ))
    }
}

/// Builder for configuring I/O handling
pub struct IoConfigBuilder {
    config: IsolateConfig,
}

impl IoConfigBuilder {
    pub fn new(mut config: IsolateConfig) -> Self {
        Self { config }
    }

    /// Enable TTY support for interactive programs
    pub fn with_tty(mut self) -> Self {
        self.config.enable_tty = true;
        self
    }

    /// Use pipes for real-time I/O instead of files
    pub fn with_pipes(mut self) -> Self {
        self.config.use_pipes = true;
        self
    }

    /// Set stdin data
    pub fn with_stdin_data(mut self, data: String) -> Self {
        self.config.stdin_data = Some(data);
        self
    }

    /// Set stdin file
    pub fn with_stdin_file<P: Into<std::path::PathBuf>>(mut self, path: P) -> Self {
        self.config.stdin_file = Some(path.into());
        self
    }

    /// Set stdout file
    pub fn with_stdout_file<P: Into<std::path::PathBuf>>(mut self, path: P) -> Self {
        self.config.stdout_file = Some(path.into());
        self
    }

    /// Set stderr file
    pub fn with_stderr_file<P: Into<std::path::PathBuf>>(mut self, path: P) -> Self {
        self.config.stderr_file = Some(path.into());
        self
    }

    /// Set I/O buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.config.io_buffer_size = size;
        self
    }

    /// Set text encoding
    pub fn with_encoding<S: Into<String>>(mut self, encoding: S) -> Self {
        self.config.text_encoding = encoding.into();
        self
    }

    /// Build the configuration
    pub fn build(self) -> IsolateConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_io_handler_creation() {
        let config = IsolateConfig::default();
        let handler = IoHandler::new(config);
        assert!(handler.is_ok());
    }

    #[test]
    fn test_io_config_builder() {
        let config = IsolateConfig::default();
        let built_config = IoConfigBuilder::new(config)
            .with_tty()
            .with_pipes()
            .with_stdin_data("test input".to_string())
            .with_buffer_size(4096)
            .with_encoding("utf-8")
            .build();

        assert!(built_config.enable_tty);
        assert!(built_config.use_pipes);
        assert_eq!(built_config.stdin_data, Some("test input".to_string()));
        assert_eq!(built_config.io_buffer_size, 4096);
        assert_eq!(built_config.text_encoding, "utf-8");
    }

    #[test]
    fn test_tty_support_detection() {
        // TTY support should be available on Unix systems
        #[cfg(unix)]
        assert!(IoHandler::is_tty_supported());
        
        #[cfg(not(unix))]
        assert!(!IoHandler::is_tty_supported());
    }

    #[test]
    fn test_file_based_io_configuration() {
        let temp_dir = TempDir::new().unwrap();
        let stdin_file = temp_dir.path().join("input.txt");
        let stdout_file = temp_dir.path().join("output.txt");
        let stderr_file = temp_dir.path().join("error.txt");

        std::fs::write(&stdin_file, "test input").unwrap();

        let config = IoConfigBuilder::new(IsolateConfig::default())
            .with_stdin_file(stdin_file)
            .with_stdout_file(stdout_file.clone())
            .with_stderr_file(stderr_file.clone())
            .build();

        let mut handler = IoHandler::new(config).unwrap();
        let mut cmd = std::process::Command::new("echo");
        
        // This should not fail
        assert!(handler.configure_command(&mut cmd).is_ok());
    }
}