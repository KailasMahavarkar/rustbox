//! Stress tests for rustbox
//!
//! Tests system behavior under load, concurrent execution,
//! and resource contention scenarios.

use crate::tests::common::{
    cleanup_test_box, execute_rustbox_command, generate_box_id, run_test, TestConfig, TestResult,
};
use crate::tests::utils::TestUtils;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Run all stress tests
pub fn run_stress_tests(config: &TestConfig) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    eprintln!("Running stress tests...");
    // Test 1: Sequential execution stress
    eprintln!("Running sequential execution stress test...");
    results.push(run_test(config, "Sequential execution stress", || {
        test_sequential_execution_stress(config)
    }));

    // Test 2: Concurrent execution stress
    eprintln!("Running concurrent execution stress test...");
    results.push(run_test(config, "Concurrent execution stress", || {
        test_concurrent_execution_stress(config)
    }));

    // Test 3: Memory pressure stress
    eprintln!("Running memory pressure stress test...");
    results.push(run_test(config, "Memory pressure stress", || {
        test_memory_pressure_stress(config)
    }));

    // Test 4: CPU intensive stress
    eprintln!("Running CPU intensive stress test...");
    results.push(run_test(config, "CPU intensive stress", || {
        test_cpu_intensive_stress(config)
    }));

    // Test 5: Resource contention stress
    eprintln!("Running resource contention stress test...");
    results.push(run_test(config, "Resource contention stress", || {
        test_resource_contention_stress(config)
    }));

    // Test 6: Rapid box creation/destruction
    eprintln!("Running rapid box creation/destruction test...");
    results.push(run_test(config, "Rapid box creation/destruction", || {
        test_rapid_box_cycle(config)
    }));

    // Test 7: Long running process stress
    eprintln!("Running long running process stress test...");
    results.push(run_test(config, "Long running process stress", || {
        test_long_running_process_stress(config)
    }));

    // Test 8: System resource exhaustion
    eprintln!("Running system resource exhaustion test...");
    results.push(run_test(config, "System resource exhaustion", || {
        test_system_resource_exhaustion(config)
    }));

    Ok(results)
}

/// Test sequential execution under stress
fn test_sequential_execution_stress(config: &TestConfig) -> Result<()> {
    let num_executions = 10;
    let mut successful_executions = 0;
    let mut _failed_executions = 0;

    for i in 0..num_executions {
        let box_id = generate_box_id();
        let code = format!(
            "import time
import random

# Simulate some work
work_time = random.uniform(0.1, 0.5)
time.sleep(work_time)

# Allocate some memory
data = [0] * random.randint(1000, 10000)

print(f'Sequential stress test execution {{}} completed', {})",
            i
        );

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
        );

        match result {
            Ok(json) => {
                if TestUtils::validate_success_result(&json).is_ok() {
                    successful_executions += 1;
                } else {
                    _failed_executions += 1;
                }
            }
            Err(_) => {
                _failed_executions += 1;
            }
        }

        cleanup_test_box(config, box_id);
    }

    let success_rate = (successful_executions * 100) / num_executions;
    if success_rate < 80 {
        return Err(anyhow::anyhow!(
            "Sequential stress test success rate too low: {}% ({}/{} passed)",
            success_rate,
            successful_executions,
            num_executions
        ));
    }

    Ok(())
}

/// Test concurrent execution under stress
fn test_concurrent_execution_stress(config: &TestConfig) -> Result<()> {
    let num_threads = 5;
    let executions_per_thread = 3;
    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = Vec::new();

    for thread_id in 0..num_threads {
        let config = config.clone();
        let results = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let mut thread_results = Vec::new();

            for i in 0..executions_per_thread {
                let box_id = generate_box_id();
                let code = format!(
                    "import time
import random

# Simulate concurrent work
work_time = random.uniform(0.2, 1.0)
time.sleep(work_time)

# Allocate memory
data = [0] * random.randint(5000, 15000)

print(f'Concurrent stress test thread {{}} execution {{}} completed', {}, {})",
                    thread_id, i
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
                        "15",
                        "--mem",
                        "128",
                    ],
                );

                let success = match result {
                    Ok(json) => TestUtils::validate_success_result(&json).is_ok(),
                    Err(_) => false,
                };

                thread_results.push(success);
                cleanup_test_box(&config, box_id);
            }

            let mut results_guard = results.lock().unwrap();
            results_guard.extend(thread_results);
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let results_guard = results.lock().unwrap();
    let total_executions = results_guard.len();
    let successful_executions = results_guard.iter().filter(|&&x| x).count();
    let success_rate = (successful_executions * 100) / total_executions;

    if success_rate < 70 {
        return Err(anyhow::anyhow!(
            "Concurrent stress test success rate too low: {}% ({}/{} passed)",
            success_rate,
            successful_executions,
            total_executions
        ));
    }

    Ok(())
}

/// Test memory pressure stress
fn test_memory_pressure_stress(config: &TestConfig) -> Result<()> {
    let num_executions = 5;
    let mut successful_executions = 0;

    for i in 0..num_executions {
        let box_id = generate_box_id();
        let code = format!(
            "import time
import random

# Create memory pressure
memory_chunks = []
chunk_size = random.randint(10000, 50000)

for j in range(10):
    chunk = [0] * chunk_size
    memory_chunks.append(chunk)
    time.sleep(0.1)  # Small delay between allocations

print(f'Memory pressure test {{}} completed with {{}} chunks', {}, len(memory_chunks))",
            i
        );

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
                "15",
                "--mem",
                "64", // Low memory limit to create pressure
            ],
        );

        match result {
            Ok(json) => {
                // Accept either success or memory limit (both are valid outcomes)
                if TestUtils::validate_success_result(&json).is_ok()
                    || TestUtils::validate_memory_limit_result(&json).is_ok()
                {
                    successful_executions += 1;
                }
            }
            Err(_) => {
                // Some failures are expected under memory pressure
            }
        }

        cleanup_test_box(config, box_id);
    }

    // Under memory pressure, we expect some failures, but not all
    if successful_executions == 0 {
        return Err(anyhow::anyhow!(
            "Memory pressure stress test - all executions failed"
        ));
    }

    Ok(())
}

/// Test CPU intensive stress
fn test_cpu_intensive_stress(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import time
import math

# CPU intensive computation
start_time = time.time()
result = 0

for i in range(1000000):
    result += math.sqrt(i) * math.sin(i)

end_time = time.time()
computation_time = end_time - start_time

print(f'CPU intensive computation completed in {{:.2f}} seconds', computation_time)
print(f'Result: {{}}', result)";

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
            "10", // CPU time limit
            "--mem",
            "128",
        ],
    )?;

    // Should either complete successfully or hit time limit
    TestUtils::validate_execution_result(&result)?;

    let status = result.get("status").and_then(|s| s.as_str()).unwrap_or("");

    if status != "Success" && status != "TimeLimit" {
        return Err(anyhow::anyhow!(
            "CPU intensive stress test unexpected status: {}",
            status
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test resource contention stress
fn test_resource_contention_stress(config: &TestConfig) -> Result<()> {
    let num_boxes = 8;
    let mut handles = Vec::new();
    let results = Arc::new(Mutex::new(Vec::new()));

    for i in 0..num_boxes {
        let config = config.clone();
        let results = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let box_id = generate_box_id();
            let code = format!(
                "import time
import random

# Resource contention simulation
for j in range(5):
    # CPU work
    start = time.time()
    while time.time() - start < 0.1:
        _ = sum(range(1000))
    
    # Memory allocation
    data = [0] * random.randint(1000, 5000)
    
    # I/O simulation
    time.sleep(0.05)

print(f'Resource contention test {{}} completed', {})",
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
                    "64", // Low memory to create contention
                ],
            );

            let success = match result {
                Ok(json) => TestUtils::validate_success_result(&json).is_ok(),
                Err(_) => false,
            };

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

    let results_guard = results.lock().unwrap();
    let total_executions = results_guard.len();
    let successful_executions = results_guard.iter().filter(|&&x| x).count();
    let success_rate = (successful_executions * 100) / total_executions;

    // Under resource contention, we expect some failures
    if success_rate < 50 {
        return Err(anyhow::anyhow!(
            "Resource contention stress test success rate too low: {}% ({}/{} passed)",
            success_rate,
            successful_executions,
            total_executions
        ));
    }

    Ok(())
}

/// Test rapid box creation and destruction
fn test_rapid_box_cycle(config: &TestConfig) -> Result<()> {
    let num_cycles = 20;
    let mut successful_cycles = 0;

    for i in 0..num_cycles {
        let box_id = generate_box_id();

        // Quick execution
        let code = format!("print('Rapid cycle test {{}}', {})", i);

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

        let success = match result {
            Ok(json) => TestUtils::validate_success_result(&json).is_ok(),
            Err(_) => false,
        };

        if success {
            successful_cycles += 1;
        }

        cleanup_test_box(config, box_id);

        // Small delay between cycles
        thread::sleep(Duration::from_millis(50));
    }

    let success_rate = (successful_cycles * 100) / num_cycles;
    if success_rate < 80 {
        return Err(anyhow::anyhow!(
            "Rapid box cycle test success rate too low: {}% ({}/{} passed)",
            success_rate,
            successful_cycles,
            num_cycles
        ));
    }

    Ok(())
}

/// Test long running process stress
fn test_long_running_process_stress(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import time
import random

# Long running process with periodic work
start_time = time.time()
iteration = 0

while time.time() - start_time < 8:  # Run for 8 seconds
    # Do some work
    data = [0] * random.randint(1000, 3000)
    
    # Sleep briefly
    time.sleep(0.5)
    
    iteration += 1
    if iteration % 5 == 0:
        print(f'Long running process iteration {{}}', iteration)

print(f'Long running process completed after {{}} iterations', iteration)";

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
            "15", // Allow enough time
            "--mem",
            "128",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;

    let stdout = TestUtils::extract_stdout(&result);
    if !stdout.contains("Long running process completed") {
        return Err(anyhow::anyhow!(
            "Long running process did not complete properly: {}",
            stdout
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test system resource exhaustion
fn test_system_resource_exhaustion(config: &TestConfig) -> Result<()> {
    // This test is more conservative to avoid actually exhausting system resources
    let num_boxes = 10;
    let mut handles = Vec::new();
    let results = Arc::new(Mutex::new(Vec::new()));

    for i in 0..num_boxes {
        let config = config.clone();
        let results = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let box_id = generate_box_id();
            let code = format!(
                "import time
import random

# Moderate resource usage
for j in range(3):
    # Memory allocation
    data = [0] * random.randint(5000, 10000)
    
    # CPU work
    start = time.time()
    while time.time() - start < 0.2:
        _ = sum(range(1000))
    
    time.sleep(0.1)

print(f'Resource exhaustion test {{}} completed', {})",
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
                    "32", // Very low memory limit
                ],
            );

            let success = match result {
                Ok(json) => {
                    // Accept success or memory limit as valid outcomes
                    TestUtils::validate_success_result(&json).is_ok()
                        || TestUtils::validate_memory_limit_result(&json).is_ok()
                }
                Err(_) => false,
            };

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

    let results_guard = results.lock().unwrap();
    let successful_executions = results_guard.iter().filter(|&&x| x).count();

    // Under resource exhaustion conditions, we expect some failures
    if successful_executions == 0 {
        return Err(anyhow::anyhow!(
            "System resource exhaustion test - all executions failed"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_code_generation() {
        let code = TestUtils::generate_test_code("python", "memory");
        assert!(code.contains("data"));
    }

    #[test]
    fn test_concurrent_execution_setup() {
        // Test that we can set up concurrent execution structures
        let results = Arc::new(Mutex::new(Vec::new()));
        let results_clone = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let mut guard = results_clone.lock().unwrap();
            guard.push(true);
        });

        handle.join().unwrap();

        let guard = results.lock().unwrap();
        assert_eq!(guard.len(), 1);
        assert!(guard[0]);
    }

    #[test]
    fn test_stress_test_run() {
        let config = TestConfig::default();
        let results = run_stress_tests(&config).unwrap();
        assert!(!results.is_empty());
        for result in results {
            eprintln!("Test result: {}", result.name);
            eprintln!("Test passed: {}", result.passed);
            eprintln!("Test error message: {}", result.error_message.unwrap_or("None".to_string()));
            assert!(result.passed);
        }
    }
}
