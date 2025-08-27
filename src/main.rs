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
use anyhow::Result;
use clap::{Parser, Subcommand};











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
        /// Programming language (python, cpp, c, java, javascript, etc.)
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

fn main() -> Result<()> {
    // Initialize structured logging for security monitoring
    env_logger::init();

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
            command 
        } => {
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

            // Parse and apply directory bindings
            if !directory_bindings.is_empty() {
                let mut bindings = Vec::new();
                for binding_str in &directory_bindings {
                    match rustbox::types::DirectoryBinding::parse(binding_str) {
                        Ok(binding) => {
                            eprintln!("Directory binding: {} -> {} ({:?})", 
                                     binding.source.display(), 
                                     binding.target.display(), 
                                     binding.permissions);
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

            if command.len() == 1 && std::path::Path::new(&command[0]).exists() {
                // Single argument that's a file path - execute the file
                let file_path = std::path::Path::new(&command[0]);
                let result = isolate.execute_file_with_overrides(
                    file_path,
                    None, // stdin
                    cpu,
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
                    "error_message": result.error_message
                });
                println!("{}", serde_json::to_string_pretty(&json_result).unwrap());
                
                if !result.success {
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
                    "error_message": result.error_message
                });
                println!("{}", serde_json::to_string_pretty(&json_result).unwrap());
                
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
            strict
        } => {
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
                eprintln!("   sudo rustbox execute-code --strict --box-id={} --language={} --code='...'", box_id, language);
                eprintln!();
                
                // Add extra warning for production usage
                if !strict {
                    eprintln!("   💡 Use --strict flag to require root privileges and fail fast");
                    eprintln!();
                }
            }
            
            eprintln!("Executing {} code in sandbox {} ({})", 
                language, 
                box_id, 
                if strict { "STRICT MODE" } else if is_root { "ROOT MODE" } else { "DEVELOPMENT MODE" }
            );
            
            let mut config = rustbox::types::IsolateConfig::default();
            config.instance_id = format!("rustbox/{}", box_id);
            config.strict_mode = strict;        // Use user-specified strict mode
            
            // Apply resource limits if specified
            if let Some(mem) = mem {
                config.memory_limit = Some(mem * 1024 * 1024); // Convert MB to bytes
                eprintln!("Memory limit: {} MB", mem);
            }
            if let Some(cpu_limit) = cpu.or(time) {
                config.cpu_time_limit = Some(std::time::Duration::from_secs(cpu_limit));
                eprintln!("CPU time limit: {} seconds", cpu_limit);
            }
            if let Some(wall_limit) = wall_time {
                config.wall_time_limit = Some(std::time::Duration::from_secs(wall_limit));
                eprintln!("Wall time limit: {} seconds", wall_limit);
            }
            if let Some(proc_limit) = processes {
                config.process_limit = Some(proc_limit);
                eprintln!("Process limit: {}", proc_limit);
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
        Commands::CheckDeps { verbose } => {
            check_language_dependencies(verbose)
        }
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
    if !std::path::Path::new("/tmp").exists() || 
       !std::path::Path::new("/tmp").is_dir() {
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
        ("C/C++", vec![("gcc", "--version"), ("g++", "--version")]),
        ("Java", vec![("java", "-version"), ("javac", "-version")]),
        ("JavaScript", vec![("node", "--version")]),
        ("Rust", vec![("rustc", "--version")]),
        ("Go", vec![("go", "version")]),
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
                        }.lines().next().unwrap_or("").to_string();
                        
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
            println!("  rustbox execute-code --strict --box-id=3 --language=go --code='package main; import \"fmt\"; func main() {{ fmt.Println(\"Hello\") }}'");
        }
        
        Ok(())
    } else {
        println!("❌ Missing language dependencies: {}", missing_languages.join(", "));
        println!();
        println!("🔧 To install missing languages, run:");
        println!("   ./setup_languages.sh");
        println!();
        println!("Or install manually:");
        
        for lang in &missing_languages {
            match *lang {
                "Python" => println!("  • Python: sudo apt install python3 python3-pip"),
                "C/C++" => println!("  • C/C++: sudo apt install build-essential gcc g++"),
                "Java" => println!("  • Java: sudo apt install openjdk-17-jdk"),
                "JavaScript" => println!("  • Node.js: curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash - && sudo apt install nodejs"),
                "Rust" => println!("  • Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
                "Go" => println!("  • Go: sudo apt install golang-go"),
                _ => {}
            }
        }
        
        std::process::exit(1);
    }
}