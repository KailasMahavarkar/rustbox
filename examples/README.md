# Mini-Isolate Examples

This directory contains demonstration scripts and examples showing how to use mini-isolate's security features.

**Important:** Most demonstrations require sudo privileges to access full security features like cgroups, namespaces, and filesystem isolation.

## Demo Scripts

### 1. `demo_filesystem_security.sh`
Demonstrates the filesystem security features of mini-isolate:
- Chroot jail isolation
- Mount security flags (noexec, nosuid, nodev)
- Essential device file creation
- Path validation against dangerous locations
- CLI chroot option support

**Usage:**
```bash
# Make sure mini-isolate is built first
cargo build --release

# Run the filesystem security demonstration (requires sudo)
sudo ./examples/demo_filesystem_security.sh
```

### 2. `demo_namespace_isolation.sh`
Demonstrates namespace isolation capabilities:
- PID namespace isolation (limited process visibility)
- Network namespace isolation (isolated network stack)
- Mount namespace isolation (isolated filesystem view)
- Comparison with and without namespace isolation

**Usage:**
```bash
# Requires sudo privileges for namespace operations
sudo ./examples/demo_namespace_isolation.sh
```

**Note:** This demo requires root privileges because namespace operations require elevated permissions.

### 3. `io_demo.py`
Python example demonstrating I/O handling and security features:
- Input/output redirection
- Security limits and validation
- Buffer size controls

**Usage:**
```bash
# Run with mini-isolate
cargo build --release
./target/release/mini-isolate run --box-id test --strict --max-time 30 python3 examples/io_demo.py
```

## Security Features Demonstrated

### Filesystem Security
- **Chroot Isolation**: Complete filesystem isolation from host
- **Mount Security**: Secure mount options preventing execution and privilege escalation
- **Path Validation**: Prevention of directory traversal attacks
- **Device File Security**: Minimal essential device files only

### Namespace Isolation
- **Process Isolation**: Isolated PID namespace hiding host processes
- **Network Isolation**: Private network namespace with minimal connectivity
- **Mount Isolation**: Private mount namespace with controlled filesystem access
- **User Isolation**: Optional user namespace for additional security

### Resource Controls
- **Memory Limits**: Prevents memory exhaustion attacks
- **CPU Time Limits**: Prevents infinite loops and CPU exhaustion
- **File Size Limits**: Prevents disk space exhaustion
- **Process Limits**: Prevents fork bomb attacks

### I/O Security
- **Input Validation**: Prevents malicious input injection
- **Output Size Limits**: Prevents memory exhaustion from large outputs
- **Buffer Controls**: Configurable buffer sizes with security limits
- **File Permission Controls**: Restrictive permissions on all created files

## Prerequisites

- **Linux System**: All demos require a Linux environment
- **Root Access**: Namespace demos require sudo/root privileges
- **Cgroups v1**: Resource limit demos require cgroups v1 support
- **Seccomp**: Syscall filtering demos require seccomp support

## Running All Demos

To run all demonstrations in sequence:

```bash
# Build the project
cargo build --release

# Run filesystem security demo (no sudo required)
./examples/demo_filesystem_security.sh

# Run namespace isolation demo (requires sudo)
sudo ./examples/demo_namespace_isolation.sh

# Run I/O demo
./target/release/mini-isolate run --box-id io-test --strict --max-time 30 python3 examples/io_demo.py
```

## Understanding the Output

Each demo provides detailed output explaining:
- What security feature is being demonstrated
- Expected vs actual behavior
- Security implications of each feature
- How the feature protects against specific attack vectors

## Troubleshooting

### Common Issues:

1. **Permission Denied**: Namespace demos require root privileges
2. **Cgroups Not Found**: Ensure cgroups v1 is available on your system
3. **Seccomp Not Supported**: Some older kernels may not support all seccomp features
4. **Missing Dependencies**: Ensure all required system tools are installed

### System Requirements Check:

```bash
# Check cgroups support
ls /sys/fs/cgroup/

# Check seccomp support
grep CONFIG_SECCOMP /boot/config-$(uname -r)

# Check namespace support
unshare --help
```

## Security Notes

These demos are designed to be safe for demonstration purposes, but they showcase powerful isolation features. When using mini-isolate in production:

- Always run with appropriate resource limits
- Use the strictest security settings appropriate for your use case
- Regularly update and audit your security configurations
- Monitor for any unusual behavior or security violations

## Contributing

When adding new examples:
1. Include clear documentation of what security feature is demonstrated
2. Add usage instructions and prerequisites
3. Include expected output examples
4. Update this README with the new example
5. Ensure examples are safe and educational