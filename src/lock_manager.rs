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
        let flock_result = unsafe { flock(lock_file.as_raw_fd(), LOCK_EX | LOCK_NB) };
        if flock_result != 0 {
            let errno = std::io::Error::last_os_error();
            return match errno.raw_os_error() {
                Some(libc::EWOULDBLOCK) => Err(IsolateError::LockBusy),
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

            if magic != LOCK_MAGIC {
                return Err(IsolateError::LockCorrupted);
            }

            // Check ownership (allow root to take over)
            let current_uid = unsafe { libc::getuid() };
            if is_initialized && owner_uid != current_uid && current_uid != 0 {
                return Err(IsolateError::Lock(format!(
                    "Box {} is owned by uid {}, current uid is {}",
                    self.box_id, owner_uid, current_uid
                )));
            }

            LockRecord {
                magic,
                owner_uid,
                cg_enabled,
                is_initialized,
            }
        } else if is_init {
            // Create new lock record for initialization 
            LockRecord {
                magic: LOCK_MAGIC,
                owner_uid: unsafe { libc::getuid() },
                cg_enabled: true,  // Enable cgroups by default
                is_initialized: false,  // Will be set after successful init
            }
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
            self.write_lock_record(&mut lock_file, &updated_record)?;
            self.lock_record = Some(updated_record);
        } else {
            // For non-init operations, verify box is initialized
            if !lock_record.is_initialized {
                return Err(IsolateError::Lock(format!(
                    "Box {} is not properly initialized",
                    self.box_id
                )));
            }
            self.lock_record = Some(lock_record);
        }

        self.lock_file = Some(lock_file);
        Ok(())
    }

    /// Write lock record to file in binary format (isolate-reference compatible)
    fn write_lock_record(&self, lock_file: &mut File, record: &LockRecord) -> Result<()> {
        let mut buffer = [0u8; std::mem::size_of::<LockRecord>()];
        
        // Pack the structure in little endian format
        let magic_bytes = record.magic.to_le_bytes();
        let uid_bytes = record.owner_uid.to_le_bytes();
        
        buffer[0..4].copy_from_slice(&magic_bytes);
        buffer[4..8].copy_from_slice(&uid_bytes);
        buffer[8] = if record.cg_enabled { 1 } else { 0 };
        buffer[9] = if record.is_initialized { 1 } else { 0 };
        // buffer[10], buffer[11] remain 0 (reserved fields)

        lock_file.seek(SeekFrom::Start(0))?;
        lock_file.set_len(buffer.len() as u64)?;
        lock_file.write_all(&buffer)?;
        lock_file.flush()?;
        
        Ok(())
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
        assert!(matches!(
            lock_manager2.acquire_lock(false),
            Err(IsolateError::LockBusy)
        ));

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
        }
    }
}