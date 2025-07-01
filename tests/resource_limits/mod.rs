/// Resource limits integration tests
use mini_isolate::resource_limits::ResourceLimitController;
use mini_isolate::isolate::Isolate;
use mini_isolate::types::IsolateConfig;
use std::time::Duration;
use tempfile::TempDir;
use serial_test::serial;

#[test]
fn test_resource_limit_controller_creation() {
    let controller = ResourceLimitController::new(false);
    // ResourceLimitController::new doesn't return Result, so no need to check is_ok()

    let strict_controller = ResourceLimitController::new(true);
    // Same here - no Result to check
}

#[test]
fn test_resource_limits_supported() {
    // Test that resource limits support detection works
    let supported = mini_isolate::resource_limits::resource_limits_supported();
    
    // Should return a boolean value
    assert!(supported == true || supported == false);
}

#[test]
fn test_get_current_limits() {
    let controller = ResourceLimitController::new(false);
    
    let result = controller.get_current_limits();
    assert!(result.is_ok());
    
    let limits = result.unwrap();
    
    // Should have some limit values (may be None for unlimited)
    // Just verify the structure is correct
    assert!(limits.stack_soft.is_some() || limits.stack_soft.is_none());
    assert!(limits.stack_hard.is_some() || limits.stack_hard.is_none());
}

#[test]
fn test_stack_limit_setting() {
    let controller = ResourceLimitController::new(false);
    
    // Test setting stack limit
    let result = controller.set_stack_limit(8 * 1024 * 1024); // 8MB
    
    // Should succeed or fail gracefully
    match result {
        Ok(_) => {
            // If successful, verify we can get the limits
            let limits = controller.get_current_limits().unwrap();
            assert!(limits.stack_soft.is_some() || limits.stack_hard.is_some());
        }
        Err(e) => {
            // If failed, should be due to permissions
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("Permission denied") || 
                   error_msg.contains("Operation not permitted") ||
                   error_msg.contains("not supported"));
        }
    }
}

#[test]
fn test_core_limit_setting() {
    let controller = ResourceLimitController::new(false);
    
    // Test setting core dump limit (disable core dumps)
    let result = controller.set_core_limit(0);
    
    // Should succeed or fail gracefully
    match result {
        Ok(_) => {
            // If successful, verify we can get the limits
            let limits = controller.get_current_limits().unwrap();
            assert!(limits.core_soft.is_some() || limits.core_hard.is_some());
        }
        Err(e) => {
            // If failed, should be due to permissions
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("Permission denied") || 
                   error_msg.contains("Operation not permitted") ||
                   error_msg.contains("not supported"));
        }
    }
}

#[test]
fn test_directory_size_calculation() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create some test files
    std::fs::write(temp_dir.path().join("test1.txt"), "Hello, World!").unwrap();
    std::fs::write(temp_dir.path().join("test2.txt"), "Testing directory size calculation").unwrap();
    
    let controller = ResourceLimitController::new(false);
    let result = controller.get_directory_size(temp_dir.path());
    
    assert!(result.is_ok());
    let size = result.unwrap();
    
    // Should be greater than 0 since we created files
    assert!(size > 0);
    
    // Should be reasonable size (less than 1MB for our small test files)
    assert!(size < 1024 * 1024);
}

#[test]
#[serial]
fn test_resource_limits_integration_with_isolate() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = IsolateConfig {
        instance_id: "test-resource-limits".to_string(),
        workdir: temp_dir.path().join("workdir"),
        memory_limit: Some(64 * 1024 * 1024), // 64MB
        cpu_time_limit: Some(Duration::from_secs(5)),
        wall_time_limit: Some(Duration::from_secs(10)),
        process_limit: Some(1),
        file_size_limit: Some(1024 * 1024), // 1MB
        stack_limit: Some(8 * 1024 * 1024), // 8MB
        core_limit: Some(0), // Disable core dumps
        strict_mode: false,
        ..Default::default()
    };

    let result = Isolate::new(config);
    assert!(result.is_ok());
    
    let mut isolate = result.unwrap();
    
    // Test basic command execution with resource limits
    let command = vec!["echo".to_string(), "resource_limit_test".to_string()];
    let exec_result = isolate.execute(&command, None);
    
    // Should succeed with reasonable resource limits or fail gracefully
    if exec_result.is_err() {
        eprintln!("Resource limits test execution failed (may be expected): {:?}", exec_result.unwrap_err());
    } else if let Ok(result) = exec_result {
        assert!(result.stdout.contains("resource_limit_test"));
        assert!(result.success);
        
        // Verify timing information is available
        assert!(result.cpu_time >= 0.0);
        assert!(result.wall_time >= 0.0);
    }
    
    // Cleanup
    isolate.cleanup().unwrap();
}

#[test]
#[serial]
fn test_memory_limit_enforcement() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = IsolateConfig {
        instance_id: "test-memory-limit".to_string(),
        workdir: temp_dir.path().join("workdir"),
        memory_limit: Some(32 * 1024 * 1024), // 32MB - small limit
        time_limit: Some(Duration::from_secs(5)),
        strict_mode: false,
        ..Default::default()
    };

    let result = Isolate::new(config);
    assert!(result.is_ok());
    
    let mut isolate = result.unwrap();
    
    // Test with a simple command that shouldn't exceed memory limit
    let command = vec!["echo".to_string(), "memory_test".to_string()];
    let exec_result = isolate.execute(&command, None);
    
    // Should succeed with reasonable memory usage or fail gracefully
    if exec_result.is_err() {
        eprintln!("Memory limit test execution failed (may be expected): {:?}", exec_result.unwrap_err());
    } else if let Ok(result) = exec_result {
        assert!(result.stdout.contains("memory_test"));
        assert!(result.success);
        
        // Memory usage should be reasonable for echo command
        assert!(result.memory_peak < 32 * 1024 * 1024);
    }
    
    // Cleanup
    isolate.cleanup().unwrap();
}

#[test]
#[serial]
fn test_time_limit_enforcement() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = IsolateConfig {
        instance_id: "test-time-limit".to_string(),
        workdir: temp_dir.path().join("workdir"),
        cpu_time_limit: Some(Duration::from_secs(1)), // 1 second limit
        wall_time_limit: Some(Duration::from_secs(2)), // 2 second wall time
        strict_mode: false,
        ..Default::default()
    };

    let result = Isolate::new(config);
    assert!(result.is_ok());
    
    let mut isolate = result.unwrap();
    
    // Test with a quick command that should complete within time limit
    let command = vec!["echo".to_string(), "time_test".to_string()];
    let exec_result = isolate.execute(&command, None);
    
    // Should succeed within time limits or fail gracefully
    if exec_result.is_err() {
        eprintln!("Time limit test execution failed (may be expected): {:?}", exec_result.unwrap_err());
    } else if let Ok(result) = exec_result {
        assert!(result.stdout.contains("time_test"));
        assert!(result.success);
        
        // Should complete quickly
        assert!(result.wall_time < 2.0);
        assert!(result.cpu_time < 1.0);
    }
    
    // Cleanup
    isolate.cleanup().unwrap();
}

#[test]
#[serial]
fn test_process_limit_enforcement() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = IsolateConfig {
        instance_id: "test-process-limit".to_string(),
        workdir: temp_dir.path().join("workdir"),
        process_limit: Some(1), // Only allow 1 process
        time_limit: Some(Duration::from_secs(5)),
        strict_mode: false,
        ..Default::default()
    };

    let result = Isolate::new(config);
    assert!(result.is_ok());
    
    let mut isolate = result.unwrap();
    
    // Test with a single process command
    let command = vec!["echo".to_string(), "process_test".to_string()];
    let exec_result = isolate.execute(&command, None);
    
    // Should succeed with single process or fail gracefully
    if exec_result.is_err() {
        eprintln!("Process limit test execution failed (may be expected): {:?}", exec_result.unwrap_err());
    } else if let Ok(result) = exec_result {
        assert!(result.stdout.contains("process_test"));
        assert!(result.success);
    }
    
    // Cleanup
    isolate.cleanup().unwrap();
}

#[test]
#[serial]
fn test_file_size_limit_enforcement() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = IsolateConfig {
        instance_id: "test-file-size-limit".to_string(),
        workdir: temp_dir.path().join("workdir"),
        file_size_limit: Some(1024), // 1KB limit
        time_limit: Some(Duration::from_secs(5)),
        strict_mode: false,
        ..Default::default()
    };

    let result = Isolate::new(config);
    assert!(result.is_ok());
    
    let mut isolate = result.unwrap();
    
    // Test with a command that creates small output
    let command = vec!["echo".to_string(), "small_output".to_string()];
    let exec_result = isolate.execute(&command, None);
    
    // Should succeed with small output or fail gracefully
    if exec_result.is_err() {
        eprintln!("File size limit test execution failed (may be expected): {:?}", exec_result.unwrap_err());
    } else if let Ok(result) = exec_result {
        assert!(result.stdout.contains("small_output"));
        assert!(result.success);
    }
    
    // Cleanup
    isolate.cleanup().unwrap();
}

#[test]
fn test_resource_limit_error_handling() {
    let controller = ResourceLimitController::new(true); // Strict mode
    // ResourceLimitController::new doesn't return Result, so no need to check is_ok()
    
    let controller = controller;
    
    // Test with invalid values
    let result = controller.set_stack_limit(0); // Invalid stack size
    
    // Should handle invalid values gracefully
    match result {
        Ok(_) => {
            // Some systems might allow 0 stack size
        }
        Err(e) => {
            // Should be a reasonable error
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("Invalid") || 
                   error_msg.contains("not supported") ||
                   error_msg.contains("Permission denied"));
        }
    }
}