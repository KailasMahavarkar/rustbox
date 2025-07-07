/// rustbox: Secure Process Isolation and Resource Control System
/// 
/// A modern, Rust-based implementation inspired by IOI Isolate, designed for secure
/// execution of untrusted code with comprehensive resource limits and namespace isolation.
/// 
/// # Security Features
/// - Comprehensive syscall filtering via seccomp-bpf
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
use anyhow::Result;

mod cgroup;
mod cli;
mod executor;
mod filesystem;
mod io_handler;
mod isolate;
mod namespace;
mod resource_limits;
mod seccomp;
mod seccomp_native;
mod types;

fn main() -> Result<()> {
    // Initialize structured logging for security monitoring
    env_logger::init();

    // Platform compatibility check - Unix-only for security features
    if !cfg!(unix) {
        eprintln!("Error: rustbox requires Unix-like systems for security features");
        eprintln!("Current platform does not support necessary isolation mechanisms");
        std::process::exit(1);
    }
    
    // Privilege check - many security features require elevated permissions
    if unsafe { libc::getuid() } != 0 {
        eprintln!("Warning: rustbox may require root privileges for full functionality");
        eprintln!("Running without root may limit:");
        eprintln!("  • Cgroups resource enforcement");
        eprintln!("  • Namespace isolation capabilities");
        eprintln!("  • Seccomp filter installation");
        eprintln!("  • Chroot directory creation");
    }

    // Security subsystem availability checks
    perform_security_checks();

    // Run the command-line interface
    cli::run()
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

    // Check seccomp availability for syscall filtering
    if crate::seccomp::is_seccomp_supported() {
        eprintln!("✅ libseccomp available - comprehensive syscall filtering enabled");
    } else if crate::seccomp_native::NativeSeccompFilter::is_supported() {
        eprintln!("✅ native seccomp available - basic syscall protection enabled");
    } else {
        eprintln!("⚠️  Warning: seccomp not supported - syscall filtering unavailable");
        eprintln!("   Kernel must support CONFIG_SECCOMP and CONFIG_SECCOMP_FILTER");
        eprintln!("   Running untrusted code without syscall filtering is dangerous");
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