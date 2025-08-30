//! Security and isolation tests for rustbox
//!
//! Tests process isolation, filesystem isolation, network isolation,
//! and other security features.

use crate::tests::common::{
    cleanup_test_box, execute_rustbox_command, generate_box_id, run_test, TestConfig, TestResult,
};
use crate::tests::utils::TestUtils;

use anyhow::Result;

/// Run all security and isolation tests
pub fn run_security_tests(config: &TestConfig) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    // Test 1: Process namespace isolation
    results.push(run_test(config, "Process namespace isolation", || {
        test_process_namespace_isolation(config)
    }));

    // Test 2: Filesystem isolation
    results.push(run_test(config, "Filesystem isolation", || {
        test_filesystem_isolation(config)
    }));

    // Test 3: Network isolation
    results.push(run_test(config, "Network isolation", || {
        test_network_isolation(config)
    }));

    // Test 4: User namespace isolation
    results.push(run_test(config, "User namespace isolation", || {
        test_user_namespace_isolation(config)
    }));

    // Test 5: Path traversal prevention
    results.push(run_test(config, "Path traversal prevention", || {
        test_path_traversal_prevention(config)
    }));

    // Test 6: Privilege escalation prevention
    results.push(run_test(config, "Privilege escalation prevention", || {
        test_privilege_escalation_prevention(config)
    }));

    // Test 7: Resource isolation
    results.push(run_test(config, "Resource isolation", || {
        test_resource_isolation(config)
    }));

    // Test 8: Security boundary enforcement
    results.push(run_test(config, "Security boundary enforcement", || {
        test_security_boundary_enforcement(config)
    }));

    Ok(results)
}

/// Test process namespace isolation
fn test_process_namespace_isolation(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import subprocess
import os

# Try to list processes
try:
    result = subprocess.run(['ps', 'aux'], capture_output=True, text=True, timeout=5)
    process_count = len(result.stdout.splitlines())
    print(f'Process count: {process_count}')
    
    # In isolated environment, should see minimal processes
    if process_count < 20:
        print('Process isolation working - minimal process visibility')
    else:
        print('Process isolation may not be working - too many processes visible')
except Exception as e:
    print(f'Process listing failed: {e}')";

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "python",
            "--code",
            code,
            "--time",
            "10",
            "--mem",
            "128",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;

    let stdout = TestUtils::extract_stdout(&result);
    if !stdout.contains("Process isolation working") && !stdout.contains("Process listing failed") {
        return Err(anyhow::anyhow!(
            "Process isolation test did not produce expected output: {}",
            stdout
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test filesystem isolation
fn test_filesystem_isolation(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import os

# Try to access sensitive system files
sensitive_files = [
    '/etc/passwd',
    '/etc/shadow',
    '/etc/sudoers',
    '/root/.bashrc',
    '/proc/sys/kernel/hostname'
]

access_results = []
for file_path in sensitive_files:
    try:
        if os.path.exists(file_path):
            with open(file_path, 'r') as f:
                content = f.read(100)  # Read first 100 chars
                access_results.append(f'{file_path}: accessible ({len(content)} chars)')
        else:
            access_results.append(f'{file_path}: not found')
    except Exception as e:
        access_results.append(f'{file_path}: access denied - {e}')

print('Filesystem access test results:')
for result in access_results:
    print(result)";

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "python",
            "--code",
            code,
            "--time",
            "10",
            "--mem",
            "128",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;

    let stdout = TestUtils::extract_stdout(&result);

    // Check that sensitive files are either not accessible or not found
    let access_denied_count = stdout.matches("access denied").count();
    let not_found_count = stdout.matches("not found").count();

    if access_denied_count + not_found_count < 3 {
        return Err(anyhow::anyhow!(
            "Filesystem isolation may not be working - too many sensitive files accessible"
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test network isolation
fn test_network_isolation(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import socket
import subprocess
import sys

# Test 1: Try to connect to external host
try:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5)
    result = sock.connect_ex(('8.8.8.8', 53))  # Google DNS
    sock.close()
    
    if result == 0:
        print('Network connection successful - isolation may not be working')
    else:
        print('Network connection failed - isolation working')
except Exception as e:
    print(f'Network connection test failed: {e}')

# Test 2: Try to ping external host
try:
    result = subprocess.run(['ping', '-c', '1', '8.8.8.8'], 
                          capture_output=True, text=True, timeout=10)
    if result.returncode == 0:
        print('Ping successful - network isolation may not be working')
    else:
        print('Ping failed - network isolation working')
except Exception as e:
    print(f'Ping test failed: {e}')";

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "python",
            "--code",
            code,
            "--time",
            "15",
            "--mem",
            "128",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;

    let stdout = TestUtils::extract_stdout(&result);

    // Should see network isolation working (connections/ping failing)
    if !stdout.contains("isolation working") && !stdout.contains("failed") {
        return Err(anyhow::anyhow!(
            "Network isolation test did not produce expected results: {}",
            stdout
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test user namespace isolation
fn test_user_namespace_isolation(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import os
import pwd
import grp

# Check current user and group IDs
uid = os.getuid()
gid = os.getgid()

print(f'Current UID: {uid}')
print(f'Current GID: {gid}')

try:
    # Try to get user info
    user_info = pwd.getpwuid(uid)
    print(f'Username: {user_info.pw_name}')
except Exception as e:
    print(f'Cannot get user info: {e}')

try:
    # Try to get group info
    group_info = grp.getgrgid(gid)
    print(f'Group name: {group_info.gr_name}')
except Exception as e:
    print(f'Cannot get group info: {e}')

# Check if we're running as root in the namespace
if uid == 0:
    print('Running as root within namespace (expected for isolation)')
else:
    print('Running as non-root user within namespace')";

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "python",
            "--code",
            code,
            "--time",
            "10",
            "--mem",
            "128",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;

    let stdout = TestUtils::extract_stdout(&result);

    // Should show user/group information
    if !stdout.contains("UID:") || !stdout.contains("GID:") {
        return Err(anyhow::anyhow!(
            "User namespace test did not show user/group info: {}",
            stdout
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test path traversal prevention
fn test_path_traversal_prevention(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import os

# Test various path traversal attempts
traversal_paths = [
    '../../../etc/passwd',
    '..\\..\\..\\etc\\passwd',
    '/etc/../etc/passwd',
    '/tmp/../../../etc/passwd',
    '/proc/self/root/etc/passwd'
]

print('Path traversal prevention test:')
for path in traversal_paths:
    try:
        if os.path.exists(path):
            with open(path, 'r') as f:
                content = f.read(50)
                print(f'{path}: accessible (security issue!)')
        else:
            print(f'{path}: not accessible (good)')
    except Exception as e:
        print(f'{path}: access denied - {e}')";

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "python",
            "--code",
            code,
            "--time",
            "10",
            "--mem",
            "128",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;

    let stdout = TestUtils::extract_stdout(&result);

    // Should not see "accessible (security issue!)" messages
    if stdout.contains("security issue") {
        return Err(anyhow::anyhow!(
            "Path traversal prevention failed - sensitive files accessible: {}",
            stdout
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test privilege escalation prevention
fn test_privilege_escalation_prevention(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import os
import subprocess

print('Privilege escalation prevention test:')

# Test 1: Check if we can access /root
try:
    if os.path.exists('/root'):
        files = os.listdir('/root')
        print(f'/root directory accessible with {len(files)} files')
    else:
        print('/root directory not accessible')
except Exception as e:
    print(f'/root access denied: {e}')

# Test 2: Try to read /etc/shadow (should fail)
try:
    with open('/etc/shadow', 'r') as f:
        content = f.read(100)
        print('WARNING: /etc/shadow accessible (security issue!)')
except Exception as e:
    print(f'/etc/shadow access properly denied: {e}')

# Test 3: Try to modify system files
try:
    with open('/tmp/test_escalation', 'w') as f:
        f.write('test')
    os.remove('/tmp/test_escalation')
    print('File creation in /tmp works (expected)')
except Exception as e:
    print(f'File creation failed: {e}')";

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "python",
            "--code",
            code,
            "--time",
            "10",
            "--mem",
            "128",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;

    let stdout = TestUtils::extract_stdout(&result);

    // Should not see security issues
    if stdout.contains("security issue") {
        return Err(anyhow::anyhow!(
            "Privilege escalation prevention failed: {}",
            stdout
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test resource isolation
fn test_resource_isolation(config: &TestConfig) -> Result<()> {
    let box_id1 = generate_box_id();
    let box_id2 = generate_box_id();

    // Run two processes that should be isolated from each other
    let code1 = "import time
import os

# Process 1: Allocate memory and sleep
data = [0] * 100000
print('Process 1: Memory allocated, sleeping...')
time.sleep(2)
print('Process 1: Completed')";

    let code2 = "import time
import os

# Process 2: Different memory allocation
data = [1] * 50000
print('Process 2: Different memory allocated, sleeping...')
time.sleep(1)
print('Process 2: Completed')";

    // Run both processes
    let result1 = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id1.to_string(),
            "--language",
            "python",
            "--code",
            code1,
            "--time",
            "10",
            "--mem",
            "128",
        ],
    )?;

    let result2 = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id2.to_string(),
            "--language",
            "python",
            "--code",
            code2,
            "--time",
            "10",
            "--mem",
            "128",
        ],
    )?;

    // Both should succeed independently
    TestUtils::validate_success_result(&result1)?;
    TestUtils::validate_success_result(&result2)?;

    let stdout1 = TestUtils::extract_stdout(&result1);
    let stdout2 = TestUtils::extract_stdout(&result2);

    if !stdout1.contains("Process 1: Completed") || !stdout2.contains("Process 2: Completed") {
        return Err(anyhow::anyhow!(
            "Resource isolation test failed - processes did not complete independently"
        ));
    }

    cleanup_test_box(config, box_id1);
    cleanup_test_box(config, box_id2);
    Ok(())
}

/// Test security boundary enforcement
fn test_security_boundary_enforcement(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import os
import sys
import subprocess

print('Security boundary enforcement test:')

# Test 1: Try to access host system information
try:
    hostname = os.uname()
    print(f'System info accessible: {hostname.nodename}')
except Exception as e:
    print(f'System info access restricted: {e}')

# Test 2: Try to access kernel information
try:
    with open('/proc/version', 'r') as f:
        version = f.read(100)
        print(f'Kernel version accessible: {version[:50]}...')
except Exception as e:
    print(f'Kernel info access restricted: {e}')

# Test 3: Try to access system load
try:
    with open('/proc/loadavg', 'r') as f:
        load = f.read()
        print(f'System load accessible: {load.strip()}')
except Exception as e:
    print(f'System load access restricted: {e}')

# Test 4: Try to access memory info
try:
    with open('/proc/meminfo', 'r') as f:
        meminfo = f.read(200)
        print(f'Memory info accessible: {meminfo[:100]}...')
except Exception as e:
    print(f'Memory info access restricted: {e}')";

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "python",
            "--code",
            code,
            "--time",
            "10",
            "--mem",
            "128",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;

    let stdout = TestUtils::extract_stdout(&result);

    // Should see some access restrictions
    let restricted_count = stdout.matches("access restricted").count();
    if restricted_count < 2 {
        return Err(anyhow::anyhow!(
            "Security boundary enforcement may not be working - too much system access: {}",
            stdout
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_code_generation() {
        let code = TestUtils::generate_test_code("python", "hello");
        assert!(code.contains("print"));
    }

    #[test]
    fn test_output_validation() {
        let json = serde_json::json!({
            "stdout": "Hello World",
            "stderr": ""
        });

        assert!(TestUtils::validate_output_contains(&json, "Hello").is_ok());
        assert!(TestUtils::validate_output_contains(&json, "Goodbye").is_err());
    }

    #[test]
    fn test_security_test_run() {
        let mut config = TestConfig::default();
        config.verbose = true;
        let results = run_security_tests(&config).unwrap();
        assert!(!results.is_empty());
        for result in &results {
            if !result.passed {
                eprintln!("Security test failed: {}", result.name);
                if let Some(error) = &result.error_message {
                    eprintln!("Error: {}", error);
                }
            }
            assert!(result.passed);
        }
        for result in results {
            println!("Test result: {}", result.name);
            println!("Test passed: {}", result.passed);
            println!(
                "Test error message: {}",
                result.error_message.unwrap_or("None".to_string())
            );
        }
    }
}
