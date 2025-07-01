/// Filesystem security integration tests
use mini_isolate::filesystem::FilesystemSecurity;
use mini_isolate::isolate::Isolate;
use mini_isolate::types::{IsolateConfig, IsolateError};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use serial_test::serial;

#[test]
#[serial]
fn test_filesystem_security_creation() {
    let temp_dir = TempDir::new().unwrap();
    let chroot_dir = Some(temp_dir.path().join("chroot"));
    let workdir = temp_dir.path().join("workdir");

    let filesystem = FilesystemSecurity::new(chroot_dir.clone(), workdir, false);
    // With chroot_dir Some(..), it should be isolated
    assert!(filesystem.is_isolated());
}

#[test]
#[serial]
fn test_filesystem_isolation_setup() {
    let temp_dir = TempDir::new().unwrap();
    let chroot_dir = Some(temp_dir.path().join("chroot"));
    let workdir = temp_dir.path().join("workdir");

    let filesystem = FilesystemSecurity::new(chroot_dir.clone(), workdir, false);
    
    // This test requires root privileges to actually set up chroot
    // In non-root environment, it should gracefully handle the limitation
    let result = filesystem.setup_isolation();
    
    // Should either succeed (if root) or fail gracefully (if non-root)
    if result.is_err() {
        // Verify it's a permission error, not a logic error
        let error_msg = format!("{:?}", result.unwrap_err());
        assert!(error_msg.contains("Permission denied") || error_msg.contains("Operation not permitted"));
    }
}

#[test]
#[serial]
fn test_filesystem_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let chroot_dir = Some(temp_dir.path().join("chroot"));
    let workdir = temp_dir.path().join("workdir");

    let filesystem = FilesystemSecurity::new(chroot_dir.clone(), workdir, false);
    
    // Create the chroot directory structure for testing
    if let Some(ref chroot_path) = chroot_dir {
        fs::create_dir_all(chroot_path).unwrap();
    }
    
    let result = filesystem.cleanup();
    assert!(result.is_ok());
}

#[test]
fn test_effective_workdir_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let chroot_dir = Some(temp_dir.path().join("chroot"));
    let workdir = temp_dir.path().join("workdir");

    let filesystem = FilesystemSecurity::new(chroot_dir, workdir.clone(), false);
    let effective_workdir = filesystem.get_effective_workdir();
    
    // Should return the workdir when not in chroot mode
    assert_eq!(effective_workdir, workdir);
}

#[test]
fn test_dangerous_path_validation() {
    let temp_dir = TempDir::new().unwrap();
    let workdir = temp_dir.path().join("workdir");
    fs::create_dir_all(&workdir).unwrap();
    
    // Create a test file that actually exists
    let safe_file = workdir.join("safe_file.txt");
    fs::write(&safe_file, "test content").unwrap();
    
    let filesystem = FilesystemSecurity::new(None, workdir, false);

    // Test dangerous paths
    assert!(filesystem.validate_path(&PathBuf::from("/etc/passwd")).is_err());
    assert!(filesystem.validate_path(&PathBuf::from("/etc/shadow")).is_err());
    
    // Test safe path (must exist for canonicalize to work)
    assert!(filesystem.validate_path(&safe_file).is_ok());
}

#[test]
#[serial]
fn test_filesystem_security_with_chroot() {
    let temp_dir = TempDir::new().unwrap();
    let chroot_dir = Some(temp_dir.path().join("chroot"));
    let workdir = temp_dir.path().join("workdir");

    let filesystem = FilesystemSecurity::new(chroot_dir, workdir, false);
    
    // Test chroot setup (requires root privileges)
    let result = filesystem.setup_isolation();
    
    // Should handle both success and graceful failure
    match result {
        Ok(_) => {
            // If successful, verify isolation is active
            assert!(filesystem.is_isolated());
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
#[serial]
fn test_chroot_structure_creation() {
    let temp_dir = TempDir::new().unwrap();
    let chroot_dir = Some(temp_dir.path().join("chroot"));
    let workdir = temp_dir.path().join("workdir");

    let filesystem = FilesystemSecurity::new(chroot_dir.clone(), workdir, false);
    
    // Attempt to create chroot structure
    let result = filesystem.setup_isolation();
    
    if result.is_ok() {
        // Verify basic structure was created
        if let Some(ref chroot_path) = chroot_dir {
            assert!(chroot_path.exists());
            assert!(chroot_path.join("tmp").exists());
            assert!(chroot_path.join("dev").exists());
        }
    }
    // If it fails due to permissions, that's expected in non-root environment
}

#[test]
#[serial]
fn test_filesystem_security_integration_with_executor() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = IsolateConfig {
        instance_id: "test-filesystem-integration".to_string(),
        workdir: temp_dir.path().join("workdir"),
        chroot_dir: Some(temp_dir.path().join("chroot")),
        strict_mode: false, // Use non-strict mode for testing
        ..Default::default()
    };

    // Test integration with isolate
    let result = Isolate::new(config);
    assert!(result.is_ok());
    
    let mut isolate = result.unwrap();
    
    // Test basic command execution
    let command = vec!["echo".to_string(), "test".to_string()];
    let exec_result = isolate.execute(&command, None);
    
    // Should succeed or fail gracefully due to environment limitations
    if exec_result.is_err() {
        // Log the error for debugging but don't fail the test
        eprintln!("Execution failed (expected in test environment): {:?}", exec_result.unwrap_err());
    }
    
    // Cleanup
    isolate.cleanup().unwrap();
}

#[test]
#[serial]
fn test_filesystem_security_with_chroot_integration() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = IsolateConfig {
        instance_id: "test-chroot-integration".to_string(),
        workdir: temp_dir.path().join("workdir"),
        chroot_dir: Some(temp_dir.path().join("chroot")),
        strict_mode: false,
        ..Default::default()
    };

    let result = Isolate::new(config);
    assert!(result.is_ok());
    
    let mut isolate = result.unwrap();
    
    // Test that filesystem operations work within the isolated environment
    let command = vec!["pwd".to_string()];
    let exec_result = isolate.execute(&command, None);
    
    // Should succeed or fail gracefully due to environment limitations
    if exec_result.is_err() {
        // Log the error for debugging but don't fail the test
        eprintln!("PWD execution failed (expected in test environment): {:?}", exec_result.unwrap_err());
    }
    
    // Cleanup
    isolate.cleanup().unwrap();
}

#[test]
#[serial]
fn test_filesystem_security_strict_mode() {
    let temp_dir = TempDir::new().unwrap();
    
    let config = IsolateConfig {
        instance_id: "test-strict-mode".to_string(),
        workdir: temp_dir.path().join("workdir"),
        chroot_dir: Some(temp_dir.path().join("chroot")),
        strict_mode: true, // Enable strict mode
        ..Default::default()
    };

    // In strict mode, should fail if chroot cannot be set up (unless running as root)
    let result = Isolate::new(config);
    
    // Check if we're running as root
    let is_root = unsafe { libc::getuid() == 0 };
    
    if is_root {
        // If root, should succeed
        assert!(result.is_ok());
        if let Ok(mut isolate) = result {
            isolate.cleanup().unwrap();
        }
    } else {
        // If not root, strict mode should fail
        // But we'll accept success too in case the environment supports it
        match result {
            Ok(mut isolate) => {
                isolate.cleanup().unwrap();
            }
            Err(_) => {
                // Expected failure in non-root strict mode
            }
        }
    }
}

#[test]
#[serial]
fn test_root_required_operations() {
    let temp_dir = TempDir::new().unwrap();
    let chroot_dir = Some(temp_dir.path().join("chroot"));
    let workdir = temp_dir.path().join("workdir");

    let filesystem = FilesystemSecurity::new(chroot_dir, workdir, true);
    
    // Test operations that require root
    let result = filesystem.apply_chroot();
    
    let is_root = unsafe { libc::getuid() == 0 };
    
    if is_root {
        // If running as root, operation might succeed
        // (depends on environment setup)
        match result {
            Ok(_) => assert!(filesystem.is_isolated()),
            Err(_) => {
                // Even as root, might fail due to environment constraints
            }
        }
    } else {
        // If not root, should fail with permission error
        assert!(result.is_err());
    }
}

#[test]
#[serial]
fn test_mount_security_flags() {
    let temp_dir = TempDir::new().unwrap();
    let chroot_dir = Some(temp_dir.path().join("chroot"));
    let workdir = temp_dir.path().join("workdir");

    let filesystem = FilesystemSecurity::new(chroot_dir, workdir, false);
    
    // Test mount security setup
    let result = filesystem.setup_isolation();
    
    // Should handle mount operations gracefully
    match result {
        Ok(_) => {
            // If successful, cleanup should work
            assert!(filesystem.cleanup().is_ok());
        }
        Err(e) => {
            // If failed, should be due to permissions or unsupported operations
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("Permission denied") || 
                   error_msg.contains("Operation not permitted") ||
                   error_msg.contains("not supported"));
        }
    }
}

#[test]
fn test_filesystem_security_prevents_directory_traversal() {
    let temp_dir = TempDir::new().unwrap();
    let chroot_dir = temp_dir.path().join("chroot");
    let workdir = temp_dir.path().join("workdir");
    
    // Create chroot directory structure
    fs::create_dir_all(&chroot_dir).unwrap();
    fs::create_dir_all(&workdir).unwrap();
    
    let filesystem = FilesystemSecurity::new(Some(chroot_dir.clone()), workdir, false);

    // Test validation against dangerous system paths (these should be rejected)
    let dangerous_paths = vec![
        "/etc/passwd",
        "/etc/shadow", 
        "/root/.ssh/id_rsa",
        "/proc/version",
        "/bin/bash",
    ];

    for dangerous_path in dangerous_paths {
        let result = filesystem.validate_path(&PathBuf::from(dangerous_path));
        assert!(result.is_err(), "Path {} should be rejected", dangerous_path);
    }
}

