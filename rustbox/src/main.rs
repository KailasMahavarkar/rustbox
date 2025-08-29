use anyhow::Result;
use clap::{Parser, Subcommand};
/// rustbox: Secure Process Isolation and Resource Control System
///
/// A modern, Rust-based implementation inspired by IOI Isolate, designed for secure
/// execution of untrusted code with comprehensive resource limits and namespace isolation.
///
/// # Security Features
/// - Namespace isolation (PID, mount, network, user)
/// - Resource limits enforcement (memory, CPU, file size, etc.)
/// - Cgroups v1 support for maximum compatibility
/// - Path validation to prevent directory traversal
/// - Memory-safe implementation in Rust
///
/// # Platform Support
/// - Primary: Linux with cgroups v1 support
/// - Secondary: Unix-like systems with limited functionality
///
/// # Usage
/// ```bash
/// rustbox init --box-id 0
/// rustbox run --box-id 0 --mem 128 --time 10 /usr/bin/python3 solution.py
/// rustbox cleanup --box-id 0
/// ```
use rustbox::*;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new sandbox environment
    Init {
        /// Box ID for the sandbox
        #[arg(long)]
        box_id: u32,
    },
    /// Run a command in the sandbox
    Run {
        /// Box ID for the sandbox
        #[arg(long)]
        box_id: u32,
        /// Memory limit in MB
        #[arg(long)]
        mem: Option<u64>,
        /// Time limit in seconds
        #[arg(long)]
        time: Option<u64>,
        /// CPU limit in seconds
        #[arg(long)]
        cpu: Option<u64>,
        /// Wall clock time limit in seconds
        #[arg(long)]
        wall_time: Option<u64>,
        /// Maximum number of processes
        #[arg(long)]
        processes: Option<u32>,
        /// Directory bindings (format: source=target:options)
        #[arg(long = "dir", value_name = "BINDING")]
        directory_bindings: Vec<String>,
        /// Command and arguments to execute
        command: Vec<String>,
    },
    /// Execute code directly from string input (Judge0-style)
    ExecuteCode {
        /// Box ID for the sandbox
        #[arg(long)]
        box_id: u32,
        /// Programming language (python, c and java)
        #[arg(long)]
        language: String,
        /// Source code as string
        #[arg(long)]
        code: String,
        /// Input data to pass to stdin
        #[arg(long)]
        stdin: Option<String>,
        /// Memory limit in MB
        #[arg(long)]
        mem: Option<u64>,
        /// Time limit in seconds
        #[arg(long)]
        time: Option<u64>,
        /// CPU limit in seconds
        #[arg(long)]
        cpu: Option<u64>,
        /// Wall clock time limit in seconds
        #[arg(long)]
        wall_time: Option<u64>,
        /// Maximum number of processes
        #[arg(long)]
        processes: Option<u32>,
        /// Strict mode: require root privileges and fail if security features unavailable
        #[arg(long)]
        strict: bool,
    },
    /// Clean up sandbox environment
    Cleanup {
        /// Box ID for the sandbox
        #[arg(long)]
        box_id: u32,
    },
    /// Check if all language dependencies are installed
    CheckDeps {
        /// Verbose output showing detailed version information
        #[arg(long)]
        verbose: bool,
    },
}

static CURRENT_BOX_ID: AtomicU32 = AtomicU32::new(0);

extern "C" fn signal_handler(sig: i32) {
    let box_id = CURRENT_BOX_ID.load(Ordering::Relaxed);
    if box_id != 0 {
        eprintln!("Signal {} received, cleaning up box {}", sig, box_id);
        // The new lock system automatically cleans up on drop
        eprintln!("Lock cleanup will be handled automatically by the enhanced lock manager");
    }
    std::process::exit(128 + sig);
}

fn setup_signal_handlers() {
    unsafe {
        libc::signal(libc::SIGTERM, signal_handler as usize);
        libc::signal(libc::SIGINT, signal_handler as usize);
    }
}

fn main() -> Result<()> {
    setup_signal_handlers();

    // Initialize structured logging for security monitoring
    env_logger::init();

    // Initialize security logger for audit trail
    if let Err(e) = rustbox::security_logging::init_security_logger(None) {
        eprintln!("Failed to initialize security logger: {}", e);
        std::process::exit(1);
    }

    // Initialize the enhanced lock manager
    if let Err(e) = rustbox::lock_manager::init_lock_manager() {
        eprintln!("Failed to initialize lock manager: {}", e);
        std::process::exit(1);
    }

    // Platform compatibility check - Unix-only for security features
    if !cfg!(unix) {
        eprintln!("Error: rustbox requires Unix-like systems for security features");
        eprintln!("Current platform does not support necessary isolation mechanisms");
        std::process::exit(1);
    }

    // Parse command line arguments
    let cli = Cli::parse();

    // Privilege check - many security features require elevated permissions
    if unsafe { libc::getuid() } != 0 {
        eprintln!("Warning: rustbox may require root privileges for full functionality");
        eprintln!("Running without root may limit:");
        eprintln!("  • Cgroups resource enforcement");
        eprintln!("  • Namespace isolation capabilities");
        eprintln!("  • Chroot directory creation");
    }

    // Security subsystem availability checks
    perform_security_checks();

    // Execute the appropriate command
    match cli.command {
        Commands::Init { box_id } => {
            CURRENT_BOX_ID.store(box_id, Ordering::Relaxed);
            eprintln!("Initializing sandbox with box-id: {}", box_id);

            let mut config = rustbox::types::IsolateConfig::default();
            config.instance_id = format!("rustbox/{}", box_id);
            // The workdir will be created under /tmp/rustbox/{instance_id}/ by default
            // So we don't need to override it, just use the default behavior
            config.strict_mode = false;

            let _isolate = rustbox::isolate::Isolate::new(config)?;
            eprintln!("Sandbox initialized successfully");
            Ok(())
        }
        Commands::Run {
            box_id,
            mem,
            time,
            cpu,
            wall_time,
            processes,
            directory_bindings,
            command,
        } => {
            CURRENT_BOX_ID.store(box_id, Ordering::Relaxed);
            eprintln!("Running command in sandbox {}: {:?}", box_id, command);
            if let Some(mem) = mem {
                eprintln!("Memory limit: {} MB", mem);
            }
            if let Some(time) = time {
                eprintln!("Time limit: {} seconds", time);
            }
            if let Some(cpu) = cpu {
                eprintln!("CPU limit: {} seconds", cpu);
            }
            if let Some(wall_time) = wall_time {
                eprintln!("Wall time limit: {} seconds", wall_time);
            }
            if let Some(processes) = processes {
                eprintln!("Process limit: {}", processes);
            }

            let instance_id = format!("rustbox/{}", box_id);
            let mut isolate = rustbox::isolate::Isolate::load(&instance_id)?
                .ok_or_else(|| anyhow::anyhow!("Sandbox {} not found. Run init first.", box_id))?;

            // Acquire lock for exclusive execution to prevent concurrent access
            if let Err(e) = isolate.acquire_execution_lock() {
                match e {
                    rustbox::types::IsolateError::LockBusy => {
                        eprintln!("Error: Lock already held by process");
                        eprintln!("Another process is currently using sandbox {}", box_id);
                        std::process::exit(1);
                    }
                    _ => return Err(e.into()),
                }
            }

            // Parse and apply directory bindings
            if !directory_bindings.is_empty() {
                let mut bindings = Vec::new();
                for binding_str in &directory_bindings {
                    match rustbox::types::DirectoryBinding::parse_secure(binding_str) {
                        Ok(binding) => {
                            eprintln!(
                                "Directory binding: {} -> {} ({:?})",
                                binding.source.display(),
                                binding.target.display(),
                                binding.permissions
                            );
                            bindings.push(binding);
                        }
                        Err(e) => {
                            eprintln!("Error parsing directory binding '{}': {}", binding_str, e);
                            std::process::exit(1);
                        }
                    }
                }
                isolate.add_directory_bindings(bindings)?;
            }

            if command.is_empty() {
                // No command specified - look for standardized pattern /tmp/<box-id>.py in sandbox
                let standard_filename = format!("{}.py", box_id);
                let sandbox_work_dir =
                    std::path::Path::new("/tmp/rustbox").join(format!("rustbox-{}", box_id));
                let standard_path = sandbox_work_dir.join(&standard_filename);

                if standard_path.exists() {
                    eprintln!("Executing standardized file: {}", standard_filename);
                    let code = std::fs::read_to_string(&standard_path)?;
                    let result = isolate.execute_code_string(
                        "python",
                        &code,
                        None, // stdin
                        cpu,
                        mem,
                        time.or(wall_time),
                        None, // fd_limit
                    )?;

                    // Print execution results
                    let status_message = match result.status {
                        crate::types::ExecutionStatus::TimeLimit => "TLE".to_string(),
                        crate::types::ExecutionStatus::MemoryLimit => {
                            "Memory Limit Exceeded".to_string()
                        }
                        _ => format!("{:?}", result.status),
                    };

                    let json_result = serde_json::json!({
                        "status": status_message,
                        "exit_code": result.exit_code,
                        "stdout": result.stdout,
                        "stderr": result.stderr,
                        "wall_time": result.wall_time,
                        "cpu_time": result.cpu_time,
                        "memory_peak_kb": result.memory_peak / 1024,
                        "success": result.success,
                        "signal": result.signal,
                        "error_message": result.error_message
                    });
                    println!("{}", serde_json::to_string_pretty(&json_result).unwrap());

                    // Automatic cleanup after execution (no command specified path)
                    let cleanup_result = isolate.cleanup();
                    match cleanup_result {
                        Ok(_) => {
                            // Also clean up the standardized files we created
                            let sandbox_work_dir = std::path::Path::new("/tmp/rustbox")
                                .join(format!("rustbox-{}", box_id));
                            if sandbox_work_dir.exists() {
                                if let Err(e) = std::fs::remove_dir_all(&sandbox_work_dir) {
                                    eprintln!(
                                        "Warning: Failed to remove sandbox files {}: {}",
                                        sandbox_work_dir.display(),
                                        e
                                    );
                                } else {
                                    eprintln!(
                                        "Automatically cleaned up sandbox {} files and instance",
                                        box_id
                                    );
                                }
                            } else {
                                eprintln!(
                                    "Automatically cleaned up sandbox {} after execution",
                                    box_id
                                );
                            }
                        }
                        Err(e) => eprintln!("Warning: Failed to cleanup sandbox {}: {}", box_id, e),
                    }

                    if !result.success {
                        std::process::exit(1);
                    }
                } else {
                    eprintln!(
                        "Error: No command specified and standardized file {} not found in sandbox",
                        standard_filename
                    );
                    eprintln!("Usage: rustbox run --box-id {} <filename> or ensure {} exists in sandbox /tmp/", box_id, standard_filename);
                    std::process::exit(1);
                }
            } else if command.len() == 1 {
                let command_arg = &command[0];
                let current_dir =
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                let source_path = current_dir.join(command_arg);

                // Check if file exists in current directory, copy to standardized location in sandbox
                if source_path.exists() {
                    let sandbox_work_dir =
                        std::path::Path::new("/tmp/rustbox").join(format!("rustbox-{}", box_id));

                    // Ensure sandbox work directory exists
                    if !sandbox_work_dir.exists() {
                        std::fs::create_dir_all(&sandbox_work_dir).map_err(|e| {
                            anyhow::anyhow!("Failed to create sandbox work directory: {}", e)
                        })?;
                    }

                    // Determine file extension and create standardized name
                    let extension = source_path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("py"); // default to .py
                    let standardized_name = format!("{}.{}", box_id, extension);
                    let dest_path = sandbox_work_dir.join(&standardized_name);

                    // Check if standardized file already exists (conflict detection)
                    if dest_path.exists() {
                        eprintln!(
                            "Error: Standardized file {} already exists in sandbox {}",
                            standardized_name, box_id
                        );
                        eprintln!("This indicates another user/process has already initialized this box-id with a file.");
                        eprintln!(
                            "Please use a different box-id or clean up the existing sandbox first."
                        );
                        eprintln!("To cleanup: rustbox cleanup --box-id {}", box_id);
                        std::process::exit(1);
                    }

                    // Also create the standard /tmp location inside the sandbox
                    let sandbox_tmp_dir = sandbox_work_dir.join("tmp");
                    std::fs::create_dir_all(&sandbox_tmp_dir).map_err(|e| {
                        anyhow::anyhow!("Failed to create sandbox /tmp directory: {}", e)
                    })?;
                    let internal_dest_path = sandbox_tmp_dir.join(&standardized_name);

                    // Check internal path conflict as well
                    if internal_dest_path.exists() {
                        eprintln!("Error: Internal standardized file /tmp/{} already exists in sandbox {}", standardized_name, box_id);
                        eprintln!("This indicates another user/process has already initialized this box-id with a file.");
                        eprintln!(
                            "Please use a different box-id or clean up the existing sandbox first."
                        );
                        eprintln!("To cleanup: rustbox cleanup --box-id {}", box_id);
                        std::process::exit(1);
                    }

                    // Copy file to both locations (work dir and /tmp inside sandbox)
                    std::fs::copy(&source_path, &dest_path).map_err(|e| {
                        anyhow::anyhow!("Failed to copy file to sandbox work directory: {}", e)
                    })?;
                    std::fs::copy(&source_path, &internal_dest_path).map_err(|e| {
                        anyhow::anyhow!("Failed to copy file to sandbox /tmp: {}", e)
                    })?;

                    eprintln!("Copied {} to sandbox as {}", command_arg, standardized_name);
                    eprintln!(
                        "File available at: /tmp/{} inside sandbox",
                        standardized_name
                    );

                    // Execute the copied file using the standardized path
                    let code = std::fs::read_to_string(&dest_path)?;
                    let language = match extension {
                        "py" => "python",
                        "cpp" | "cc" | "cxx" => "cpp",
                        "java" => "java",
                        _ => "python", // default
                    };
                    let result = isolate.execute_code_string(
                        language,
                        &code,
                        None, // stdin
                        cpu,
                        mem,
                        time.or(wall_time),
                        None, // fd_limit
                    )?;

                    // Print execution results in JSON format
                    let status_message = match result.status {
                        crate::types::ExecutionStatus::TimeLimit => "TLE".to_string(),
                        crate::types::ExecutionStatus::MemoryLimit => {
                            "Memory Limit Exceeded".to_string()
                        }
                        _ => format!("{:?}", result.status),
                    };

                    let json_result = serde_json::json!({
                        "status": status_message,
                        "exit_code": result.exit_code,
                        "stdout": result.stdout,
                        "stderr": result.stderr,
                        "wall_time": result.wall_time,
                        "cpu_time": result.cpu_time,
                        "memory_peak_kb": result.memory_peak / 1024,
                        "success": result.success,
                        "signal": result.signal,
                        "error_message": result.error_message
                    });
                    println!("{}", serde_json::to_string_pretty(&json_result).unwrap());

                    // Automatic cleanup after execution (file specified path)
                    let cleanup_result = isolate.cleanup();
                    match cleanup_result {
                        Ok(_) => {
                            // Also clean up the standardized files we created
                            let sandbox_work_dir = std::path::Path::new("/tmp/rustbox")
                                .join(format!("rustbox-{}", box_id));
                            if sandbox_work_dir.exists() {
                                if let Err(e) = std::fs::remove_dir_all(&sandbox_work_dir) {
                                    eprintln!(
                                        "Warning: Failed to remove sandbox files {}: {}",
                                        sandbox_work_dir.display(),
                                        e
                                    );
                                } else {
                                    eprintln!(
                                        "Automatically cleaned up sandbox {} files and instance",
                                        box_id
                                    );
                                }
                            } else {
                                eprintln!(
                                    "Automatically cleaned up sandbox {} after execution",
                                    box_id
                                );
                            }
                        }
                        Err(e) => eprintln!("Warning: Failed to cleanup sandbox {}: {}", box_id, e),
                    }

                    if !result.success {
                        std::process::exit(1);
                    }
                } else if std::path::Path::new(command_arg).exists() {
                    // File exists as absolute path - execute directly
                    let file_path = std::path::Path::new(command_arg);
                    let code = std::fs::read_to_string(file_path)?;
                    let language = match file_path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("py")
                    {
                        "py" => "python",
                        "cpp" | "cc" | "cxx" => "cpp",
                        "java" => "java",
                        _ => "python", // default
                    };
                    let result = isolate.execute_code_string(
                        language,
                        &code,
                        None, // stdin
                        cpu,
                        mem,
                        time.or(wall_time),
                        None, // fd_limit
                    )?;

                    // Print execution results in JSON format
                    let status_message = match result.status {
                        crate::types::ExecutionStatus::TimeLimit => "TLE".to_string(),
                        crate::types::ExecutionStatus::MemoryLimit => {
                            "Memory Limit Exceeded".to_string()
                        }
                        _ => format!("{:?}", result.status),
                    };

                    let json_result = serde_json::json!({
                        "status": status_message,
                        "exit_code": result.exit_code,
                        "stdout": result.stdout,
                        "stderr": result.stderr,
                        "wall_time": result.wall_time,
                        "cpu_time": result.cpu_time,
                        "memory_peak_kb": result.memory_peak / 1024,
                        "success": result.success,
                        "signal": result.signal,
                        "error_message": result.error_message
                    });
                    println!("{}", serde_json::to_string_pretty(&json_result).unwrap());

                    // Automatic cleanup after execution (absolute path)
                    let cleanup_result = isolate.cleanup();
                    match cleanup_result {
                        Ok(_) => {
                            // Also clean up the standardized files we created
                            let sandbox_work_dir = std::path::Path::new("/tmp/rustbox")
                                .join(format!("rustbox-{}", box_id));
                            if sandbox_work_dir.exists() {
                                if let Err(e) = std::fs::remove_dir_all(&sandbox_work_dir) {
                                    eprintln!(
                                        "Warning: Failed to remove sandbox files {}: {}",
                                        sandbox_work_dir.display(),
                                        e
                                    );
                                } else {
                                    eprintln!(
                                        "Automatically cleaned up sandbox {} files and instance",
                                        box_id
                                    );
                                }
                            } else {
                                eprintln!(
                                    "Automatically cleaned up sandbox {} after execution",
                                    box_id
                                );
                            }
                        }
                        Err(e) => eprintln!("Warning: Failed to cleanup sandbox {}: {}", box_id, e),
                    }

                    if !result.success {
                        std::process::exit(1);
                    }
                } else {
                    eprintln!(
                        "Error: File '{}' not found in current directory or as absolute path",
                        command_arg
                    );
                    std::process::exit(1);
                }
            } else {
                // Multiple arguments or command - execute directly
                let result = isolate.execute_with_overrides(
                    &command,
                    None, // stdin
                    cpu,
                    mem,
                    time.or(wall_time),
                    None, // fd_limit
                )?;

                // Print execution results in JSON format
                let status_message = match result.status {
                    crate::types::ExecutionStatus::TimeLimit => "TLE".to_string(),
                    crate::types::ExecutionStatus::MemoryLimit => {
                        "Memory Limit Exceeded".to_string()
                    }
                    _ => format!("{:?}", result.status),
                };

                let json_result = serde_json::json!({
                    "status": status_message,
                    "exit_code": result.exit_code,
                    "stdout": result.stdout,
                    "stderr": result.stderr,
                    "wall_time": result.wall_time,
                    "cpu_time": result.cpu_time,
                    "memory_peak_kb": result.memory_peak / 1024,
                    "success": result.success,
                    "signal": result.signal,
                    "error_message": result.error_message
                });
                println!("{}", serde_json::to_string_pretty(&json_result).unwrap());

                // Automatic cleanup after execution (multiple arguments path)
                let cleanup_result = isolate.cleanup();
                match cleanup_result {
                    Ok(_) => {
                        // Also clean up the standardized files we created (if any)
                        let sandbox_work_dir = std::path::Path::new("/tmp/rustbox")
                            .join(format!("rustbox-{}", box_id));
                        if sandbox_work_dir.exists() {
                            if let Err(e) = std::fs::remove_dir_all(&sandbox_work_dir) {
                                eprintln!(
                                    "Warning: Failed to remove sandbox files {}: {}",
                                    sandbox_work_dir.display(),
                                    e
                                );
                            } else {
                                eprintln!(
                                    "Automatically cleaned up sandbox {} files and instance",
                                    box_id
                                );
                            }
                        } else {
                            eprintln!(
                                "Automatically cleaned up sandbox {} after execution",
                                box_id
                            );
                        }
                    }
                    Err(e) => eprintln!("Warning: Failed to cleanup sandbox {}: {}", box_id, e),
                }

                if !result.success {
                    std::process::exit(1);
                }
            }

            Ok(())
        }
        Commands::ExecuteCode {
            box_id,
            language,
            code,
            stdin,
            mem,
            time,
            cpu,
            wall_time,
            processes,
            strict,
        } => {
            CURRENT_BOX_ID.store(box_id, Ordering::Relaxed);

            // Security check for strict mode
            let is_root = unsafe { libc::getuid() } == 0;

            if strict && !is_root {
                eprintln!("❌ SECURITY ERROR: --strict mode requires root privileges");
                eprintln!("   Strict mode enforces full security isolation for untrusted code");
                eprintln!("   Run with sudo: sudo rustbox execute-code --strict ...");
                std::process::exit(1);
            }

            if !is_root {
                eprintln!("🚨 SECURITY WARNING: Running without root privileges!");
                eprintln!("   ⚠️  Resource limits will NOT be enforced");
                eprintln!("   ⚠️  Namespace isolation will NOT work");
                eprintln!("   ⚠️  Code can access host filesystem and network");
                eprintln!("   ⚠️  UNSUITABLE for untrusted code execution");
                eprintln!();
                eprintln!("   For secure execution of untrusted code, use:");
                eprintln!(
                    "   sudo rustbox execute-code --strict --box-id={} --language={} --code='...'",
                    box_id, language
                );
                eprintln!();

                // Add extra warning for production usage
                if !strict {
                    eprintln!("   💡 Use --strict flag to require root privileges and fail fast");
                    eprintln!();
                }
            }

            eprintln!(
                "Executing {} code in sandbox {} ({})",
                language,
                box_id,
                if strict {
                    "STRICT MODE"
                } else if is_root {
                    "ROOT MODE"
                } else {
                    "DEVELOPMENT MODE"
                }
            );

            // Load language-specific defaults from config.json first
            let mut config = rustbox::types::IsolateConfig::with_language_defaults(
                &language,
                format!("rustbox/{}", box_id),
            )?;
            config.strict_mode = strict; // Use user-specified strict mode

            // Apply CLI overrides if specified (these override config.json values)
            if let Some(mem) = mem {
                config.memory_limit = Some(mem * 1024 * 1024); // Convert MB to bytes
                eprintln!("🔧 CLI Override - Memory limit: {} MB", mem);
            }
            if let Some(cpu_limit) = cpu.or(time) {
                config.cpu_time_limit = Some(std::time::Duration::from_secs(cpu_limit));
                config.time_limit = Some(std::time::Duration::from_secs(cpu_limit));
                eprintln!("🔧 CLI Override - CPU time limit: {} seconds", cpu_limit);
            }
            if let Some(wall_limit) = wall_time {
                config.wall_time_limit = Some(std::time::Duration::from_secs(wall_limit));
                eprintln!("🔧 CLI Override - Wall time limit: {} seconds", wall_limit);
            }
            if let Some(proc_limit) = processes {
                config.process_limit = Some(proc_limit);
                eprintln!("🔧 CLI Override - Process limit: {}", proc_limit);
            }

            let mut isolate = rustbox::isolate::Isolate::new(config)?;

            // Execute code string directly
            let result = isolate.execute_code_string(
                &language,
                &code,
                stdin.as_deref(),
                cpu.or(time),
                mem,
                time.or(wall_time),
                None, // fd_limit
            )?;

            // Print execution results in JSON format
            let status_message = match result.status {
                crate::types::ExecutionStatus::TimeLimit => "TLE".to_string(),
                crate::types::ExecutionStatus::MemoryLimit => "Memory Limit Exceeded".to_string(),
                _ => format!("{:?}", result.status),
            };

            let json_result = serde_json::json!({
                "status": status_message,
                "exit_code": result.exit_code,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "wall_time": result.wall_time,
                "cpu_time": result.cpu_time,
                "memory_peak_kb": result.memory_peak / 1024,
                "success": result.success,
                "signal": result.signal,
                "error_message": result.error_message,
                "language": language
            });
            println!("{}", serde_json::to_string_pretty(&json_result).unwrap());

            if !result.success {
                std::process::exit(1);
            }

            Ok(())
        }
        Commands::Cleanup { box_id } => {
            eprintln!("Cleaning up sandbox with box-id: {}", box_id);

            let instance_id = format!("rustbox/{}", box_id);
            if let Some(isolate) = rustbox::isolate::Isolate::load(&instance_id)? {
                isolate.cleanup()?;
                eprintln!("Sandbox cleaned up successfully");
            } else {
                eprintln!("Sandbox {} not found", box_id);
            }
            Ok(())
        }
        Commands::CheckDeps { verbose } => check_language_dependencies(verbose),
    }
}

/// Perform comprehensive security subsystem checks
///
/// This function validates that all necessary security mechanisms are available
/// and properly configured on the host system.
fn perform_security_checks() {
    // Check cgroups availability for resource control
    if !crate::cgroup::cgroups_available() {
        eprintln!("⚠️  Warning: cgroups not available - resource limits will not be enforced");
        eprintln!("   Ensure /proc/cgroups and /sys/fs/cgroup are properly mounted");
        eprintln!("   Some contest systems may not function correctly without cgroups");
    } else {
        eprintln!("✅ cgroups v1 available - resource limits enabled");
    }

    // Check namespace support for process isolation
    if crate::namespace::NamespaceIsolation::is_supported() {
        eprintln!("✅ namespace isolation available - full process isolation enabled");
    } else {
        eprintln!("⚠️  Warning: namespace isolation not supported");
        eprintln!("   Limited process isolation capabilities available");
    }

    // Check filesystem security capabilities
    if std::path::Path::new("/proc/self/ns").exists() {
        eprintln!("✅ namespace filesystem available - isolation monitoring enabled");
    }

    // Validate critical system directories
    validate_system_directories();
}

/// Validate that critical system directories are properly configured
///
/// # Security Considerations
/// - Ensures /tmp is writable for sandbox operations
/// - Validates /proc and /sys are mounted for system information
/// - Checks that sensitive directories are protected
fn validate_system_directories() {
    // Check /tmp accessibility for sandbox operations
    if !std::path::Path::new("/tmp").exists() || !std::path::Path::new("/tmp").is_dir() {
        eprintln!("⚠️  Warning: /tmp directory not accessible");
        eprintln!("   Sandbox operations may fail without writable temporary space");
    }

    // Validate /proc filesystem for process monitoring
    if !std::path::Path::new("/proc/self").exists() {
        eprintln!("⚠️  Warning: /proc filesystem not mounted");
        eprintln!("   Process monitoring and resource tracking may be limited");
    }

    // Check /sys for cgroups and system information
    if !std::path::Path::new("/sys").exists() {
        eprintln!("⚠️  Warning: /sys filesystem not mounted");
        eprintln!("   Cgroups and hardware information may be unavailable");
    }

    // Validate that sensitive directories exist and are protected
    let sensitive_dirs = ["/etc", "/root", "/boot"];
    for dir in &sensitive_dirs {
        if !std::path::Path::new(dir).exists() {
            eprintln!("⚠️  Warning: {} directory not found", dir);
        }
    }
}

/// Check if all required language dependencies are installed
fn check_language_dependencies(verbose: bool) -> Result<()> {
    use std::process::Command;

    println!("🔍 Checking language dependencies...");
    println!();

    let mut all_ok = true;
    let mut missing_languages = Vec::new();

    // Define languages and their required commands
    let languages = [
        ("Python", vec![("python3", "--version")]),
        ("C++", vec![("gcc", "--version"), ("g++", "--version")]),
        ("Java", vec![("java", "-version"), ("javac", "-version")]),
    ];

    for (lang_name, commands) in &languages {
        let mut lang_ok = true;
        let mut versions = Vec::new();

        for (cmd, version_arg) in commands {
            match Command::new(cmd).arg(version_arg).output() {
                Ok(output) => {
                    if output.status.success() {
                        let version_info = if !output.stdout.is_empty() {
                            String::from_utf8_lossy(&output.stdout)
                        } else {
                            String::from_utf8_lossy(&output.stderr)
                        }
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string();

                        if verbose {
                            versions.push(format!("  {} -> {}", cmd, version_info.trim()));
                        }
                    } else {
                        lang_ok = false;
                        if verbose {
                            versions.push(format!("  {} -> FAILED", cmd));
                        }
                    }
                }
                Err(_) => {
                    lang_ok = false;
                    if verbose {
                        versions.push(format!("  {} -> NOT FOUND", cmd));
                    }
                }
            }
        }

        if lang_ok {
            println!("✅ {} - OK", lang_name);
            if verbose {
                for version in versions {
                    println!("{}", version);
                }
            }
        } else {
            println!("❌ {} - MISSING", lang_name);
            if verbose {
                for version in versions {
                    println!("{}", version);
                }
            }
            missing_languages.push(*lang_name);
            all_ok = false;
        }

        if verbose {
            println!();
        }
    }

    println!();

    if all_ok {
        println!("🎉 All language dependencies are installed!");
        println!("✅ RustBox is ready to use");

        if verbose {
            println!();
            println!("💡 Usage examples:");
            println!("  rustbox execute-code --strict --box-id=1 --language=python --code='print(\"Hello World\")'");
            println!("  rustbox execute-code --strict --box-id=2 --language=cpp --processes=10 --code='#include<iostream>...'");
        }

        Ok(())
    } else {
        println!(
            "❌ Missing language dependencies: {}",
            missing_languages.join(", ")
        );
        println!();
        println!("🔧 To install missing languages, run:");
        println!("   ./setup_languages.sh");
        println!();
        println!("Or install manually:");

        for lang in &missing_languages {
            match *lang {
                "Python" => println!("  • Python: sudo apt install python3 python3-pip"),
                "C++" => println!("  • C++: sudo apt install build-essential gcc g++"),
                "Java" => println!("  • Java: sudo apt install openjdk-17-jdk"),
                _ => {}
            }
        }

        std::process::exit(1);
    }
}
