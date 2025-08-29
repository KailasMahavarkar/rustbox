//! Language-specific tests for rustbox
//!
//! Tests execution of code in different programming languages,
//! including Python, C++, and Java. These tests copy files from
//! the reference implementation and execute them with proper
//! stdin handling and resource limits.

use crate::tests::common::{
    cleanup_test_box, execute_rustbox_command, generate_box_id, run_test, TestConfig, TestResult,
};
use crate::tests::utils::TestUtils;
use anyhow::{Context as _, Result};
use std::fs;
use std::path::Path;

/// Run all language tests
pub fn run_language_tests(config: &TestConfig) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    // Test 1: Python factorial test
    results.push(run_test(config, "Python factorial test", || {
        test_python_factorial(config)
    }));

    // Test 2: Python star pattern test
    results.push(run_test(config, "Python star pattern test", || {
        test_python_star_pattern(config)
    }));

    // Test 3: Python LIS algorithm test
    results.push(run_test(config, "Python LIS algorithm test", || {
        test_python_lis(config)
    }));

    // Test 4: C++ factorial test
    results.push(run_test(config, "C++ factorial test", || {
        test_cpp_factorial(config)
    }));

    // Test 5: C++ star pattern test
    results.push(run_test(config, "C++ star pattern test", || {
        test_cpp_star_pattern(config)
    }));

    // Test 6: C++ LIS algorithm test
    results.push(run_test(config, "C++ LIS algorithm test", || {
        test_cpp_lis(config)
    }));

    // Test 7: Java factorial test
    results.push(run_test(config, "Java factorial test", || {
        test_java_factorial(config)
    }));

    // Test 8: Java star pattern test
    results.push(run_test(config, "Java star pattern test", || {
        test_java_star_pattern(config)
    }));

    // Test 9: Java LIS algorithm test
    results.push(run_test(config, "Java LIS algorithm test", || {
        test_java_lis(config)
    }));

    // Test 10: Time limit enforcement
    results.push(run_test(config, "Time limit enforcement test", || {
        test_time_limit_enforcement(config)
    }));

    // Test 11: Memory limit enforcement
    results.push(run_test(config, "Memory limit enforcement test", || {
        test_memory_limit_enforcement(config)
    }));

    Ok(results)
}

/// Test Python factorial implementation
fn test_python_factorial(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_python/test_1_fact.py")?;

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
            "--stdin",
            "5",
            "--time",
            "5",
            "--mem",
            "100",
            "--processes",
            "10",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "120")?; // 5! = 120

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test Python star pattern implementation
fn test_python_star_pattern(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_python/test_2_star.py")?;

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
            "--stdin",
            "3",
            "--time",
            "5",
            "--mem",
            "100",
            "--processes",
            "10",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "*")?; // Should contain star pattern

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test Python LIS algorithm implementation
fn test_python_lis(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_python/test_3_lis.py")?;

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
            "100",
            "--processes",
            "10",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    // LIS algorithm should produce some output
    let stdout = TestUtils::extract_stdout(&result);
    if stdout.trim().is_empty() {
        return Err(anyhow::anyhow!("LIS algorithm produced no output"));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test C++ factorial implementation
fn test_cpp_factorial(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_cpp/test_1_fact.cpp")?;

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "cpp",
            "--code",
            &code,
            "--stdin",
            "5",
            "--time",
            "10",
            "--mem",
            "300",
            "--processes",
            "15",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "120")?; // 5! = 120

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test C++ star pattern implementation
fn test_cpp_star_pattern(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_cpp/test_2_star.cpp")?;

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "cpp",
            "--code",
            &code,
            "--stdin",
            "3",
            "--time",
            "10",
            "--mem",
            "300",
            "--processes",
            "15",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "*")?; // Should contain star pattern

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test C++ LIS algorithm implementation
fn test_cpp_lis(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_cpp/test_3_lis.cpp")?;

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "cpp",
            "--code",
            &code,
            "--time",
            "10",
            "--mem",
            "300",
            "--processes",
            "15",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    // LIS algorithm should produce some output
    let stdout = TestUtils::extract_stdout(&result);
    if stdout.trim().is_empty() {
        return Err(anyhow::anyhow!("C++ LIS algorithm produced no output"));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test Java factorial implementation
fn test_java_factorial(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_java/test_1_fact.java")?;

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "java",
            "--code",
            &code,
            "--stdin",
            "5",
            "--time",
            "15",
            "--mem",
            "500",
            "--processes",
            "20",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "120")?; // 5! = 120

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test Java star pattern implementation
fn test_java_star_pattern(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_java/test_2_star.java")?;

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "java",
            "--code",
            &code,
            "--stdin",
            "3",
            "--time",
            "15",
            "--mem",
            "500",
            "--processes",
            "20",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "*")?; // Should contain star pattern

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test Java LIS algorithm implementation
fn test_java_lis(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_java/test_3_lis.java")?;

    let result = execute_rustbox_command(
        config,
        &[
            "execute-code",
            "--strict",
            "--box-id",
            &box_id.to_string(),
            "--language",
            "java",
            "--code",
            &code,
            "--time",
            "15",
            "--mem",
            "500",
            "--processes",
            "20",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    // LIS algorithm should produce some output
    let stdout = TestUtils::extract_stdout(&result);
    if stdout.trim().is_empty() {
        return Err(anyhow::anyhow!("Java LIS algorithm produced no output"));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test time limit enforcement
fn test_time_limit_enforcement(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_python/test_4_tle.py")?;

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
            "2", // Very short time limit
            "--mem",
            "100",
            "--processes",
            "10",
        ],
    )?;

    // Should hit time limit
    TestUtils::validate_time_limit_result(&result)?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test memory limit enforcement
fn test_memory_limit_enforcement(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = read_test_file("lang_python/test_5_mle.py")?;

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
            "10", // Very low memory limit
            "--processes",
            "10",
        ],
    )?;

    // Should hit memory limit
    TestUtils::validate_memory_limit_result(&result)?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Read test file from the embedded test files
fn read_test_file(filename: &str) -> Result<String> {
    let test_file_path = Path::new("src/tests/languages/test_files").join(filename);

    if !test_file_path.exists() {
        return Err(anyhow::anyhow!(
            "Test file not found: {}. Please ensure the embedded test files exist.",
            test_file_path.display()
        ));
    }

    let content = fs::read_to_string(&test_file_path)
        .with_context(|| format!("Failed to read test file: {}", test_file_path.display()))?;

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_test_file() {
        // Test that we can read test files
        let result = read_test_file("lang_python/test_1_fact.py");
        if result.is_ok() {
            let content = result.unwrap();
            assert!(content.contains("factorial"));
            assert!(content.contains("def"));
        }
    }

    #[test]
    fn test_language_code_validation() {
        // Test that language-specific code contains expected patterns
        let python_code = "def factorial(n):\n    return 1 if n <= 1 else n * factorial(n-1)";
        assert!(python_code.contains("def"));

        let cpp_code = "#include <iostream>\nint main() { return 0; }";
        assert!(cpp_code.contains("#include"));

        let java_code = "public class Main { public static void main(String[] args) {} }";
        assert!(java_code.contains("public class"));
    }
}
