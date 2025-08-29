/// Enhanced "Living Locks" with heartbeat-based locking system for rustbox
/// Based on the senior SDE design from new_lock.md

use crate::types::{LockError, LockResult, LockInfo, LockManagerHealth, LockMetrics, HealthStatus};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Write, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::thread::{JoinHandle, sleep};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use nix::fcntl::{flock, FlockArg};
use log::{warn, error, info, debug};

/// The core lock manager implementing "Living Locks" with heartbeat
pub struct RustboxLockManager {
    lock_dir: PathBuf,
    heartbeat_interval: Duration,
    stale_timeout: Duration,
    cleanup_thread: Option<JoinHandle<()>>,
    shutdown_signal: Arc<AtomicBool>,
    metrics: Arc<Mutex<LockMetrics>>,
}

/// Individual box lock with heartbeat functionality
pub struct BoxLock {
    box_id: u32,
    lock_file: File,
    lock_path: PathBuf,
    owner_pid: u32,
    created_at: SystemTime,
    heartbeat_file: File,
    heartbeat_handle: Option<JoinHandle<()>>,
    shutdown_signal: Arc<AtomicBool>,
}

/// RAII guard for box locks
pub struct BoxLockGuard {
    lock: Arc<Mutex<Option<BoxLock>>>,
    cleanup_on_drop: bool,
}

/// Drop guard for automatic cleanup
struct DropGuard {
    box_id: u32,
    manager: Arc<RustboxLockManager>,
}

impl RustboxLockManager {
    /// Create new lock manager with default settings
    pub fn new() -> LockResult<Self> {
        Self::with_config(
            PathBuf::from("/var/run/rustbox/locks"),
            Duration::from_secs(1),
            Duration::from_secs(10)
        )
    }

    /// Create lock manager with custom configuration
    pub fn with_config(
        lock_dir: PathBuf, 
        heartbeat_interval: Duration, 
        stale_timeout: Duration
    ) -> LockResult<Self> {
        // Fail fast if we can't create lock directory
        std::fs::create_dir_all(&lock_dir)
            .map_err(|e| LockError::PermissionDenied { 
                details: format!("Cannot create lock directory: {}", e) 
            })?;

        // Test write permissions immediately
        let test_file = lock_dir.join(".write_test");
        std::fs::write(&test_file, b"test")
            .map_err(|_| LockError::PermissionDenied {
                details: "Lock directory not writable".to_string()
            })?;
        std::fs::remove_file(&test_file)?;

        let shutdown_signal = Arc::new(AtomicBool::new(false));
        let metrics = Arc::new(Mutex::new(LockMetrics {
            total_acquisitions: 0,
            average_acquisition_time_ms: 0.0,
            lock_contentions: 0,
            cleanup_operations: 0,
            errors_by_type: HashMap::new(),
        }));

        let manager = Self {
            lock_dir,
            heartbeat_interval,
            stale_timeout,
            cleanup_thread: None,
            shutdown_signal: shutdown_signal.clone(),
            metrics,
        };

        Ok(manager)
    }

    /// Start the background cleanup thread
    pub fn start_cleanup_thread(&mut self) -> LockResult<()> {
        let lock_dir = self.lock_dir.clone();
        let stale_timeout = self.stale_timeout;
        let shutdown_signal = self.shutdown_signal.clone();
        let metrics = self.metrics.clone();

        let handle = std::thread::spawn(move || {
            while !shutdown_signal.load(Ordering::Relaxed) {
                if let Err(e) = Self::cleanup_stale_locks_internal(&lock_dir, stale_timeout) {
                    error!("Background cleanup failed: {}", e);
                } else {
                    if let Ok(mut m) = metrics.lock() {
                        m.cleanup_operations += 1;
                    }
                }

                // Sleep for 30 seconds before next cleanup cycle
                for _ in 0..30 {
                    if shutdown_signal.load(Ordering::Relaxed) {
                        break;
                    }
                    sleep(Duration::from_secs(1));
                }
            }
            debug!("Cleanup thread shutting down");
        });

        self.cleanup_thread = Some(handle);
        Ok(())
    }

    /// Acquire lock for a specific box with timeout
    pub fn acquire_lock(&self, box_id: u32, timeout: Duration) -> LockResult<BoxLockGuard> {
        let start_time = Instant::now();
        let lock_path = self.lock_dir.join(format!("box-{}.lock", box_id));
        let heartbeat_path = self.lock_dir.join(format!("box-{}.heartbeat", box_id));

        // Update metrics
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.total_acquisitions += 1;
        }

        // Step 1: Clean any stale lock first
        if let Err(e) = self.cleanup_stale_lock_if_needed(box_id) {
            warn!("Failed to cleanup stale lock for box {}: {}", box_id, e);
            // Continue anyway - might not be stale
        }

        // Step 2: Retry loop with exponential backoff
        let mut retry_delay = Duration::from_millis(10);
        let mut contention_count = 0u64;

        loop {
            match self.try_acquire_immediate(box_id, &lock_path, &heartbeat_path) {
                Ok(lock_guard) => {
                    let elapsed = start_time.elapsed();
                    info!("Acquired lock for box {} in {:?}", box_id, elapsed);
                    
                    // Update metrics
                    if let Ok(mut metrics) = self.metrics.lock() {
                        metrics.lock_contentions += contention_count;
                        let total = metrics.total_acquisitions as f64;
                        let new_time = elapsed.as_millis() as f64;
                        metrics.average_acquisition_time_ms = 
                            (metrics.average_acquisition_time_ms * (total - 1.0) + new_time) / total;
                    }
                    
                    return Ok(lock_guard);
                }
                Err(LockError::Busy { .. }) => {
                    contention_count += 1;
                    
                    if start_time.elapsed() >= timeout {
                        return Err(LockError::Timeout {
                            box_id,
                            waited: start_time.elapsed(),
                            current_owner: self.get_lock_owner(box_id),
                        });
                    }

                    // Exponential backoff with jitter
                    let jitter = Duration::from_millis(
                        fastrand::u64(0..=retry_delay.as_millis() as u64)
                    );
                    sleep(retry_delay + jitter);
                    retry_delay = std::cmp::min(retry_delay * 2, Duration::from_millis(500));
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Atomic lock acquisition attempt
    fn try_acquire_immediate(
        &self,
        box_id: u32,
        lock_path: &Path,
        heartbeat_path: &Path
    ) -> LockResult<BoxLockGuard> {
        // Step 1: Create lock file with exclusive access
        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(lock_path)?;

        // Step 2: Try to acquire exclusive lock (non-blocking)
        match flock(lock_file.as_raw_fd(), FlockArg::LockExclusiveNonblock) {
            Ok(()) => {
                // Success! We have the lock
            }
            Err(nix::errno::Errno::EWOULDBLOCK) => {
                return Err(LockError::Busy { 
                    box_id, 
                    owner_pid: self.get_lock_owner_pid(box_id) 
                });
            }
            Err(e) => {
                return Err(LockError::SystemError {
                    message: format!("flock failed: {}", e)
                });
            }
        }

        // Step 3: Write our PID to lock file (for debugging)
        let lock_info = LockInfo {
            pid: std::process::id(),
            box_id,
            created_at: SystemTime::now(),
            rustbox_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let lock_content = serde_json::to_string(&lock_info)
            .map_err(|e| LockError::SystemError {
                message: format!("Failed to serialize lock info: {}", e)
            })?;

        let mut lock_file = lock_file;
        writeln!(lock_file, "{}", lock_content)?;
        lock_file.sync_all()?;

        // Step 4: Create heartbeat file
        let heartbeat_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(heartbeat_path)?;

        // Step 5: Create the lock object
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        let heartbeat_handle = self.start_heartbeat_thread(
            box_id, 
            heartbeat_path.to_owned(), 
            shutdown_signal.clone()
        );

        let lock = BoxLock {
            box_id,
            lock_file,
            lock_path: lock_path.to_owned(),
            owner_pid: std::process::id(),
            created_at: SystemTime::now(),
            heartbeat_file,
            heartbeat_handle: Some(heartbeat_handle),
            shutdown_signal,
        };

        // Step 6: Return RAII guard
        Ok(BoxLockGuard {
            lock: Arc::new(Mutex::new(Some(lock))),
            cleanup_on_drop: true,
        })
    }

    /// Background thread that updates heartbeat every interval
    fn start_heartbeat_thread(
        &self, 
        box_id: u32, 
        heartbeat_path: PathBuf, 
        shutdown_signal: Arc<AtomicBool>
    ) -> JoinHandle<()> {
        let interval = self.heartbeat_interval;

        std::thread::spawn(move || {
            while !shutdown_signal.load(Ordering::Relaxed) {
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                match OpenOptions::new().write(true).truncate(true).open(&heartbeat_path) {
                    Ok(mut heartbeat_file) => {
                        if let Err(e) = writeln!(heartbeat_file, "{}", timestamp) {
                            warn!("Failed to write heartbeat for box {}: {}", box_id, e);
                            break;
                        }
                        if let Err(e) = heartbeat_file.flush() {
                            warn!("Failed to flush heartbeat for box {}: {}", box_id, e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to open heartbeat file for box {}: {}", box_id, e);
                        break;
                    }
                }

                sleep(interval);
            }
            debug!("Heartbeat thread for box {} shutting down", box_id);
        })
    }

    /// Cleanup logic for stale locks
    fn cleanup_stale_lock_if_needed(&self, box_id: u32) -> LockResult<()> {
        let lock_path = self.lock_dir.join(format!("box-{}.lock", box_id));
        let heartbeat_path = self.lock_dir.join(format!("box-{}.heartbeat", box_id));

        // If no lock file exists, nothing to clean
        if !lock_path.exists() {
            return Ok(());
        }

        // Try to read lock info
        let lock_content = std::fs::read_to_string(&lock_path)?;
        let lock_info: LockInfo = serde_json::from_str(&lock_content)
            .map_err(|e| LockError::CorruptedLock { 
                box_id, 
                details: format!("Invalid lock file format: {}", e)
            })?;

        // Check if the owning process is still alive
        if self.is_process_alive(lock_info.pid) {
            // Process exists, check heartbeat
            if let Ok(last_heartbeat) = self.get_last_heartbeat(&heartbeat_path) {
                let heartbeat_age = SystemTime::now()
                    .duration_since(last_heartbeat)
                    .unwrap_or(Duration::from_secs(999));

                if heartbeat_age < self.stale_timeout {
                    // Lock is active and healthy
                    return Err(LockError::Busy { 
                        box_id, 
                        owner_pid: Some(lock_info.pid) 
                    });
                }
            }
        }

        // Lock is stale - clean it up
        warn!("Cleaning up stale lock for box {} (pid {} not responding)", box_id, lock_info.pid);

        // Remove lock files
        let _ = std::fs::remove_file(&lock_path);
        let _ = std::fs::remove_file(&heartbeat_path);

        Ok(())
    }

    /// Check if process is alive using /proc filesystem
    fn is_process_alive(&self, pid: u32) -> bool {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }

    /// Get last heartbeat timestamp from file
    fn get_last_heartbeat(&self, heartbeat_path: &Path) -> LockResult<SystemTime> {
        let content = std::fs::read_to_string(heartbeat_path)?;
        let timestamp_str = content.trim();
        let timestamp: u64 = timestamp_str.parse()
            .map_err(|_| LockError::CorruptedLock {
                box_id: 0,
                details: "Invalid heartbeat format".to_string()
            })?;
        
        Ok(UNIX_EPOCH + Duration::from_secs(timestamp))
    }

    /// Get current lock owner information
    fn get_lock_owner(&self, box_id: u32) -> Option<String> {
        self.get_lock_owner_pid(box_id)
            .map(|pid| format!("PID {}", pid))
    }

    /// Get current lock owner PID
    fn get_lock_owner_pid(&self, box_id: u32) -> Option<u32> {
        let lock_path = self.lock_dir.join(format!("box-{}.lock", box_id));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock_info) = serde_json::from_str::<LockInfo>(&content) {
                return Some(lock_info.pid);
            }
        }
        None
    }

    /// Internal cleanup function for background thread
    fn cleanup_stale_locks_internal(lock_dir: &Path, stale_timeout: Duration) -> LockResult<()> {
        if !lock_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(lock_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with("box-") && file_name.ends_with(".lock") {
                    // Extract box ID from filename
                    if let Some(box_id_str) = file_name.strip_prefix("box-").and_then(|s| s.strip_suffix(".lock")) {
                        if let Ok(box_id) = box_id_str.parse::<u32>() {
                            // This is a simplified cleanup that just checks stale locks
                            let _ = Self::cleanup_single_stale_lock(lock_dir, box_id, stale_timeout);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Cleanup a single stale lock (used by background thread)
    fn cleanup_single_stale_lock(lock_dir: &Path, box_id: u32, stale_timeout: Duration) -> LockResult<()> {
        let lock_path = lock_dir.join(format!("box-{}.lock", box_id));
        let heartbeat_path = lock_dir.join(format!("box-{}.heartbeat", box_id));

        // Basic stale check without trying to acquire lock
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock_info) = serde_json::from_str::<LockInfo>(&content) {
                // Check process + heartbeat
                let process_alive = std::path::Path::new(&format!("/proc/{}", lock_info.pid)).exists();
                let heartbeat_stale = if let Ok(timestamp_str) = std::fs::read_to_string(&heartbeat_path) {
                    if let Ok(timestamp) = timestamp_str.trim().parse::<u64>() {
                        let last_heartbeat = UNIX_EPOCH + Duration::from_secs(timestamp);
                        SystemTime::now().duration_since(last_heartbeat).unwrap_or(Duration::from_secs(999)) > stale_timeout
                    } else {
                        true // Invalid heartbeat format = stale
                    }
                } else {
                    true // No heartbeat file = stale
                };

                if !process_alive || heartbeat_stale {
                    warn!("Background cleanup: removing stale lock for box {} (PID {})", box_id, lock_info.pid);
                    let _ = std::fs::remove_file(&lock_path);
                    let _ = std::fs::remove_file(&heartbeat_path);
                }
            }
        }

        Ok(())
    }

    /// Health check for monitoring
    pub fn health_check(&self) -> LockManagerHealth {
        let mut active_locks = 0u32;
        let lock_directory_writable = self.test_directory_writable();
        let cleanup_thread_alive = self.cleanup_thread.as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false);

        // Count active locks
        if let Ok(entries) = std::fs::read_dir(&self.lock_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("box-") && name.ends_with(".lock") {
                        active_locks += 1;
                    }
                }
            }
        }

        let metrics = self.metrics.lock().unwrap().clone();

        let status = if lock_directory_writable && cleanup_thread_alive {
            HealthStatus::Healthy
        } else if lock_directory_writable || cleanup_thread_alive {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        LockManagerHealth {
            status,
            active_locks,
            stale_locks_cleaned: metrics.cleanup_operations,
            lock_directory_writable,
            cleanup_thread_alive,
            metrics,
        }
    }

    /// Test if lock directory is writable
    fn test_directory_writable(&self) -> bool {
        let test_file = self.lock_dir.join(".health_test");
        std::fs::write(&test_file, b"test").is_ok() &&
        std::fs::remove_file(&test_file).is_ok()
    }

    /// Export Prometheus-style metrics
    pub fn export_metrics(&self) -> String {
        let metrics = self.metrics.lock().unwrap();
        
        format!(
r#"# HELP rustbox_lock_acquisitions_total Total lock acquisitions
# TYPE rustbox_lock_acquisitions_total counter
rustbox_lock_acquisitions_total {}

# HELP rustbox_lock_contentions_total Total lock contentions
# TYPE rustbox_lock_contentions_total counter  
rustbox_lock_contentions_total {}

# HELP rustbox_lock_acquisition_duration_ms Average lock acquisition time
# TYPE rustbox_lock_acquisition_duration_ms gauge
rustbox_lock_acquisition_duration_ms {}

# HELP rustbox_lock_cleanup_operations_total Total cleanup operations
# TYPE rustbox_lock_cleanup_operations_total counter
rustbox_lock_cleanup_operations_total {}
"#,
            metrics.total_acquisitions,
            metrics.lock_contentions, 
            metrics.average_acquisition_time_ms,
            metrics.cleanup_operations
        )
    }
}

impl Drop for RustboxLockManager {
    fn drop(&mut self) {
        // Signal shutdown to cleanup thread
        self.shutdown_signal.store(true, Ordering::Relaxed);
        
        // Wait for cleanup thread to finish
        if let Some(handle) = self.cleanup_thread.take() {
            let _ = handle.join();
        }
    }
}

impl BoxLock {
    /// Get box ID
    pub fn box_id(&self) -> u32 {
        self.box_id
    }

    /// Get owner PID
    pub fn owner_pid(&self) -> u32 {
        self.owner_pid
    }

    /// Get creation time
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }
}

impl Drop for BoxLock {
    fn drop(&mut self) {
        // Signal heartbeat thread to stop
        self.shutdown_signal.store(true, Ordering::Relaxed);
        
        // Wait for heartbeat thread
        if let Some(handle) = self.heartbeat_handle.take() {
            let _ = handle.join();
        }
        
        // Remove heartbeat file
        let heartbeat_path = self.lock_path.with_extension("heartbeat");
        let _ = std::fs::remove_file(&heartbeat_path);
        
        debug!("Dropped lock for box {}", self.box_id);
    }
}

impl BoxLockGuard {
    /// Get box ID from the guard
    pub fn box_id(&self) -> Option<u32> {
        self.lock.lock().ok()?.as_ref().map(|l| l.box_id)
    }
    
    /// Get owner PID
    pub fn owner_pid(&self) -> Option<u32> {
        self.lock.lock().ok()?.as_ref().map(|l| l.owner_pid)
    }

    /// Release the lock explicitly (normally done automatically on drop)
    pub fn release(mut self) {
        self.cleanup_on_drop = false;
        if let Ok(mut guard) = self.lock.lock() {
            *guard = None; // This will drop the BoxLock
        }
    }
}

impl Drop for BoxLockGuard {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            if let Ok(mut guard) = self.lock.lock() {
                *guard = None; // This will drop the BoxLock
            }
        }
    }
}

// Add required traits for File
use std::os::unix::io::AsRawFd;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lock_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RustboxLockManager::with_config(
            temp_dir.path().to_path_buf(),
            Duration::from_millis(100),
            Duration::from_secs(5)
        ).unwrap();

        let health = manager.health_check();
        assert!(matches!(health.status, HealthStatus::Healthy | HealthStatus::Degraded));
    }

    #[test] 
    fn test_lock_acquisition_and_release() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RustboxLockManager::with_config(
            temp_dir.path().to_path_buf(),
            Duration::from_millis(100), 
            Duration::from_secs(5)
        ).unwrap();

        // Acquire lock
        let guard = manager.acquire_lock(42, Duration::from_secs(1)).unwrap();
        assert_eq!(guard.box_id(), Some(42));

        // Try to acquire same lock (should fail)
        let result = manager.acquire_lock(42, Duration::from_millis(100));
        assert!(matches!(result, Err(LockError::Busy { .. }) | Err(LockError::Timeout { .. })));

        // Release and try again
        drop(guard);
        std::thread::sleep(Duration::from_millis(50)); // Allow cleanup

        let guard2 = manager.acquire_lock(42, Duration::from_secs(1)).unwrap();
        assert_eq!(guard2.box_id(), Some(42));
    }

    #[test]
    fn test_heartbeat_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let manager = RustboxLockManager::with_config(
            temp_dir.path().to_path_buf(),
            Duration::from_millis(50), // Fast heartbeat for testing
            Duration::from_millis(200) // Short stale timeout
        ).unwrap();

        let _guard = manager.acquire_lock(123, Duration::from_secs(1)).unwrap();
        
        // Check that heartbeat file exists and gets updated
        let heartbeat_path = temp_dir.path().join("box-123.heartbeat");
        std::thread::sleep(Duration::from_millis(100));
        assert!(heartbeat_path.exists());

        // Read heartbeat file
        let content = std::fs::read_to_string(&heartbeat_path).unwrap();
        let timestamp1: u64 = content.trim().parse().unwrap();

        // Wait and check it updates
        std::thread::sleep(Duration::from_millis(100));
        let content2 = std::fs::read_to_string(&heartbeat_path).unwrap();
        let timestamp2: u64 = content2.trim().parse().unwrap();

        assert!(timestamp2 > timestamp1, "Heartbeat should update");
    }
}