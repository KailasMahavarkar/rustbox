# Rustbox Rust-based Test Suite

This directory contains a comprehensive native Rust test suite for the rustbox sandboxing system. The tests are organized into logical categories and provide better integration with the Rust ecosystem compared to the shell-based tests.

## 🏗️ Test Structure

```
tests/
├── main.rs                 # Main test runner with CLI
├── mod.rs                  # Core test framework
├── utils.rs                # Test utilities and helpers
├── core.rs                 # Core functionality tests
├── resource.rs             # Resource limit tests
├── security.rs             # Security and isolation tests
├── stress.rs               # Stress and load tests
├── integration.rs          # Integration tests
├── performance.rs          # Performance benchmarks
├── languages/              # Language-specific tests
│   ├── mod.rs             # Language test framework
│   └── test_files/        # Embedded test files
│       ├── lang_python/   # Python test files
│       ├── lang_cpp/      # C++ test files
│       └── lang_java/     # Java test files
└── Cargo.toml             # Test suite dependencies
```

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Built rustbox binary (`cargo build --release`)
- Root privileges (for most tests)
- Linux with cgroups v1 support

### Running Tests

```bash
# Build the test suite
cd tests
cargo build --release

# Check system requirements
sudo cargo run --bin tests check

# Run all tests
sudo cargo run --bin tests all

# Run specific test category
sudo cargo run --bin tests category core
sudo cargo run --bin tests category languages
sudo cargo run --bin tests category security

# List available categories
cargo run --bin tests list
```

## 📋 Test Categories

### Core Tests (`core`)
- Basic Python, C++, and Java execution
- Language dependency checking
- Init and cleanup commands
- Error handling validation

### Resource Tests (`resource`)
- Memory limit enforcement
- CPU time limit enforcement
- Wall time limit enforcement
- Process limit enforcement
- File descriptor limits
- Resource monitoring accuracy

### Security Tests (`security`)
- Process namespace isolation
- Filesystem isolation
- Network isolation
- User namespace isolation
- Path traversal prevention
- Privilege escalation prevention

### Stress Tests (`stress`)
- Sequential execution under load
- Concurrent execution stress
- Memory pressure scenarios
- CPU intensive workloads
- Resource contention testing
- Rapid box creation/destruction

### Integration Tests (`integration`)
- Complete workflow testing
- Multi-language integration
- Resource limit integration
- Error recovery scenarios
- Concurrent management
- System integration

### Performance Tests (`performance`)
- Startup time benchmarks
- Execution overhead measurement
- Memory usage profiling
- Throughput testing
- Resource monitoring performance
- System resource utilization

### Language Tests (`languages`)
- Python execution tests (factorial, star pattern, LIS algorithm)
- C++ execution tests (factorial, star pattern, LIS algorithm)
- Java execution tests (factorial, star pattern, LIS algorithm)
- Time limit enforcement
- Memory limit enforcement

## 🔧 Configuration

The test suite can be configured through command-line arguments:

```bash
# Custom rustbox binary path
cargo run --bin tests all --rustbox-path /custom/path/rustbox

# Disable verbose output
cargo run --bin tests all --quiet

# Disable sudo requirement (limited functionality)
cargo run --bin tests all --no-sudo
```

## 📊 Test Results

The test suite provides comprehensive reporting:

- ✅ **Passed tests** with execution time
- ❌ **Failed tests** with error details
- 📈 **Success rate** percentage
- ⏱️ **Total execution time**
- 📋 **Category summaries**

Example output:
```
🧪 Running rustbox comprehensive test suite
==========================================

📋 Running Core Tests
=====================
✅ Core Tests completed in 2.34s (8/8 passed)

📋 Running Resource Tests
=========================
✅ Resource Tests completed in 1.87s (8/8 passed)

📊 Test Summary
===============
Total Tests: 64
✅ Passed: 64
❌ Failed: 0
📈 Success Rate: 100%
⏱️  Total Duration: 45.67s

🎉 All tests passed! rustbox is working correctly.
```

## 🛠️ Development

### Adding New Tests

1. **Core functionality**: Add to `core.rs`
2. **Resource limits**: Add to `resource.rs`
3. **Security features**: Add to `security.rs`
4. **Performance**: Add to `performance.rs`
5. **Language support**: Add to `languages/mod.rs`

### Test Utilities

Use the provided utilities for common operations:

```rust
use tests::utils::TestUtils;

// Validate execution results
TestUtils::validate_success_result(&json)?;
TestUtils::validate_memory_limit_result(&json)?;
TestUtils::validate_time_limit_result(&json)?;

// Extract information
let stdout = TestUtils::extract_stdout(&json);
let memory_usage = TestUtils::extract_memory_usage(&json);
let wall_time = TestUtils::extract_wall_time(&json);
```

### Language Test Files

Language tests use embedded test files in `languages/test_files/`. To add new language tests:

1. Create test files in the appropriate language directory
2. Add test functions in `languages/mod.rs`
3. Update the test runner to include new tests

## 🔍 Troubleshooting

### Common Issues

1. **Permission denied**: Run with `sudo`
2. **Binary not found**: Build rustbox first with `cargo build --release`
3. **Cgroups unavailable**: Ensure cgroups v1 is mounted
4. **Memory issues**: Check available system memory

### Debug Mode

Enable verbose output for detailed debugging:

```bash
cargo run --bin tests category core  # Verbose by default
```

### System Requirements Check

Always run the system check first:

```bash
sudo cargo run --bin tests check
```

## 📈 Performance Targets

The test suite validates these performance characteristics:

- **Startup time**: < 0.5s average, < 1.0s maximum
- **Execution overhead**: < 0.2s
- **Base memory usage**: < 10MB
- **Throughput**: > 2 operations/second
- **Concurrent throughput**: > 1 operations/second

## 🔄 Migration from Shell Tests

This Rust-based test suite replaces the shell-based tests in `tests/` while maintaining compatibility with the reference implementation. Key improvements:

- **Better error handling** with detailed error messages
- **Parallel execution** support for stress tests
- **Comprehensive reporting** with metrics and timing
- **Type safety** with Rust's type system
- **Easier maintenance** with modular structure
- **Better integration** with CI/CD systems

## 📝 Contributing

When adding new tests:

1. Follow the existing patterns and structure
2. Add comprehensive error handling
3. Include performance targets where applicable
4. Update this README with new test descriptions
5. Ensure tests work with both root and non-root execution where possible