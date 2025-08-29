/// Enhanced multi-user lock management for rustbox
/// Based on isolate-reference lock file format for compatibility

use crate::types::{IsolateError, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

/// Lock record structure compatible with isolate-reference
#[derive(Debug, Clone)]
pub struct LockRecord {
    /// Magic number for file format validation (0x48736f6c = "Hsol")
    pub magic: u32,
    /// UID of the box owner  
    pub owner_uid: u32,
    /// Whether cgroups are enabled
    pub cg_enabled: bool,
    /// Whether the box is properly initialized
    pub is_initialized: bool,
    /// PID of the process holding the lock (for orphan detection)
    pub holder_pid: u32,
    /// Timestamp when lock was acquired (for timeout detection)
    pub timestamp: u64,
}

/// Default lock directory (can be configured)
const DEFAULT_LOCK_ROOT: &str = "/tmp/rustbox-locks";

/// Magic number for lock files (same as isolate-reference)
const LOCK_MAGIC: u32 = 0x48736f6c;

/// File lock operations
const LOCK_EX: i32 = 2;   // Exclusive lock
const LOCK_NB: i32 = 4;   // Non-blocking

extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

/// Enhanced lock manager for box ID-based multi-user safety
pub struct BoxLockManager {
    lock_root: PathBuf,
    box_id: u32,
    lock_file: Option<File>,
    lock_record: Option<LockRecord>,
}

impl BoxLockManager {
    /// Create new lock manager for a specific box ID
    pub fn new(box_id: u32) -> Self {
        Self {
            lock_root: PathBuf::from(DEFAULT_LOCK_ROOT),
            box_id,
            lock_file: None,
            lock_record: None,
        }
    }

    /// Create new lock manager with custom lock directory
    pub fn with_lock_root<P: Into<PathBuf>>(box_id: u32, lock_root: P) -> Self {
        Self {
            lock_root: lock_root.into(),
            box_id,
            lock_file: None,
            lock_record: None,
        }
    }

    /// Acquire exclusive lock for box (init=true for initialization)
    pub fn acquire_lock(&mut self, is_init: bool) -> Result<()> {
        // Create lock directory if it doesn't exist
        if !self.lock_root.exists() {
            std::fs::create_dir_all(&self.lock_root)?;
        }

        let lock_path = self.lock_root.join(self.box_id.to_string());
        
        // First attempt: Clean up any potentially orphaned locks
        self.cleanup_orphaned_locks()?;

        // Open lock file (create only for init operations)
        let mut lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(is_init)
            .open(&lock_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    IsolateError::Lock(format!("Box {} not initialized", self.box_id))
                } else {
                    IsolateError::Io(e)
                }
            })?;

        // Acquire exclusive lock (non-blocking to detect busy state)
        #[cfg(unix)]
        let flock_result = unsafe { flock(lock_file.as_raw_fd(), LOCK_EX | LOCK_NB) };
        #[cfg(not(unix))]
        let flock_result = 0; // Always succeed on non-Unix systems
        
        if flock_result != 0 {
            let errno = std::io::Error::last_os_error();
            return match errno.raw_os_error() {
                Some(libc::EWOULDBLOCK) | Some(libc::EAGAIN) => {
                    Err(IsolateError::Lock(format!(
                        "Box {} is currently in use by another process", 
                        self.box_id
                    )))
                },
                _ => Err(IsolateError::Lock(format!(
                    "Cannot acquire lock for box {}: {}",
                    self.box_id, errno
                ))),
            };
        }

        // Read and validate lock record
        let mut buffer = [0u8; std::mem::size_of::<LockRecord>()];
        let bytes_read = lock_file.read(&mut buffer)?;

        let lock_record = if bytes_read == buffer.len() {
            // Parse existing lock record
            let magic = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
            let owner_uid = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
            let cg_enabled = buffer[8] != 0;
            let is_initialized = buffer[9] != 0;
            let holder_pid = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
            let timestamp = u64::from_le_bytes([
                buffer[16], buffer[17], buffer[18], buffer[19],
                buffer[20], buffer[21], buffer[22], buffer[23]
            ]);

            if magic != LOCK_MAGIC {
                return Err(IsolateError::Lock(format!(
                    "Box {} has corrupted lock file (invalid magic)", 
                    self.box_id
                )));
            }

            // Check if lock holder process is still alive
            if holder_pid != 0 && !Self::is_process_alive(holder_pid) {
                eprintln!("Detected orphaned lock for box {} (dead PID {}), cleaning up", 
                         self.box_id, holder_pid);
                // Clear the lock file to allow reacquisition
                lock_file.set_len(0)?;
                lock_file.flush()?;
                lock_file.seek(SeekFrom::Start(0))?;
                
                // Retry with fresh acquisition
                return self.create_new_lock_record(is_init, lock_file);
            }

            // Multi-user access control: Check ownership
            let current_uid = unsafe { libc::getuid() };
            
            // For initialized boxes, enforce ownership unless user is root
            if is_initialized && owner_uid != current_uid && current_uid != 0 {
                return Err(IsolateError::Lock(format!(
                    "Access denied: Box {} is owned by user {} (uid: {}), you are user {} (uid: {}). Only the owner or root can access this box.",
                    self.box_id, 
                    Self::get_username_from_uid(owner_uid).unwrap_or_else(|| "unknown".to_string()),
                    owner_uid,
                    Self::get_username_from_uid(current_uid).unwrap_or_else(|| "unknown".to_string()),
                    current_uid
                )));
            }

            // For init operations on existing boxes, verify ownership
            if is_init && is_initialized && owner_uid != current_uid && current_uid != 0 {
                return Err(IsolateError::Lock(format!(
                    "Cannot reinitialize box {}: already owned by uid {}", 
                    self.box_id, owner_uid
                )));
            }

            LockRecord {
                magic,
                owner_uid,
                cg_enabled,
                is_initialized,
                holder_pid,
                timestamp,
            }
        } else if is_init {
            return self.create_new_lock_record(is_init, lock_file);
        } else {
            return Err(IsolateError::Lock(format!(
                "Box {} has corrupted or empty lock file",
                self.box_id
            )));
        };

        // For init operations, mark as initialized and write the record
        if is_init {
            let mut updated_record = lock_record.clone();
            updated_record.is_initialized = true;
            updated_record.holder_pid = std::process::id();
            updated_record.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.write_lock_record(&mut lock_file, &updated_record)?;
            self.lock_record = Some(updated_record);
        } else {
            // For non-init operations, verify box is initialized
            if !lock_record.is_initialized {
                return Err(IsolateError::Lock(format!(
                    "Box {} is not properly initialized. Please run init first.",
                    self.box_id
                )));
            }
            
            // Update holder PID to current process
            let mut updated_record = lock_record.clone();
            updated_record.holder_pid = std::process::id();
            updated_record.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.write_lock_record(&mut lock_file, &updated_record)?;
            self.lock_record = Some(updated_record);
        }

        self.lock_file = Some(lock_file);
        Ok(())
    }

    /// Create new lock record with current process info
    fn create_new_lock_record(&mut self, is_init: bool, mut lock_file: File) -> Result<()> {
        let record = LockRecord {
            magic: LOCK_MAGIC,
            owner_uid: unsafe { libc::getuid() },
            cg_enabled: true,
            is_initialized: is_init,
            holder_pid: std::process::id(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.write_lock_record(&mut lock_file, &record)?;
        self.lock_record = Some(record);
        self.lock_file = Some(lock_file);
        Ok(())
    }

    /// Write lock record to file in binary format (isolate-reference compatible)
    fn write_lock_record(&self, lock_file: &mut File, record: &LockRecord) -> Result<()> {
        let mut buffer = [0u8; std::mem::size_of::<LockRecord>()];
        
        // Pack the structure in little endian format
        let magic_bytes = record.magic.to_le_bytes();
        let uid_bytes = record.owner_uid.to_le_bytes();
        let pid_bytes = record.holder_pid.to_le_bytes();
        let timestamp_bytes = record.timestamp.to_le_bytes();
        
        buffer[0..4].copy_from_slice(&magic_bytes);
        buffer[4..8].copy_from_slice(&uid_bytes);
        buffer[8] = if record.cg_enabled { 1 } else { 0 };
        buffer[9] = if record.is_initialized { 1 } else { 0 };
        // buffer[10], buffer[11] remain 0 (reserved fields)
        buffer[12..16].copy_from_slice(&pid_bytes);
        buffer[16..24].copy_from_slice(&timestamp_bytes);

        lock_file.seek(SeekFrom::Start(0))?;
        lock_file.set_len(buffer.len() as u64)?;
        lock_file.write_all(&buffer)?;
        lock_file.flush()?;
        
        Ok(())
    }

    /// Clean up orphaned locks for this specific box
    fn cleanup_orphaned_locks(&self) -> Result<()> {
        let lock_path = self.lock_root.join(self.box_id.to_string());
        
        if !lock_path.exists() {
            return Ok(()); // No lock file to clean
        }

        // Try to read the lock file without acquiring flock
        match std::fs::File::open(&lock_path) {
            Ok(mut file) => {
                let mut buffer = [0u8; std::mem::size_of::<LockRecord>()];
                if let Ok(bytes_read) = file.read(&mut buffer) {
                    if bytes_read == buffer.len() {
                        let holder_pid = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
                        
                        // Check if the holder process is dead
                        if holder_pid != 0 && !Self::is_process_alive(holder_pid) {
                            eprintln!("Cleaning up orphaned lock for box {} (dead PID {})", 
                                     self.box_id, holder_pid);
                            
                            // Try to acquire the lock to verify it's actually orphaned
                            #[cfg(unix)]
                            let flock_result = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
                            #[cfg(not(unix))]
                            let flock_result = 0;
                            
                            if flock_result == 0 {
                                // We successfully acquired the lock, meaning it was orphaned
                                eprintln!("Confirmed orphaned lock for box {} - lock file will be recreated", self.box_id);
                                // Truncate the file to clear the old lock record
                                if let Err(e) = file.set_len(0) {
                                    eprintln!("Warning: Failed to truncate orphaned lock file: {}", e);
                                }
                                if let Err(e) = file.flush() {
                                    eprintln!("Warning: Failed to flush orphaned lock file: {}", e);
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {} // File doesn't exist or can't be read
        }
        
        Ok(())
    }
    
    /// Check if a process is still alive (enhanced with multiple methods)
    fn is_process_alive(pid: u32) -> bool {
        // Method 1: Use kill(0) to check process existence
        let kill_result = unsafe { libc::kill(pid as i32, 0) };
        
        if kill_result == 0 {
            return true; // Process exists and we can signal it
        }

        // If kill failed, check errno to distinguish between "no such process" and "permission denied"
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        if errno == libc::EPERM {
            // Process exists but we don't have permission to signal it
            return true;
        }
        
        // Method 2: Check /proc filesystem as fallback
        let proc_path = format!("/proc/{}", pid);
        std::path::Path::new(&proc_path).exists()
    }

    /// Get username from UID for better error messages
    fn get_username_from_uid(uid: u32) -> Option<String> {
        use std::ffi::CStr;
        use std::ptr;
        
        unsafe {
            let passwd = libc::getpwuid(uid);
            if passwd.is_null() {
                return None;
            }
            
            let name_ptr = (*passwd).pw_name;
            if name_ptr.is_null() {
                return None;
            }
            
            CStr::from_ptr(name_ptr)
                .to_str()
                .ok()
                .map(|s| s.to_string())
        }
    }

    /// Remove lock file (cleanup operation)
    pub fn remove_lock(&mut self) -> Result<()> {
        if let Some(mut lock_file) = self.lock_file.take() {
            // Following isolate-reference: truncate instead of unlink to avoid races
            lock_file.set_len(0)?;
            lock_file.flush()?;
        }
        
        self.lock_record = None;
        Ok(())
    }

    /// Get current lock record
    pub fn lock_record(&self) -> Option<&LockRecord> {
        self.lock_record.as_ref()
    }

    /// Check if box is currently locked by this instance
    pub fn is_locked(&self) -> bool {
        self.lock_file.is_some()
    }

    /// Get box ID
    pub fn box_id(&self) -> u32 {
        self.box_id
    }

    /// Get owner UID from lock record
    pub fn owner_uid(&self) -> Option<u32> {
        self.lock_record.as_ref().map(|r| r.owner_uid)
    }

    /// Check if box is properly initialized
    pub fn is_initialized(&self) -> bool {
        self.lock_record
            .as_ref()
            .map(|r| r.is_initialized)
            .unwrap_or(false)
    }

    /// Get all active box IDs (for listing boxes)
    pub fn list_active_boxes(lock_root: Option<&PathBuf>) -> Result<Vec<u32>> {
        let lock_dir = lock_root
            .cloned()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_LOCK_ROOT));

        if !lock_dir.exists() {
            return Ok(Vec::new());
        }

        let mut box_ids = Vec::new();
        
        for entry in std::fs::read_dir(lock_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            if let Some(name_str) = file_name.to_str() {
                if let Ok(box_id) = name_str.parse::<u32>() {
                    // Check if lock file is valid and non-empty
                    if entry.metadata()?.len() == std::mem::size_of::<LockRecord>() as u64 {
                        box_ids.push(box_id);
                    }
                }
            }
        }

        box_ids.sort();
        Ok(box_ids)
    }

    /// Force cleanup of a specific box (admin function)
    pub fn force_cleanup_box(box_id: u32, lock_root: Option<&PathBuf>) -> Result<bool> {
        let lock_dir = lock_root
            .cloned()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_LOCK_ROOT));
        
        let lock_path = lock_dir.join(box_id.to_string());
        
        if !lock_path.exists() {
            return Ok(false); // No lock to clean
        }

        // Only root can force cleanup
        let current_uid = unsafe { libc::getuid() };
        if current_uid != 0 {
            return Err(IsolateError::Lock(
                "Force cleanup requires root privileges".to_string()
            ));
        }

        // Remove the lock file entirely
        std::fs::remove_file(lock_path)?;
        println!("Force removed lock for box {}", box_id);
        Ok(true)
    }
}

impl Drop for BoxLockManager {
    fn drop(&mut self) {
        // Lock is automatically released when file descriptor closes
        self.lock_file = None;
        self.lock_record = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_box_lock_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut lock_manager = BoxLockManager::with_lock_root(42, temp_dir.path());

        // Should successfully acquire lock for init
        assert!(lock_manager.acquire_lock(true).is_ok());
        assert!(lock_manager.is_locked());
        assert!(lock_manager.is_initialized());
        assert_eq!(lock_manager.box_id(), 42);

        // Should have created the lock file
        let lock_path = temp_dir.path().join("42");
        assert!(lock_path.exists());
    }

    #[test]  
    fn test_box_lock_ownership() {
        let temp_dir = TempDir::new().unwrap();
        let mut lock_manager1 = BoxLockManager::with_lock_root(123, temp_dir.path());
        let mut lock_manager2 = BoxLockManager::with_lock_root(123, temp_dir.path());

        // First manager should acquire lock successfully
        assert!(lock_manager1.acquire_lock(true).is_ok());

        // Second manager should fail to acquire the same lock (busy)
        let result = lock_manager2.acquire_lock(false);
        assert!(result.is_err());
        
        // After dropping first lock, second should succeed for non-init
        drop(lock_manager1);
        assert!(lock_manager2.acquire_lock(false).is_ok());
    }

    #[test]
    fn test_list_active_boxes() {
        let temp_dir = TempDir::new().unwrap();

        // Create a few boxes
        let mut lock1 = BoxLockManager::with_lock_root(10, temp_dir.path());
        let mut lock2 = BoxLockManager::with_lock_root(20, temp_dir.path());
        let mut lock3 = BoxLockManager::with_lock_root(5, temp_dir.path());

        lock1.acquire_lock(true).unwrap();
        lock2.acquire_lock(true).unwrap();
        lock3.acquire_lock(true).unwrap();

        let boxes = BoxLockManager::list_active_boxes(Some(&temp_dir.path().to_path_buf())).unwrap();
        assert_eq!(boxes, vec![5, 10, 20]); // Should be sorted
    }

    #[test]
    fn test_lock_record_format() {
        let temp_dir = TempDir::new().unwrap();
        let mut lock_manager = BoxLockManager::with_lock_root(999, temp_dir.path());

        lock_manager.acquire_lock(true).unwrap();
        
        if let Some(record) = lock_manager.lock_record() {
            assert_eq!(record.magic, LOCK_MAGIC);
            assert_eq!(record.owner_uid, unsafe { libc::getuid() });
            assert!(record.is_initialized);
            assert!(record.cg_enabled);
            assert_eq!(record.holder_pid, std::process::id());
            assert!(record.timestamp > 0);
        }
    }

    #[test]
    fn test_multi_user_protection() {
        let temp_dir = TempDir::new().unwrap();
        let mut lock_manager = BoxLockManager::with_lock_root(777, temp_dir.path());

        // Initialize the box
        lock_manager.acquire_lock(true).unwrap();
        
        // Simulate another user trying to access (this is hard to test without actually changing UIDs)
        // In practice, the ownership check would prevent access
        assert!(lock_manager.owner_uid().is_some());
        assert_eq!(lock_manager.owner_uid().unwrap(), unsafe { libc::getuid() });
    }
}