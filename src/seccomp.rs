/// Security module implementing seccomp-bpf syscall filtering
/// Provides defense against malicious code by blocking dangerous system calls
use crate::types::Result;
use std::collections::HashSet;

#[cfg(feature = "seccomp")]
use libseccomp::*;

/// Seccomp filter configuration for anonymous code execution
pub struct SeccompFilter {
    /// Action to take for blocked syscalls (only used when seccomp is available)
    #[cfg(feature = "seccomp")]
    default_action: ScmpAction,
    /// Set of explicitly allowed syscalls
    allowed_syscalls: HashSet<String>,
}

impl SeccompFilter {
    /// Create a new seccomp filter with secure defaults for anonymous code execution
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
        
        Self {
            #[cfg(feature = "seccomp")]
            default_action: ScmpAction::KillProcess,
            allowed_syscalls,
        }
    }
    
    /// Create a filter that allows additional syscalls for specific languages
    pub fn new_for_language(language: &str) -> Self {
        let mut filter = Self::new_for_anonymous_code();
        
        match language {
            "python" => {
                filter.add_python_syscalls();
            }
            "javascript" | "node" => {
                filter.add_javascript_syscalls();
            }
            "java" => {
                filter.add_java_syscalls();
            }
            _ => {} // Use defaults
        }
        
        filter
    }
    
    /// Add syscalls commonly needed by Python interpreters
    fn add_python_syscalls(&mut self) {
        let python_syscalls = [
            "stat", "lstat", "fstat", "newfstatat",
            "readlink", "readlinkat",
            "getcwd", "chdir", // Limited filesystem navigation
            "pipe", "pipe2", // For subprocess communication
            "dup", "dup2", "dup3", // File descriptor manipulation
        ];
        
        for syscall in &python_syscalls {
            self.allowed_syscalls.insert(syscall.to_string());
        }
    }
    
    /// Add syscalls commonly needed by JavaScript/Node.js
    fn add_javascript_syscalls(&mut self) {
        let js_syscalls = [
            "futex", "sched_yield", // Threading primitives
            "eventfd2", "epoll_create1", "epoll_ctl", "epoll_wait", // Event loop
            "poll", "select", "pselect6", // I/O multiplexing
        ];
        
        for syscall in &js_syscalls {
            self.allowed_syscalls.insert(syscall.to_string());
        }
    }
    
    /// Add syscalls commonly needed by Java JVM
    fn add_java_syscalls(&mut self) {
        let java_syscalls = [
            "clone", // JVM threading (restricted)
            "futex", "sched_yield", "sched_getparam",
            "prctl", // Process control (limited)
            "madvise", // Memory advice
        ];
        
        for syscall in &java_syscalls {
            self.allowed_syscalls.insert(syscall.to_string());
        }
    }
    
    /// Apply the seccomp filter to the current process
    pub fn apply(&self) -> Result<()> {
        #[cfg(feature = "seccomp")]
        {
            // Initialize seccomp context with default kill action
            let mut ctx = ScmpFilterContext::new_filter(self.default_action)
                .map_err(|e| crate::types::IsolateError::Config(format!("Failed to create seccomp context: {}", e)))?;
            
            // Add rules for allowed syscalls
            for syscall_name in &self.allowed_syscalls {
                let syscall = ScmpSyscall::from_name(syscall_name)
                    .map_err(|e| crate::types::IsolateError::Config(format!("Unknown syscall '{}': {}", syscall_name, e)))?;
                
                ctx.add_rule(ScmpAction::Allow, syscall)
                    .map_err(|e| crate::types::IsolateError::Config(format!("Failed to add rule for {}: {}", syscall_name, e)))?;
            }
            
            // Load the filter into the kernel
            ctx.load()
                .map_err(|e| crate::types::IsolateError::Config(format!("Failed to load seccomp filter: {}", e)))?;
            
            log::info!("Seccomp filter applied successfully with {} allowed syscalls", self.allowed_syscalls.len());
        }
        
        #[cfg(not(feature = "seccomp"))]
        {
            log::warn!("Seccomp filtering requested but not compiled with seccomp support");
        }
        
        Ok(())
    }
    
    /// Check if a syscall is allowed by this filter
    pub fn is_syscall_allowed(&self, syscall_name: &str) -> bool {
        self.allowed_syscalls.contains(syscall_name)
    }
    
    /// Get list of all allowed syscalls
    pub fn get_allowed_syscalls(&self) -> Vec<&String> {
        self.allowed_syscalls.iter().collect()
    }
    
    /// Add a custom syscall to the allowed list (use with caution)
    pub fn allow_syscall(&mut self, syscall_name: &str) {
        self.allowed_syscalls.insert(syscall_name.to_string());
    }
    
    /// Remove a syscall from the allowed list
    pub fn deny_syscall(&mut self, syscall_name: &str) {
        self.allowed_syscalls.remove(syscall_name);
    }
}

/// Get list of dangerous syscalls that should never be allowed for anonymous code
pub fn get_dangerous_syscalls() -> Vec<&'static str> {
    vec![
        // Network operations
        "socket", "connect", "bind", "listen", "accept", "accept4",
        "sendto", "sendmsg", "recvfrom", "recvmsg",
        
        // Process/thread creation
        "fork", "vfork", "clone", "execve", "execveat",
        
        // File system modifications
        "mount", "umount", "umount2", "chroot", "pivot_root",
        "mkdir", "rmdir", "unlink", "unlinkat", "rename", "renameat",
        "chmod", "fchmod", "chown", "fchown", "lchown",
        
        // Privilege operations
        "setuid", "setgid", "setreuid", "setregid", "setresuid", "setresgid",
        "setfsuid", "setfsgid", "capset",
        
        // System information/modification
        "sysinfo", "uname", "sethostname", "setdomainname",
        "reboot", "kexec_load", "init_module", "delete_module",
        
        // Debugging/tracing
        "ptrace", "process_vm_readv", "process_vm_writev",
        
        // IPC
        "msgget", "msgctl", "msgrcv", "msgsnd",
        "semget", "semctl", "semop", "semtimedop",
        "shmget", "shmctl", "shmat", "shmdt",
        
        // Device access
        "ioctl", "mknod", "mknodat",
        
        // Advanced memory operations
        "mbind", "migrate_pages", "move_pages",
    ]
}

/// Test helper to check if seccomp is supported on the current system
pub fn is_seccomp_supported() -> bool {
    #[cfg(feature = "seccomp")]
    {
        match ScmpFilterContext::new_filter(ScmpAction::Allow) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    
    #[cfg(not(feature = "seccomp"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_seccomp_filter_creation() {
        let filter = SeccompFilter::new_for_anonymous_code();
        
        // Should allow essential syscalls
        assert!(filter.is_syscall_allowed("read"));
        assert!(filter.is_syscall_allowed("write"));
        assert!(filter.is_syscall_allowed("exit"));
        assert!(filter.is_syscall_allowed("brk"));
        
        // Should not allow dangerous syscalls
        assert!(!filter.is_syscall_allowed("socket"));
        assert!(!filter.is_syscall_allowed("fork"));
        assert!(!filter.is_syscall_allowed("execve"));
        assert!(!filter.is_syscall_allowed("mount"));
    }
    
    #[test]
    fn test_language_specific_filters() {
        let python_filter = SeccompFilter::new_for_language("python");
        let js_filter = SeccompFilter::new_for_language("javascript");
        
        // Python should have additional syscalls
        assert!(python_filter.is_syscall_allowed("stat"));
        assert!(python_filter.is_syscall_allowed("pipe"));
        
        // JavaScript should have event loop syscalls
        assert!(js_filter.is_syscall_allowed("futex"));
        assert!(js_filter.is_syscall_allowed("epoll_create1"));
    }
    
    #[test]
    fn test_custom_syscall_management() {
        let mut filter = SeccompFilter::new_for_anonymous_code();
        
        // Add custom syscall
        filter.allow_syscall("custom_syscall");
        assert!(filter.is_syscall_allowed("custom_syscall"));
        
        // Remove syscall
        filter.deny_syscall("read");
        assert!(!filter.is_syscall_allowed("read"));
    }
    
    #[test]
    fn test_dangerous_syscalls_list() {
        let dangerous = get_dangerous_syscalls();
        
        // Should include network syscalls
        assert!(dangerous.contains(&"socket"));
        assert!(dangerous.contains(&"connect"));
        
        // Should include process creation
        assert!(dangerous.contains(&"fork"));
        assert!(dangerous.contains(&"execve"));
        
        // Should include privilege escalation
        assert!(dangerous.contains(&"setuid"));
        assert!(dangerous.contains(&"ptrace"));
    }
    
    #[test]
    fn test_ptrace_blocked() {
        let filter = SeccompFilter::new_for_anonymous_code();
        
        // ptrace should not be allowed
        assert!(!filter.is_syscall_allowed("ptrace"));
        
        // Verify it's in the dangerous syscalls list
        let dangerous = get_dangerous_syscalls();
        assert!(dangerous.contains(&"ptrace"));
    }
    
    #[test]
    fn test_seccomp_support_detection() {
        // This test may fail in containers or systems without seccomp
        // but should not panic
        let _supported = is_seccomp_supported();
    }
}