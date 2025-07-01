use mini_isolate::resource_limits::ResourceLimitController;
use mini_isolate::types::{IsolateConfig, IsolateError};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_resource_limit_controller_creation() {
    let controller = ResourceLimitController::new(false);
    // Controller should be created successfully (can't access private fields)
    
    let strict_controller = ResourceLimitController::new(true);
    // Both should be created successfully
}

#[test]
fn test_stack_limit_setting() {
    let controller = ResourceLimitController::new(false);
    // Set a reasonable stack limit (8MB)
    let result = controller.set_stack_limit(8 * 1024 * 1024);
    assert!(result.is_ok(), "Should be able to set stack limit: {:?}", result);
}

#[test]
fn test_core_limit_setting() {
    let controller = ResourceLimitController::new(false);
    // Disable core dumps
    let result = controller.set_core_limit(0);
    assert!(result.is_ok(), "Should be able to disable core dumps: {:?}", result);
    
    // Set a small core limit
    let result = controller.set_core_limit(1024 * 1024); // 1MB
    assert!(result.is_ok(), "Should be able to set core limit: {:?}", result);
}

#[test]
fn test_file_size_limit_setting() {
    let controller = ResourceLimitController::new(false);
    // Set file size limit to 10MB
    let result = controller.set_file_size_limit(10 * 1024 * 1024);
    assert!(result.is_ok(), "Should be able to set file size limit: {:?}", result);
}

#[test]
fn test_cpu_time_limit_setting() {
    let controller = ResourceLimitController::new(false);
    // Set CPU time limit to 30 seconds
    let result = controller.set_cpu_time_limit(30);
    assert!(result.is_ok(), "Should be able to set CPU time limit: {:?}", result);
}

#[test]
fn test_process_limit_setting() {
    let controller = ResourceLimitController::new(false);
    // Set process limit to 5
    let result = controller.set_process_limit(5);
    assert!(result.is_ok(), "Should be able to set process limit: {:?}", result);
}

#[test]
fn test_directory_size_calculation() {
    let controller = ResourceLimitController::new(false);
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // Create some test files
    let file1 = temp_dir.path().join("test1.txt");
    let file2 = temp_dir.path().join("test2.txt");
    
    fs::write(&file1, "Hello, World!").expect("Failed to write file1");
    fs::write(&file2, "This is a test file with more content").expect("Failed to write file2");
    
    let size = controller.get_directory_size(temp_dir.path());
    assert!(size.is_ok(), "Should be able to calculate directory size: {:?}", size);
    
    let calculated_size = size.unwrap();
    assert!(calculated_size > 0, "Directory size should be greater than 0");
    assert!(calculated_size >= 50, "Directory size should be at least 50 bytes");
}

#[test]
fn test_disk_quota_check() {
    let controller = ResourceLimitController::new(false);
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // Create a small test file
    let test_file = temp_dir.path().join("quota_test.txt");
    fs::write(&test_file, "Small test file").expect("Failed to write test file");
    
    // Set a quota that should be sufficient
    let result = controller.set_disk_quota(temp_dir.path(), 1024 * 1024); // 1MB quota
    assert!(result.is_ok(), "Should be able to set disk quota: {:?}", result);
    
    // Check quota compliance
    let quota_check = controller.check_disk_quota(temp_dir.path(), 1024 * 1024);
    assert!(quota_check.is_ok(), "Should be able to check disk quota: {:?}", quota_check);
    assert!(quota_check.unwrap(), "Should be within quota limits");
    
    // Check with very small quota (should fail)
    let quota_check_small = controller.check_disk_quota(temp_dir.path(), 10); // 10 bytes
    assert!(quota_check_small.is_ok(), "Should be able to check disk quota");
    assert!(!quota_check_small.unwrap(), "Should exceed small quota limits");
}

#[test]
fn test_disk_quota_with_nonexistent_directory() {
    let controller = ResourceLimitController::new(false);
    let nonexistent_path = Path::new("/nonexistent/directory");
    
    let result = controller.set_disk_quota(nonexistent_path, 1024 * 1024);
    assert!(result.is_err(), "Should fail for nonexistent directory");
    
    if let Err(IsolateError::ResourceLimit(msg)) = result {
        assert!(msg.contains("does not exist"), "Error message should mention directory doesn't exist");
    } else {
        panic!("Expected ResourceLimit error");
    }
}

#[test]
fn test_get_current_limits() {
    let controller = ResourceLimitController::new(false);
    let limits = controller.get_current_limits();
    assert!(limits.is_ok(), "Should be able to get current limits: {:?}", limits);
    
    let resource_limits = limits.unwrap();
    // At least some limits should be set (stack is usually set by default)
    assert!(resource_limits.stack_soft.is_some() || resource_limits.stack_hard.is_some(),
            "At least stack limits should be available");
}

#[test]
fn test_strict_mode_behavior() {
    let strict_controller = ResourceLimitController::new(true);
    let non_strict_controller = ResourceLimitController::new(false);
    
    // Both should handle normal operations the same way
    let strict_result = strict_controller.set_stack_limit(8 * 1024 * 1024);
    let non_strict_result = non_strict_controller.set_stack_limit(8 * 1024 * 1024);
    
    assert_eq!(strict_result.is_ok(), non_strict_result.is_ok(),
               "Both strict and non-strict modes should handle valid operations similarly");
}

#[test]
fn test_isolate_config_resource_limits() {
    let mut config = IsolateConfig::default();
    
    // Test default values
    assert_eq!(config.stack_limit, Some(8 * 1024 * 1024)); // 8MB default
    assert_eq!(config.core_limit, Some(0)); // Disabled by default
    assert_eq!(config.disk_quota, None); // No quota by default
    
    // Test setting custom values
    config.stack_limit = Some(16 * 1024 * 1024); // 16MB
    config.core_limit = Some(1024 * 1024); // 1MB
    config.disk_quota = Some(100 * 1024 * 1024); // 100MB
    
    assert_eq!(config.stack_limit, Some(16 * 1024 * 1024));
    assert_eq!(config.core_limit, Some(1024 * 1024));
    assert_eq!(config.disk_quota, Some(100 * 1024 * 1024));
}

#[test]
fn test_resource_limits_integration() {
    let controller = ResourceLimitController::new(false);
    
    // Test setting multiple limits together
    let stack_result = controller.set_stack_limit(8 * 1024 * 1024);
    let core_result = controller.set_core_limit(0);
    let file_result = controller.set_file_size_limit(50 * 1024 * 1024);
    let cpu_result = controller.set_cpu_time_limit(60);
    let proc_result = controller.set_process_limit(10);
    
    assert!(stack_result.is_ok(), "Stack limit should be set successfully");
    assert!(core_result.is_ok(), "Core limit should be set successfully");
    assert!(file_result.is_ok(), "File size limit should be set successfully");
    assert!(cpu_result.is_ok(), "CPU time limit should be set successfully");
    assert!(proc_result.is_ok(), "Process limit should be set successfully");
    
    // Verify we can still get current limits after setting them
    let limits = controller.get_current_limits();
    assert!(limits.is_ok(), "Should be able to get limits after setting them");
}