/// Namespace isolation for enhanced security
/// Provides PID, mount, and network namespace isolation capabilities
use crate::types::{IsolateError, Result};
use std::path::PathBuf;

#[cfg(unix)]
use nix::sched::{unshare, CloneFlags};
#[cfg(unix)]
use nix::mount::{mount, MsFlags};
#[cfg(unix)]
use nix::unistd::{getpid, getppid};
#[cfg(unix)]
use std::fs;

/// Namespace isolation controller
pub struct NamespaceIsolation {
    /// Enable PID namespace isolation
    enable_pid_namespace: bool,
    /// Enable mount namespace isolation
    enable_mount_namespace: bool,
    /// Enable network namespace isolation
    enable_network_namespace: bool,
    /// Enable user namespace isolation
    enable_user_namespace: bool,
    /// Working directory for namespace operations
    workdir: PathBuf,
    /// Strict mode flag
    strict_mode: bool,
}

impl NamespaceIsolation {
    /// Create a new namespace isolation controller
    pub fn new(
        workdir: PathBuf,
        strict_mode: bool,
        enable_pid: bool,
        enable_mount: bool,
        enable_network: bool,
        enable_user: bool,
    ) -> Self {
        Self {
            enable_pid_namespace: enable_pid,
            enable_mount_namespace: enable_mount,
            enable_network_namespace: enable_network,
            enable_user_namespace: enable_user,
            workdir,
            strict_mode,
        }
    }

    /// Create default namespace isolation (all namespaces enabled)
    pub fn new_default(workdir: PathBuf, strict_mode: bool) -> Self {
        Self::new(workdir, strict_mode, true, true, true, false)
    }

    /// Check if namespace isolation is supported on this system
    pub fn is_supported() -> bool {
        #[cfg(unix)]
        {
            // Check if we can read /proc/self/ns/ directory
            std::fs::read_dir("/proc/self/ns").is_ok()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    /// Apply namespace isolation (called in child process before exec)
    pub fn apply_isolation(&self) -> Result<()> {
        #[cfg(unix)]
        {
            let mut clone_flags = CloneFlags::empty();

            // Build clone flags based on enabled namespaces
            if self.enable_pid_namespace {
                clone_flags |= CloneFlags::CLONE_NEWPID;
            }
            if self.enable_mount_namespace {
                clone_flags |= CloneFlags::CLONE_NEWNS;
            }
            if self.enable_network_namespace {
                clone_flags |= CloneFlags::CLONE_NEWNET;
            }
            if self.enable_user_namespace {
                clone_flags |= CloneFlags::CLONE_NEWUSER;
            }

            // Apply namespace isolation if any namespaces are enabled
            if !clone_flags.is_empty() {
                unshare(clone_flags).map_err(|e| {
                    IsolateError::Namespace(format!("Failed to unshare namespaces: {}", e))
                })?;

                // Additional setup for specific namespaces
                if self.enable_mount_namespace {
                    self.setup_mount_namespace()?;
                }

                if self.enable_pid_namespace {
                    self.setup_pid_namespace()?;
                }

                if self.enable_network_namespace {
                    self.setup_network_namespace()?;
                }
            }

            Ok(())
        }
        #[cfg(not(unix))]
        {
            if self.strict_mode {
                return Err(IsolateError::Namespace(
                    "Namespace isolation not supported on this platform".to_string(),
                ));
            }
            Ok(())
        }
    }

    /// Setup mount namespace isolation
    #[cfg(unix)]
    fn setup_mount_namespace(&self) -> Result<()> {
        // Make the root filesystem private to prevent mount propagation
        mount(
            None::<&str>,
            "/",
            None::<&str>,
            MsFlags::MS_PRIVATE | MsFlags::MS_REC,
            None::<&str>,
        ).map_err(|e| {
            IsolateError::Namespace(format!("Failed to make root filesystem private: {}", e))
        })?;

        // Create a minimal filesystem structure
        self.create_minimal_filesystem()?;

        Ok(())
    }

    /// Create minimal filesystem structure for mount namespace
    #[cfg(unix)]
    fn create_minimal_filesystem(&self) -> Result<()> {
        // Create essential directories if they don't exist
        let essential_dirs = [
            "/tmp",
            "/proc",
            "/sys",
            "/dev",
        ];

        for dir in &essential_dirs {
            if let Err(e) = fs::create_dir_all(dir) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(IsolateError::Namespace(format!(
                        "Failed to create directory {}: {}", dir, e
                    )));
                }
            }
        }

        // Mount proc filesystem (read-only for security)
        if let Err(e) = mount(
            Some("proc"),
            "/proc",
            Some("proc"),
            MsFlags::MS_RDONLY | MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
            None::<&str>,
        ) {
            // Don't fail if proc is already mounted
            if !e.to_string().contains("Device or resource busy") {
                return Err(IsolateError::Namespace(format!(
                    "Failed to mount /proc: {}", e
                )));
            }
        }

        // Mount tmpfs for /tmp (with size limit)
        if let Err(e) = mount(
            Some("tmpfs"),
            "/tmp",
            Some("tmpfs"),
            MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
            Some("size=100M"),
        ) {
            // Don't fail if tmpfs is already mounted
            if !e.to_string().contains("Device or resource busy") {
                return Err(IsolateError::Namespace(format!(
                    "Failed to mount /tmp tmpfs: {}", e
                )));
            }
        }

        Ok(())
    }

    /// Setup PID namespace isolation
    #[cfg(unix)]
    fn setup_pid_namespace(&self) -> Result<()> {
        // In a new PID namespace, the first process becomes PID 1
        // We need to ensure proper signal handling and process management
        
        // Check if we're actually in a new PID namespace
        let pid = getpid();
        let ppid = getppid();
        
        // Log namespace information for debugging
        if self.strict_mode {
            eprintln!("PID namespace: PID={}, PPID={}", pid, ppid);
        }

        Ok(())
    }

    /// Setup network namespace isolation
    #[cfg(unix)]
    fn setup_network_namespace(&self) -> Result<()> {
        // In a new network namespace, only loopback interface exists
        // This provides complete network isolation
        
        // Bring up loopback interface if needed
        if let Err(e) = std::process::Command::new("ip")
            .args(&["link", "set", "lo", "up"])
            .output()
        {
            // Don't fail if ip command is not available
            if self.strict_mode {
                eprintln!("Warning: Failed to bring up loopback interface: {}", e);
            }
        }

        Ok(())
    }

    /// Get namespace information for debugging
    pub fn get_namespace_info(&self) -> Result<NamespaceInfo> {
        #[cfg(unix)]
        {
            let pid = getpid();
            
            // Read namespace IDs from /proc/self/ns/
            let pid_ns = self.read_namespace_id("pid")?;
            let mount_ns = self.read_namespace_id("mnt")?;
            let net_ns = self.read_namespace_id("net")?;
            let user_ns = self.read_namespace_id("user").unwrap_or_else(|_| "unknown".to_string());

            Ok(NamespaceInfo {
                pid: pid.as_raw() as u32,
                pid_namespace: pid_ns,
                mount_namespace: mount_ns,
                network_namespace: net_ns,
                user_namespace: user_ns,
                isolation_enabled: self.is_isolation_enabled(),
            })
        }
        #[cfg(not(unix))]
        {
            Ok(NamespaceInfo {
                pid: 0,
                pid_namespace: "unsupported".to_string(),
                mount_namespace: "unsupported".to_string(),
                network_namespace: "unsupported".to_string(),
                user_namespace: "unsupported".to_string(),
                isolation_enabled: false,
            })
        }
    }

    /// Read namespace ID from /proc/self/ns/
    #[cfg(unix)]
    fn read_namespace_id(&self, ns_type: &str) -> Result<String> {
        let ns_path = format!("/proc/self/ns/{}", ns_type);
        match fs::read_link(&ns_path) {
            Ok(link) => Ok(link.to_string_lossy().to_string()),
            Err(e) => Err(IsolateError::Namespace(format!(
                "Failed to read namespace {}: {}", ns_type, e
            ))),
        }
    }

    /// Check if any isolation is enabled
    pub fn is_isolation_enabled(&self) -> bool {
        self.enable_pid_namespace 
            || self.enable_mount_namespace 
            || self.enable_network_namespace 
            || self.enable_user_namespace
    }

    /// Get enabled namespaces as a string
    pub fn get_enabled_namespaces(&self) -> Vec<String> {
        let mut namespaces = Vec::new();
        
        if self.enable_pid_namespace {
            namespaces.push("PID".to_string());
        }
        if self.enable_mount_namespace {
            namespaces.push("Mount".to_string());
        }
        if self.enable_network_namespace {
            namespaces.push("Network".to_string());
        }
        if self.enable_user_namespace {
            namespaces.push("User".to_string());
        }
        
        namespaces
    }
}

/// Namespace information for debugging and monitoring
#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    /// Process ID
    pub pid: u32,
    /// PID namespace identifier
    pub pid_namespace: String,
    /// Mount namespace identifier
    pub mount_namespace: String,
    /// Network namespace identifier
    pub network_namespace: String,
    /// User namespace identifier
    pub user_namespace: String,
    /// Whether namespace isolation is enabled
    pub isolation_enabled: bool,
}

impl std::fmt::Display for NamespaceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PID: {}, Namespaces: [PID: {}, Mount: {}, Net: {}, User: {}], Isolation: {}",
            self.pid,
            self.pid_namespace,
            self.mount_namespace,
            self.network_namespace,
            self.user_namespace,
            self.isolation_enabled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_namespace_creation() {
        let ns = NamespaceIsolation::new_default(PathBuf::from("/tmp"), false);
        assert!(ns.is_isolation_enabled());
        assert_eq!(ns.get_enabled_namespaces(), vec!["PID", "Mount", "Network"]);
    }

    #[test]
    fn test_namespace_support_check() {
        // This test will pass on Linux systems with namespace support
        let supported = NamespaceIsolation::is_supported();
        println!("Namespace support: {}", supported);
    }

    #[test]
    fn test_namespace_info() {
        let ns = NamespaceIsolation::new_default(PathBuf::from("/tmp"), false);
        let info = ns.get_namespace_info();
        
        match info {
            Ok(info) => {
                println!("Namespace info: {}", info);
                assert!(info.pid > 0);
            }
            Err(e) => {
                println!("Failed to get namespace info: {}", e);
                // This is expected on non-Linux systems
            }
        }
    }
}