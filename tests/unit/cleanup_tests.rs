//! Unit tests for cleanup functionality
//! 
//! Tests the ProcessCleanupManager and ProcessGuard for reliable process cleanup

use rustbox::cleanup::{ProcessCleanupManager, ProcessGuard, ProcessType, CleanupStats};
use rustbox::types::{IsolateError, Result};
use nix::sys::signal::{self, Signal};
use nix::unistd::{fork, ForkResult, Pid, getpid};
use std::sync::Arc;
use std::time::Duration;
use std::thread;
use std::process::{Command, Child};

#[test]
fn test_cleanup_manager_creation() {
    let manager = ProcessCleanupManager::new(false);
    let stats = manager.get_stats();
    assert_eq!(stats.processes_cleaned, 0);
    assert_eq!(stats.alive_processes, 0);
}

#[test]
fn test_cleanup_manager_with_cgroups() {
    let manager = ProcessCleanupManager::new(true);
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
    assert_eq!(stats.alive_processes, 1);
}

#[test]
fn test_multiple_process_registration() {
    let manager = ProcessCleanupManager::new(false);
    
    manager.register_process(Pid::from_raw(100), ProcessType::Keeper);
    manager.register_process(Pid::from_raw(200), ProcessType::Proxy);
    manager.register_process(Pid::from_raw(300), ProcessType::Inside);
    
    let stats = manager.get_stats();
    assert_eq!(stats.processes_cleaned, 3);
    assert_eq!(stats.alive_processes, 3);
}

#[test]
fn test_process_marking_exited() {
    let manager = ProcessCleanupManager::new(false);
    let pid = Pid::from_raw(12345);
    
    manager.register_process(pid, ProcessType::Inside);
    manager.mark_process_exited(pid);
    
    let stats = manager.get_stats();
    assert_eq!(stats.processes_cleaned, 1);
    assert_eq!(stats.alive_processes, 0);
}

#[test]
fn test_pipe_registration() {
    let manager = ProcessCleanupManager::new(false);
    
    // Register some dummy file descriptors
    let error_pipes = vec![10, 11, 12];
    let status_pipes = vec![20, 21, 22];
    
    manager.register_pipes(error_pipes, status_pipes);
    
    // Should not crash - pipes are stored for cleanup
    let stats = manager.get_stats();
    assert_eq!(stats.processes_cleaned, 0);
}

#[test]
fn test_process_guard_creation() {
    let manager = Arc::new(ProcessCleanupManager::new(false));
    let guard = ProcessGuard::new(manager.clone(), Duration::from_secs(1));
    
    // Guard should be created successfully
    // Drop will trigger cleanup automatically
}

#[test]
fn test_process_guard_raii() {
    let manager = Arc::new(ProcessCleanupManager::new(false));
    
    {
        let _guard = ProcessGuard::new(manager.clone(), Duration::from_secs(1));
        // Guard is in scope
    }
    // Guard should have triggered cleanup on drop
}

#[test]
fn test_emergency_cleanup_idempotent() {
    let manager = ProcessCleanupManager::new(false);
    
    // First cleanup should succeed
    assert!(manager.emergency_cleanup("test reason 1").is_ok());
    
    // Second cleanup should be idempotent (no-op)
    assert!(manager.emergency_cleanup("test reason 2").is_ok());
}

#[test]
fn test_cleanup_with_fake_processes() {
    let manager = ProcessCleanupManager::new(false);
    
    // Register some fake PIDs that don't exist
    manager.register_process(Pid::from_raw(99999), ProcessType::Inside);
    manager.register_process(Pid::from_raw(99998), ProcessType::Proxy);
    
    // Emergency cleanup should handle non-existent processes gracefully
    assert!(manager.emergency_cleanup("test cleanup").is_ok());
}

#[test]
fn test_graceful_cleanup_timeout() {
    let manager = ProcessCleanupManager::new(false);
    
    // Register a fake process
    manager.register_process(Pid::from_raw(99999), ProcessType::Inside);
    
    // Graceful cleanup with very short timeout should fall back to force cleanup
    let result = manager.graceful_cleanup(Duration::from_millis(1));
    
    // Should succeed even with timeout
    assert!(result.is_ok());
    
    if let Ok(stats) = result {
        assert_eq!(stats.processes_cleaned, 1);
    }
}

#[test]
fn test_cleanup_stats() {
    let manager = ProcessCleanupManager::new(false);
    
    // Register multiple processes
    manager.register_process(Pid::from_raw(100), ProcessType::Keeper);
    manager.register_process(Pid::from_raw(200), ProcessType::Proxy);
    manager.register_process(Pid::from_raw(300), ProcessType::Inside);
    
    // Mark one as exited
    manager.mark_process_exited(Pid::from_raw(200));
    
    let stats = manager.get_stats();
    assert_eq!(stats.processes_cleaned, 3);
    assert_eq!(stats.alive_processes, 2);
}

#[test]
fn test_cgroup_vs_non_cgroup_mode() {
    let manager_cgroup = ProcessCleanupManager::new(true);
    let manager_no_cgroup = ProcessCleanupManager::new(false);
    
    // Both should handle cleanup gracefully
    assert!(manager_cgroup.emergency_cleanup("cgroup test").is_ok());
    assert!(manager_no_cgroup.emergency_cleanup("non-cgroup test").is_ok());
}

/// Integration test with actual process spawning
/// Note: This test requires careful handling to avoid leaving zombie processes
#[test]
fn test_cleanup_with_real_process() {
    let manager = Arc::new(ProcessCleanupManager::new(false));
    
    // Spawn a sleep process that we can cleanup
    let mut child = Command::new("sleep")
        .arg("10")
        .spawn()
        .expect("Failed to spawn test process");
    
    let pid = Pid::from_raw(child.id() as i32);
    manager.register_process(pid, ProcessType::Inside);
    
    // Create guard for automatic cleanup
    let _guard = ProcessGuard::new(manager.clone(), Duration::from_secs(1));
    
    // Manually trigger cleanup to test it
    let result = manager.emergency_cleanup("test cleanup");
    assert!(result.is_ok());
    
    // Wait for the child to be cleaned up
    let _ = child.wait();
}

/// Test cleanup behavior under stress
#[test]
fn test_cleanup_multiple_processes() {
    let manager = Arc::new(ProcessCleanupManager::new(false));
    let mut children = Vec::new();
    
    // Spawn multiple test processes
    for i in 0..5 {
        if let Ok(mut child) = Command::new("sleep").arg("10").spawn() {
            let pid = Pid::from_raw(child.id() as i32);
            manager.register_process(pid, ProcessType::Inside);
            children.push(child);
        }
    }
    
    // Cleanup all processes
    let result = manager.emergency_cleanup("stress test cleanup");
    assert!(result.is_ok());
    
    // Wait for all children to be cleaned up
    for mut child in children {
        let _ = child.wait();
    }
}

/// Test that ProcessGuard cleanup is called on panic
#[test]
#[should_panic(expected = "intentional panic")]
fn test_cleanup_on_panic() {
    let manager = Arc::new(ProcessCleanupManager::new(false));
    let _guard = ProcessGuard::new(manager.clone(), Duration::from_secs(1));
    
    // Register a fake process
    manager.register_process(Pid::from_raw(99999), ProcessType::Inside);
    
    // Panic should still trigger cleanup via Drop
    panic!("intentional panic");
}

/// Test manual cleanup via ProcessGuard
#[test]
fn test_manual_cleanup_via_guard() {
    let manager = Arc::new(ProcessCleanupManager::new(false));
    let guard = ProcessGuard::new(manager.clone(), Duration::from_secs(1));
    
    // Register a fake process
    manager.register_process(Pid::from_raw(99999), ProcessType::Inside);
    
    // Manually trigger cleanup
    let result = guard.cleanup_now();
    assert!(result.is_ok());
    
    if let Ok(stats) = result {
        assert_eq!(stats.processes_cleaned, 1);
    }
}

/// Test process type handling
#[test]
fn test_process_type_variants() {
    let manager = ProcessCleanupManager::new(false);
    
    manager.register_process(Pid::from_raw(100), ProcessType::Keeper);
    manager.register_process(Pid::from_raw(200), ProcessType::Proxy);
    manager.register_process(Pid::from_raw(300), ProcessType::Inside);
    
    // All process types should be registered
    let stats = manager.get_stats();
    assert_eq!(stats.processes_cleaned, 3);
    
    // Emergency cleanup should handle all types
    assert!(manager.emergency_cleanup("type test").is_ok());
}

/// Test concurrent access to cleanup manager
#[test]
fn test_concurrent_cleanup() {
    let manager = Arc::new(ProcessCleanupManager::new(false));
    let mut handles = Vec::new();
    
    // Spawn multiple threads trying to cleanup
    for i in 0..5 {
        let manager_clone = manager.clone();
        let handle = thread::spawn(move || {
            manager_clone.register_process(Pid::from_raw(1000 + i), ProcessType::Inside);
            manager_clone.emergency_cleanup(&format!("thread {}", i))
        });
        handles.push(handle);
    }
    
    // All threads should complete successfully
    for handle in handles {
        assert!(handle.join().unwrap().is_ok());
    }
}

/// Test edge case: cleanup with no registered processes
#[test]
fn test_cleanup_no_processes() {
    let manager = ProcessCleanupManager::new(false);
    
    // Cleanup with no processes should succeed
    assert!(manager.emergency_cleanup("no processes").is_ok());
    
    let result = manager.graceful_cleanup(Duration::from_millis(100));
    assert!(result.is_ok());
    
    if let Ok(stats) = result {
        assert_eq!(stats.processes_cleaned, 0);
    }
}