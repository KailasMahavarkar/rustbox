/// Improved cleanup implementation for multi-process architecture
///
/// This module provides robust cleanup mechanisms inspired by IOI isolate
/// but implemented in idiomatic Rust with proper error handling and resource management.

use crate::types::{IsolateError, Result};
use nix::sys::signal::{self, Signal};
use nix::sys::wait::{self, WaitStatus};
use nix::unistd::{close, Pid};
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Process information for cleanup tracking
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: Pid,
    pub process_type: ProcessType,
    pub is_alive: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessType {
    
    
    Keeper,Proxy,Inside,
}

/// Cleanup manager for multi-process architecture
///
/// Handles reliable process cleanup with proper signal handling and resource management.
/// Inspired by IOI isolate's box_exit() function but implemented in Rust with RAII.
pub struct ProcessCleanupManager {
    processes: Mutex<HashMap<Pid, ProcessInfo>>,
    cleanup_started: AtomicBool,
    pipes: RefCell<(Vec<RawFd>, Vec<RawFd>)>, // (error_pipes, status_pipes)
    cgroup_enabled: bool,
}

impl ProcessCleanupManager {
    /// Create new cleanup manager
    pub fn new(cgroup_enabled: bool) -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
            cleanup_started: AtomicBool::new(false),
            pipes: RefCell::new((Vec::new(), Vec::new())),
            cgroup_enabled,
        }
    }

    /// Register a process for cleanup tracking
    pub fn register_process(&self, pid: Pid, process_type: ProcessType) {
        let mut processes = self.processes.lock().unwrap();
        processes.insert(pid, ProcessInfo {
            pid,
            process_type,
            is_alive: true,
        });
    }

    /// Perform emergency cleanup - kill all processes immediately
    ///
    /// This is equivalent to isolate's die() function behavior
    pub fn emergency_cleanup(&self, reason: &str) -> Result<()> {
        if self.cleanup_started.swap(true, Ordering::SeqCst) {
            return Ok(()); // Cleanup already in progress
        }

        eprintln!("Emergency cleanup triggered: {}", reason);
        self.cleanup_file_descriptors();

        let processes = self.processes.lock().unwrap();

        // Kill processes in reverse order: inside -> proxy -> keeper
        let mut inside_pids = Vec::new();
        let mut proxy_pids = Vec::new();
        let mut keeper_pids: Vec<Pid> = Vec::new();

        for (_, info) in processes.iter() {
            if !info.is_alive {
                continue;
            }

            match info.process_type {
                ProcessType::Inside => inside_pids.push(info.pid),
                ProcessType::Proxy => proxy_pids.push(info.pid),
                ProcessType::Keeper => keeper_pids.push(info.pid),
            }
        }

        // Kill inside processes first (most dangerous)
        for pid in inside_pids {
            self.kill_process_group(pid, "inside process")?;
        }

        // Kill proxy processes
        for pid in proxy_pids {
            if self.cgroup_enabled {
                // In cgroup mode, killing proxy kills all processes in the cgroup
                self.kill_process_group(pid, "proxy process (cgroup mode)")?;
            } else {
                // In non-cgroup mode, be more careful to preserve rusage
                self.kill_process_gracefully(pid, "proxy process")?;
            }
        }

        // Keeper processes should clean themselves up
        for pid in keeper_pids {
            self.kill_process_gracefully(pid, "keeper process")?;
        }

        Ok(())
    }

    /// Graceful cleanup - wait for processes to exit naturally, then force cleanup
    ///
    
    /// This is equivalent to isolate's box_exit() function
    pub fn graceful_cleanup(&self, timeout: Duration) -> Result<()> {
        if self.cleanup_started.swap(true, Ordering::SeqCst) {
            return Ok(()); // Cleanup already in progress, exit gracefully
        }

        log::debug!("Starting graceful cleanup...");
        let start_time = Instant::now();

        // Attempt to terminate all processes gracefully
        if let Err(e) = self.send_termination_signals() {
            log::warn!("Failed to send termination signals: {}", e);
        }

        // Wait for a reasonable time before forcing termination
        let wait_result = self.wait_for_processes(timeout);

        // If graceful shutdown failed or timed out, force cleanup
        if wait_result.is_err() || start_time.elapsed() >= timeout {
            log::warn!("Graceful cleanup timed out, forcing termination.");
            self.force_cleanup()?;
        }

        // Close all registered file descriptors
        self.cleanup_file_descriptors();

        log::info!("Graceful cleanup completed in {:?}.", start_time.elapsed());
        Ok(())
    }

    

    /// Kill a process and its process group with SIGKILL
    ///
    /// Equivalent to isolate's kill(-pid, SIGKILL); kill(pid, SIGKILL);
    fn kill_process_group(&self, pid: Pid, description: &str) -> Result<()> {
        // Kill process group first (negative PID)
        if let Err(e) = signal::kill(Pid::from_raw(-pid.as_raw()), Signal::SIGKILL) {
            eprintln!("Warning: Failed to kill process group for {}: {}", description, e);
        }

        // Kill the process itself
        if let Err(e) = signal::kill(pid, Signal::SIGKILL) {
            eprintln!("Warning: Failed to kill {}: {}", description, e);
        }

        Ok(())
    }

    /// Kill a process more gracefully (SIGTERM first, then SIGKILL)
    fn kill_process_gracefully(&self, pid: Pid, description: &str) -> Result<()> {
        // Try SIGTERM first
        if signal::kill(pid, Signal::SIGTERM).is_ok() {
            // Give it a moment to exit gracefully
            thread::sleep(Duration::from_millis(100));

            // Check if it's still alive
            if self.is_process_alive(pid) {
                // Force kill with SIGKILL
                if let Err(e) = signal::kill(pid, Signal::SIGKILL) {
                    eprintln!("Warning: Failed to force kill {}: {}", description, e);
                }
            }
        } else {
            // SIGTERM failed, try SIGKILL directly
            if let Err(e) = signal::kill(pid, Signal::SIGKILL) {
                eprintln!("Warning: Failed to kill {}: {}", description, e);
            }
        }

        Ok(())
    }

    
    /// Send termination signals to all processes
    fn send_termination_signals(&self) -> Result<()> {
        let processes = self.processes.lock().unwrap();

        for (_, info) in processes.iter() {
            if info.is_alive {
                let _ = signal::kill(info.pid, Signal::SIGTERM);
            }
        }

        Ok(())
    }

    

    
    /// Wait for processes to exit naturally
    fn wait_for_processes(&self, timeout: Duration) -> Result<()> {
        let start_time = Instant::now();

        loop {
            let mut all_exited = true;
            let processes = self.processes.lock().unwrap();

            for (_, info) in processes.iter() {
                if info.is_alive && self.is_process_alive(info.pid) {
                    all_exited = false;
                    break;
                }
            }

            if all_exited {
                return Ok(());
            }

            if start_time.elapsed() >= timeout {
                return Err(IsolateError::Process("Timeout waiting for processes".to_string()));
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    

    
    /// Force cleanup all remaining processes
    fn force_cleanup(&self) -> Result<()> {
        let processes = self.processes.lock().unwrap();

        for (_, info) in processes.iter() {
            if info.is_alive && self.is_process_alive(info.pid) {
                self.kill_process_group(info.pid, &format!("{:?}", info.process_type))?;
            }
        }

        Ok(())
    }

    

    /// Check if a process is still alive
    fn is_process_alive(&self, pid: Pid) -> bool {
        match wait::waitpid(pid, Some(wait::WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => true,
            Ok(_) => false, // Process has exited
            Err(_) => false, // Error means process doesn't exist
        }
    }

    /// Close all file descriptors
    fn cleanup_file_descriptors(&self) {
        let pipes = self.pipes.borrow();
        for &fd in &pipes.0 {
            let _ = close(fd);
        }

        for &fd in &pipes.1 {
            let _ = close(fd);
        }
    }

    /// Get cleanup statistics
    pub fn get_stats(&self) -> CleanupStats {
        let processes = self.processes.lock().unwrap();
        let mut stats = CleanupStats::default();

        stats.processes_cleaned = processes.len();
        stats.alive_processes = processes.values().filter(|p| p.is_alive).count();

        stats
    }
}

/// Statistics about cleanup operations
#[derive(Debug, Default)]
pub struct CleanupStats {
    pub processes_cleaned: usize,
    pub alive_processes: usize,
}

/// RAII wrapper that ensures cleanup happens on drop
///
/// This provides the Rust equivalent of isolate's cleanup guarantees
pub struct ProcessGuard {
    cleanup_manager: Arc<ProcessCleanupManager>,
}

impl ProcessGuard {
    pub fn new(cleanup_manager: Arc<ProcessCleanupManager>) -> Self {
        Self {
            cleanup_manager,
        }
    }
}

impl Drop for ProcessGuard {
    /// Automatic cleanup on drop - this is the Rust way of ensuring cleanup
    ///
    /// Unlike C's manual cleanup, Rust's RAII guarantees this will run
    /// even if the program panics or exits unexpectedly
    fn drop(&mut self) {
        if !self.cleanup_manager.cleanup_started.load(Ordering::SeqCst) {
            if let Err(e) = self.cleanup_manager.emergency_cleanup("ProcessGuard dropped") {
                eprintln!("Warning: Cleanup failed during drop: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_manager_creation() {
        let manager = ProcessCleanupManager::new(false);
        let stats = manager.get_stats();
        assert_eq!(stats.processes_cleaned, 0);
    }

    #[test]
    fn test_process_registration() {
        let manager = ProcessCleanupManager::new(false);
        let pid = Pid::from_raw(12345);

        manager.register_process(pid, ProcessType::Inside);
        let stats = manager.get_stats();
        assert_eq!(stats.processes_cleaned, 1);
    }

    #[test]
    fn test_process_guard_raii() {
        let manager = Arc::new(ProcessCleanupManager::new(false));
        let _guard = ProcessGuard::new(manager.clone());

        // Guard should automatically cleanup on drop
    }
}
