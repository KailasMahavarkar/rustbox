/// Simple demonstration of the advanced I/O features for code sandbox
use mini_isolate::io_handler::{IoConfigBuilder};
use mini_isolate::types::IsolateConfig;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_io_config_builder_functionality() {
    // Test that the IoConfigBuilder works correctly
    let config = IoConfigBuilder::new(IsolateConfig::default())
        .with_pipes()
        .with_tty()
        .with_stdin_data("test input".to_string())
        .with_buffer_size(4096)
        .with_encoding("utf-8")
        .build();

    assert!(config.use_pipes);
    assert!(config.enable_tty);
    assert_eq!(config.stdin_data, Some("test input".to_string()));
    assert_eq!(config.io_buffer_size, 4096);
    assert_eq!(config.text_encoding, "utf-8");
}

#[test]
fn test_file_based_io_configuration() {
    let temp_dir = TempDir::new().unwrap();
    let stdin_file = temp_dir.path().join("input.txt");
    let stdout_file = temp_dir.path().join("output.txt");
    let stderr_file = temp_dir.path().join("error.txt");

    // Create input file
    fs::write(&stdin_file, "test input data").unwrap();

    let config = IoConfigBuilder::new(IsolateConfig::default())
        .with_stdin_file(stdin_file.clone())
        .with_stdout_file(stdout_file.clone())
        .with_stderr_file(stderr_file.clone())
        .build();

    assert_eq!(config.stdin_file, Some(stdin_file));
    assert_eq!(config.stdout_file, Some(stdout_file));
    assert_eq!(config.stderr_file, Some(stderr_file));
}

#[cfg(unix)]
#[test]
fn test_tty_support_detection() {
    use mini_isolate::io_handler::IoHandler;
    
    // TTY support should be available on Unix
    assert!(IoHandler::is_tty_supported());
}

#[test]
fn test_encoding_and_buffer_configuration() {
    let config = IoConfigBuilder::new(IsolateConfig::default())
        .with_encoding("utf-16")
        .with_buffer_size(16384)
        .build();

    assert_eq!(config.text_encoding, "utf-16");
    assert_eq!(config.io_buffer_size, 16384);
}

#[test]
fn test_default_io_configuration() {
    let config = IsolateConfig::default();
    
    // Verify default values for new I/O fields
    assert!(!config.enable_tty);
    assert!(!config.use_pipes);
    assert_eq!(config.stdin_data, None);
    assert_eq!(config.stdin_file, None);
    assert_eq!(config.io_buffer_size, 8192);
    assert_eq!(config.text_encoding, "utf-8");
}