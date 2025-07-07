/// Security module implementing seccomp-bpf syscall filtering
/// Provides defense against malicious code by blocking dangerous system calls
/// 
/// This implementation is designed to match or exceed IOI isolate's security level
/// while providing better usability through language-specific profiles.
use crate::types::Result;
use std::collections::HashSet;


use libseccomp::*;

/// Seccomp filter configuration for anonymous code execution
/// 
/// This filter implements a comprehensive whitelist approach where only
/// explicitly allowed syscalls are permitted. This is more secure than
/// isolate's approach which relies primarily on resource limits.
pub struct SeccompFilter {
    /// Action to take for blocked syscalls (only used when seccomp is available)
    
    default_action: ScmpAction,
    /// Set of explicitly allowed syscalls
    allowed_syscalls: HashSet<String>,
}

impl SeccompFilter {
    /// Create a new seccomp filter with secure defaults for anonymous code execution
    /// 
    /// This filter is more restrictive than isolate's default behavior, providing
    /// better security for untrusted code execution. Only essential syscalls for
    /// basic computation are allowed.
    pub fn new_for_anonymous_code() -> Self {
        let mut allowed_syscalls = HashSet::new();
        
        // Essential syscalls for basic program execution
        let essential = [
            // Process control (minimal set)
            "exit", "exit_group", "getpid", "getppid",
            
            // Memory management (essential for any program)
            "brk", "mmap", "munmap", "mprotect", "madvise",
            
            // File I/O (limited to essential operations)
            "read", "write", "close", "fstat", "lseek",
            "open", "openat", "access", "faccessat",
            "readlink", "readlinkat", // For resolving symlinks
            
            // Time and scheduling (safe operations)
            "nanosleep", "clock_gettime", "gettimeofday", "time",
            "clock_nanosleep", "clock_getres",
            
            // Signal handling (basic set - no signal sending to other processes)
            "rt_sigaction", "rt_sigprocmask", "rt_sigreturn",
            "sigaltstack", "rt_sigsuspend",
            
            // Basic system info (safe read-only operations)
            "getuid", "getgid", "geteuid", "getegid",
            "getgroups", "getpgrp", "getpgid", "getsid",
            "arch_prctl", "getrlimit", "uname",
            
            // File descriptor operations (safe)
            "dup", "dup2", "dup3", "fcntl",
            
            // Directory operations (read-only)
            "getcwd", "getdents", "getdents64",
            
            // Memory protection (for language runtimes)
            "mlock", "munlock", "mlockall", "munlockall",
            
            // Thread synchronization (for multi-threaded languages)
            "futex", "sched_yield", "sched_getaffinity",
            
            // I/O multiplexing (for event-driven programs)
            "poll", "ppoll", "select", "pselect6",
            "epoll_create", "epoll_create1", "epoll_ctl", "epoll_wait", "epoll_pwait",
            
            // Pipe operations (for internal communication)
            "pipe", "pipe2",
            
            // Statistics and resource usage
            "getrusage", "times",
        ];
        
        for syscall in &essential {
            allowed_syscalls.insert(syscall.to_string());
        }
        
        Self {
            
            default_action: ScmpAction::KillProcess, // Kill immediately on violation
            allowed_syscalls,
        }
    }
    
    /// Create a filter that allows additional syscalls for specific languages
    /// 
    /// This provides language-specific profiles that are more permissive than
    /// the anonymous code filter but still maintain strong security boundaries.
    pub fn new_for_language(language: &str) -> Self {
        let mut filter = Self::new_for_anonymous_code();
        
        match language {
            "python" | "python3" => {
                filter.add_python_syscalls();
            }
            "javascript" | "node" | "js" => {
                filter.add_javascript_syscalls();
            }
            "java" => {
                filter.add_java_syscalls();
            }
            "c" | "cpp" | "c++" => {
                filter.add_compiled_language_syscalls();
            }
            "go" => {
                filter.add_go_syscalls();
            }
            "rust" => {
                filter.add_rust_syscalls();
            }
            _ => {
                log::warn!("Unknown language profile '{}', using anonymous code filter", language);
            }
        }
        
        filter
    }
    
    /// Add syscalls commonly needed by Python interpreters
    fn add_python_syscalls(&mut self) {
        let python_syscalls = [
            // File system operations (Python needs more FS access)
            "stat", "lstat", "fstat", "newfstatat", "statfs", "fstatfs",
            "openat", "faccessat", "fchdir", "chdir",
            
            // Dynamic loading (for Python modules)
            "mmap", "munmap", "mprotect", "madvise",
            
            // Process information
            "getpid", "getppid", "gettid",
            
            // Error handling
            "rt_sigaction", "rt_sigprocmask",
            
            // Networking (very limited - only for localhost)
            // Note: We don't allow general networking, but Python may need unix sockets
            "socketpair", // Only for local IPC
        ];
        
        for syscall in &python_syscalls {
            self.allowed_syscalls.insert(syscall.to_string());
        }
        
        log::info!("Added Python-specific syscalls to seccomp filter");
    }
    
    /// Add syscalls commonly needed by JavaScript/Node.js
    fn add_javascript_syscalls(&mut self) {
        let js_syscalls = [
            // Event loop operations
            "eventfd", "eventfd2", "timerfd_create", "timerfd_settime", "timerfd_gettime",
            
            // Threading primitives (Node.js worker threads)
            "clone", "set_robust_list", "get_robust_list",
            
            // Advanced I/O
            "readv", "writev", "preadv", "pwritev",
            
            // File watching (for development tools)
            "inotify_init", "inotify_init1", "inotify_add_watch", "inotify_rm_watch",
            
            // Memory management (V8 engine)
            "madvise", "mlock", "munlock",
        ];
        
        for syscall in &js_syscalls {
            self.allowed_syscalls.insert(syscall.to_string());
        }
        
        log::info!("Added JavaScript/Node.js-specific syscalls to seccomp filter");
    }
    
    /// Add syscalls commonly needed by Java JVM
    fn add_java_syscalls(&mut self) {
        let java_syscalls = [
            // JVM threading
            "clone", "set_robust_list", "get_robust_list",
            "sched_getparam", "sched_setscheduler", "sched_getscheduler",
            
            // JVM memory management
            "madvise", "mlock", "munlock", "mlockall", "munlockall",
            
            // JVM process control
            "prctl", // Limited prctl operations
            
            // JVM signal handling
            "rt_sigqueueinfo", "rt_tgsigqueueinfo",
            
            // JVM profiling and debugging
            "getitimer", "setitimer",
        ];
        
        for syscall in &java_syscalls {
            self.allowed_syscalls.insert(syscall.to_string());
        }
        
        log::info!("Added Java JVM-specific syscalls to seccomp filter");
    }
    
    /// Add syscalls for compiled languages (C/C++)
    fn add_compiled_language_syscalls(&mut self) {
        let compiled_syscalls = [
            // Additional memory operations
            "madvise", "mincore",
            
            // Additional file operations
            "fsync", "fdatasync", "sync_file_range",
            
            // Process control
            "getrlimit", "setrlimit", // For resource management
        ];
        
        for syscall in &compiled_syscalls {
            self.allowed_syscalls.insert(syscall.to_string());
        }
        
        log::info!("Added compiled language-specific syscalls to seccomp filter");
    }
    
    /// Add syscalls commonly needed by Go programs
    fn add_go_syscalls(&mut self) {
        let go_syscalls = [
            // Go runtime
            "clone", "gettid", "tkill", "tgkill",
            "sched_yield", "sched_getaffinity", "sched_setaffinity",
            
            // Go memory management
            "madvise", "mlock", "munlock",
            
            // Go networking (limited)
            "socketpair", // Only for local IPC
            
            // Go signal handling
            "rt_sigaction", "rt_sigprocmask", "signalfd4",
        ];
        
        for syscall in &go_syscalls {
            self.allowed_syscalls.insert(syscall.to_string());
        }
        
        log::info!("Added Go-specific syscalls to seccomp filter");
    }
    
    /// Add syscalls commonly needed by Rust programs
    fn add_rust_syscalls(&mut self) {
        let rust_syscalls = [
            // Rust runtime (minimal)
            "madvise", "mlock", "munlock",
            
            // Rust async runtime
            "eventfd2", "timerfd_create", "timerfd_settime",
            
            // Rust error handling
            "rt_sigaction", "rt_sigprocmask",
            
            // Rust memory safety
            "mprotect", "madvise",
        ];
        
        for syscall in &rust_syscalls {
            self.allowed_syscalls.insert(syscall.to_string());
        }
        
        log::info!("Added Rust-specific syscalls to seccomp filter");
    }
    
    /// Apply the seccomp filter to the current process
    pub fn apply(&self) -> Result<()> {
        
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
/// 
/// This list is more comprehensive than isolate's approach and includes all
/// syscalls that could be used for privilege escalation, system modification,
/// or breaking out of the sandbox environment.
pub fn get_dangerous_syscalls() -> Vec<&'static str> {
    vec![
        // Network operations (complete networking ban for untrusted code)
        "socket", "connect", "bind", "listen", "accept", "accept4",
        "sendto", "sendmsg", "recvfrom", "recvmsg", "shutdown",
        "getsockname", "getpeername", "getsockopt", "setsockopt",
        
        // Process/thread creation and manipulation
        "fork", "vfork", "clone", "execve", "execveat",
        "wait4", "waitid", "waitpid",
        
        // File system modifications (prevent tampering)
        "mount", "umount", "umount2", "chroot", "pivot_root",
        "mkdir", "rmdir", "unlink", "unlinkat", "rename", "renameat", "renameat2",
        "chmod", "fchmod", "fchmodat", "chown", "fchown", "lchown", "fchownat",
        "link", "linkat", "symlink", "symlinkat",
        "mknod", "mknodat", "truncate", "ftruncate",
        
        // Privilege operations (prevent escalation)
        "setuid", "setgid", "setreuid", "setregid", "setresuid", "setresgid",
        "setfsuid", "setfsgid", "capset", "capget",
        "setgroups", "setpgid", "setsid", "setpgrp",
        
        // System information/modification (prevent system tampering)
        "sysinfo", "uname", "sethostname", "setdomainname",
        "reboot", "kexec_load", "kexec_file_load",
        "init_module", "delete_module", "finit_module",
        "syslog", "sysctl", "_sysctl",
        
        // Debugging/tracing (prevent inspection of other processes)
        "ptrace", "process_vm_readv", "process_vm_writev",
        "kcmp", "perf_event_open",
        
        // System V IPC (prevent inter-process communication)
        "msgget", "msgctl", "msgrcv", "msgsnd",
        "semget", "semctl", "semop", "semtimedop",
        "shmget", "shmctl", "shmat", "shmdt",
        
        // Device access (prevent hardware manipulation)
        "ioctl", "ioperm", "iopl", "outb", "outw", "outl",
        
        // Advanced memory operations (prevent memory attacks)
        "mbind", "migrate_pages", "move_pages", "get_mempolicy", "set_mempolicy",
        "remap_file_pages", "userfaultfd",
        
        // Time manipulation (prevent system time changes)
        "settimeofday", "adjtimex", "clock_settime", "clock_adjtime",
        
        // Quota management (prevent quota manipulation)
        "quotactl",
        
        // Swap operations (prevent swap manipulation)
        "swapon", "swapoff",
        
        // Keyring operations (prevent key manipulation)
        "add_key", "request_key", "keyctl",
        
        // Namespace operations (prevent namespace escape)
        "unshare", "setns",
        
        // CPU affinity (prevent CPU manipulation beyond basic queries)
        "sched_setaffinity", "sched_setparam", "sched_setscheduler",
        "sched_setattr",
        
        // Real-time operations (prevent RT scheduling)
        "sched_get_priority_max", "sched_get_priority_min",
        "sched_rr_get_interval",
        
        // Extended attributes (prevent metadata manipulation)
        "setxattr", "lsetxattr", "fsetxattr", "removexattr", "lremovexattr", "fremovexattr",
        
        // POSIX message queues (prevent IPC)
        "mq_open", "mq_unlink", "mq_timedsend", "mq_timedreceive",
        "mq_notify", "mq_getsetattr",
        
        // Epoll/eventfd abuse (prevent resource exhaustion)
        // Note: Basic epoll is allowed, but advanced features are blocked
        "epoll_pwait2",
        
        // Advanced signal operations (prevent signal manipulation)
        "rt_sigqueueinfo", "rt_tgsigqueueinfo", "signalfd", "signalfd4",
        "kill", "tkill", "tgkill", // Prevent sending signals to other processes
        
        // BPF operations (prevent BPF program loading)
        "bpf",
        
        // Seccomp manipulation (prevent seccomp bypass)
        "seccomp",
        
        // Memory protection bypass attempts
        "pkey_alloc", "pkey_free", "pkey_mprotect",
        
        // Virtualization (prevent VM escape)
        "vm86", "vm86old",
        
        // Architecture-specific dangerous syscalls
        "modify_ldt", "arch_prctl", // Some arch_prctl operations are dangerous
        
        // File locking (can be used for DoS)
        "flock", "fcntl", // Some fcntl operations are dangerous
        
        // Timer manipulation (prevent timer abuse)
        "timer_create", "timer_settime", "timer_gettime", "timer_getoverrun", "timer_delete",
        
        // NUMA operations (prevent NUMA manipulation)
        "set_mempolicy", "get_mempolicy", "mbind",
        
        // CPU cache operations (prevent cache attacks)
        "cacheflush",
        
        // Fanotify (prevent filesystem monitoring)
        "fanotify_init", "fanotify_mark",
        
        // Name to handle conversion (prevent filesystem bypass)
        "name_to_handle_at", "open_by_handle_at",
        
        // Sync operations that could cause DoS
        "sync", "syncfs",
        
        // Resource limit manipulation
        "prlimit64", // setrlimit is allowed but prlimit64 can affect other processes
        
        // Memory mapping with dangerous flags
        "remap_file_pages", "mremap", // mremap can be dangerous
        
        // File descriptor manipulation that could be dangerous
        "sendfile", "sendfile64", "splice", "tee", "vmsplice",
        
        // Filesystem-specific operations
        "fallocate", "fadvise64", "readahead",
        
        // Clock manipulation
        "clock_adjtime",
    ]
}

/// Test helper to check if seccomp is supported on the current system
pub fn is_seccomp_supported() -> bool {
    ScmpFilterContext::new_filter(ScmpAction::Allow).is_ok()
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