use mini_isolate::{isolate, types};
use tempfile::TempDir;

/// Helper function to create test directory
fn setup_test_env() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Helper function to create test isolate with common configuration
pub fn create_test_isolate(box_id: &str) -> types::Result<isolate::Isolate> {
    let mut config = types::IsolateConfig::default();
    config.instance_id = box_id.to_string();
    config.memory_limit = Some(64 * 1024 * 1024); // 64MB default limit
    config.time_limit = Some(std::time::Duration::from_secs(30));
    config.wall_time_limit = Some(std::time::Duration::from_secs(60));
    config.process_limit = Some(5);
    config.file_size_limit = Some(32 * 1024 * 1024); // 32MB default file limit

    // Create working directory
    let temp_dir = setup_test_env();
    config.workdir = temp_dir.path().to_path_buf();

    // Leak the temp_dir so it doesn't get dropped
    std::mem::forget(temp_dir);

    isolate::Isolate::new(config)
}

// Import all test modules
mod basic;
mod concurrent;
mod file_locking;
mod filesize;
mod invalid;
mod io;
mod language;
mod memory;
mod process;
mod resource_limits;
mod seccomp;
mod security;
mod strict_mode;
mod timeout;
