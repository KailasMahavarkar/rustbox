use serial_test::serial;
use std::path::Path;

#[test]
#[serial]
fn test_cgroup_controller_functionality() {
    // Skip test if not running with appropriate permissions
    if !Path::new("/sys/fs/cgroup").exists() {
        println!("Skipping cgroup test - cgroups not available");
        return;
    }

    let controller = mini_isolate::cgroup::CgroupController::new("test_cgroup", false);

    match controller {
        Ok(cg) => {
            // Test setting memory limit
            let limit_result = cg.set_memory_limit(64 * 1024 * 1024);
            println!("Memory limit result: {:?}", limit_result);

            // Cleanup
            let _ = cg.cleanup();
        }
        Err(e) => {
            println!(
                "Cgroup controller creation failed (expected without root): {}",
                e
            );
        }
    }
}
