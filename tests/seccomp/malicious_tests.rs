/// Integration tests for seccomp filtering against malicious code
use mini_isolate::types::IsolateConfig;
use mini_isolate::isolate::Isolate;
use mini_isolate::seccomp::is_seccomp_supported;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

/// Helper to create test scripts from the malicious code templates
fn create_malicious_script(temp_dir: &TempDir, attack_type: &str) -> std::path::PathBuf {
    let script_path = temp_dir.path().join(format!("{}_attack.py", attack_type));
    
    let script_content = match attack_type {
        "network" => r#"
import socket
import sys

try:
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    print("SECURITY_BREACH: Socket created")
    s.connect(("127.0.0.1", 80))
    print("SECURITY_BREACH: Network connection established")
    s.close()
except Exception as e:
    print(f"Expected security error: {e}")
    sys.exit(1)
"#,
        "process" => r#"
import os
import subprocess
import sys

try:
    pid = os.fork()
    if pid == 0:
        print("SECURITY_BREACH: Child process created via fork")
        os._exit(0)
    else:
        print("SECURITY_BREACH: Fork succeeded")
        os.waitpid(pid, 0)
except Exception as e:
    print(f"Expected fork error: {e}")

try:
    result = subprocess.run(['echo', 'SECURITY_BREACH: Subprocess executed'], 
                          capture_output=True, text=True)
    print(f"SECURITY_BREACH: {result.stdout.strip()}")
except Exception as e:
    print(f"Expected subprocess error: {e}")
    sys.exit(1)
"#,
        "filesystem" => r#"
import os
import sys

try:
    with open('/etc/passwd', 'r') as f:
        content = f.read()
        print(f"SECURITY_BREACH: Read /etc/passwd")
except Exception as e:
    print(f"Expected passwd read error: {e}")

try:
    with open('/tmp/security_breach.txt', 'w') as f:
        f.write("SECURITY_BREACH: Wrote outside workdir")
    print("SECURITY_BREACH: File written outside workdir")
except Exception as e:
    print(f"Expected write error: {e}")
    sys.exit(1)
"#,
        "privilege" => r#"
import os
import ctypes
import ctypes.util
import sys

try:
    os.setuid(0)
    print("SECURITY_BREACH: setuid to root succeeded")
except Exception as e:
    print(f"Expected setuid error: {e}")

try:
    libc = ctypes.CDLL(ctypes.util.find_library("c"))
    result = libc.mount(None, b"/tmp", None, 0, None)
    if result == 0:
        print("SECURITY_BREACH: Mount syscall succeeded")
except Exception as e:
    print(f"Expected ctypes error: {e}")
    sys.exit(1)
"#,
        _ => r#"
print("Unknown attack type")
import sys
sys.exit(1)
"#,
    };
    
    fs::write(&script_path, script_content).expect("Failed to write attack script");
    script_path
}

#[test]
#[serial]
fn test_seccomp_blocks_network_attacks() {
    if !is_seccomp_supported() {
        println!("Skipping network attack test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let script = create_malicious_script(&temp_dir, "network");
    
    let config = IsolateConfig {
        instance_id: "network_attack_test".to_string(),
        workdir: temp_dir.path().to_path_buf(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        time_limit: Some(std::time::Duration::from_secs(5)),
        ..Default::default()
    };
    
    let mut isolate = Isolate::new(config).expect("Failed to create isolate");
    let result = isolate.execute_file(&script, None);
    
    match result {
        Ok(exec_result) => {
            // Process should not succeed in creating network connections
            assert!(
                !exec_result.stdout.contains("SECURITY_BREACH"),
                "Network attack should be blocked by seccomp. Output: {}",
                exec_result.stdout
            );
            
            // Process should either be killed by signal or get permission error
            assert!(
                !exec_result.success || exec_result.signal.is_some(),
                "Network attack should fail or be killed"
            );
        }
        Err(_) => {
            // Process failed to start - acceptable as seccomp might block early
            println!("Network attack process failed to start (blocked by seccomp)");
        }
    }
}

#[test] 
#[serial]
fn test_seccomp_blocks_process_creation_attacks() {
    if !is_seccomp_supported() {
        println!("Skipping process creation attack test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let script = create_malicious_script(&temp_dir, "process");
    
    let config = IsolateConfig {
        instance_id: "process_attack_test".to_string(),
        workdir: temp_dir.path().to_path_buf(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        time_limit: Some(std::time::Duration::from_secs(5)),
        ..Default::default()
    };
    
    let mut isolate = Isolate::new(config).expect("Failed to create isolate");
    let result = isolate.execute_file(&script, None);
    
    match result {
        Ok(exec_result) => {
            // Process creation should be blocked
            assert!(
                !exec_result.stdout.contains("SECURITY_BREACH"),
                "Process creation attack should be blocked. Output: {}",
                exec_result.stdout
            );
            
            // Should be killed by seccomp or fail
            assert!(
                !exec_result.success,
                "Process creation attack should not succeed"
            );
        }
        Err(_) => {
            println!("Process creation attack blocked at startup");
        }
    }
}

#[test]
#[serial] 
fn test_seccomp_blocks_filesystem_attacks() {
    if !is_seccomp_supported() {
        println!("Skipping filesystem attack test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let script = create_malicious_script(&temp_dir, "filesystem");
    
    let config = IsolateConfig {
        instance_id: "filesystem_attack_test".to_string(),
        workdir: temp_dir.path().to_path_buf(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        time_limit: Some(std::time::Duration::from_secs(5)),
        ..Default::default()
    };
    
    let mut isolate = Isolate::new(config).expect("Failed to create isolate");
    let result = isolate.execute_file(&script, None);
    
    match result {
        Ok(exec_result) => {
            // Should not be able to read sensitive files or write outside workdir
            assert!(
                !exec_result.stdout.contains("SECURITY_BREACH"),
                "Filesystem attack should be blocked. Output: {}",
                exec_result.stdout
            );
        }
        Err(_) => {
            println!("Filesystem attack blocked at startup");
        }
    }
}

#[test]
#[serial]
fn test_seccomp_blocks_privilege_escalation_attacks() {
    if !is_seccomp_supported() {
        println!("Skipping privilege escalation attack test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let script = create_malicious_script(&temp_dir, "privilege");
    
    let config = IsolateConfig {
        instance_id: "privilege_attack_test".to_string(),
        workdir: temp_dir.path().to_path_buf(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        time_limit: Some(std::time::Duration::from_secs(5)),
        ..Default::default()
    };
    
    let mut isolate = Isolate::new(config).expect("Failed to create isolate");
    let result = isolate.execute_file(&script, None);
    
    match result {
        Ok(exec_result) => {
            // Privilege escalation should be blocked
            assert!(
                !exec_result.stdout.contains("SECURITY_BREACH"),
                "Privilege escalation should be blocked. Output: {}",
                exec_result.stdout
            );
            
            // Should fail or be killed
            assert!(
                !exec_result.success || exec_result.signal.is_some(),
                "Privilege escalation should not succeed"
            );
        }
        Err(_) => {
            println!("Privilege escalation attack blocked at startup");
        }
    }
}

#[test]
#[serial]
fn test_seccomp_allows_safe_operations() {
    if !is_seccomp_supported() {
        println!("Skipping safe operations test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let script = temp_dir.path().join("safe_script.py");
    
    // Script that only does safe operations
    fs::write(&script, r#"
import os
import time
import math

# Safe mathematical operations
result = 0
for i in range(1000):
    result += math.sqrt(i)

print(f"Mathematical computation result: {result:.2f}")

# Safe file operations within workdir
with open("test_output.txt", "w") as f:
    f.write("Hello, secure world!")

with open("test_output.txt", "r") as f:
    content = f.read()
    print(f"File content: {content}")

# Safe system information (allowed syscalls)
print(f"Process ID: {os.getpid()}")
print(f"Current time: {time.time()}")

print("All safe operations completed successfully")
"#).expect("Failed to write safe script");
    
    let config = IsolateConfig {
        instance_id: "safe_operations_test".to_string(),
        workdir: temp_dir.path().to_path_buf(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        time_limit: Some(std::time::Duration::from_secs(10)),
        ..Default::default()
    };
    
    let mut isolate = Isolate::new(config).expect("Failed to create isolate");
    let result = isolate.execute_file(&script, None).expect("Safe script should execute");
    
    // Safe operations should complete successfully
    assert!(result.success, "Safe operations should succeed with seccomp");
    assert!(
        result.stdout.contains("All safe operations completed successfully"),
        "Safe script should complete. Output: {}",
        result.stdout
    );
    assert!(
        result.stdout.contains("Mathematical computation result"),
        "Math operations should work"
    );
    assert!(
        result.stdout.contains("File content: Hello, secure world!"),
        "File operations should work within workdir"
    );
}

#[test]
#[serial]
fn test_seccomp_different_languages() {
    if !is_seccomp_supported() {
        println!("Skipping language-specific test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    
    // Test Python-specific seccomp profile
    let python_script = temp_dir.path().join("python_test.py");
    fs::write(&python_script, r#"
import stat
import os

# Python should be able to use stat syscall
try:
    file_stat = os.stat(".")
    print("Python stat syscall works")
except Exception as e:
    print(f"Python stat failed: {e}")

# But should not be able to create sockets
try:
    import socket
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    print("SECURITY_BREACH: Socket created in Python")
    s.close()
except Exception as e:
    print(f"Expected Python socket error: {e}")

print("Python test completed")
"#).expect("Failed to write Python test");
    
    let config = IsolateConfig {
        instance_id: "python_seccomp_test".to_string(),
        workdir: temp_dir.path().to_path_buf(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        time_limit: Some(std::time::Duration::from_secs(5)),
        ..Default::default()
    };
    
    let mut isolate = Isolate::new(config).expect("Failed to create isolate");
    let result = isolate.execute_file(&python_script, None);
    
    match result {
        Ok(exec_result) => {
            // Should complete the stat operation but block socket
            assert!(
                exec_result.stdout.contains("Python stat syscall works"),
                "Python should be able to use stat. Output: {}",
                exec_result.stdout
            );
            assert!(
                !exec_result.stdout.contains("SECURITY_BREACH"),
                "Python socket creation should be blocked"
            );
        }
        Err(e) => {
            println!("Python test failed to execute: {}", e);
        }
    }
}

#[test]
#[serial]
fn test_seccomp_performance_overhead() {
    if !is_seccomp_supported() {
        println!("Skipping performance test - seccomp not supported");
        return;
    }

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let perf_script = temp_dir.path().join("performance_test.py");
    
    fs::write(&perf_script, r#"
import time
import math

start_time = time.time()

# Computationally intensive but safe operations
result = 0
for i in range(50000):
    result += math.sin(i) * math.cos(i)
    if i % 10000 == 0:
        # File I/O operations
        with open(f"temp_{i}.txt", "w") as f:
            f.write(f"Iteration {i}: {result}")

end_time = time.time()
execution_time = end_time - start_time

print(f"Performance test completed in {execution_time:.3f} seconds")
print(f"Final result: {result:.6f}")
"#).expect("Failed to write performance test");
    
    // Test with seccomp enabled
    let config_with_seccomp = IsolateConfig {
        instance_id: "perf_with_seccomp".to_string(),
        workdir: temp_dir.path().to_path_buf(),
        enable_seccomp: true,
        seccomp_profile: Some("python".to_string()),
        strict_mode: false,
        time_limit: Some(std::time::Duration::from_secs(30)),
        ..Default::default()
    };
    
    // Test without seccomp
    let config_without_seccomp = IsolateConfig {
        instance_id: "perf_without_seccomp".to_string(),
        workdir: temp_dir.path().to_path_buf(),
        enable_seccomp: false,
        strict_mode: false,
        time_limit: Some(std::time::Duration::from_secs(30)),
        ..Default::default()
    };
    
    let mut isolate_with = Isolate::new(config_with_seccomp).expect("Failed to create isolate with seccomp");
    let mut isolate_without = Isolate::new(config_without_seccomp).expect("Failed to create isolate without seccomp");
    
    let result_with = isolate_with.execute_file(&perf_script, None).expect("Performance test with seccomp failed");
    let result_without = isolate_without.execute_file(&perf_script, None).expect("Performance test without seccomp failed");
    
    // Both should succeed
    assert!(result_with.success, "Performance test should succeed with seccomp");
    assert!(result_without.success, "Performance test should succeed without seccomp");
    
    // Performance impact should be reasonable (less than 50% overhead)
    let overhead_ratio = result_with.wall_time / result_without.wall_time;
    
    println!("Performance overhead: {:.2}x", overhead_ratio);
    println!("With seccomp: {:.3}s", result_with.wall_time);
    println!("Without seccomp: {:.3}s", result_without.wall_time);
    
    assert!(
        overhead_ratio < 1.5,
        "Seccomp overhead should be less than 50%, got {:.2}x",
        overhead_ratio
    );
}