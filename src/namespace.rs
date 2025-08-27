/// Namespace isolation for enhanced security
/// Provides PID, mount, and network namespace isolation capabilities
use crate::types::{IsolateError, Result};

#[cfg(unix)]
use nix::sched::{unshare, CloneFlags};
#[cfg(unix)]
use nix::unistd::{getpid, sethostname};
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
    /// Enable IPC namespace isolation
    enable_ipc_namespace: bool,
    /// Enable UTS namespace isolation  
    enable_uts_namespace: bool,
}

impl NamespaceIsolation {
    /// Create a new namespace isolation controller
    pub fn new(
        enable_pid: bool,
        enable_mount: bool,
        enable_network: bool,
        enable_user: bool,
        enable_ipc: bool,
        enable_uts: bool,
    ) -> Self {
        Self {
            enable_pid_namespace: enable_pid,
            enable_mount_namespace: enable_mount,
            enable_network_namespace: enable_network,
            enable_user_namespace: enable_user,
            enable_ipc_namespace: enable_ipc,
            enable_uts_namespace: enable_uts,
        }
    }

    /// Create default namespace isolation (all namespaces enabled)
    pub fn new_default() -> Self {
        Self::new(true, true, true, false, true, true)
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

    /// Apply namespace isolation using unshare syscalls
    /// This must be called before forking the target process
    pub fn apply_isolation(&self) -> Result<()> {
        #[cfg(unix)]
        {
            let mut flags = CloneFlags::empty();

            // Build clone flags for unshare
            if self.enable_pid_namespace {
                flags |= CloneFlags::CLONE_NEWPID;
            }
            if self.enable_mount_namespace {
                flags |= CloneFlags::CLONE_NEWNS;
            }
            if self.enable_network_namespace {
                flags |= CloneFlags::CLONE_NEWNET;
            }
            if self.enable_user_namespace {
                flags |= CloneFlags::CLONE_NEWUSER;
            }
            if self.enable_ipc_namespace {
                flags |= CloneFlags::CLONE_NEWIPC;
            }
            if self.enable_uts_namespace {
                flags |= CloneFlags::CLONE_NEWUTS;
            }

            if !flags.is_empty() {
                unshare(flags).map_err(|e| {
                    IsolateError::Namespace(format!("Failed to unshare namespaces: {}", e))
                })?;

                // Set hostname in UTS namespace if enabled
                if self.enable_uts_namespace {
                    if let Err(e) = sethostname("rustbox-sandbox") {
                        log::warn!("Failed to set hostname in UTS namespace: {}", e);
                    }
                }

                log::info!("Successfully applied namespace isolation: {:?}", self.get_enabled_namespaces());
            }

            Ok(())
        }
        #[cfg(not(unix))]
        {
            if self.is_isolation_enabled() {
                Err(IsolateError::Namespace(
                    "Namespace isolation is only supported on Unix systems".to_string(),
                ))
            } else {
                Ok(())
            }
        }
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
            || self.enable_ipc_namespace
            || self.enable_uts_namespace
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
        if self.enable_ipc_namespace {
            namespaces.push("IPC".to_string());
        }
        if self.enable_uts_namespace {
            namespaces.push("UTS".to_string());
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

    #[test]
    fn test_namespace_creation() {
        let ns = NamespaceIsolation::new_default();
        assert!(ns.is_isolation_enabled());
        assert_eq!(ns.get_enabled_namespaces(), vec!["PID", "Mount", "Network", "IPC", "UTS"]);
    }

    #[test]
    fn test_namespace_support_check() {
        // This test will pass on Linux systems with namespace support
        let supported = NamespaceIsolation::is_supported();
        println!("Namespace support: {}", supported);
    }

    #[test]
    fn test_namespace_info() {
        let ns = NamespaceIsolation::new_default();
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