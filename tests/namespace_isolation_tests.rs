/// Comprehensive tests for namespace isolation functionality
use mini_isolate::namespace::{NamespaceIsolation, NamespaceInfo};
use mini_isolate::types::{IsolateConfig, IsolateError};
use mini_isolate::executor::ProcessExecutor;
use std::path::PathBuf;
use std::time::Duration;

/// Test namespace isolation creation and configuration
#[test]
fn test_namespace_creation() {
    let workdir = PathBuf::from("/tmp/test_namespace");
    let ns = NamespaceIsolation::new_default(workdir, false);
    
    assert!(ns.is_isolation_enabled());
    let enabled = ns.get_enabled_namespaces();
    assert!(enabled.contains(&"PID".to_string()));
    assert!(enabled.contains(&"Mount".to_string()));
    assert!(enabled.contains(&"Network".to_string()));
    assert!(!enabled.contains(&"User".to_string())); // User namespace disabled by default
}

/// Test selective namespace configuration
#[test]
fn test_selective_namespace_configuration() {
    let workdir = PathBuf::from("/tmp/test_selective");
    
    // Test PID-only namespace
    let ns_pid = NamespaceIsolation::new(workdir.clone(), false, true, false, false, false);
    let enabled = ns_pid.get_enabled_namespaces();
    assert_eq!(enabled, vec!["PID"]);
    
    // Test Mount + Network namespace
    let ns_mount_net = NamespaceIsolation::new(workdir.clone(), false, false, true, true, false);
    let enabled = ns_mount_net.get_enabled_namespaces();
    assert_eq!(enabled, vec!["Mount", "Network"]);
    
    // Test no namespaces
    let ns_none = NamespaceIsolation::new(workdir, false, false, false, false, false);
    assert!(!ns_none.is_isolation_enabled());
    assert!(ns_none.get_enabled_namespaces().is_empty());
}

/// Test namespace support detection
#[test]
fn test_namespace_support_detection() {
    let supported = NamespaceIsolation::is_supported();
    
    // On Linux systems with namespace support, this should be true
    // On other systems or containers without namespace support, this may be false
    println!("Namespace support detected: {}", supported);
    
    // The test should not fail regardless of support level
    // This is informational
}

/// Test namespace info retrieval
#[test]
fn test_namespace_info_retrieval() {
    let workdir = PathBuf::from("/tmp/test_info");
    let ns = NamespaceIsolation::new_default(workdir, false);
    
    match ns.get_namespace_info() {
        Ok(info) => {
            println!("Namespace info: {}", info);
            assert!(info.pid > 0);
            assert!(!info.pid_namespace.is_empty());
            assert!(!info.mount_namespace.is_empty());
            assert!(!info.network_namespace.is_empty());
        }
        Err(e) => {
            println!("Failed to get namespace info (expected on non-Linux): {}", e);
            // This is expected on non-Linux systems
        }
    }
}

/// Test namespace isolation with process execution (requires root)
#[test]
#[ignore] // Ignored by default, run with --ignored flag and sudo
fn test_namespace_isolation_with_execution() {
    let mut config = IsolateConfig::default();
    config.enable_pid_namespace = true;
    config.enable_mount_namespace = true;
    config.enable_network_namespace = true;
    config.enable_user_namespace = false;
    config.strict_mode = true;
    config.wall_time_limit = Some(Duration::from_secs(5));
    
    let mut executor = ProcessExecutor::new(config).expect("Failed to create executor");
    
    // Test command that checks PID namespace isolation
    let result = executor.execute(
        &["sh".to_string(), "-c".to_string(), "echo \"PID: $$\"; ps aux | wc -l".to_string()],
        None,
    );
    
    match result {
        Ok(exec_result) => {
            println!("Execution result: {:?}", exec_result);
            assert!(exec_result.success);
            
            // In a PID namespace, we should see fewer processes
            let output = exec_result.stdout;
            println!("Command output: {}", output);
        }
        Err(e) => {
            println!("Execution failed (may require root): {}", e);
        }
    }
    
    executor.cleanup().expect("Failed to cleanup");
}

/// Test mount namespace isolation (requires root)
#[test]
#[ignore] // Ignored by default, run with --ignored flag and sudo
fn test_mount_namespace_isolation() {
    let mut config = IsolateConfig::default();
    config.enable_pid_namespace = false;
    config.enable_mount_namespace = true;
    config.enable_network_namespace = false;
    config.enable_user_namespace = false;
    config.strict_mode = true;
    config.wall_time_limit = Some(Duration::from_secs(5));
    
    let mut executor = ProcessExecutor::new(config).expect("Failed to create executor");
    
    // Test command that checks mount namespace isolation
    let result = executor.execute(
        &["sh".to_string(), "-c".to_string(), "mount | grep -c tmpfs || echo 0".to_string()],
        None,
    );
    
    match result {
        Ok(exec_result) => {
            println!("Mount namespace test result: {:?}", exec_result);
            assert!(exec_result.success);
            
            let output = exec_result.stdout.trim();
            println!("Mount count: {}", output);
            
            // We should see some tmpfs mounts in the isolated namespace
        }
        Err(e) => {
            println!("Mount namespace test failed (may require root): {}", e);
        }
    }
    
    executor.cleanup().expect("Failed to cleanup");
}

/// Test network namespace isolation (requires root)
#[test]
#[ignore] // Ignored by default, run with --ignored flag and sudo
fn test_network_namespace_isolation() {
    let mut config = IsolateConfig::default();
    config.enable_pid_namespace = false;
    config.enable_mount_namespace = false;
    config.enable_network_namespace = true;
    config.enable_user_namespace = false;
    config.strict_mode = true;
    config.wall_time_limit = Some(Duration::from_secs(5));
    
    let mut executor = ProcessExecutor::new(config).expect("Failed to create executor");
    
    // Test command that checks network namespace isolation
    let result = executor.execute(
        &["sh".to_string(), "-c".to_string(), "ip link show | wc -l".to_string()],
        None,
    );
    
    match result {
        Ok(exec_result) => {
            println!("Network namespace test result: {:?}", exec_result);
            assert!(exec_result.success);
            
            let output = exec_result.stdout.trim();
            println!("Network interfaces count: {}", output);
            
            // In a network namespace, we should only see loopback interface
            let interface_count: i32 = output.parse().unwrap_or(0);
            assert!(interface_count <= 2); // Should be 1-2 lines (header + loopback)
        }
        Err(e) => {
            println!("Network namespace test failed (may require root): {}", e);
        }
    }
    
    executor.cleanup().expect("Failed to cleanup");
}

/// Test combined namespace isolation (requires root)
#[test]
#[ignore] // Ignored by default, run with --ignored flag and sudo
fn test_combined_namespace_isolation() {
    let mut config = IsolateConfig::default();
    config.enable_pid_namespace = true;
    config.enable_mount_namespace = true;
    config.enable_network_namespace = true;
    config.enable_user_namespace = false;
    config.strict_mode = true;
    config.wall_time_limit = Some(Duration::from_secs(10));
    
    let mut executor = ProcessExecutor::new(config).expect("Failed to create executor");
    
    // Test command that checks all namespace isolations
    let result = executor.execute(
        &[
            "sh".to_string(), 
            "-c".to_string(), 
            "echo \"=== PID Info ===\"; echo \"PID: $$\"; echo \"=== Process Count ===\"; ps aux | wc -l; echo \"=== Network ===\"; ip link show | wc -l; echo \"=== Mounts ===\"; mount | wc -l".to_string()
        ],
        None,
    );
    
    match result {
        Ok(exec_result) => {
            println!("Combined namespace test result: {:?}", exec_result);
            assert!(exec_result.success);
            
            let output = exec_result.stdout;
            println!("Combined test output:\n{}", output);
            
            // Verify the command executed successfully
            assert!(output.contains("PID Info"));
            assert!(output.contains("Process Count"));
            assert!(output.contains("Network"));
            assert!(output.contains("Mounts"));
        }
        Err(e) => {
            println!("Combined namespace test failed (may require root): {}", e);
        }
    }
    
    executor.cleanup().expect("Failed to cleanup");
}

/// Test namespace isolation error handling
#[test]
fn test_namespace_error_handling() {
    let workdir = PathBuf::from("/tmp/test_errors");
    let ns = NamespaceIsolation::new_default(workdir, true); // strict mode
    
    // Test applying isolation without root (should fail gracefully)
    let result = ns.apply_isolation();
    
    match result {
        Ok(_) => {
            println!("Namespace isolation applied successfully");
        }
        Err(e) => {
            println!("Namespace isolation failed as expected: {}", e);
            // Verify it's the right type of error
            match e {
                IsolateError::Namespace(_) => {
                    // This is expected when not running as root
                }
                _ => {
                    panic!("Unexpected error type: {}", e);
                }
            }
        }
    }
}

/// Test namespace configuration integration with IsolateConfig
#[test]
fn test_isolate_config_namespace_integration() {
    let mut config = IsolateConfig::default();
    
    // Test default namespace settings
    assert!(config.enable_pid_namespace);
    assert!(config.enable_mount_namespace);
    assert!(config.enable_network_namespace);
    assert!(!config.enable_user_namespace);
    
    // Test custom configuration
    config.enable_pid_namespace = false;
    config.enable_user_namespace = true;
    
    let executor_result = ProcessExecutor::new(config);
    assert!(executor_result.is_ok());
}

/// Test namespace isolation with different working directories
#[test]
fn test_namespace_with_different_workdirs() {
    let workdirs = vec![
        PathBuf::from("/tmp/ns_test_1"),
        PathBuf::from("/tmp/ns_test_2"),
        PathBuf::from("/var/tmp/ns_test_3"),
    ];
    
    for workdir in workdirs {
        let ns = NamespaceIsolation::new_default(workdir.clone(), false);
        assert!(ns.is_isolation_enabled());
        
        // Test namespace info retrieval with different workdirs
        match ns.get_namespace_info() {
            Ok(info) => {
                println!("Workdir: {:?}, Namespace info: {}", workdir, info);
            }
            Err(e) => {
                println!("Workdir: {:?}, Failed to get namespace info: {}", workdir, e);
            }
        }
    }
}

/// Benchmark namespace isolation setup performance
#[test]
#[ignore] // Performance test, run manually
fn benchmark_namespace_setup() {
    use std::time::Instant;
    
    let workdir = PathBuf::from("/tmp/ns_benchmark");
    let iterations = 100;
    
    let start = Instant::now();
    
    for i in 0..iterations {
        let ns = NamespaceIsolation::new_default(workdir.clone(), false);
        
        // Simulate configuration checks
        assert!(ns.is_isolation_enabled());
        let _enabled = ns.get_enabled_namespaces();
        
        if i % 10 == 0 {
            println!("Completed {} iterations", i);
        }
    }
    
    let duration = start.elapsed();
    let avg_time = duration / iterations;
    
    println!("Namespace setup benchmark:");
    println!("Total time: {:?}", duration);
    println!("Average time per setup: {:?}", avg_time);
    println!("Setups per second: {:.2}", 1.0 / avg_time.as_secs_f64());
}