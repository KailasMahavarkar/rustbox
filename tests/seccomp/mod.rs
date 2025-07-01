/// Comprehensive tests for seccomp syscall filtering
use mini_isolate::seccomp::{SeccompFilter, get_dangerous_syscalls, is_seccomp_supported};
use mini_isolate::types::IsolateConfig;
use mini_isolate::isolate::Isolate;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

// Include malicious code tests
mod malicious_tests;

#[test]
fn test_seccomp_filter_basic_functionality() {
    let filter = SeccompFilter::new_for_anonymous_code();
    
    // Essential syscalls should be allowed
    assert!(filter.is_syscall_allowed("read"));
    assert!(filter.is_syscall_allowed("write"));
    assert!(filter.is_syscall_allowed("exit"));
    assert!(filter.is_syscall_allowed("exit_group"));
    assert!(filter.is_syscall_allowed("brk"));
    assert!(filter.is_syscall_allowed("mmap"));
    
    // Dangerous syscalls should be blocked
    assert!(!filter.is_syscall_allowed("socket"));
    assert!(!filter.is_syscall_allowed("connect"));
    assert!(!filter.is_syscall_allowed("fork"));
    assert!(!filter.is_syscall_allowed("execve"));
    assert!(!filter.is_syscall_allowed("mount"));
    assert!(!filter.is_syscall_allowed("setuid"));
}

#[test]
fn test_language_specific_filters() {
    let python_filter = SeccompFilter::new_for_language("python");
    let js_filter = SeccompFilter::new_for_language("javascript");
    let java_filter = SeccompFilter::new_for_language("java");
    let unknown_filter = SeccompFilter::new_for_language("unknown_language");
    
    // Python should have additional file system syscalls
    assert!(python_filter.is_syscall_allowed("stat"));
    assert!(python_filter.is_syscall_allowed("pipe"));
    assert!(python_filter.is_syscall_allowed("readlink"));
    
    // JavaScript should have async I/O syscalls
    assert!(js_filter.is_syscall_allowed("futex"));
    assert!(js_filter.is_syscall_allowed("epoll_create1"));
    assert!(js_filter.is_syscall_allowed("poll"));
    
    // Java should have threading syscalls
    assert!(java_filter.is_syscall_allowed("clone"));
    assert!(java_filter.is_syscall_allowed("futex"));
    assert!(java_filter.is_syscall_allowed("madvise"));
    
    // Unknown language should use defaults
    assert_eq!(
        unknown_filter.get_allowed_syscalls().len(),
        SeccompFilter::new_for_anonymous_code().get_allowed_syscalls().len()
    );
}

#[test]
fn test_custom_syscall_management() {
    let mut filter = SeccompFilter::new_for_anonymous_code();
    let initial_count = filter.get_allowed_syscalls().len();
    
    // Add custom syscall
    filter.allow_syscall("custom_test_syscall");
    assert!(filter.is_syscall_allowed("custom_test_syscall"));
    assert_eq!(filter.get_allowed_syscalls().len(), initial_count + 1);
    
    // Remove existing syscall
    filter.deny_syscall("read");
    assert!(!filter.is_syscall_allowed("read"));
    assert_eq!(filter.get_allowed_syscalls().len(), initial_count);
}

#[test]
fn test_dangerous_syscalls_comprehensive() {
    let dangerous = get_dangerous_syscalls();
    let filter = SeccompFilter::new_for_anonymous_code();
    
    // All dangerous syscalls should be blocked
    for syscall in &dangerous {
        assert!(
            !filter.is_syscall_allowed(syscall),
            "Dangerous syscall '{}' should be blocked",
            syscall
        );
    }
    
    // Verify we have comprehensive coverage
    assert!(dangerous.len() > 30, "Should have comprehensive dangerous syscall list");
    
    // Check specific categories
    let network_syscalls = ["socket", "connect", "bind", "listen", "accept"];
    let process_syscalls = ["fork", "vfork", "clone", "execve"];
    let privilege_syscalls = ["setuid", "setgid", "capset"];
    
    for syscall in &network_syscalls {
        assert!(dangerous.contains(syscall), "Missing network syscall: {}", syscall);
    }
    
    for syscall in &process_syscalls {
        assert!(dangerous.contains(syscall), "Missing process syscall: {}", syscall);
    }
    
    for syscall in &privilege_syscalls {
        assert!(dangerous.contains(syscall), "Missing privilege syscall: {}", syscall);
    }
}

#[test]
#[serial]
fn test_seccomp_integration_with_isolate() {
    if !is_seccomp_supported() {
        println!("Skipping seccomp integration test - not supported on this system");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let workdir = temp_dir.path().to_path_buf();
    
    let mut config = IsolateConfig {
        instance_id: "seccomp_test".to_string(),
        workdir: workdir.clone(),
        enable_seccomp: true,
        seccomp_profile: None,
        strict_mode: false,
        ..Default::default()
    };
    
    // Test basic execution with seccomp enabled
    config.enable_seccomp = true;
    let mut isolate = Isolate::new(config.clone()).expect("Failed to create isolate");
    
    // Create a simple test script
    let test_script = workdir.join("test.py");
    fs::write(&test_script, "print('Hello, seccomp!')").expect("Failed to write test script");
    
    // This should work - basic I/O is allowed
    let result = isolate.execute_file(&test_script, None);
    assert!(result.is_ok(), "Basic execution should work with seccomp");
    
    let exec_result = result.unwrap();
    assert!(exec_result.success, "Execution should succeed");
    assert_eq!(exec_result.stdout.trim(), "Hello, seccomp!");
}

#[test]
#[serial]
fn test_seccomp_blocks_network_operations() {
    if !is_seccomp_supported() {
        println!("Skipping network blocking test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let workdir = temp_dir.path().to_path_buf();
    
    let config = IsolateConfig {
        instance_id: "seccomp_network_test".to_string(),
        workdir: workdir.clone(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        ..Default::default()
    };
    
    let mut isolate = Isolate::new(config).expect("Failed to create isolate");
    
    // Create a Python script that tries to create a socket
    let malicious_script = workdir.join("network_test.py");
    fs::write(&malicious_script, r#"
import socket
try:
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    print("SECURITY BREACH: Socket created successfully!")
    s.close()
except OSError as e:
    print(f"Expected error: {e}")
"#).expect("Failed to write network test script");
    
    let result = isolate.execute_file(&malicious_script, None);
    
    // The process should be killed by seccomp before it can print anything
    if let Ok(exec_result) = result {
        // If process completed, it should have been killed by signal
        assert!(!exec_result.success, "Network operation should be blocked");
        assert!(
            exec_result.signal.is_some() || exec_result.stderr.contains("Operation not permitted"),
            "Process should be killed by seccomp or get permission error"
        );
        // Should NOT contain the security breach message
        assert!(
            !exec_result.stdout.contains("SECURITY BREACH"),
            "Socket creation should be blocked by seccomp"
        );
    } else {
        // Process failed to start - this is also acceptable as seccomp might block early
        println!("Process failed to start - likely blocked by seccomp");
    }
}

#[test]
#[serial]
fn test_seccomp_blocks_process_creation() {
    if !is_seccomp_supported() {
        println!("Skipping process creation blocking test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let workdir = temp_dir.path().to_path_buf();
    
    let config = IsolateConfig {
        instance_id: "seccomp_process_test".to_string(),
        workdir: workdir.clone(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        ..Default::default()
    };
    
    let mut isolate = Isolate::new(config).expect("Failed to create isolate");
    
    // Create a Python script that tries to fork
    let fork_script = workdir.join("fork_test.py");
    fs::write(&fork_script, r#"
import os
try:
    pid = os.fork()
    if pid == 0:
        print("SECURITY BREACH: Child process created!")
        os._exit(0)
    else:
        print("SECURITY BREACH: Fork succeeded!")
        os.waitpid(pid, 0)
except OSError as e:
    print(f"Expected error: {e}")
"#).expect("Failed to write fork test script");
    
    let result = isolate.execute_file(&fork_script, None);
    
    // The process should be killed by seccomp
    if let Ok(exec_result) = result {
        assert!(!exec_result.success, "Fork operation should be blocked");
        // Should NOT contain the security breach message
        assert!(
            !exec_result.stdout.contains("SECURITY BREACH"),
            "Fork should be blocked by seccomp"
        );
    }
}

#[test]
#[serial]
fn test_seccomp_blocks_file_system_modifications() {
    if !is_seccomp_supported() {
        println!("Skipping filesystem blocking test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let workdir = temp_dir.path().to_path_buf();
    
    let config = IsolateConfig {
        instance_id: "seccomp_fs_test".to_string(),
        workdir: workdir.clone(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        ..Default::default()
    };
    
    let mut isolate = Isolate::new(config).expect("Failed to create isolate");
    
    // Create a Python script that tries to mount filesystems
    let mount_script = workdir.join("mount_test.py");
    fs::write(&mount_script, r#"
import ctypes
import ctypes.util
import os

try:
    # Try to use mount syscall
    libc = ctypes.CDLL(ctypes.util.find_library("c"))
    result = libc.mount(None, b"/tmp", None, 0, None)
    print("SECURITY BREACH: Mount syscall succeeded!")
except Exception as e:
    print(f"Expected error: {e}")

try:
    # Try to create a directory outside workdir
    os.mkdir("/tmp/escape_attempt")
    print("SECURITY BREACH: Directory created outside workdir!")
    os.rmdir("/tmp/escape_attempt")
except Exception as e:
    print(f"Expected error: {e}")
"#).expect("Failed to write mount test script");
    
    let result = isolate.execute_file(&mount_script, None);
    
    if let Ok(exec_result) = result {
        // Should NOT contain security breach messages
        assert!(
            !exec_result.stdout.contains("SECURITY BREACH"),
            "Dangerous filesystem operations should be blocked"
        );
    }
}

#[test]
fn test_seccomp_performance_impact() {
    // Test that seccomp doesn't significantly impact performance for allowed operations
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let workdir = temp_dir.path().to_path_buf();
    
    // Test script that performs many allowed operations
    let perf_script = workdir.join("perf_test.py");
    fs::write(&perf_script, r#"
import time
start = time.time()
# Perform many allowed operations
for i in range(1000):
    data = str(i) * 100
    # Basic I/O and memory operations
    len(data)
    data.upper()
    data.lower()
end = time.time()
print(f"Completed 1000 iterations in {end - start:.3f} seconds")
"#).expect("Failed to write performance test script");
    
    // Test with seccomp enabled
    let config_with_seccomp = IsolateConfig {
        instance_id: "seccomp_perf_test".to_string(),
        workdir: workdir.clone(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        ..Default::default()
    };
    
    // Test without seccomp
    let config_without_seccomp = IsolateConfig {
        instance_id: "no_seccomp_perf_test".to_string(),
        workdir: workdir.clone(),
        enable_seccomp: false,
        strict_mode: false,
        ..Default::default()
    };
    
    if is_seccomp_supported() {
        let mut isolate_with = Isolate::new(config_with_seccomp).expect("Failed to create isolate with seccomp");
        let result_with = isolate_with.execute_file(&perf_script, None);
        assert!(result_with.is_ok(), "Performance test should work with seccomp");
        
        let mut isolate_without = Isolate::new(config_without_seccomp).expect("Failed to create isolate without seccomp");
        let result_without = isolate_without.execute_file(&perf_script, None);
        assert!(result_without.is_ok(), "Performance test should work without seccomp");
        
        // Both should complete successfully
        let exec_with = result_with.unwrap();
        let exec_without = result_without.unwrap();
        
        assert!(exec_with.success, "Seccomp test should succeed");
        assert!(exec_without.success, "Non-seccomp test should succeed");
        
        // Performance difference should be minimal (< 100% overhead)
        let time_ratio = exec_with.wall_time / exec_without.wall_time;
        assert!(
            time_ratio < 2.0,
            "Seccomp should not add more than 100% overhead, got {}x",
            time_ratio
        );
    } else {
        println!("Skipping performance test - seccomp not supported");
    }
}

#[cfg(test)]
mod edge_cases {
    use super::*;
    
    #[test]
    fn test_seccomp_with_empty_allowed_list() {
        let mut filter = SeccompFilter::new_for_anonymous_code();
        
        // Remove all syscalls
        let allowed_clone = filter.get_allowed_syscalls().into_iter().cloned().collect::<Vec<_>>();
        for syscall in allowed_clone {
            filter.deny_syscall(&syscall);
        }
        
        assert_eq!(filter.get_allowed_syscalls().len(), 0);
        
        // Adding back essential syscalls should work
        filter.allow_syscall("exit");
        assert!(filter.is_syscall_allowed("exit"));
    }
    
    #[test]
    fn test_seccomp_invalid_syscall_names() {
        let mut filter = SeccompFilter::new_for_anonymous_code();
        
        // These should not crash
        filter.allow_syscall("nonexistent_syscall_12345");
        filter.deny_syscall("another_nonexistent_syscall");
        
        assert!(filter.is_syscall_allowed("nonexistent_syscall_12345"));
        assert!(!filter.is_syscall_allowed("another_nonexistent_syscall"));
    }
    
    #[test]
    fn test_multiple_language_profiles() {
        let mut filter = SeccompFilter::new_for_language("python");
        
        // Adding JavaScript syscalls to Python filter
        filter.allow_syscall("eventfd2");
        filter.allow_syscall("epoll_wait");
        
        // Should now have both Python and JavaScript syscalls
        assert!(filter.is_syscall_allowed("stat")); // Python
        assert!(filter.is_syscall_allowed("eventfd2")); // JavaScript
    }
}