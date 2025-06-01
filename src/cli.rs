/// Command Line Interface for the mini-isolate system
use crate::isolate::Isolate;
use crate::types::{IsolateConfig, ExecutionStatus};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
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
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
        
        /// Override CPU time limit in seconds
        #[arg(long)]
        max_cpu: Option<u64>,
        
        /// Override memory limit in MB
        #[arg(long)]
        max_memory: Option<u64>,
        
        /// Override execution time limit in seconds
        #[arg(long)]
        max_time: Option<u64>,
        
        /// Strict mode: require root privileges and fail if cgroups unavailable
        #[arg(long)]
        strict: bool,
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
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
        
        /// Override CPU time limit in seconds
        #[arg(long)]
        max_cpu: Option<u64>,
        
        /// Override memory limit in MB
        #[arg(long)]
        max_memory: Option<u64>,
        
        /// Override execution time limit in seconds
        #[arg(long)]
        max_time: Option<u64>,
        
        /// Strict mode: require root privileges and fail if cgroups unavailable
        #[arg(long)]
        strict: bool,
    },
    
    /// List all isolate instances
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
            
            let isolate = Isolate::new(config)?;
            println!("Isolate instance '{}' initialized at: {}", 
                     box_id, isolate.config().workdir.display());
        },
        
        Commands::Run { 
            box_id, 
            program, 
            args, 
            input, 
            output, 
            verbose,
            max_cpu,
            max_memory,
            max_time,
            strict,
        } => {
            let mut isolate = match Isolate::load(&box_id)? {
                Some(mut isolate) => {
                    // Update strict mode if specified
                    if strict {
                        let mut config = isolate.config().clone();
                        config.strict_mode = true;
                        isolate = Isolate::new(config)?;
                    }
                    isolate
                },
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
            let result = if max_cpu.is_some() || max_memory.is_some() || max_time.is_some() {
                isolate.execute_with_overrides(&command, stdin_data.as_deref(), max_cpu, max_memory, max_time)?
            } else {
                isolate.execute(&command, stdin_data.as_deref())?
            };

            // Handle output
            if let Some(output_file) = output {
                let json_output = serde_json::to_string_pretty(&result)?;
                std::fs::write(output_file, json_output)?;
                println!("Results written to output file");
            } else {
                // Print summary
                println!("Status: {:?}", result.status);
                println!("Exit code: {:?}", result.exit_code);
                println!("Time: {:.3}s (wall), {:.3}s (CPU)", result.wall_time, result.cpu_time);
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
        },

        Commands::Execute { 
            box_id, 
            source, 
            input, 
            output, 
            verbose,
            max_cpu,
            max_memory,
            max_time,
            strict,
        } => {
            let mut isolate = match Isolate::load(&box_id)? {
                Some(mut isolate) => {
                    // Update strict mode if specified
                    if strict {
                        let mut config = isolate.config().clone();
                        config.strict_mode = true;
                        isolate = Isolate::new(config)?;
                    }
                    isolate
                },
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
            let result = if max_cpu.is_some() || max_memory.is_some() || max_time.is_some() {
                isolate.execute_file_with_overrides(&source, stdin_data.as_deref(), max_cpu, max_memory, max_time)?
            } else {
                isolate.execute_file(&source, stdin_data.as_deref())?
            };

            // Handle output (same as Run command)
            if let Some(output_file) = output {
                let json_output = serde_json::to_string_pretty(&result)?;
                std::fs::write(output_file, json_output)?;
                println!("Results written to output file");
            } else {
                println!("Status: {:?}", result.status);
                println!("Exit code: {:?}", result.exit_code);
                println!("Time: {:.3}s (wall), {:.3}s (CPU)", result.wall_time, result.cpu_time);
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
        },
        
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
        },
        
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
        },
        
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
        },
    }
    
    Ok(())
}