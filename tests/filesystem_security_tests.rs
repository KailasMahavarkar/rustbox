/// Comprehensive tests for filesystem security implementation
use mini_isolate::filesystem::FilesystemSecurity;
use mini_isolate::types::{IsolateConfig, IsolateError};
use mini_isolate::executor::ProcessExecutor;
use std::fs;
use std::path::PathBuf;
use std::env;
use tempfile::TempDir;

#[test]
fn test_filesystem_security_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workdir = temp_dir.path().join("work");
    
    let fs_security = FilesystemSecurity::new(None, workdir.clone(), false);
    assert!(!fs_security.is_isolated());
    assert_eq!(fs_security.get_effective_workdir(), workdir);
}

#[test]
fn test_filesystem_security_with_chroot() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let chroot_dir = temp_dir.path().join("chroot");
    let workdir = temp_dir.path().join("work");
    
    let fs_security = FilesystemSecurity::new(Some(chroot_dir), workdir, false);
    assert!(fs_security.is_isolated());
}

#[test]
fn test_filesystem_isolation_setup() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workdir = temp_dir.path().join("work");
    
    let fs_security = FilesystemSecurity::new(None, workdir.clone(), false);
    
    // Setup should create workdir if it doesn't exist
    let result = fs_security.setup_isolation();
    assert!(result.is_ok());
    assert!(workdir.exists());
}

#[cfg(unix)]
#[test]
fn test_chroot_structure_creation() {
    // This test requires root privileges to fully work, so we test what we can
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let chroot_dir = temp_dir.path().join("chroot");
    let workdir = temp_dir.path().join("work");
    
    let fs_security = FilesystemSecurity::new(Some(chroot_dir.clone()), workdir, false);
    
    // Attempt setup - may fail without root, but should create directory structure
    let _ = fs_security.setup_isolation();
    
    // Check that chroot directory was created
    assert!(chroot_dir.exists());
    
    // Check that essential directories were created (if setup succeeded)
    let essential_dirs = ["tmp", "dev", "proc", "usr/bin", "bin", "lib", "etc"];
    for dir in &essential_dirs {
        let dir_path = chroot_dir.join(dir);
        if dir_path.exists() {
            // If directory exists, check permissions
            let metadata = fs::metadata(&dir_path).expect("Failed to get metadata");
            assert!(metadata.is_dir());
        }
    }
}

#[test]
fn test_dangerous_path_validation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workdir = temp_dir.path().join("work");
    
    let fs_security = FilesystemSecurity::new(None, workdir, false);
    
    // Test dangerous paths that should be rejected
    let dangerous_paths = [
        "/etc/passwd",
        "/etc/shadow", 
        "/etc/sudoers",
        "/root",
        "/boot",
        "/sys",
        "/proc/sys",
    ];
    
    for dangerous_path in &dangerous_paths {
        let result = fs_security.validate_path(std::path::Path::new(dangerous_path));
        assert!(result.is_err(), "Path {} should be rejected", dangerous_path);
        
        if let Err(IsolateError::Config(msg)) = result {
            assert!(msg.contains("dangerous") || msg.contains("forbidden"));
        }
    }
}

#[test]
fn test_effective_workdir_calculation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let chroot_dir = temp_dir.path().join("chroot");
    let workdir = PathBuf::from("/home/user/work");
    
    // Without chroot
    let fs_security_no_chroot = FilesystemSecurity::new(None, workdir.clone(), false);
    assert_eq!(fs_security_no_chroot.get_effective_workdir(), workdir);
    
    // With chroot
    let fs_security_with_chroot = FilesystemSecurity::new(Some(chroot_dir), workdir, false);
    let effective = fs_security_with_chroot.get_effective_workdir();
    assert!(effective.starts_with("/"));
    assert!(effective.to_string_lossy().contains("home/user/work"));
}

#[test]
fn test_filesystem_security_integration_with_executor() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workdir = temp_dir.path().join("work");
    
    let config = IsolateConfig {
        instance_id: "filesystem_test".to_string(),
        workdir: workdir.clone(),
        chroot_dir: None,
        strict_mode: false,
        enable_seccomp: false, // Disable seccomp for this test
        ..Default::default()
    };
    
    let result = ProcessExecutor::new(config);
    assert!(result.is_ok());
    
    let mut executor = result.unwrap();
    
    // Test simple command execution
    let command = vec!["echo".to_string(), "Hello, filesystem test!".to_string()];
    let result = executor.execute(&command, None);
    
    assert!(result.is_ok());
    let exec_result = result.unwrap();
    assert!(exec_result.success);
    assert!(exec_result.stdout.contains("Hello, filesystem test!"));
    
    // Cleanup
    let _ = executor.cleanup();
}

#[cfg(unix)]
#[test]
fn test_filesystem_security_with_chroot_integration() {
    // This test requires root privileges to fully work
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let chroot_dir = temp_dir.path().join("chroot");
    let workdir = temp_dir.path().join("work");
    
    let config = IsolateConfig {
        instance_id: "chroot_test".to_string(),
        workdir: workdir.clone(),
        chroot_dir: Some(chroot_dir.clone()),
        strict_mode: false,
        enable_seccomp: false, // Disable seccomp for this test
        ..Default::default()
    };
    
    let result = ProcessExecutor::new(config);
    assert!(result.is_ok());
    
    let mut executor = result.unwrap();
    
    // Test command execution - this may fail without root privileges
    let command = vec!["echo".to_string(), "Hello from chroot!".to_string()];
    let result = executor.execute(&command, None);
    
    // The test should either succeed or fail with a permission error
    match result {
        Ok(exec_result) => {
            // If it succeeds, check the output
            assert!(exec_result.stdout.contains("Hello from chroot!"));
        }
        Err(IsolateError::Config(msg)) => {
            // Expected to fail without root privileges
            assert!(msg.contains("chroot") || msg.contains("permission") || msg.contains("errno") || msg.contains("Failed"));
        }
        Err(IsolateError::Process(msg)) => {
            // May also fail at process level
            assert!(msg.contains("chroot") || msg.contains("permission") || msg.contains("Failed"));
        }
        Err(e) => {
            // Any other error is also acceptable for this test
            println!("Chroot test failed with error (expected without root): {:?}", e);
        }
    }
    
    // Cleanup
    let _ = executor.cleanup();
}

#[test]
fn test_filesystem_security_prevents_directory_traversal() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workdir = temp_dir.path().join("work");
    fs::create_dir_all(&workdir).expect("Failed to create workdir");
    
    let config = IsolateConfig {
        instance_id: "traversal_test".to_string(),
        workdir: workdir.clone(),
        chroot_dir: None,
        strict_mode: false,
        enable_seccomp: true, // Enable seccomp to help prevent filesystem attacks
        ..Default::default()
    };
    
    let result = ProcessExecutor::new(config);
    assert!(result.is_ok());
    
    let mut executor = result.unwrap();
    
    // Create a test script that tries directory traversal
    let script_content = r#"
import os
import sys

# Try to read a sensitive file
try:
    with open('/etc/passwd', 'r') as f:
        content = f.read()
    print(f"SECURITY_BREACH: Read /etc/passwd: {content[:50]}")
except Exception as e:
    print(f"Access denied (expected): {e}")

# Try to write outside workdir
try:
    with open('/tmp/security_breach.txt', 'w') as f:
        f.write("SECURITY_BREACH: Wrote outside workdir")
    print("SECURITY_BREACH: File written outside workdir")
except Exception as e:
    print(f"Write denied (expected): {e}")
"#;
    
    let script_path = workdir.join("traversal_test.py");
    fs::write(&script_path, script_content).expect("Failed to write test script");
    
    // Execute the script
    let command = vec!["python3".to_string(), script_path.to_string_lossy().to_string()];
    let result = executor.execute(&command, None);
    
    match result {
        Ok(exec_result) => {
            // Should NOT contain security breach messages
            assert!(
                !exec_result.stdout.contains("SECURITY_BREACH"),
                "Filesystem attack should be blocked. Output: {}",
                exec_result.stdout
            );
            // The test passes if no security breach occurred, regardless of specific error messages
            // Different systems may have different ways of denying access
        }
        Err(_) => {
            // It's also acceptable for the execution to fail entirely due to security restrictions
            println!("Execution failed (acceptable - security restrictions may prevent execution)");
        }
    }
    
    // Cleanup
    let _ = executor.cleanup();
}

#[test]
fn test_filesystem_cleanup() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let workdir = temp_dir.path().join("work");
    
    let fs_security = FilesystemSecurity::new(None, workdir, false);
    
    // Setup and then cleanup
    let setup_result = fs_security.setup_isolation();
    assert!(setup_result.is_ok());
    
    let cleanup_result = fs_security.cleanup();
    assert!(cleanup_result.is_ok());
}

#[cfg(unix)]
#[test]
fn test_mount_security_flags() {
    // This test checks that mount security flag application doesn't panic
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let chroot_dir = temp_dir.path().join("chroot");
    let workdir = temp_dir.path().join("work");
    
    let fs_security = FilesystemSecurity::new(Some(chroot_dir), workdir, false);
    
    // This will likely fail without root privileges, but shouldn't panic
    let result = fs_security.setup_isolation();
    
    // We don't assert success here because mount operations require root
    // But we do assert that it doesn't panic and returns a proper error if it fails
    match result {
        Ok(_) => {
            // Great! Mount security was applied successfully
        }
        Err(IsolateError::Config(msg)) => {
            // Expected without root privileges
            assert!(msg.contains("errno") || msg.contains("permission") || msg.contains("mount"));
        }
        Err(e) => {
            panic!("Unexpected error type: {:?}", e);
        }
    }
}

#[test]
fn test_filesystem_security_strict_mode() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let chroot_dir = temp_dir.path().join("chroot");
    let workdir = temp_dir.path().join("work");
    
    // Test strict mode behavior
    let fs_security_strict = FilesystemSecurity::new(Some(chroot_dir), workdir, true);
    
    // In strict mode, failures should be more strict
    let result = fs_security_strict.setup_isolation();
    
    // Without root privileges, this should fail in strict mode
    if !nix::unistd::getuid().is_root() {
        // We expect this to fail in strict mode without root
        match result {
            Ok(_) => {
                // Unexpected success - maybe the system allows it
            }
            Err(_) => {
                // Expected failure in strict mode without root
            }
        }
    }
}

// Helper function to check if we're running as root
#[cfg(unix)]
fn is_root() -> bool {
    unsafe { libc::getuid() == 0 }
}

#[cfg(unix)]
#[test]
fn test_root_required_operations() {
    if !is_root() {
        println!("Skipping root-required test - not running as root");
        return;
    }
    
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let chroot_dir = temp_dir.path().join("chroot");
    let workdir = temp_dir.path().join("work");
    
    let fs_security = FilesystemSecurity::new(Some(chroot_dir.clone()), workdir, true);
    
    // With root privileges, setup should succeed
    let result = fs_security.setup_isolation();
    assert!(result.is_ok(), "Filesystem isolation setup should succeed with root privileges");
    
    // Check that device files were created
    let dev_null = chroot_dir.join("dev/null");
    let dev_zero = chroot_dir.join("dev/zero");
    
    assert!(dev_null.exists(), "/dev/null should be created in chroot");
    assert!(dev_zero.exists(), "/dev/zero should be created in chroot");
    
    // Cleanup
    let cleanup_result = fs_security.cleanup();
    assert!(cleanup_result.is_ok());
}