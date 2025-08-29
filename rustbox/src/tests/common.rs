//! Common utilities and types for rustbox tests

use anyhow::{Context, Result};
use serde_json::Value;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Test result structure for consistent reporting
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration: Duration,
    pub error_message: Option<String>,
    pub details: Option<String>,
}

/// Test suite configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub rustbox_path: String,
    pub require_sudo: bool,
    pub timeout: Duration,
    pub verbose: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            rustbox_path: "target/release/rustbox".to_string(),
            require_sudo: true,
            timeout: Duration::from_secs(30),
            verbose: false,
        }
    }
}

/// Utility function to execute rustbox command and parse JSON output
pub fn execute_rustbox_command(config: &TestConfig, args: &[&str]) -> Result<Value> {
    let mut cmd = if config.require_sudo {
        let mut sudo_cmd = Command::new("sudo");
        sudo_cmd.arg(&config.rustbox_path);
        sudo_cmd.args(args);
        sudo_cmd
    } else {
        let mut cmd = Command::new(&config.rustbox_path);
        cmd.args(args);
        cmd
    };

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute rustbox with args: {:?}", args))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if config.verbose {
        println!("Command: rustbox {}", args.join(" "));
        println!("Exit code: {}", output.status.code().unwrap_or(-1));
        if !stdout.is_empty() {
            println!("Stdout: {}", stdout);
        }
        if !stderr.is_empty() {
            println!("Stderr: {}", stderr);
        }
    }

    // Try to parse as JSON first
    if let Ok(json) = serde_json::from_str::<Value>(&stdout) {
        Ok(json)
    } else {
        // If not JSON, create a simple result structure
        Ok(serde_json::json!({
            "status": if output.status.success() { "Success" } else { "RuntimeError" },
            "success": output.status.success(),
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": output.status.code()
        }))
    }
}

/// Utility function to generate unique box IDs for tests
pub fn generate_box_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(1000);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Utility function to clean up test boxes
pub fn cleanup_test_box(config: &TestConfig, box_id: u32) {
    let _ = execute_rustbox_command(config, &["cleanup", "--box-id", &box_id.to_string()]);
}

/// Function to simplify test execution and result collection
pub fn run_test<F, E>(_config: &TestConfig, name: &str, test_fn: F) -> TestResult
where
    F: FnOnce() -> Result<(), E>,
    E: std::fmt::Display,
{
    let start = std::time::Instant::now();
    let result = test_fn();
    let duration = start.elapsed();

    TestResult {
        name: name.to_string(),
        passed: result.is_ok(),
        duration,
        error_message: result.err().map(|e| e.to_string()),
        details: None,
    }
}
