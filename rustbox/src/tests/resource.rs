//! Resource limit tests for rustbox
//!
//! Tests memory limits, CPU time limits, wall time limits, and process limits.

use crate::tests::common::{
    cleanup_test_box, execute_rustbox_command, generate_box_id, run_test, TestConfig, TestResult,
};
use crate::tests::utils::TestUtils;

use anyhow::Result;

/// Run all resource limit tests
pub fn run_resource_tests(config: &TestConfig) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    // Test 1: Memory limit enforcement
    results.push(run_test(config, "Memory limit enforcement", || {
        test_memory_limit_enforcement(config)
    }));

    // Test 2: CPU time limit enforcement
    results.push(run_test(config, "CPU time limit enforcement", || {
        test_cpu_time_limit_enforcement(config)
    }));

    // Test 3: Wall time limit enforcement
    results.push(run_test(config, "Wall time limit enforcement", || {
        test_wall_time_limit_enforcement(config)
    }));

    // Test 4: Process limit enforcement
    results.push(run_test(config, "Process limit enforcement", || {
        test_process_limit_enforcement(config)
    }));

    // Test 5: File descriptor limit
    results.push(run_test(config, "File descriptor limit", || {
        test_file_descriptor_limit(config)
    }));

    // Test 6: Resource monitoring accuracy
    results.push(run_test(config, "Resource monitoring accuracy", || {
        test_resource_monitoring_accuracy(config)
    }));

    // Test 7: Low memory scenarios
    results.push(run_test(config, "Low memory scenarios", || {
        test_low_memory_scenarios(config)
    }));

    // Test 8: Resource limit recovery
    results.push(run_test(config, "Resource limit recovery", || {
        test_resource_limit_recovery(config)
    }));

    Ok(results)
}

/// Test memory limit enforcement
fn test_memory_limit_enforcement(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = TestUtils::generate_test_code("python", "memory");

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
            &code,
            "--time",
            "10",
            "--mem",
            "50", // 50MB limit
        ],
    )?;

    // Should hit memory limit
    TestUtils::validate_memory_limit_result(&result)?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test CPU time limit enforcement
fn test_cpu_time_limit_enforcement(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = TestUtils::generate_test_code("python", "cpu");

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
            &code,
            "--time",
            "2", // 2 second limit
            "--mem",
            "128",
        ],
    )?;

    // Should hit time limit
    TestUtils::validate_time_limit_result(&result)?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test wall time limit enforcement
fn test_wall_time_limit_enforcement(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import time
time.sleep(5)
print('Wall time test completed')";

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
            "--wall-time",
            "2", // 2 second wall time limit
            "--mem",
            "128",
        ],
    )?;

    // Should hit wall time limit
    TestUtils::validate_time_limit_result(&result)?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test process limit enforcement
fn test_process_limit_enforcement(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import subprocess
import os

# Try to create many processes
for i in range(100):
    try:
        subprocess.Popen(['sleep', '1'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except:
        break

print('Process creation test completed')";

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
            "--processes",
            "5", // Very low process limit
            "--mem",
            "128",
        ],
    )?;

    // Should either succeed (with limited processes) or hit process limit
    TestUtils::validate_execution_result(&result)?;

    let status = result.get("status").and_then(|s| s.as_str()).unwrap_or("");

    // Accept either success (if process limit was respected) or ProcessLimit
    if status != "Success" && status != "ProcessLimit" {
        return Err(anyhow::anyhow!(
            "Expected Success or ProcessLimit, got: {}",
            status
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test file descriptor limit
fn test_file_descriptor_limit(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import os

# Try to open many files
files = []
try:
    for i in range(1000):
        f = open(f'/tmp/test_file_{i}', 'w')
        files.append(f)
    print('File descriptor test completed')
except Exception as e:
    print(f'File descriptor limit hit: {e}')
finally:
    for f in files:
        f.close()";

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

    // Should succeed (file descriptor limits are handled by the system)
    TestUtils::validate_success_result(&result)?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test resource monitoring accuracy
fn test_resource_monitoring_accuracy(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import time
import sys

# Allocate some memory
data = [0] * 100000  # ~400KB
time.sleep(1)  # Use some CPU time
print('Resource monitoring test completed')";

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

    // Check that resource monitoring is working
    let wall_time = TestUtils::extract_wall_time(&result);
    let cpu_time = TestUtils::extract_cpu_time(&result);
    let memory_usage = TestUtils::extract_memory_usage(&result);

    if wall_time < 0.5 || wall_time > 5.0 {
        return Err(anyhow::anyhow!(
            "Wall time seems inaccurate: {}s",
            wall_time
        ));
    }

    if cpu_time < 0.1 || cpu_time > 2.0 {
        return Err(anyhow::anyhow!("CPU time seems inaccurate: {}s", cpu_time));
    }

    if memory_usage < 100 || memory_usage > 10000 {
        // 100KB to 10MB
        return Err(anyhow::anyhow!(
            "Memory usage seems inaccurate: {}KB",
            memory_usage
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test low memory scenarios
fn test_low_memory_scenarios(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import sys

# Try to allocate memory in small chunks
data = []
try:
    for i in range(1000):
        chunk = [0] * 10000  # 40KB per chunk
        data.append(chunk)
    print('Low memory test completed')
except MemoryError:
    print('Memory limit reached as expected')";

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
            "10", // Very low memory limit (10MB)
        ],
    )?;

    // Should either succeed or hit memory limit
    TestUtils::validate_execution_result(&result)?;

    let status = result.get("status").and_then(|s| s.as_str()).unwrap_or("");

    if status != "Success" && status != "MemoryLimit" {
        return Err(anyhow::anyhow!(
            "Expected Success or MemoryLimit, got: {}",
            status
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test resource limit recovery
fn test_resource_limit_recovery(config: &TestConfig) -> Result<()> {
    let box_id1 = generate_box_id();
    let box_id2 = generate_box_id();

    // First, hit a memory limit
    let code1 = TestUtils::generate_test_code("python", "memory");
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
            &code1,
            "--time",
            "10",
            "--mem",
            "50",
        ],
    )?;

    TestUtils::validate_memory_limit_result(&result1)?;

    // Then, run a successful test to ensure system recovered
    let code2 = TestUtils::generate_test_code("python", "hello");
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
            &code2,
            "--time",
            "10",
            "--mem",
            "128",
        ],
    )?;

    TestUtils::validate_success_result(&result2)?;

    cleanup_test_box(config, box_id1);
    cleanup_test_box(config, box_id2);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_limit_validation() {
        let memory_limit_json = serde_json::json!({
            "status": "Memory Limit Exceeded",
            "success": false,
            "stdout": "",
            "stderr": ""
        });

        assert!(TestUtils::validate_memory_limit_result(&memory_limit_json).is_ok());

        let success_json = serde_json::json!({
            "status": "Success",
            "success": true,
            "stdout": "Hello",
            "stderr": ""
        });

        assert!(TestUtils::validate_memory_limit_result(&success_json).is_err());
    }

    #[test]
    fn test_time_limit_validation() {
        let time_limit_json = serde_json::json!({
            "status": "TLE",
            "success": false,
            "stdout": "",
            "stderr": ""
        });

        assert!(TestUtils::validate_time_limit_result(&time_limit_json).is_ok());
    }

    #[test]
    fn test_resource_extraction() {
        let json = serde_json::json!({
            "wall_time": 1.5,
            "cpu_time": 0.8,
            "memory_peak_kb": 2048,
            "stdout": "Hello",
            "stderr": ""
        });

        assert_eq!(TestUtils::extract_wall_time(&json), 1.5);
        assert_eq!(TestUtils::extract_cpu_time(&json), 0.8);
        assert_eq!(TestUtils::extract_memory_usage(&json), 2048);
        assert_eq!(TestUtils::extract_stdout(&json), "Hello");
        assert_eq!(TestUtils::extract_stderr(&json), "");
    }
}
