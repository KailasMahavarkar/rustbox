/// Native seccomp implementation using raw syscalls (no libseccomp dependency)
/// This provides basic syscall filtering for production environments
use crate::types::{IsolateError, Result};
use std::collections::HashSet;

#[cfg(target_os = "linux")]
use std::os::raw::{c_int, c_ulong};

/// Native seccomp filter using raw syscalls
pub struct NativeSeccompFilter {
    allowed_syscalls: HashSet<String>,
}

impl NativeSeccompFilter {
    /// Create a new native seccomp filter for anonymous code execution
    pub fn new_for_anonymous_code() -> Self {
        let mut allowed_syscalls = HashSet::new();
        
        // Essential syscalls for basic program execution
        let essential = [
            // Process control
            "exit", "exit_group", "getpid", "getppid",
            
            // Memory management  
            "brk", "mmap", "munmap", "mprotect",
            
            // File I/O (limited set)
            "read", "write", "close", "fstat", "lseek",
            "open", "openat", "access", "faccessat",
            
            // Time and scheduling
            "nanosleep", "clock_gettime", "gettimeofday",
            
            // Signal handling (basic)
            "rt_sigaction", "rt_sigprocmask", "rt_sigreturn",
            
            // Basic system info (safe)
            "getuid", "getgid", "geteuid", "getegid",
            "arch_prctl", "getrlimit",
        ];
        
        for syscall in &essential {
            allowed_syscalls.insert(syscall.to_string());
        }
        
        Self { allowed_syscalls }
    }
    
    /// Apply basic seccomp filtering using kernel APIs directly
    pub fn apply_basic_protection(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.apply_linux_seccomp()
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            log::warn!("Native seccomp only supported on Linux");
            Ok(())
        }
    }
    
    #[cfg(target_os = "linux")]
    fn apply_linux_seccomp(&self) -> Result<()> {
        // Use prctl to enable seccomp mode 1 (strict mode)
        // This blocks all syscalls except read, write, exit, and sigreturn
        // It's more restrictive than our custom filter but provides strong protection
        
        const PR_SET_SECCOMP: c_int = 22;
        const SECCOMP_MODE_STRICT: c_ulong = 1;
        
        unsafe {
            let result = libc::prctl(PR_SET_SECCOMP, SECCOMP_MODE_STRICT, 0, 0, 0);
            if result == 0 {
                log::info!("Native seccomp strict mode enabled successfully");
                Ok(())
            } else {
                Err(IsolateError::Config(
                    "Failed to enable seccomp strict mode".to_string()
                ))
            }
        }
    }
    
    /// Apply no-new-privs restriction (prevents privilege escalation)
    pub fn apply_no_new_privs(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            const PR_SET_NO_NEW_PRIVS: c_int = 38;
            
            unsafe {
                let result = libc::prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0);
                if result == 0 {
                    log::info!("No-new-privs protection enabled");
                    Ok(())
                } else {
                    Err(IsolateError::Config(
                        "Failed to enable no-new-privs protection".to_string()
                    ))
                }
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            log::warn!("No-new-privs only supported on Linux");
            Ok(())
        }
    }
    
    /// Check if native seccomp is supported
    pub fn is_supported() -> bool {
        #[cfg(target_os = "linux")]
        {
            // Check if seccomp is available by testing prctl
            use std::fs;
            
            // Check for seccomp in /proc/sys/kernel/seccomp/actions_avail
            if let Ok(actions) = fs::read_to_string("/proc/sys/kernel/seccomp/actions_avail") {
                actions.contains("kill_process") || actions.contains("kill")
            } else {
                false
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}

/// Test helper function to verify seccomp is working
pub fn test_seccomp_blocking() -> bool {
    #[cfg(target_os = "linux")]
    {
        // Fork a child process to test seccomp without affecting parent
        match unsafe { libc::fork() } {
            0 => {
                // Child process - apply seccomp and try dangerous syscall
                let filter = NativeSeccompFilter::new_for_anonymous_code();
                if filter.apply_basic_protection().is_ok() {
                    // Try to create a socket (should be blocked)
                    unsafe {
                        let result = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
                        if result >= 0 {
                            // Socket creation succeeded - seccomp not working
                            libc::_exit(1);
                        } else {
                            // Socket creation failed - seccomp working
                            libc::_exit(0);
                        }
                    }
                } else {
                    unsafe { libc::_exit(1); }
                }
            }
            child_pid if child_pid > 0 => {
                // Parent process - wait for child
                let mut status = 0;
                unsafe {
                    libc::waitpid(child_pid, &mut status, 0);
                }
                
                // Check if child was killed by seccomp or exited successfully
                let exit_code = libc::WEXITSTATUS(status);
                let signaled = libc::WIFSIGNALED(status);
                
                // Success if child was killed by signal (seccomp) or exited with 0
                signaled || exit_code == 0
            }
            _ => {
                // Fork failed or other error
                false
            }
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_native_seccomp_creation() {
        let filter = NativeSeccompFilter::new_for_anonymous_code();
        // Should create without error
        assert!(!filter.allowed_syscalls.is_empty());
    }
    
    #[test]
    fn test_native_seccomp_support_detection() {
        // This should work on most Linux systems
        let supported = NativeSeccompFilter::is_supported();
        println!("Native seccomp supported: {}", supported);
        // Don't assert since it depends on the system
    }
    
    #[test] 
    #[ignore] // Marked as ignore since it modifies process state
    fn test_no_new_privs() {
        let filter = NativeSeccompFilter::new_for_anonymous_code();
        // This should succeed on Linux
        let result = filter.apply_no_new_privs();
        #[cfg(target_os = "linux")]
        assert!(result.is_ok());
    }
}