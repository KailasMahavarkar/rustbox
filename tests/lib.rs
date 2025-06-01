use tempfile::TempDir;
use mini_isolate::{types, isolate};

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
mod memory;
mod timeout;
mod process;
mod filesize;
mod invalid;
mod concurrent;
mod io;
mod language;
mod security;
mod resource_limits;
mod strict_mode;