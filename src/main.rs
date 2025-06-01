/// Mini-Isolate: A process isolation and resource control system
/// Inspired by IOI Isolate, focused on secure process execution with cgroup-v1 support
use anyhow::Result;

mod cgroup;
mod cli;
mod executor;
mod isolate;
mod types;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Check if we're running on a supported platform
    if !cfg!(unix) {
        eprintln!("Error: mini-isolate currently only supports Unix-like systems");
        std::process::exit(1);
    }
    // Check if we have necessary permissions
    if unsafe { libc::getuid() } != 0 {
        eprintln!("Warning: mini-isolate may require root privileges for full functionality");
        eprintln!("Some features like cgroups may not work without proper permissions");
    }

    // Check cgroup availability
    if !crate::cgroup::cgroups_available() {
        eprintln!("Warning: cgroups not available - resource limits will not be enforced");
        eprintln!("Make sure /proc/cgroups and /sys/fs/cgroup are available");
    }

    // Run the CLI
    cli::run()
}
