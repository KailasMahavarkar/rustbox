//! Performance tests for rustbox
//!
//! Tests startup time, execution overhead, memory usage,
//! and throughput characteristics.

use crate::tests::common::{
    cleanup_test_box, execute_rustbox_command, generate_box_id, run_test, TestConfig, TestResult,
};
use crate::tests::utils::TestUtils;

use anyhow::Result;
use std::time::{Duration, Instant};

/// Run all performance tests
pub fn run_performance_tests(config: &TestConfig) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    // Test 1: Startup time performance
    results.push(run_test(config, "Startup time performance", || {
        test_startup_time_performance(config)
    }));

    // Test 2: Execution overhead performance
    results.push(run_test(config, "Execution overhead performance", || {
        test_execution_overhead_performance(config)
    }));

    // Test 3: Memory usage performance
    results.push(run_test(config, "Memory usage performance", || {
        test_memory_usage_performance(config)
    }));

    // Test 4: Throughput performance
    results.push(run_test(config, "Throughput performance", || {
        test_throughput_performance(config)
    }));

    // Test 5: Resource monitoring performance
    results.push(run_test(config, "Resource monitoring performance", || {
        test_resource_monitoring_performance(config)
    }));

    // Test 6: Concurrent execution performance
    results.push(run_test(config, "Concurrent execution performance", || {
        test_concurrent_execution_performance(config)
    }));

    // Test 7: Large code execution performance
    results.push(run_test(config, "Large code execution performance", || {
        test_large_code_execution_performance(config)
    }));

    // Test 8: System resource utilization
    results.push(run_test(config, "System resource utilization", || {
        test_system_resource_utilization(config)
    }));

    Ok(results)
}

/// Test startup time performance
fn test_startup_time_performance(config: &TestConfig) -> Result<()> {
    let num_iterations = 10;
    let mut startup_times = Vec::new();

    for _ in 0..num_iterations {
        let box_id = generate_box_id();
        let start = Instant::now();

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
                "print('Startup time test')",
                "--time",
                "5",
                "--mem",
                "64",
            ],
        );

        let startup_time = start.elapsed();

        if result.is_ok() {
            startup_times.push(startup_time);
        }

        cleanup_test_box(config, box_id);
    }

    if startup_times.is_empty() {
        return Err(anyhow::anyhow!("No successful startup time measurements"));
    }

    let avg_startup_time = startup_times.iter().sum::<Duration>() / startup_times.len() as u32;
    let max_startup_time = startup_times.iter().max().unwrap();

    // Performance targets: average < 0.5s, max < 1.0s
    if avg_startup_time > Duration::from_millis(500) {
        return Err(anyhow::anyhow!(
            "Average startup time too slow: {:?} (target: <500ms)",
            avg_startup_time
        ));
    }

    if *max_startup_time > Duration::from_millis(1000) {
        return Err(anyhow::anyhow!(
            "Maximum startup time too slow: {:?} (target: <1000ms)",
            max_startup_time
        ));
    }

    println!(
        "✅ Startup time performance: avg={:?}, max={:?}",
        avg_startup_time, max_startup_time
    );
    Ok(())
}

/// Test execution overhead performance
fn test_execution_overhead_performance(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();

    // Simple command that should execute quickly
    let code = "print('Execution overhead test')";

    let start = Instant::now();
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
            "5",
            "--mem",
            "64",
        ],
    )?;
    let total_time = start.elapsed();

    TestUtils::validate_success_result(&result)?;

    let wall_time = TestUtils::extract_wall_time(&result);
    let overhead = total_time.as_secs_f64() - wall_time;

    // Performance target: overhead < 0.2s
    if overhead > 0.2 {
        return Err(anyhow::anyhow!(
            "Execution overhead too high: {:.3}s (target: <0.2s)",
            overhead
        ));
    }

    println!("✅ Execution overhead: {:.3}s (target: <0.2s)", overhead);
    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test memory usage performance
fn test_memory_usage_performance(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();

    // Code that uses minimal memory
    let code = "print('Memory usage performance test')";

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
            "5",
            "--mem",
            "64",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;

    let memory_usage = TestUtils::extract_memory_usage(&result);
    let memory_usage_mb = memory_usage / 1024;

    // Performance target: base memory usage < 10MB
    if memory_usage_mb > 10 {
        return Err(anyhow::anyhow!(
            "Base memory usage too high: {}MB (target: <10MB)",
            memory_usage_mb
        ));
    }

    println!(
        "✅ Base memory usage: {}MB (target: <10MB)",
        memory_usage_mb
    );
    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test throughput performance
fn test_throughput_performance(config: &TestConfig) -> Result<()> {
    let num_executions = 20;
    let start = Instant::now();
    let mut successful_executions = 0;

    for i in 0..num_executions {
        let box_id = generate_box_id();
        let code = format!("print('Throughput test execution {}')", i);

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
                "5",
                "--mem",
                "64",
            ],
        );

        if result.is_ok() {
            successful_executions += 1;
        }

        cleanup_test_box(config, box_id);
    }

    let total_time = start.elapsed();
    let throughput = successful_executions as f64 / total_time.as_secs_f64();

    // Performance target: >2 operations/second
    if throughput < 2.0 {
        return Err(anyhow::anyhow!(
            "Throughput too low: {:.2} ops/sec (target: >2 ops/sec)",
            throughput
        ));
    }

    println!(
        "✅ Throughput: {:.2} ops/sec (target: >2 ops/sec)",
        throughput
    );
    Ok(())
}

/// Test resource monitoring performance
fn test_resource_monitoring_performance(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();

    // Code that uses measurable resources
    let code = "import time
import random

# Use some CPU time
start = time.time()
while time.time() - start < 0.5:
    _ = sum(range(1000))

# Use some memory
data = [0] * 10000

# Use some wall time
time.sleep(0.2)

print('Resource monitoring performance test completed')";

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

    let wall_time = TestUtils::extract_wall_time(&result);
    let cpu_time = TestUtils::extract_cpu_time(&result);
    let memory_usage = TestUtils::extract_memory_usage(&result);

    // Validate that resource monitoring is working and accurate
    if wall_time < 0.1 || wall_time > 2.0 {
        return Err(anyhow::anyhow!(
            "Wall time monitoring inaccurate: {:.3}s (expected ~0.7s)",
            wall_time
        ));
    }

    if cpu_time < 0.1 || cpu_time > 1.0 {
        return Err(anyhow::anyhow!(
            "CPU time monitoring inaccurate: {:.3}s (expected ~0.5s)",
            cpu_time
        ));
    }

    if memory_usage < 100 || memory_usage > 10000 {
        return Err(anyhow::anyhow!(
            "Memory usage monitoring inaccurate: {}KB (expected ~400KB)",
            memory_usage
        ));
    }

    println!(
        "✅ Resource monitoring: wall={:.3}s, cpu={:.3}s, mem={}KB",
        wall_time, cpu_time, memory_usage
    );

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test concurrent execution performance
fn test_concurrent_execution_performance(config: &TestConfig) -> Result<()> {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let num_concurrent = 5;
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    let start = Instant::now();

    for i in 0..num_concurrent {
        let config = config.clone();
        let results = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let box_id = generate_box_id();
            let code = format!(
                "import time
time.sleep(0.5)  # Simulate work
print('Concurrent performance test {}')",
                i
            );

            let result = execute_rustbox_command(
                &config,
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
                    "64",
                ],
            );

            let success = result.is_ok();
            let mut results_guard = results.lock().unwrap();
            results_guard.push(success);

            cleanup_test_box(&config, box_id);
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    let total_time = start.elapsed();
    let results_guard = results.lock().unwrap();
    let successful_executions = results_guard.iter().filter(|&&x| x).count();
    let concurrent_throughput = successful_executions as f64 / total_time.as_secs_f64();

    // Performance target: concurrent throughput > 1 ops/sec
    if concurrent_throughput < 1.0 {
        return Err(anyhow::anyhow!(
            "Concurrent throughput too low: {:.2} ops/sec (target: >1 ops/sec)",
            concurrent_throughput
        ));
    }

    println!(
        "✅ Concurrent throughput: {:.2} ops/sec (target: >1 ops/sec)",
        concurrent_throughput
    );
    Ok(())
}

/// Test large code execution performance
fn test_large_code_execution_performance(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();

    // Generate large code
    let mut code = String::new();
    code.push_str("print('Large code execution performance test starting')\n");

    // Add many lines of code
    for i in 0..1000 {
        code.push_str(&format!("x{} = {}\n", i, i));
    }

    code.push_str("print('Large code execution performance test completed')\n");

    let start = Instant::now();
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
            "128",
        ],
    )?;
    let execution_time = start.elapsed();

    TestUtils::validate_success_result(&result)?;

    // Performance target: large code execution < 2s
    if execution_time > Duration::from_secs(2) {
        return Err(anyhow::anyhow!(
            "Large code execution too slow: {:?} (target: <2s)",
            execution_time
        ));
    }

    println!(
        "✅ Large code execution: {:?} (target: <2s)",
        execution_time
    );
    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test system resource utilization
fn test_system_resource_utilization(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();

    // Code that should use minimal system resources
    let code = "import os
import sys

print('System resource utilization test')

# Check system resource usage
try:
    # Get process memory info
    with open('/proc/self/status', 'r') as f:
        status = f.read()
        for line in status.splitlines():
            if line.startswith('VmRSS:'):
                print(f'Memory usage: {{line}}')
                break
except Exception as e:
    print(f'Memory info not available: {{e}}')

# Check file descriptor usage
try:
    fd_count = len(os.listdir('/proc/self/fd'))
    print(f'File descriptors: {{fd_count}}')
except Exception as e:
    print(f'FD count not available: {{e}}')

print('System resource utilization test completed')";

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
    TestUtils::validate_output_contains(&result, "System resource utilization test completed")?;

    let stdout = TestUtils::extract_stdout(&result);

    // Check that system resource information is available
    if !stdout.contains("Memory usage:") && !stdout.contains("Memory info not available") {
        return Err(anyhow::anyhow!(
            "System resource utilization test did not show memory information"
        ));
    }

    println!("✅ System resource utilization test completed successfully");
    cleanup_test_box(config, box_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_measurement_setup() {
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(100));
    }

    #[test]
    fn test_large_code_generation() {
        let mut code = String::new();
        for i in 0..1001 {
            code.push_str(&format!("x{} = {}\n", i, i));
        }

        assert!(code.len() > 1000);
        assert!(code.contains("x0 = 0"));
        assert!(code.contains("x99 = 99"));
    }

    #[test]
    fn test_throughput_calculation() {
        let successful_executions = 10;
        let total_time = Duration::from_secs(5);
        let throughput = successful_executions as f64 / total_time.as_secs_f64();

        assert_eq!(throughput, 2.0);
    }

    #[test]
    fn test_performance_test_run() {
        let config = TestConfig::default();
        let results = run_performance_tests(&config).unwrap();
        assert!(!results.is_empty());
        for result in results {
            assert!(result.passed);
        }
    }
}
