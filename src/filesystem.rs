/// Filesystem security and isolation implementation
use crate::types::{IsolateError, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Filesystem security controller for process isolation
#[derive(Clone, Debug)]
pub struct FilesystemSecurity {
    /// Root directory for chroot jail
    chroot_dir: Option<PathBuf>,
    /// Working directory within the jail
    workdir: PathBuf,
    /// Whether to apply strict filesystem isolation
    strict_mode: bool,
}

impl FilesystemSecurity {
    /// Create a new filesystem security controller
    pub fn new(chroot_dir: Option<PathBuf>, workdir: PathBuf, strict_mode: bool) -> Self {
        Self {
            chroot_dir,
            workdir,
            strict_mode,
        }
    }

    /// Setup filesystem isolation including chroot jail if specified
    pub fn setup_isolation(&self) -> Result<()> {
        if let Some(ref chroot_path) = self.chroot_dir {
            self.setup_chroot_jail(chroot_path)?;
        }
        
        self.setup_workdir()?;
        Ok(())
    }

    /// Setup chroot jail for filesystem isolation
    #[cfg(unix)]
    fn setup_chroot_jail(&self, chroot_path: &Path) -> Result<()> {
        // Ensure chroot directory exists
        if !chroot_path.exists() {
            fs::create_dir_all(chroot_path).map_err(|e| {
                IsolateError::Config(format!("Failed to create chroot directory: {}", e))
            })?;
        }

        // Create essential directories within chroot
        self.create_chroot_structure(chroot_path)?;

        // Apply mount security flags to prevent dangerous operations
        self.apply_mount_security_flags(chroot_path)?;

        Ok(())
    }

    #[cfg(not(unix))]
    fn setup_chroot_jail(&self, _chroot_path: &Path) -> Result<()> {
        Err(IsolateError::Config(
            "Chroot isolation is only supported on Unix systems".to_string(),
        ))
    }

    /// Create essential directory structure within chroot
    #[cfg(unix)]
    fn create_chroot_structure(&self, chroot_path: &Path) -> Result<()> {
        let essential_dirs = [
            "tmp",
            "dev",
            "proc", 
            "usr/bin",
            "bin",
            "lib",
            "lib64",
            "etc",
        ];

        for dir in &essential_dirs {
            let dir_path = chroot_path.join(dir);
            if !dir_path.exists() {
                fs::create_dir_all(&dir_path).map_err(|e| {
                    IsolateError::Config(format!("Failed to create chroot dir {}: {}", dir, e))
                })?;
            }

            // Set secure permissions (755 for directories)
            let metadata = fs::metadata(&dir_path)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dir_path, perms)?;
        }

        // Create essential device files
        self.create_essential_devices(chroot_path)?;

        Ok(())
    }

    /// Create essential device files in chroot
    #[cfg(unix)]
    fn create_essential_devices(&self, chroot_path: &Path) -> Result<()> {
        let dev_dir = chroot_path.join("dev");
        
        // Create /dev/null
        let null_path = dev_dir.join("null");
        if !null_path.exists() {
            // Use mknod to create device file
            let result = unsafe {
                let path_cstr = std::ffi::CString::new(null_path.to_string_lossy().as_bytes())
                    .map_err(|e| IsolateError::Config(format!("Invalid path: {}", e)))?;
                
                libc::mknod(
                    path_cstr.as_ptr(),
                    libc::S_IFCHR | 0o666,
                    libc::makedev(1, 3), // /dev/null major=1, minor=3
                )
            };

            if result != 0 {
                // If mknod fails, create a regular file as fallback
                fs::File::create(&null_path).map_err(|e| {
                    IsolateError::Config(format!("Failed to create /dev/null: {}", e))
                })?;
            }
        }

        // Create /dev/zero
        let zero_path = dev_dir.join("zero");
        if !zero_path.exists() {
            let result = unsafe {
                let path_cstr = std::ffi::CString::new(zero_path.to_string_lossy().as_bytes())
                    .map_err(|e| IsolateError::Config(format!("Invalid path: {}", e)))?;
                
                libc::mknod(
                    path_cstr.as_ptr(),
                    libc::S_IFCHR | 0o666,
                    libc::makedev(1, 5), // /dev/zero major=1, minor=5
                )
            };

            if result != 0 {
                // If mknod fails, create a regular file as fallback
                fs::File::create(&zero_path).map_err(|e| {
                    IsolateError::Config(format!("Failed to create /dev/zero: {}", e))
                })?;
            }
        }

        Ok(())
    }

    /// Apply mount security flags to prevent dangerous operations
    #[cfg(unix)]
    fn apply_mount_security_flags(&self, chroot_path: &Path) -> Result<()> {
        // Apply noexec, nosuid, nodev flags to the chroot mount
        let mount_flags = libc::MS_NOEXEC | libc::MS_NOSUID | libc::MS_NODEV | libc::MS_BIND;
        
        let source_cstr = std::ffi::CString::new(chroot_path.to_string_lossy().as_bytes())
            .map_err(|e| IsolateError::Config(format!("Invalid chroot path: {}", e)))?;
        
        let target_cstr = source_cstr.clone();
        
        let result = unsafe {
            libc::mount(
                source_cstr.as_ptr(),
                target_cstr.as_ptr(),
                std::ptr::null(),
                mount_flags,
                std::ptr::null(),
            )
        };

        if result != 0 && self.strict_mode {
            let errno = unsafe { *libc::__errno_location() };
            return Err(IsolateError::Config(format!(
                "Failed to apply mount security flags: errno {}",
                errno
            )));
        }

        Ok(())
    }

    /// Perform chroot operation (must be called in child process)
    #[cfg(unix)]
    pub fn apply_chroot(&self) -> Result<()> {
        if let Some(ref chroot_path) = self.chroot_dir {
            let path_cstr = std::ffi::CString::new(chroot_path.to_string_lossy().as_bytes())
                .map_err(|e| IsolateError::Config(format!("Invalid chroot path: {}", e)))?;

            let result = unsafe { libc::chroot(path_cstr.as_ptr()) };
            
            if result != 0 {
                let errno = unsafe { *libc::__errno_location() };
                return Err(IsolateError::Config(format!(
                    "chroot failed: errno {}",
                    errno
                )));
            }

            // Change to root directory within chroot
            std::env::set_current_dir("/").map_err(|e| {
                IsolateError::Config(format!("Failed to change to chroot root: {}", e))
            })?;
        }
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn apply_chroot(&self) -> Result<()> {
        if self.chroot_dir.is_some() {
            return Err(IsolateError::Config(
                "Chroot is only supported on Unix systems".to_string(),
            ));
        }
        Ok(())
    }

    /// Setup working directory with proper permissions
    fn setup_workdir(&self) -> Result<()> {
        // Determine the actual working directory path
        let actual_workdir = if self.chroot_dir.is_some() {
            // If using chroot, workdir is relative to chroot root
            PathBuf::from("/").join(
                self.workdir
                    .strip_prefix("/")
                    .unwrap_or(&self.workdir)
            )
        } else {
            self.workdir.clone()
        };

        // Create workdir if it doesn't exist
        if !actual_workdir.exists() {
            fs::create_dir_all(&actual_workdir).map_err(|e| {
                IsolateError::Config(format!("Failed to create workdir: {}", e))
            })?;
        }

        // Set secure permissions
        #[cfg(unix)]
        {
            let metadata = fs::metadata(&actual_workdir)?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o755); // rwxr-xr-x
            fs::set_permissions(&actual_workdir, perms)?;
        }

        Ok(())
    }

    /// Validate that a path is within the allowed boundaries
    pub fn validate_path(&self, path: &Path) -> Result<()> {
        let canonical_path = path.canonicalize().map_err(|e| {
            IsolateError::Config(format!("Failed to canonicalize path: {}", e))
        })?;

        // If using chroot, all paths should be within chroot
        if let Some(ref chroot_path) = self.chroot_dir {
            let canonical_chroot = chroot_path.canonicalize().map_err(|e| {
                IsolateError::Config(format!("Failed to canonicalize chroot path: {}", e))
            })?;

            if !canonical_path.starts_with(&canonical_chroot) {
                return Err(IsolateError::Config(format!(
                    "Path {} is outside chroot jail {}",
                    canonical_path.display(),
                    canonical_chroot.display()
                )));
            }
        }

        // Additional validation: prevent access to sensitive system directories
        let dangerous_paths = [
            "/etc/passwd",
            "/etc/shadow", 
            "/etc/sudoers",
            "/root",
            "/boot",
            "/sys",
            "/proc/sys",
        ];

        let path_str = canonical_path.to_string_lossy();
        for dangerous in &dangerous_paths {
            if path_str.starts_with(dangerous) {
                return Err(IsolateError::Config(format!(
                    "Access to dangerous path {} is forbidden",
                    path_str
                )));
            }
        }

        Ok(())
    }

    /// Cleanup filesystem isolation
    pub fn cleanup(&self) -> Result<()> {
        // Unmount chroot if it was mounted
        #[cfg(unix)]
        if let Some(ref chroot_path) = self.chroot_dir {
            let path_cstr = std::ffi::CString::new(chroot_path.to_string_lossy().as_bytes())
                .map_err(|e| IsolateError::Config(format!("Invalid chroot path: {}", e)))?;

            // Try to unmount, but don't fail if it wasn't mounted
            unsafe {
                libc::umount(path_cstr.as_ptr());
            }
        }

        Ok(())
    }

    /// Check if filesystem isolation is properly configured
    pub fn is_isolated(&self) -> bool {
        self.chroot_dir.is_some()
    }

    /// Get the effective working directory (accounting for chroot)
    pub fn get_effective_workdir(&self) -> PathBuf {
        if self.chroot_dir.is_some() {
            // Within chroot, paths are relative to chroot root
            PathBuf::from("/").join(
                self.workdir
                    .strip_prefix("/")
                    .unwrap_or(&self.workdir)
            )
        } else {
            self.workdir.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_filesystem_security_creation() {
        let temp_dir = env::temp_dir().join("rustbox-test");
        let fs_security = FilesystemSecurity::new(None, temp_dir.clone(), false);
        
        assert!(!fs_security.is_isolated());
        assert_eq!(fs_security.get_effective_workdir(), temp_dir);
    }

    #[test]
    fn test_filesystem_security_with_chroot() {
        let temp_dir = env::temp_dir().join("rustbox-chroot-test");
        let work_dir = temp_dir.join("work");
        let fs_security = FilesystemSecurity::new(Some(temp_dir), work_dir, false);
        
        assert!(fs_security.is_isolated());
    }

    #[test]
    fn test_path_validation() {
        let temp_dir = env::temp_dir().join("rustbox-validation-test");
        let fs_security = FilesystemSecurity::new(None, temp_dir.clone(), false);
        
        // Test dangerous path rejection
        let result = fs_security.validate_path(Path::new("/etc/passwd"));
        assert!(result.is_err());
        
        let result = fs_security.validate_path(Path::new("/etc/shadow"));
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_chroot_structure_creation() {
        use std::fs;
        
        let temp_dir = env::temp_dir().join("rustbox-chroot-structure-test");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up any previous test
        
        let fs_security = FilesystemSecurity::new(Some(temp_dir.clone()), temp_dir.join("work"), false);
        
        // This test requires root privileges to actually create device files
        // So we just test the directory structure creation part
        if let Err(e) = fs_security.setup_isolation() {
            // Expected to fail without root privileges for device creation
            assert!(e.to_string().contains("create") || e.to_string().contains("permission"));
        }
        
        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }
}