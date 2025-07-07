# rustbox

A Rust-based secure sandbox for executing untrusted code safely, inspired by IOI isolate.

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

rustbox provides a secure, resource-controlled environment for executing untrusted code, making it ideal for programming contests, educational platforms, and automated judging systems.

## 🚀 Features

- **🔒 Secure process isolation** with resource limits using cgroups
- **🌐 Multi-language support** - Python, C, C++, Rust, Java, and more
- **💻 IOI isolate compatibility** - Drop-in replacement for contest environments
- **⚡ High performance** written in Rust with minimal overhead
- **📊 Structured output** - JSON format for easy automation
- **🔧 Source file execution** with automatic compilation and dependency management
- **🛡️ Graceful degradation** - Works without root privileges (limited functionality)
- **📈 Resource monitoring** - CPU time, memory usage, and wall clock time tracking

## 🔐 Permission Requirements

**Important**: rustbox requires sudo/root permissions for full cgroup-based resource isolation, but works gracefully without it.

### Without sudo:
- ✅ **All functionality works** (process execution, timeouts, file operations)
- ⚠️ **Resource limits are not enforced** (memory, CPU limits are ineffective)  
- ⚠️ **Warning displayed**: "Cannot create cgroup (permission denied). Resource limits will not be enforced."

### With sudo:
- ✅ **Full cgroup support** with actual memory/CPU/process limits
- ✅ **Resource monitoring** (peak memory usage, CPU time tracking)
- ✅ **Enhanced isolation** and security

### Usage Examples:
```bash
# Basic functionality (no sudo required)
rustbox init --box-id 0
rustbox execute --box-id 0 --source hello.py

# Full functionality (sudo required for resource limits)
sudo rustbox init --box-id 0 --mem 128 --time 10
sudo rustbox execute --box-id 0 --source memory_test.py

# Strict mode (like IOI isolate - requires sudo, fails without cgroups)
sudo rustbox init --box-id 0 --mem 128 --time 10 --strict
sudo rustbox run --box-id 0 --strict python3 solution.py

# Strict mode without sudo (will fail)
rustbox init --box-id 0 --strict  # Error: requires root privileges
```

### Alternative Solutions:
- **Docker containers**: Run rustbox inside containers with cgroup delegation
- **User namespaces**: Configure for unprivileged cgroup access
- **Systemd user slices**: Use systemd for resource management

## 📋 Requirements

- **Operating System**: Linux (Ubuntu 18+, CentOS 7+, etc.)
- **Rust**: 1.70 or later (for building from source)
- **Permissions**: Root privileges recommended for full cgroup functionality
- **Dependencies**: Standard Linux utilities (`gcc`, `python3`, etc. for compilation)

## 🛠️ Installation

### Option 1: Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd rustbox

# Build the release version
cargo build --release

# The binary will be available at ./target/release/rustbox
# Optionally, copy to system PATH
sudo cp target/release/rustbox /usr/local/bin/
```

### Option 2: Using Cargo

```bash
cargo install --git <repository-url>
```

## 🚦 Quick Start

### 1. Initialize an Isolate Instance

```bash
# Create a new isolate with resource limits
rustbox init --box-id 0 --mem 128 --time 10 --wall-time 20
```

### 2. Execute Code

```bash
# Run a simple command
rustbox run --box-id 0 "/bin/echo" -- "Hello, World!"

# Execute a Python script directly
echo 'print("Hello from Python!")' > hello.py
rustbox execute --box-id 0 --source hello.py --verbose

# Execute with input file
echo "5 3" > input.txt
echo 'a, b = map(int, input().split()); print(a + b)' > sum.py
rustbox execute --box-id 0 --source sum.py --input input.txt
```

### 3. Get Structured Results

```bash
# Execute with JSON output
rustbox execute --box-id 0 --source hello.py --output result.json
cat result.json
```

Example output:
```json
{
  "exit_code": 0,
  "status": "Success",
  "stdout": "Hello from Python!\n",
  "stderr": "",
  "cpu_time": 0.015,
  "wall_time": 0.045,
  "memory_peak": 8642560,
  "signal": null,
  "success": true,
  "error_message": null
}
```

### 4. Clean Up

```bash
# Clean up a specific instance
rustbox cleanup --box-id 0

# Clean up all instances
rustbox cleanup --all
```

## 📚 Documentation

- **[User Guide](docs/user-guide.md)** - Comprehensive usage instructions
- **[Module Documentation](docs/modules.md)** - Technical architecture details
- **[Examples](docs/examples.md)** - Practical examples and use cases
- **[IOI Compatibility](docs/ioi-compatibility.md)** - Migration from IOI isolate
- **[API Documentation](docs/api.md)** - Library usage and integration

## 🏗️ Architecture

rustbox is built with a modular architecture:

```
src/
├── main.rs           # Application entry point
├── types.rs          # Core data structures  
├── cgroup.rs         # Resource control via cgroups
├── executor.rs       # Process execution and monitoring
├── isolate.rs        # High-level isolation management
└── cli.rs            # Command-line interface
```

## 🎯 Use Cases

### Programming Contests
```bash
# Setup for contest environment
for i in {0..9}; do
    rustbox init --box-id $i --mem 256 --time 30 --wall-time 60
done

# Judge a submission
rustbox execute --box-id 0 --source solution.cpp \
    --input test1.in --output result.json
```

### Educational Platforms
```bash
# Safe execution of student code
rustbox init --box-id classroom --mem 128 --time 10
rustbox execute --box-id classroom --source student_code.py --verbose
```

### Automated Testing
```bash
# CI/CD integration
rustbox execute --box-id ci --source test_suite.py \
    --output test_results.json --time 60
```

## ⚙️ Configuration Options

### Resource Limits
- `--mem SIZE` - Memory limit in MB (default: 128)
- `--time SECONDS` - CPU time limit (default: 10)
- `--wall-time SECONDS` - Wall clock limit (default: 20) 
- `--processes COUNT` - Process limit (default: 1)
- `--fsize SIZE` - File size limit in MB (default: 64)

### Execution Options
- `--verbose` - Show detailed output including stdout/stderr
- `--quiet` - Minimal output mode
- `--input FILE` - Redirect stdin from file
- `--output FILE` - Save results as JSON
- `--source FILE` - Execute source file with automatic compilation
- `--strict` - Strict mode: require root privileges and fail if cgroups unavailable (like IOI isolate)

## 🔧 Advanced Usage

### Custom Compilation
```bash
# Custom C++ compilation with specific flags
rustbox execute --box-id 0 --source solution.cpp \
    --compile-flags "-O2 -std=c++17" --verbose
```

### Multiple Test Cases
```bash
# Batch testing
for test in tests/*.in; do
    output="${test%.in}.out"
    rustbox execute --box-id 0 --source solution.py \
        --input "$test" --output "result_$(basename $test .in).json"
done
```

### Performance Analysis
```bash
# Performance monitoring
rustbox execute --box-id perf --source benchmark.py \
    --output perf.json --verbose

# Extract timing information
python3 -c "
import json
with open('perf.json') as f:
    data = json.load(f)
    print(f'CPU time: {data[\"cpu_time\"]}s')
    print(f'Wall time: {data[\"wall_time\"]}s') 
    print(f'Peak memory: {data[\"memory_peak\"]/1024/1024:.1f}MB')
"
```

## 🐛 Troubleshooting

### Common Issues

1. **Permission Denied for Cgroups**
   ```bash
   # Run with sudo for full functionality
   sudo rustbox init --box-id 0 --mem 128
   
   # Alternative: Run without sudo (limited functionality)
   rustbox init --box-id 0  # Resource limits won't be enforced
   ```

2. **Compilation Errors**
   ```bash
   # Check if required compilers are installed
   gcc --version
   python3 --version
   ```

3. **Resource Limit Not Enforced**
   - **With root**: Ensure running with root privileges for cgroup support
   - **Without root**: Resource limits won't be enforced (expected behavior)
   - Check if cgroups v1 is available: `ls /sys/fs/cgroup/`
   - Try using systemd or Docker for alternative resource management

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup
```bash
# Clone and build
git clone <repository-url>
cd rustbox
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by the [IOI isolate](https://github.com/ioi/isolate) sandbox
- Built with [Rust](https://www.rust-lang.org/) and the amazing Rust ecosystem
- Thanks to the competitive programming and educational communities

## 📞 Support

- 📖 Check the [documentation](docs/) for detailed guides
- 🐛 Report issues on our [issue tracker](issues)
- 💬 Ask questions in [discussions](discussions)
- 📧 Contact maintainers for enterprise support