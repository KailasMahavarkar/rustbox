//! Utility functions and helpers for rustbox tests

use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;
use std::time::Duration;

/// Test utilities for common operations
pub struct TestUtils;

impl TestUtils {
    /// Check if running as root (required for most tests)
    pub fn check_root_privileges() -> Result<()> {
        let output = Command::new("id")
            .arg("-u")
            .output()
            .context("Failed to check user ID")?;

        let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if uid != "0" {
            return Err(anyhow::anyhow!(
                "Tests require root privileges (current UID: {}). Run with sudo.",
                uid
            ));
        }

        Ok(())
    }

    /// Check if cgroups are available on the system
    pub fn check_cgroups_available() -> Result<()> {
        let cgroups_path = std::path::Path::new("/proc/cgroups");
        if !cgroups_path.exists() {
            return Err(anyhow::anyhow!(
                "Cgroups not available - /proc/cgroups not found"
            ));
        }

        let cgroup_fs_path = std::path::Path::new("/sys/fs/cgroup");
        if !cgroup_fs_path.exists() {
            return Err(anyhow::anyhow!(
                "Cgroups not available - /sys/fs/cgroup not found"
            ));
        }

        Ok(())
    }

    /// Check if namespaces are supported
    pub fn check_namespaces_supported() -> Result<()> {
        let ns_path = std::path::Path::new("/proc/self/ns");
        if !ns_path.exists() {
            return Err(anyhow::anyhow!(
                "Namespaces not supported - /proc/self/ns not found"
            ));
        }
        Ok(())
    }

    /// Validate JSON execution result
    pub fn validate_execution_result(json: &Value) -> Result<()> {
        // Check required fields
        if !json.get("status").is_some() {
            return Err(anyhow::anyhow!("Missing 'status' field in result"));
        }

        if !json.get("success").is_some() {
            return Err(anyhow::anyhow!("Missing 'success' field in result"));
        }

        // Validate status field
        let status = json
            .get("status")
            .and_then(|s| s.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid 'status' field"))?;

        let valid_statuses = [
            "Success",
            "TimeLimit",
            "MemoryLimit",
            "TLE",  // Time Limit Exceeded (actual rustbox status)
            "Memory Limit Exceeded",  // Memory Limit Exceeded (actual rustbox status)
            "RuntimeError",
            "InternalError",
            "Signaled",
            "SecurityViolation",
            "ProcessLimit",
            "FileSizeLimit",
            "StackLimit",
            "CoreLimit",
            "DiskQuotaExceeded",
        ];

        if !valid_statuses.contains(&status) {
            return Err(anyhow::anyhow!("Invalid status: {}", status));
        }

        // Validate success field
        let success = json
            .get("success")
            .and_then(|s| s.as_bool())
            .ok_or_else(|| anyhow::anyhow!("Invalid 'success' field"))?;

        // Validate that success matches status
        let expected_success = status == "Success";
        if success != expected_success {
            return Err(anyhow::anyhow!(
                "Status '{}' should have success={}, but got success={}",
                status,
                expected_success,
                success
            ));
        }

        Ok(())
    }

    /// Validate that execution was successful
    pub fn validate_success_result(json: &Value) -> Result<()> {
        Self::validate_execution_result(json)?;

        let status = json.get("status").and_then(|s| s.as_str()).unwrap_or("");

        if status != "Success" {
            return Err(anyhow::anyhow!("Expected success, got status: {}", status));
        }

        let success = json
            .get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        if !success {
            return Err(anyhow::anyhow!("Expected success=true, got success=false"));
        }

        Ok(())
    }

    /// Validate that execution hit a specific limit
    pub fn validate_limit_result(json: &Value, expected_status: &str) -> Result<()> {
        Self::validate_execution_result(json)?;

        let status = json.get("status").and_then(|s| s.as_str()).unwrap_or("");

        if status != expected_status {
            return Err(anyhow::anyhow!(
                "Expected status '{}', got '{}'",
                expected_status,
                status
            ));
        }

        let success = json
            .get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(true);

        if success {
            return Err(anyhow::anyhow!(
                "Expected success=false for limit violation, got success=true"
            ));
        }

        Ok(())
    }

    /// Validate memory limit result
    pub fn validate_memory_limit_result(json: &Value) -> Result<()> {
        Self::validate_limit_result(json, "Memory Limit Exceeded")
    }

    /// Validate time limit result
    pub fn validate_time_limit_result(json: &Value) -> Result<()> {
        Self::validate_limit_result(json, "TLE")
    }

    /// Extract stdout from JSON result
    pub fn extract_stdout(json: &Value) -> String {
        json.get("stdout")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string()
    }

    /// Extract stderr from JSON result
    pub fn extract_stderr(json: &Value) -> String {
        json.get("stderr")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string()
    }

    /// Extract execution time from JSON result
    pub fn extract_wall_time(json: &Value) -> f64 {
        json.get("wall_time")
            .and_then(|t| t.as_f64())
            .unwrap_or(0.0)
    }

    /// Extract CPU time from JSON result
    pub fn extract_cpu_time(json: &Value) -> f64 {
        json.get("cpu_time").and_then(|t| t.as_f64()).unwrap_or(0.0)
    }

    /// Extract memory usage from JSON result
    pub fn extract_memory_usage(json: &Value) -> u64 {
        json.get("memory_peak_kb")
            .and_then(|m| m.as_u64())
            .unwrap_or(0)
    }

    /// Wait for a condition to be true with timeout
    pub fn wait_for_condition<F>(
        condition: F,
        timeout: Duration,
        check_interval: Duration,
    ) -> Result<()>
    where
        F: Fn() -> bool,
    {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if condition() {
                return Ok(());
            }
            std::thread::sleep(check_interval);
        }

        Err(anyhow::anyhow!(
            "Condition not met within timeout: {:?}",
            timeout
        ))
    }

    /// Clean up any remaining test boxes
    pub fn cleanup_all_test_boxes() {
        // Clean up boxes 1000-9999 (test range)
        for box_id in 1000..10000 {
            let _ = Command::new("sudo")
                .arg("target/release/rustbox")
                .arg("cleanup")
                .arg("--box-id")
                .arg(&box_id.to_string())
                .output();
        }
    }

    /// Check if a process is running
    pub fn is_process_running(pid: u32) -> bool {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }

    /// Get system memory info
    pub fn get_system_memory_info() -> Result<(u64, u64)> {
        let meminfo =
            std::fs::read_to_string("/proc/meminfo").context("Failed to read /proc/meminfo")?;

        let mut total_kb = 0u64;
        let mut available_kb = 0u64;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    total_kb = value.parse().unwrap_or(0);
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    available_kb = value.parse().unwrap_or(0);
                }
            }
        }

        Ok((total_kb, available_kb))
    }

    /// Check if system has enough memory for test
    pub fn check_sufficient_memory(required_mb: u64) -> Result<()> {
        let (_total_kb, available_kb) = Self::get_system_memory_info()?;
        let available_mb = available_kb / 1024;

        if available_mb < required_mb {
            return Err(anyhow::anyhow!(
                "Insufficient memory: need {}MB, have {}MB available",
                required_mb,
                available_mb
            ));
        }

        Ok(())
    }

    /// Generate test code for different languages
    pub fn generate_test_code(language: &str, test_type: &str) -> String {
        match (language, test_type) {
            ("python", "hello") => "print('Hello from Python')".to_string(),
            ("python", "memory") => "data = []
for i in range(1000000):
    data.append([0] * 1000)
print('Memory test completed')"
                .to_string(),
            ("python", "cpu") => "import time
time.sleep(10)
print('CPU test completed')"
                .to_string(),
            ("cpp", "hello") => "#include <iostream>
int main() { 
    std::cout << \"Hello from C++\" << std::endl; 
    return 0; 
}"
            .to_string(),
            ("cpp", "memory") => "#include <iostream>
#include <vector>
int main() {
    std::vector<std::vector<int>> data;
    for(int i = 0; i < 1000000; i++) {
        data.push_back(std::vector<int>(1000, 0));
    }
    std::cout << \"Memory test completed\" << std::endl;
    return 0;
}"
            .to_string(),
            ("java", "hello") => "public class Main { 
    public static void main(String[] args) { 
        System.out.println(\"Hello from Java\"); 
    } 
}"
            .to_string(),
            _ => format!("// Test code for {} - {}", language, test_type),
        }
    }

    /// Validate that output contains expected content
    pub fn validate_output_contains(json: &Value, expected: &str) -> Result<()> {
        let stdout = Self::extract_stdout(json);
        if !stdout.contains(expected) {
            return Err(anyhow::anyhow!(
                "Expected output to contain '{}', got: '{}'",
                expected,
                stdout
            ));
        }
        Ok(())
    }

    /// Validate that execution time is within expected range
    pub fn validate_execution_time(json: &Value, min_seconds: f64, max_seconds: f64) -> Result<()> {
        let wall_time = Self::extract_wall_time(json);

        if wall_time < min_seconds {
            return Err(anyhow::anyhow!(
                "Execution too fast: {}s < {}s",
                wall_time,
                min_seconds
            ));
        }

        if wall_time > max_seconds {
            return Err(anyhow::anyhow!(
                "Execution too slow: {}s > {}s",
                wall_time,
                max_seconds
            ));
        }

        Ok(())
    }

    /// Validate that memory usage is within expected range
    pub fn validate_memory_usage(json: &Value, max_mb: u64) -> Result<()> {
        let memory_kb = Self::extract_memory_usage(json);
        let memory_mb = memory_kb / 1024;

        if memory_mb > max_mb {
            return Err(anyhow::anyhow!(
                "Memory usage too high: {}MB > {}MB",
                memory_mb,
                max_mb
            ));
        }

        Ok(())
    }
}
