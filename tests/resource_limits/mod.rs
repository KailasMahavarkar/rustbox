//! Resource limit override tests
//!
//! Tests for the new execute_with_overrides functionality

#[test]
fn test_execute_with_overrides_cpu_limit() {
    let mut isolate = crate::create_test_isolate("test_cpu_override").expect("Failed to create isolate");
    
    // Test that execute_with_overrides accepts CPU override
    let result = isolate.execute_with_overrides(
        &["/bin/echo".to_string(), "test".to_string()], 
        None,
        Some(30), // max_cpu: 30 seconds
        None,     // max_memory: None 
        None      // max_time: None
    );
    
    assert!(result.is_ok(), "execute_with_overrides should work with CPU override");
    let result = result.unwrap();
    assert!(result.success, "Command should succeed");
    assert_eq!(result.exit_code, Some(0), "Exit code should be 0");
    
    isolate.cleanup().expect("Failed to cleanup");
}

#[test]
fn test_execute_with_overrides_memory_limit() {
    let mut isolate = crate::create_test_isolate("test_mem_override").expect("Failed to create isolate");
    
    // Test that execute_with_overrides accepts memory override
    let result = isolate.execute_with_overrides(
        &["/bin/echo".to_string(), "test".to_string()], 
        None,
        None,        // max_cpu: None
        Some(512),   // max_memory: 512MB
        None         // max_time: None
    );
    
    assert!(result.is_ok(), "execute_with_overrides should work with memory override");
    let result = result.unwrap();
    assert!(result.success, "Command should succeed");
    
    isolate.cleanup().expect("Failed to cleanup");
}

#[test]
fn test_execute_with_overrides_time_limit() {
    let mut isolate = crate::create_test_isolate("test_time_override").expect("Failed to create isolate");
    
    // Test that execute_with_overrides accepts time override
    let result = isolate.execute_with_overrides(
        &["/bin/echo".to_string(), "test".to_string()], 
        None,
        None,       // max_cpu: None
        None,       // max_memory: None
        Some(60)    // max_time: 60 seconds
    );
    
    assert!(result.is_ok(), "execute_with_overrides should work with time override");
    let result = result.unwrap();
    assert!(result.success, "Command should succeed");
    
    isolate.cleanup().expect("Failed to cleanup");
}

#[test]
fn test_execute_with_overrides_all_limits() {
    let mut isolate = crate::create_test_isolate("test_all_override").expect("Failed to create isolate");
    
    // Test that execute_with_overrides accepts all overrides
    let result = isolate.execute_with_overrides(
        &["/bin/echo".to_string(), "test".to_string()], 
        None,
        Some(30),   // max_cpu: 30 seconds
        Some(256),  // max_memory: 256MB
        Some(60)    // max_time: 60 seconds
    );
    
    assert!(result.is_ok(), "execute_with_overrides should work with all overrides");
    let result = result.unwrap();
    assert!(result.success, "Command should succeed");
    
    isolate.cleanup().expect("Failed to cleanup");
}

#[test]
fn test_execute_file_with_overrides() {
    use std::fs;
    use tempfile::tempdir;
    
    let tempdir = tempdir().expect("Failed to create tempdir");
    let mut isolate = crate::create_test_isolate("test_file_override").expect("Failed to create isolate");
    
    // Test that execute_file_with_overrides method exists and works
    // Use a simple shell script to avoid Python environment issues
    let shell_content = r#"#!/bin/bash
echo "Hello from shell script with overrides!"
"#;
    
    // Write to a temporary file outside the working directory
    let test_shell_file = tempdir.path().join("test_override.sh");
    fs::write(&test_shell_file, shell_content).expect("Failed to write test file");
    
    // Make it executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&test_shell_file).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&test_shell_file, perms).unwrap();
    }
    
    
    let result = isolate.execute_file_with_overrides(
        &test_shell_file,
        None,
        Some(10),   // max_cpu
        Some(128),  // max_memory  
        Some(30)    // max_time
    );
    
    match &result {
        Ok(r) => println!("Success: stdout='{}', stderr='{}'", r.stdout, r.stderr),
        Err(e) => println!("Error: {}", e),
    }
    
    assert!(result.is_ok(), "execute_file_with_overrides should work");
    let result = result.unwrap();
    assert!(result.success, "Command should succeed");
    assert!(result.stdout.contains("Hello from shell script"), "Should contain expected output");
    
    isolate.cleanup().expect("Failed to cleanup");
}

#[test]
fn test_original_execute_still_works() {
    let mut isolate = crate::create_test_isolate("test_original").expect("Failed to create isolate");
    
    // Test that the original execute method still works (backward compatibility)
    let result = isolate.execute(
        &["/bin/echo".to_string(), "original method".to_string()], 
        None
    );
    
    assert!(result.is_ok(), "Original execute method should still work");
    let result = result.unwrap();
    assert!(result.success, "Command should succeed");
    assert!(result.stdout.contains("original method"), "Should contain expected output");
    
    isolate.cleanup().expect("Failed to cleanup");
}