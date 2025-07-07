/// Command Line Interface for the mini-isolate system
use crate::isolate::Isolate;
use crate::types::{ExecutionResult, ExecutionStatus, IsolateConfig};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "mini-isolate")]
#[command(about = "A process isolation and resource control system inspired by IOI Isolate", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new isolate instance
    Init {
        /// Instance identifier (box-id)
        #[arg(short, long, default_value = "0")]
        box_id: String,

        /// Working directory for the isolate
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Memory limit in MB
        #[arg(short, long, default_value = "128")]
        mem: u64,

        /// Time limit in seconds
        #[arg(short, long, default_value = "10")]
        time: u64,

        /// Wall clock time limit in seconds (defaults to 2x time limit)
        #[arg(short, long)]
        wall_time: Option<u64>,

        /// Process limit
        #[arg(short, long, default_value = "1")]
        processes: u32,

        /// File size limit in MB
        #[arg(short, long, default_value = "64")]
        fsize: u64,

        /// Stack size limit in MB
        #[arg(long, default_value = "8")]
        stack: u64,

        /// Core dump size limit in MB (0 to disable)
        #[arg(long, default_value = "0")]
        core: u64,

        /// File descriptor limit (max open files)
        #[arg(long, default_value = "64")]
        fd_limit: u64,

        /// Disk quota limit in MB (0 to disable)
        #[arg(long)]
        quota: Option<u64>,

        /// Strict mode: require root privileges and fail if cgroups unavailable
        #[arg(long)]
        strict: bool,
    },

    /// Run a program in the isolate
    Run {
        /// Instance identifier (box-id)
        #[arg(short, long, default_value = "0")]
        box_id: String,

        /// Program to run (path to executable or script)
        program: String,

        /// Arguments for the program
        #[arg(last = true)]
        args: Vec<String>,

        /// Input file (stdin will be redirected from this file)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output results to JSON file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output meta information to file (isolate-compatible format)
        #[arg(short = 'M', long = "meta")]
        meta: Option<PathBuf>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Silent mode - suppress all output except results
        #[arg(long)]
        silent: bool,

        /// Override CPU time limit in seconds
        #[arg(long)]
        max_cpu: Option<u64>,

        /// Override memory limit in MB
        #[arg(long)]
        max_memory: Option<u64>,

        /// Override execution time limit in seconds
        #[arg(long)]
        max_time: Option<u64>,

        /// Override file descriptor limit (max open files)
        #[arg(long)]
        fd_limit: Option<u64>,

        /// Strict mode: require root privileges and fail if cgroups unavailable
        #[arg(long)]
        strict: bool,

        /// Chroot directory for filesystem isolation
        #[arg(long)]
        chroot: Option<PathBuf>,

        /// Environment variable (can be used multiple times): -E VAR=value
        #[arg(short = 'E', long = "env", value_name = "VAR=VALUE")]
        env_vars: Vec<String>,

        /// Inherit all environment variables from parent
        #[arg(long)]
        full_env: bool,

        /// Inherit file descriptors from parent process
        #[arg(long)]
        inherit_fds: bool,

        /// Redirect stdout to file
        #[arg(long)]
        stdout_file: Option<PathBuf>,

        /// Redirect stderr to file
        #[arg(long)]
        stderr_file: Option<PathBuf>,        /// Redirect stdin from file
        #[arg(long)]
        stdin_file: Option<PathBuf>,

        /// Enable TTY support for interactive programs
        #[arg(long)]
        enable_tty: bool,

        /// Use pipes for real-time I/O instead of files
        #[arg(long)]
        use_pipes: bool,

        /// I/O buffer size in bytes
        #[arg(long, default_value = "8192")]
        io_buffer_size: usize,

        /// Text encoding for I/O operations
        #[arg(long, default_value = "utf-8")]
        text_encoding: String,

        /// Disable PID namespace isolation
        #[arg(long)]
        no_pid_namespace: bool,

        /// Disable mount namespace isolation
        #[arg(long)]
        no_mount_namespace: bool,

        /// Disable network namespace isolation
        #[arg(long)]
        no_network_namespace: bool,

        /// Enable user namespace isolation (experimental)
        #[arg(long)]
        enable_user_namespace: bool,

        /// Run as specific user ID (requires root privileges)
        #[arg(long)]
        as_uid: Option<u32>,

        /// Run as specific group ID (requires root privileges)
        #[arg(long)]
        as_gid: Option<u32>,
    },

    /// Execute a source file directly
    Execute {
        /// Instance identifier (box-id)
        #[arg(short, long, default_value = "0")]
        box_id: String,

        /// Source file to execute
        #[arg(short, long)]
        source: PathBuf,

        /// Input file (stdin)
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output results to JSON file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output meta information to file (isolate-compatible format)
        #[arg(short = 'M', long = "meta")]
        meta: Option<PathBuf>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Silent mode - suppress all output except results
        #[arg(long)]
        silent: bool,

        /// Override CPU time limit in seconds
        #[arg(long)]
        max_cpu: Option<u64>,

        /// Override memory limit in MB
        #[arg(long)]
        max_memory: Option<u64>,

        /// Override execution time limit in seconds
        #[arg(long)]
        max_time: Option<u64>,

        /// Override file descriptor limit (max open files)
        #[arg(long)]
        fd_limit: Option<u64>,

        /// Strict mode: require root privileges and fail if cgroups unavailable
        #[arg(long)]
        strict: bool,

        /// Environment variable (can be used multiple times): -E VAR=value
        #[arg(short = 'E', long = "env", value_name = "VAR=VALUE")]
        env_vars: Vec<String>,

        /// Inherit all environment variables from parent
        #[arg(long)]
        full_env: bool,

        /// Inherit file descriptors from parent process
        #[arg(long)]
        inherit_fds: bool,

        /// Redirect stdout to file
        #[arg(long)]
        stdout_file: Option<PathBuf>,

        /// Redirect stderr to file
        #[arg(long)]
        stderr_file: Option<PathBuf>,

        /// Redirect stdin from file
        #[arg(long)]
        stdin_file: Option<PathBuf>,

        /// Enable TTY support for interactive programs
        #[arg(long)]
        enable_tty: bool,

        /// Use pipes for real-time I/O instead of files
        #[arg(long)]
        use_pipes: bool,

        /// I/O buffer size in bytes
        #[arg(long, default_value = "8192")]
        io_buffer_size: usize,

        /// Text encoding for I/O operations
        #[arg(long, default_value = "utf-8")]
        text_encoding: String,

        /// Disable PID namespace isolation
        #[arg(long)]
        no_pid_namespace: bool,

        /// Disable mount namespace isolation
        #[arg(long)]
        no_mount_namespace: bool,

        /// Disable network namespace isolation
        #[arg(long)]
        no_network_namespace: bool,

        /// Enable user namespace isolation (experimental)
        #[arg(long)]
        enable_user_namespace: bool,

        /// Run as specific user ID (requires root privileges)
        #[arg(long)]
        as_uid: Option<u32>,

        /// Run as specific group ID (requires root privileges)
        #[arg(long)]
        as_gid: Option<u32>,
    },
    List,

    /// Clean up isolate instance(s)
    Cleanup {
        /// Instance identifier (box-id) to clean up
        #[arg(short, long)]
        box_id: Option<String>,

        /// Clean up all instances
        #[arg(short, long)]
        all: bool,
    },

    /// Show system information
    Info {
        /// Show detailed cgroup information
        #[arg(short, long)]
        cgroups: bool,
    },
}

fn parse_environment_vars(env_vars: &[String], full_env: bool) -> Vec<(String, String)> {
    let mut environment = Vec::new();

    // If full_env is specified, inherit all current environment variables
    if full_env {
        for (key, value) in std::env::vars() {
            environment.push((key, value));
        }
    }

    // Parse custom environment variables
    for env_var in env_vars {
        if let Some(pos) = env_var.find('=') {
            let key = env_var[..pos].to_string();
            let value = env_var[pos + 1..].to_string();

            // Remove existing entry if present (custom vars override inherited ones)
            environment.retain(|(k, _)| k != &key);
            environment.push((key, value));
        } else {
            eprintln!("Warning: Invalid environment variable format: {}", env_var);
        }
    }

    environment
}

fn write_meta_file(meta_path: &Path, result: &ExecutionResult) -> anyhow::Result<()> {
    let mut content = String::new();

    // Write timing information
    content.push_str(&format!("time:{:.3}\n", result.cpu_time));
    content.push_str(&format!("time-wall:{:.3}\n", result.wall_time));

    // Write memory usage
    content.push_str(&format!("max-rss:{}\n", result.memory_peak));

    // Write exit information
    match result.status {
        ExecutionStatus::Success => {
            content.push_str(&format!("exitcode:{}\n", result.exit_code.unwrap_or(0)));
        }
        ExecutionStatus::TimeLimit => {
            content.push_str("status:TO\n");
            content.push_str("message:Time limit exceeded\n");
        }
        ExecutionStatus::MemoryLimit => {
            content.push_str("status:RE\n");
            content.push_str("message:Memory limit exceeded\n");
        }
        ExecutionStatus::RuntimeError => {
            content.push_str(&format!("exitcode:{}\n", result.exit_code.unwrap_or(1)));
        }
        ExecutionStatus::Signaled => {
            if let Some(signal) = result.signal {
                content.push_str(&format!("exitsig:{}\n", signal));
            }
            content.push_str("killed:1\n");
        }
        ExecutionStatus::SecurityViolation => {
            content.push_str("status:SG\n");
            content.push_str("message:Security violation\n");
        }
        ExecutionStatus::ProcessLimit => {
            content.push_str("status:RE\n");
            content.push_str("message:Process limit exceeded\n");
        }
        ExecutionStatus::FileSizeLimit => {
            content.push_str("status:RE\n");
            content.push_str("message:File size limit exceeded\n");
        }
        ExecutionStatus::StackLimit => {
            content.push_str("status:RE\n");
            content.push_str("message:Stack limit exceeded\n");
        }
        ExecutionStatus::CoreLimit => {
            content.push_str("status:RE\n");
            content.push_str("message:Core dump limit exceeded\n");
        }
        ExecutionStatus::DiskQuotaExceeded => {
            content.push_str("status:RE\n");
            content.push_str("message:Disk quota exceeded\n");
        }
        ExecutionStatus::InternalError => {
            content.push_str("status:XX\n");
            if let Some(ref error_msg) = result.error_message {
                content.push_str(&format!("message:{}\n", error_msg));
            } else {
                content.push_str("message:Internal error\n");
            }
        }
    }

    std::fs::write(meta_path, content)?;
    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            box_id,
            dir,
            mem,
            time,
            wall_time,
            processes,
            fsize,
            stack,
            core,
            fd_limit,
            quota,
            strict,
        } => {
            let mut config = IsolateConfig {
                instance_id: box_id.clone(),
                strict_mode: strict,
                ..Default::default()
            };

            // Set working directory
            if let Some(workdir) = dir {
                config.workdir = workdir;
            } else {
                let mut default_dir = std::env::temp_dir();
                default_dir.push("mini-isolate");
                default_dir.push(&box_id);
                config.workdir = default_dir;
            }

            // Set resource limits
            config.memory_limit = Some(mem * 1024 * 1024); // Convert MB to bytes
            config.time_limit = Some(Duration::from_secs(time));
            config.cpu_time_limit = Some(Duration::from_secs(time));
            config.wall_time_limit = Some(Duration::from_secs(wall_time.unwrap_or(time * 2)));
            config.process_limit = Some(processes);
            config.file_size_limit = Some(fsize * 1024 * 1024); // Convert MB to bytes
            config.stack_limit = Some(stack * 1024 * 1024); // Convert MB to bytes
            config.core_limit = Some(core * 1024 * 1024); // Convert MB to bytes
            config.fd_limit = Some(fd_limit); // File descriptor limit (no conversion needed)
            config.disk_quota = quota.map(|q| q * 1024 * 1024); // Convert MB to bytes

            let isolate = Isolate::new(config)?;
            println!(
                "Isolate instance '{}' initialized at: {}",
                box_id,
                isolate.config().workdir.display()
            );
        }

        Commands::Run {
            box_id,
            program,
            args,
            input,
            output,
            meta,
            verbose,
            silent,
            max_cpu,
            max_memory,
            max_time,
            fd_limit,
            strict,
            chroot,
            env_vars,
            full_env,
            inherit_fds,
            stdout_file,
            stderr_file,
            stdin_file,
            enable_tty,
            use_pipes,
            io_buffer_size,
            text_encoding,
            no_pid_namespace: _,
            no_mount_namespace: _,
            no_network_namespace: _,
            enable_user_namespace: _,
            as_uid,
            as_gid,
        } => {
            let mut isolate = match Isolate::load(&box_id)? {
                Some(mut isolate) => {
                    // Update configuration if specified
                    let mut config = isolate.config().clone();
                    if strict {
                        config.strict_mode = true;
                    }

                    // Update chroot directory - enable by default for security
                    if let Some(chroot_dir) = chroot {
                        config.chroot_dir = Some(chroot_dir);
                    } else if config.strict_mode {
                        // In strict mode, create a default chroot jail
                        let mut chroot_path = config.workdir.clone();
                        chroot_path.push("jail");
                        config.chroot_dir = Some(chroot_path);
                    }

                    // Update environment variables
                    config.environment = parse_environment_vars(&env_vars, full_env);

                    // Update inherit_fds
                    config.inherit_fds = inherit_fds;

                    // Update I/O redirection
                    config.stdout_file = stdout_file;
                    config.stderr_file = stderr_file;

                    // Set uid/gid if specified
                    config.uid = as_uid;
                    config.gid = as_gid;
                    config.stdin_file = stdin_file;
                    config.enable_tty = enable_tty;
                    config.use_pipes = use_pipes;
                    config.io_buffer_size = io_buffer_size;
                    config.text_encoding = text_encoding;

                    isolate = Isolate::new(config)?;
                    isolate
                }
                None => {
                    eprintln!("Isolate instance '{}' not found. Run 'init' first.", box_id);
                    std::process::exit(1);
                }
            };

            // Read input data if specified
            let stdin_data = if let Some(input_file) = input {
                Some(std::fs::read_to_string(input_file)?)
            } else {
                None
            };

            // Prepare command
            let mut command = vec![program];
            command.extend(args);

            // Execute command with optional overrides
            let result = if max_cpu.is_some() || max_memory.is_some() || max_time.is_some() || fd_limit.is_some() {
                isolate.execute_with_overrides(
                    &command,
                    stdin_data.as_deref(),
                    max_cpu,
                    max_memory,
                    max_time,
                    fd_limit,
                )?
            } else {
                isolate.execute(&command, stdin_data.as_deref())?
            };

            // Handle output
            if let Some(ref output_file) = output {
                let json_output = serde_json::to_string_pretty(&result)?;
                std::fs::write(output_file, json_output)?;
                if !silent {
                    println!("Results written to output file");
                }
            }

            // Write meta file if specified
            if let Some(meta_file) = meta {
                write_meta_file(&meta_file, &result)?;
                if !silent {
                    println!("Meta information written to meta file");
                }
            }

            if !silent && output.is_none() {
                // Print summary
                println!("Status: {:?}", result.status);
                println!("Exit code: {:?}", result.exit_code);
                println!(
                    "Time: {:.3}s (wall), {:.3}s (CPU)",
                    result.wall_time, result.cpu_time
                );
                println!("Memory peak: {} KB", result.memory_peak / 1024);

                if verbose || result.status != ExecutionStatus::Success {
                    if !result.stdout.is_empty() {
                        println!("\n--- STDOUT ---");
                        println!("{}", result.stdout);
                    }

                    if !result.stderr.is_empty() {
                        println!("\n--- STDERR ---");
                        println!("{}", result.stderr);
                    }

                    if let Some(error_msg) = &result.error_message {
                        println!("\n--- ERROR ---");
                        println!("{}", error_msg);
                    }
                }
            }

            // Exit with appropriate code
            match result.status {
                ExecutionStatus::Success => std::process::exit(0),
                ExecutionStatus::RuntimeError => std::process::exit(result.exit_code.unwrap_or(1)),
                ExecutionStatus::TimeLimit => std::process::exit(2),
                ExecutionStatus::MemoryLimit => std::process::exit(3),
                ExecutionStatus::SecurityViolation => std::process::exit(4),
                ExecutionStatus::InternalError => std::process::exit(5),
                _ => std::process::exit(1),
            }
        }

        Commands::Execute {
            box_id,
            source,
            input,
            output,
            meta,
            verbose,
            silent,
            max_cpu,
            max_memory,
            max_time,
            fd_limit,
            strict,
            env_vars,
            full_env,
            inherit_fds,
            stdout_file,
            stderr_file,
            stdin_file: _,
            enable_tty: _,
            use_pipes: _,
            io_buffer_size: _,
            text_encoding: _,
            no_pid_namespace: _,
            no_mount_namespace: _,
            no_network_namespace: _,
            enable_user_namespace: _,
            as_uid: _,
            as_gid: _,
        } => {
            let mut isolate = match Isolate::load(&box_id)? {
                Some(mut isolate) => {
                    // Update configuration if specified
                    let mut config = isolate.config().clone();
                    if strict {
                        config.strict_mode = true;
                    }

                    // Update environment variables
                    config.environment = parse_environment_vars(&env_vars, full_env);

                    // Update inherit_fds
                    config.inherit_fds = inherit_fds;

                    // Update I/O redirection
                    config.stdout_file = stdout_file;
                    config.stderr_file = stderr_file;

                    isolate = Isolate::new(config)?;
                    isolate
                }
                None => {
                    eprintln!("Isolate instance '{}' not found. Run 'init' first.", box_id);
                    std::process::exit(1);
                }
            };

            // Read input data if specified
            let stdin_data = if let Some(input_file) = input {
                Some(std::fs::read_to_string(input_file)?)
            } else {
                None
            };

            // Execute source file with optional overrides
            let result = if max_cpu.is_some() || max_memory.is_some() || max_time.is_some() || fd_limit.is_some() {
                isolate.execute_file_with_overrides(
                    &source,
                    stdin_data.as_deref(),
                    max_cpu,
                    max_memory,
                    max_time,
                    fd_limit,
                )?
            } else {
                isolate.execute_file(&source, stdin_data.as_deref())?
            };

            // Handle output (same as Run command)
            if let Some(ref output_file) = output {
                let json_output = serde_json::to_string_pretty(&result)?;
                std::fs::write(output_file, json_output)?;
                if !silent {
                    println!("Results written to output file");
                }
            }

            // Write meta file if specified
            if let Some(meta_file) = meta {
                write_meta_file(&meta_file, &result)?;
                if !silent {
                    println!("Meta information written to meta file");
                }
            }

            if !silent && output.is_none() {
                println!("Status: {:?}", result.status);
                println!("Exit code: {:?}", result.exit_code);
                println!(
                    "Time: {:.3}s (wall), {:.3}s (CPU)",
                    result.wall_time, result.cpu_time
                );
                println!("Memory peak: {} KB", result.memory_peak / 1024);

                if verbose || result.status != ExecutionStatus::Success {
                    if !result.stdout.is_empty() {
                        println!("\n--- STDOUT ---");
                        println!("{}", result.stdout);
                    }

                    if !result.stderr.is_empty() {
                        println!("\n--- STDERR ---");
                        println!("{}", result.stderr);
                    }

                    if let Some(error_msg) = &result.error_message {
                        println!("\n--- ERROR ---");
                        println!("{}", error_msg);
                    }
                }
            }

            // Exit with appropriate code based on result
            match result.status {
                ExecutionStatus::Success => std::process::exit(0),
                ExecutionStatus::RuntimeError => std::process::exit(result.exit_code.unwrap_or(1)),
                ExecutionStatus::TimeLimit => std::process::exit(2),
                ExecutionStatus::MemoryLimit => std::process::exit(3),
                ExecutionStatus::SecurityViolation => std::process::exit(4),
                ExecutionStatus::InternalError => std::process::exit(5),
                _ => std::process::exit(1),
            }
        }

        Commands::List => {
            let instances = Isolate::list_all()?;
            if instances.is_empty() {
                println!("No isolate instances found.");
            } else {
                println!("Available isolate instances:");
                for instance_id in instances {
                    if let Ok(Some(isolate)) = Isolate::load(&instance_id) {
                        println!("  {} - {}", instance_id, isolate.config().workdir.display());
                    }
                }
            }
        }

        Commands::Cleanup { box_id, all } => {
            if all {
                let instances = Isolate::list_all()?;
                for instance_id in instances {
                    if let Ok(Some(isolate)) = Isolate::load(&instance_id) {
                        isolate.cleanup()?;
                        println!("Cleaned up isolate instance: {}", instance_id);
                    }
                }
                println!("All isolate instances cleaned up.");
            } else if let Some(instance_id) = box_id {
                if let Ok(Some(isolate)) = Isolate::load(&instance_id) {
                    isolate.cleanup()?;
                    println!("Cleaned up isolate instance: {}", instance_id);
                } else {
                    eprintln!("Isolate instance '{}' not found.", instance_id);
                    std::process::exit(1);
                }
            } else {
                eprintln!("Specify --box-id <ID> or --all");
                std::process::exit(1);
            }
        }

        Commands::Info { cgroups } => {
            println!("Mini-Isolate System Information");
            println!("==============================");

            // Check cgroup support
            if crate::cgroup::cgroups_available() {
                println!("Cgroups: Available");
                if let Ok(mount_point) = crate::cgroup::get_cgroup_mount() {
                    println!("Cgroup mount: {}", mount_point);
                }

                if cgroups {
                    // Show detailed cgroup info
                    println!("\nCgroup Controllers:");
                    if std::path::Path::new("/proc/cgroups").exists() {
                        let content = std::fs::read_to_string("/proc/cgroups")?;
                        println!("{}", content);
                    }
                }
            } else {
                println!("Cgroups: Not available");
            }

            // Show system limits
            println!("\nSystem Information:");
            println!("Platform: {}", std::env::consts::OS);
            println!("Architecture: {}", std::env::consts::ARCH);

            // Show active instances
            let instances = Isolate::list_all()?;
            println!("Active instances: {}", instances.len());
        }
    }

    Ok(())
}
