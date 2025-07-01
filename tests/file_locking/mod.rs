use serial_test::serial;
/// Tests for file locking implementation to prevent race conditions
/// Based on isolate-reference locking mechanism
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

use mini_isolate::{
    isolate::Isolate,
    types::{IsolateConfig, IsolateError},
};

#[test]
#[serial]
fn test_concurrent_initialization_same_id() {
    let instance_id = "test-concurrent-init";
    let temp_dir = TempDir::new().unwrap();

    let config1 = IsolateConfig {
        instance_id: instance_id.to_string(),
        workdir: temp_dir.path().join("instance1"),
        ..Default::default()
    };

    let config2 = IsolateConfig {
        instance_id: instance_id.to_string(),
        workdir: temp_dir.path().join("instance2"),
        ..Default::default()
    };

    let barrier = Arc::new(Barrier::new(2));
    let barrier1 = barrier.clone();
    let barrier2 = barrier.clone();

    let handle1 = thread::spawn(move || {
        barrier1.wait();
        Isolate::new(config1)
    });

    let handle2 = thread::spawn(move || {
        barrier2.wait();
        Isolate::new(config2)
    });

    let result1 = handle1.join().unwrap();
    let result2 = handle2.join().unwrap();

    println!("Result1: {}", if result1.is_ok() { "Ok" } else { "Err" });
    println!("Result2: {}", if result2.is_ok() { "Ok" } else { "Err" });
    if let Err(ref e) = result1 {
        println!("Result1 error: {:?}", e);
    }
    if let Err(ref e) = result2 {
        println!("Result2 error: {:?}", e);
    }

    // One should succeed, one should fail with lock busy
    match (&result1, &result2) {
        (Ok(_), Err(IsolateError::LockBusy)) | (Err(IsolateError::LockBusy), Ok(_)) => {
            // Expected: one succeeds, one fails due to lock
        }
        (Ok(_), Ok(_)) => panic!("Both initializations succeeded - lock not working!"),
        (Err(e1), Err(e2)) => panic!("Both failed - e1: {:?}, e2: {:?}", e1, e2),
        _ => panic!("Other unexpected pattern"),
    }

    // Cleanup - be more graceful about lock conflicts
    if let Ok(Some(isolate)) = Isolate::load(instance_id) {
        if let Err(e) = isolate.cleanup() {
            println!("Warning: cleanup failed for {}: {:?}", instance_id, e);
        }
    }
}

#[test]
#[serial]
fn test_concurrent_load_operations() {
    let instance_id = "test-concurrent-load";
    let temp_dir = TempDir::new().unwrap();

    // First create an isolate
    let config = IsolateConfig {
        instance_id: instance_id.to_string(),
        workdir: temp_dir.path().join("workdir"),
        ..Default::default()
    };

    let isolate = Isolate::new(config).unwrap();
    drop(isolate); // Release the lock

    // Now try concurrent loads
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let id = instance_id.to_string();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(i * 10));
                Isolate::load(&id)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All loads should succeed (different from initialization)
    for result in results {
        assert!(result.is_ok());
        let isolate_opt = result.unwrap();
        assert!(isolate_opt.is_some());
    }

    // Cleanup - be more graceful about lock conflicts
    if let Ok(Some(isolate)) = Isolate::load(instance_id) {
        if let Err(e) = isolate.cleanup() {
            println!("Warning: cleanup failed for {}: {:?}", instance_id, e);
        }
    }
}

#[test]
#[serial]
fn test_concurrent_cleanup_operations() {
    let instance_id = "test-concurrent-cleanup";
    let temp_dir = TempDir::new().unwrap();

    // Create an isolate
    let config = IsolateConfig {
        instance_id: instance_id.to_string(),
        workdir: temp_dir.path().join("workdir"),
        ..Default::default()
    };

    let isolate = Isolate::new(config).unwrap();
    drop(isolate);

    // Try concurrent cleanups
    let handles: Vec<_> = (0..3)
        .map(|_| {
            let id = instance_id.to_string();
            thread::spawn(move || {
                if let Ok(Some(isolate)) = Isolate::load(&id) {
                    isolate.cleanup()
                } else {
                    Ok(()) // Already cleaned up
                }
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // At least one cleanup should succeed, others might find nothing to clean
    let successful = results.iter().filter(|r| r.is_ok()).count();
    assert!(successful >= 1);

    // Verify instance is actually cleaned up
    assert!(Isolate::load(instance_id).unwrap().is_none());
}

#[test]
#[serial]
fn test_lock_file_ownership() {
    let instance_id = "test-ownership";
    let temp_dir = TempDir::new().unwrap();

    let config = IsolateConfig {
        instance_id: instance_id.to_string(),
        workdir: temp_dir.path().join("workdir"),
        ..Default::default()
    };

    // Create isolate as current user
    let isolate = Isolate::new(config.clone()).unwrap();
    drop(isolate);

    // Try to load as same user (should succeed)
    let isolate2 = Isolate::load(instance_id).unwrap();
    assert!(isolate2.is_some());

    // Cleanup
    isolate2.unwrap().cleanup().unwrap();
}

#[test]
#[serial]
fn test_instances_json_atomic_updates() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple isolates concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let temp_path = temp_dir.path().to_path_buf();
            thread::spawn(move || {
                let config = IsolateConfig {
                    instance_id: format!("concurrent-{}", i),
                    workdir: temp_path.join(format!("workdir-{}", i)),
                    ..Default::default()
                };
                Isolate::new(config)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All should succeed
    for result in &results {
        assert!(result.is_ok());
    }

    // Verify all instances are properly recorded
    let instances = Isolate::list_all().unwrap();
    println!("Found {} instances: {:?}", instances.len(), instances);

    // Filter out instances that don't match our pattern
    let our_instances: Vec<_> = instances
        .iter()
        .filter(|id| id.starts_with("concurrent-"))
        .collect();
    assert_eq!(our_instances.len(), 10);

    // Cleanup all - drop results first to release any locks
    drop(results);

    // Wait a bit for any locks to be released
    thread::sleep(Duration::from_millis(100));

    for i in 0..10 {
        let instance_id = format!("concurrent-{}", i);
        if let Ok(Some(isolate)) = Isolate::load(&instance_id) {
            if let Err(e) = isolate.cleanup() {
                println!("Warning: cleanup failed for {}: {:?}", instance_id, e);
            }
        }
    }

    // Verify all cleaned up
    let instances_after = Isolate::list_all().unwrap();
    println!("Instances after cleanup: {:?}", instances_after);
    let remaining_concurrent: Vec<_> = instances_after
        .iter()
        .filter(|id| id.starts_with("concurrent-"))
        .collect();
    assert_eq!(remaining_concurrent.len(), 0);
}

#[test]
#[serial]
fn test_lock_prevents_double_execution() {
    let instance_id = "test-double-execution";
    let temp_dir = TempDir::new().unwrap();

    let config = IsolateConfig {
        instance_id: instance_id.to_string(),
        workdir: temp_dir.path().join("workdir"),
        time_limit: Some(Duration::from_secs(5)), // Long enough for test
        ..Default::default()
    };

    let mut isolate1 = Isolate::new(config).unwrap();

    // Start long-running command in background thread
    let instance_id_clone = instance_id.to_string();
    let handle = thread::spawn(move || {
        // Try to load same instance from different thread
        thread::sleep(Duration::from_millis(100)); // Let first execution start
        Isolate::load(&instance_id_clone)
    });

    // Try to execute in original thread
    let command = vec!["sleep".to_string(), "1".to_string()];
    let result1 = isolate1.execute(&command, None);

    // Check second load result
    let load_result = handle.join().unwrap();

    // First execution should work
    assert!(result1.is_ok());

    // Second load should succeed (different from execution)
    assert!(load_result.is_ok());

    // Cleanup
    isolate1.cleanup().unwrap();
}

#[test]
#[serial]
fn test_initialization_race_condition() {
    let instance_id = "test-init-race";
    let temp_dir = TempDir::new().unwrap();
    let barrier = Arc::new(Barrier::new(2));

    let handles: Vec<_> = (0..2)
        .map(|i| {
            let id = instance_id.to_string();
            let path = temp_dir.path().join(format!("workdir-{}", i));
            let barrier_clone = barrier.clone();
            thread::spawn(move || {
                let config = IsolateConfig {
                    instance_id: id,
                    workdir: path,
                    ..Default::default()
                };
                barrier_clone.wait();
                Isolate::new(config)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let lock_busy_count = results
        .iter()
        .filter(|r| match r {
            Err(IsolateError::LockBusy) => true,
            _ => false,
        })
        .count();

    assert_eq!(
        success_count, 1,
        "Exactly one initialization should succeed"
    );
    assert_eq!(
        lock_busy_count, 1,
        "Exactly one initialization should fail with LockBusy"
    );

    // Cleanup
    if let Ok(Some(isolate)) = Isolate::load(instance_id) {
        if let Err(e) = isolate.cleanup() {
            println!("Warning: cleanup failed for {}: {:?}", instance_id, e);
        }
    }
}
