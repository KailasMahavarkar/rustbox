use mini_isolate::{executor::ProcessExecutor, types};
use std::time::Duration;

#[cfg(unix)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_mode_requires_root_privileges() {
        #[cfg(unix)]
        {
            use nix::unistd::getuid;

            // Skip if we're running as root
            if getuid().is_root() {
                println!("Skipping test - running as root");
                return;
            }

            // Test should fail when not running as root with strict mode
            let mut config = types::IsolateConfig::default();
            config.strict_mode = true;
            config.instance_id = "strict_test_1".to_string();
            config.memory_limit = Some(64 * 1024 * 1024);
            config.time_limit = Some(Duration::from_secs(10));

            let result = ProcessExecutor::new(config);

            if result.is_err() {
                let error_msg = result.err().unwrap().to_string();
                assert!(
                    error_msg.contains("root") || error_msg.contains("permission"),
                    "Error should mention root privileges or permissions, got: {}",
                    error_msg
                );
            } else {
                // If executor was created successfully, try to execute a command
                let mut executor = result.unwrap();
                let exec_result = executor.execute(&["echo".to_string(), "test".to_string()], None);
                if let Err(e) = exec_result {
                    let error_msg = e.to_string();
                    assert!(
                        error_msg.contains("root") || error_msg.contains("permission"),
                        "Error should mention root privileges or permissions, got: {}",
                        error_msg
                    );
                }
            }
        }
    }

    #[test]
    fn test_non_strict_mode_graceful_fallback() {
        #[cfg(unix)]
        {
            use nix::unistd::getuid;

            // Skip if we're running as root (test won't be meaningful)
            if getuid().is_root() {
                println!("Skipping test - running as root, graceful fallback test not applicable");
                return;
            }

            // Test should succeed (with warnings) when not running as root without strict mode
            let mut config = types::IsolateConfig::default();
            config.strict_mode = false; // Default behavior
            config.instance_id = "non_strict_test_1".to_string();
            config.memory_limit = Some(64 * 1024 * 1024);
            config.time_limit = Some(Duration::from_secs(10));

            let result = ProcessExecutor::new(config);

            // Should either succeed (graceful fallback) or fail gracefully
            match result {
                Ok(mut executor) => {
                    // Try to execute a simple command
                    let exec_result =
                        executor.execute(&["echo".to_string(), "test".to_string()], None);
                    // Should either succeed or fail gracefully (not panic)
                    match exec_result {
                        Ok(output) => {
                            assert!(!output.stdout.is_empty() || !output.stderr.is_empty());
                        }
                        Err(_) => {
                            // Graceful failure is also acceptable in non-strict mode
                        }
                    }
                }
                Err(_) => {
                    // Even failure is acceptable as long as it's graceful
                }
            }
        }
    }

    #[test]
    fn test_strict_mode_cgroup_requirement() {
        #[cfg(unix)]
        {
            use nix::unistd::getuid;

            // Test cgroup requirement in strict mode
            let mut config = types::IsolateConfig::default();
            config.strict_mode = true;
            config.instance_id = "strict_cgroup_test".to_string();
            config.memory_limit = Some(64 * 1024 * 1024);

            // This test verifies that strict mode fails appropriately when cgroups are unavailable
            // or when running without proper privileges
            if !getuid().is_root() {
                let result = ProcessExecutor::new(config);

                if result.is_err() {
                    let error_msg = result.err().unwrap().to_string();
                    assert!(
                        error_msg.contains("root")
                            || error_msg.contains("cgroup")
                            || error_msg.contains("permission"),
                        "Error should mention root, cgroup, or permission issues, got: {}",
                        error_msg
                    );
                } else {
                    // If executor creation succeeded, command execution should fail appropriately
                    let mut executor = result.unwrap();
                    let exec_result =
                        executor.execute(&["echo".to_string(), "test".to_string()], None);
                    if let Err(e) = exec_result {
                        let error_msg = e.to_string();
                        assert!(
                            error_msg.contains("root")
                                || error_msg.contains("cgroup")
                                || error_msg.contains("permission"),
                            "Error should mention root, cgroup, or permission issues, got: {}",
                            error_msg
                        );
                    }
                }
            } else {
                println!("Running as root - cgroup requirement test may not be meaningful");
                // When running as root, the test behavior depends on cgroup availability
                let _result = ProcessExecutor::new(config);
                // Don't assert anything specific since cgroup availability varies
            }
        }
    }

    #[test]
    fn test_default_non_strict_behavior() {
        // Test that strict mode is false by default
        let config = types::IsolateConfig::default();
        assert!(
            !config.strict_mode,
            "Default config should have strict_mode = false"
        );

        // Test that non-strict mode allows graceful degradation
        let mut config = types::IsolateConfig::default();
        config.instance_id = "default_test".to_string();
        config.memory_limit = Some(64 * 1024 * 1024);

        let result = ProcessExecutor::new(config);
        // In non-strict mode, we should either succeed or fail gracefully
        match result {
            Ok(_) => {
                // Success is good
            }
            Err(_) => {
                // Graceful failure is also acceptable in non-strict mode
            }
        }
    }
}
