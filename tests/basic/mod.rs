mod cgroup;

#[test] 
fn test_isolate_creation_and_cleanup() {
    let isolate = crate::create_test_isolate("test_basic").expect("Failed to create isolate");
    
    // Test that isolate directory is created
    assert!(isolate.config().workdir.exists(), "Work directory should exist");
    
    // Test cleanup
    isolate.cleanup().expect("Failed to cleanup isolate");
    
    // Note: We intentionally leak the temp directory in tests to avoid cleanup races
    // In real usage, the temp directory would be properly cleaned up
}

#[test]
fn test_basic_command_execution() {
    let mut isolate = crate::create_test_isolate("test_cmd").expect("Failed to create isolate");
    
    let result = isolate.execute(&[
        "/bin/echo".to_string(),
        "Hello from isolate".to_string()
    ], None).expect("Failed to execute command");
    
    assert!(result.success, "Command should succeed");
    assert_eq!(result.exit_code, Some(0), "Exit code should be 0");
    assert!(result.stdout.contains("Hello from isolate"), "Output should contain expected text");
    
    isolate.cleanup().expect("Failed to cleanup");
}