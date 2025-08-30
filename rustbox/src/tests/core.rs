use crate::tests::common::{
    cleanup_test_box, execute_rustbox_command, generate_box_id, run_test, TestConfig, TestResult,
};
use crate::tests::utils::TestUtils;
use anyhow::Result;

/// Run all core functionality tests
pub fn run_core_tests(config: &TestConfig) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    // Test 1: Basic Python execution
    results.push(run_test(config, "Basic Python execution", || {
        test_basic_python_execution(config)
    }));

    // Test 2: Basic C++ execution
    results.push(run_test(
        config,
        "Basic C++ compilation and execution",
        || test_basic_cpp_execution(config),
    ));

    // Test 3: Basic Java execution
    results.push(run_test(
        config,
        "Basic Java compilation and execution",
        || test_basic_java_execution(config),
    ));

    // Test 4: Dependency checker
    results.push(run_test(config, "Language dependency checker", || {
        test_dependency_checker(config)
    }));

    // Test 5: Init and cleanup commands
    results.push(run_test(config, "Init and cleanup commands", || {
        test_init_cleanup_commands(config)
    }));

    // Test 6: Execute code with stdin
    results.push(run_test(config, "Execute code with stdin input", || {
        test_execute_with_stdin(config)
    }));

    // Test 7: Multiple language support
    results.push(run_test(config, "Multiple language support", || {
        test_multiple_languages(config)
    }));

    // Test 8: Error handling
    results.push(run_test(config, "Error handling for invalid code", || {
        test_error_handling(config)
    }));

    Ok(results)
}

/// Test basic Python execution
fn test_basic_python_execution(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = TestUtils::generate_test_code("python", "hello");

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
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "Hello from Python")?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test basic C++ compilation and execution
fn test_basic_cpp_execution(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = TestUtils::generate_test_code("cpp", "hello");

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
            "--processes",
            "50",
            "--mem",
            "256",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "Hello from C++")?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test basic Java compilation and execution
fn test_basic_java_execution(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = TestUtils::generate_test_code("java", "hello");

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
            "10",
            "--processes",
            "50",
            "--mem",
            "256",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "Hello from Java")?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test language dependency checker
fn test_dependency_checker(config: &TestConfig) -> Result<()> {
    let result = execute_rustbox_command(config, &["check-deps"])?;

    // The check-deps command should return success and contain dependency information
    let stdout = TestUtils::extract_stdout(&result);
    if !stdout.contains("Checking language dependencies") {
        return Err(anyhow::anyhow!(
            "Dependency checker output missing expected content"
        ));
    }

    Ok(())
}

/// Test init and cleanup commands
fn test_init_cleanup_commands(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();

    // Test init command
    let init_result = execute_rustbox_command(config, &["init", "--box-id", &box_id.to_string()])?;

    // Init should succeed (may not return JSON)
    if let Some(stderr) = init_result.get("stderr").and_then(|s| s.as_str()) {
        if stderr.contains("Error") || stderr.contains("Failed") {
            return Err(anyhow::anyhow!("Init command failed: {}", stderr));
        }
    }

    // Test cleanup command
    let cleanup_result =
        execute_rustbox_command(config, &["cleanup", "--box-id", &box_id.to_string()])?;

    // Cleanup should succeed
    if let Some(stderr) = cleanup_result.get("stderr").and_then(|s| s.as_str()) {
        if stderr.contains("Error") || stderr.contains("Failed") {
            return Err(anyhow::anyhow!("Cleanup command failed: {}", stderr));
        }
    }

    Ok(())
}

/// Test execute code with stdin input
fn test_execute_with_stdin(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();
    let code = "import sys
input_data = sys.stdin.read()
print(f'Received input: {input_data.strip()}')";

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
            "--stdin",
            "Hello from stdin",
        ],
    )?;

    TestUtils::validate_success_result(&result)?;
    TestUtils::validate_output_contains(&result, "Received input: Hello from stdin")?;

    cleanup_test_box(config, box_id);
    Ok(())
}

/// Test multiple language support
fn test_multiple_languages(config: &TestConfig) -> Result<()> {
    let languages = [
        ("python", "print('Python works')"),
        ("cpp", "#include <iostream>\nint main() { std::cout << \"C++ works\" << std::endl; return 0; }"),
        ("java", "public class Main { public static void main(String[] args) { System.out.println(\"Java works\"); } }"),
    ];

    for (lang, code) in languages {
        let box_id = generate_box_id();

        let result = execute_rustbox_command(
            config,
            &[
                "execute-code",
                "--strict",
                "--box-id",
                &box_id.to_string(),
                "--language",
                lang,
                "--code",
                code,
                "--time",
                "10",
                "--processes",
                "50",
                "--mem",
                "256",
            ],
        )?;

        TestUtils::validate_success_result(&result)?;
        cleanup_test_box(config, box_id);
    }

    Ok(())
}

/// Test error handling for invalid code
fn test_error_handling(config: &TestConfig) -> Result<()> {
    let box_id = generate_box_id();

    // Test with invalid Python syntax
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
            "print('Hello'  # Missing closing parenthesis",
        ],
    )?;

    // Should return a runtime error, not success
    TestUtils::validate_execution_result(&result)?;

    let status = result.get("status").and_then(|s| s.as_str()).unwrap_or("");

    if status != "RuntimeError" {
        return Err(anyhow::anyhow!(
            "Expected RuntimeError for invalid syntax, got: {}",
            status
        ));
    }

    cleanup_test_box(config, box_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_test_code() {
        let python_code = TestUtils::generate_test_code("python", "hello");
        assert!(python_code.contains("print"));

        let cpp_code = TestUtils::generate_test_code("cpp", "hello");
        assert!(cpp_code.contains("iostream"));

        let java_code = TestUtils::generate_test_code("java", "hello");
        assert!(java_code.contains("public class"));
    }

    #[test]
    fn test_validate_execution_result() {
        let valid_json = serde_json::json!({
            "status": "Success",
            "success": true,
            "stdout": "Hello",
            "stderr": ""
        });

        assert!(TestUtils::validate_execution_result(&valid_json).is_ok());

        let invalid_json = serde_json::json!({
            "status": "InvalidStatus",
            "success": true
        });

        assert!(TestUtils::validate_execution_result(&invalid_json).is_err());
    }

    #[test]
    fn test_core_test_run() {
        let mut config = TestConfig::default();
        config.verbose = true;
        let results = run_core_tests(&config).unwrap();
        assert!(!results.is_empty());
        for result in results {
            if !result.passed {
                eprintln!("Test failed: {}", result.name);
                if let Some(error) = &result.error_message {
                    eprintln!("Error: {}", error);
                }
            }
            assert!(result.passed);
        }
    }
}
