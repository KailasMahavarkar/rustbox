# Mini-Isolate Test Suite Documentation

This document describes the comprehensive test suite for Mini-Isolate, covering functionality, security, performance, and code quality.

## Test Structure

### Integration Tests (`tests/lib.rs`)
The test suite is organized in a modular structure with external Python scripts for better maintainability:

- **Basic Tests**: Core isolate functionality (`tests/basic/`)
  - Isolate creation and cleanup (`tests/basic/mod.rs`)
  - Command execution and basic functionality
  - Cgroup controller functionality (`tests/basic/cgroup.rs`)

- **Memory Tests**: Memory limit enforcement (`tests/memory/`)
  - External Python script: `memory_test.py`

- **Timeout Tests**: Time limit enforcement (`tests/timeout/`)
  - External Python script: `cpu_intensive.py`

- **Process Tests**: Process isolation and limits (`tests/process/`)
  - External Python script: `fork_bomb.py`

- **File Size Tests**: File size limit enforcement (`tests/filesize/`)
  - External Python script: `large_file_creator.py`

- **Security Tests**: Security isolation (`tests/security/`)
  - External Python script: `malicious.py`

- **Language Tests**: Multi-language support (`tests/language/`)
  - External Python script: `test.py`

- **I/O Tests**: Input/output handling (`tests/io/`)
  - External Python script: `math.py`

- **Invalid Tests**: Error handling for invalid inputs (`tests/invalid/`)
- **Concurrent Tests**: Multiple isolate instances (`tests/concurrent/`)

All tests use external Python scripts in their respective directories for improved organization and reusability.

## Test Categories

### 1. Memory Isolation Tests

**Purpose**: Verify memory limits are properly enforced

**Test Cases**:
- Small allocation within limits (should succeed)
- Large allocation exceeding limits (should fail/be limited)
- Progressive memory allocation with monitoring
- Memory leak prevention

**Expected Behavior**:
- With root: Cgroup-enforced memory limits
- Without root: Process-level limits with warnings

**Example**:
```bash
cargo test memory -- --nocapture
```

### 2. Fork Bomb Protection Tests

**Purpose**: Test process limits and fork bomb containment

**Test Cases**:
- Controlled fork testing within limits
- Fork bomb attempts (should be contained)
- Process count enforcement
- Resource cleanup after limit breach

**External Script**: `tests/process/fork_bomb.py`

**Example**:
```bash
cargo test process -- --nocapture
```

### 3. Cgroup Functionality Tests

**Purpose**: Verify cgroup integration works correctly

**Test Cases**:
- Cgroup creation and configuration
- Memory limit setting via cgroups
- Process limit enforcement
- Cleanup and removal

**Requirements**:
- Root privileges for full testing
- Linux cgroups v1 support

**Example**:
```bash
sudo cargo test cgroup -- --nocapture
```

### 4. Security Isolation Tests

**Purpose**: Ensure proper security isolation

**Test Cases**:
- File access restriction verification
- Command injection prevention
- Path traversal attack prevention
- Privilege escalation blocking
- Network isolation testing

**External Script**: `tests/security/malicious.py`

## Running Tests

### Prerequisites

1. **System Requirements**:
   - Linux with cgroups support (for full functionality)
   - Python 3.x for external test scripts
   - Sufficient permissions for cgroup tests

2. **Install test dependencies**:
   ```bash
   cargo test --no-run
   ```

3. **Ensure permissions** (for full testing):
   ```bash
   # Run with sudo for cgroup tests
   sudo cargo test
   ```

### Test Execution

#### Run All Tests
```bash
# Basic test run
cargo test

# With output
cargo test -- --nocapture

# Include ignored tests
cargo test -- --include-ignored

# Single-threaded (recommended for integration tests)
cargo test -- --test-threads=1
```

#### Run Specific Test Categories

```bash
# All tests
cargo test

# With debug output
RUST_LOG=debug cargo test -- --nocapture

# Tests requiring root privileges for full cgroup functionality  
sudo cargo test

# Run tests in single-threaded mode (for integration tests)
cargo test -- --test-threads=1
```

#### Run Individual Tests

```bash
# Specific test function
cargo test basic

# Tests matching pattern
cargo test memory

# With detailed output
cargo test cgroup -- --nocapture
```

### Root Privilege Tests

Some tests require root privileges for full cgroup functionality:

```bash
# Run all tests with root
sudo cargo test

# Run only cgroup-specific tests
sudo cargo test cgroup

# With debug output
sudo RUST_LOG=debug cargo test
```

### Test Environment Setup

For comprehensive testing, ensure:

1. **Linux system** with cgroups v1 support
2. **Root access** for cgroup functionality tests
3. **Python 3.x** installed for external scripts
4. **Write permissions** to `/tmp` for temporary files
5. **Available memory** for memory limit tests
6. **Clean state** between test runs

## Test Results Interpretation

### Success Indicators

- **All tests pass**: System functions correctly
- **Warnings about cgroups**: Expected without root privileges
- **Performance metrics**: Logged execution times

### Common Issues

- **Cgroup test failures**: Usually permission-related
- **Memory test failures**: Insufficient isolation
- **Timeout issues**: Resource contention or slow system

When tests fail:

1. **Check permissions**: Ensure proper access rights
2. **Verify dependencies**: Check Python availability and scripts
3. **Verify system state**: Ensure clean test environment
4. **Review logs**: Use `RUST_LOG=debug` for detailed output

### Example Test Output

```
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

Basic functionality verified
Cgroup integration tested (with limitations without root)
Command execution working correctly
```

## Test Maintenance

### Automated Testing

Example GitHub Actions workflow:

```yaml
- name: Run Tests
  run: |
    cargo test --lib
    cargo test basic
    # Note: cgroup tests require root and may not work in CI
```

### Test Coverage

The current test suite covers:
- **Core functionality**: 100% of basic operations
- **Error handling**: Major error paths
- **Security**: Key isolation features
- **Performance**: Basic benchmarking

### Performance Testing

```bash
# Benchmark specific tests
cargo test basic -- --nocapture | grep "execution time"
```

## Troubleshooting

### Common Problems

1. **Permission denied errors**
   - Solution: Run with `sudo` for full cgroup testing
   - Alternative: Tests will run with warnings but limited functionality

2. **Python script missing errors**
   - Solution: Ensure Python 3.x is installed
   - Check: External script files exist in test directories

3. **Tests hanging or timing out**
   - Solution: Use single-threaded mode
   - Try: Single-threaded execution with `--test-threads=1`

4. **Cgroup functionality limited**
   - Check: `/sys/fs/cgroup` availability
   - Verify: cgroups v1 support

5. **Memory tests not enforcing limits**
   - Note: Some tests will skip automatically
   - Verify: System has sufficient memory for testing

### Debug Mode

For detailed troubleshooting:
```bash
RUST_LOG=debug cargo test test_name -- --nocapture
```

### Manual Testing

For interactive testing:

```bash
# Build and test manually
cargo build --release

# Initialize test environment
./target/release/mini-isolate init --box-id manual_test --mem 64

# Run test script
echo 'print("Manual test")' > test.py
./target/release/mini-isolate execute --box-id manual_test --source test.py --verbose

# Cleanup
./target/release/mini-isolate cleanup --box-id manual_test
```

## Contributing Tests

When adding new functionality:

1. **Add integration tests** in appropriate `tests/*/mod.rs` file
2. **Create external scripts** if needed in test directories
3. **Update documentation** with new test cases
4. **Verify all tests pass** before submitting

### Test Naming Convention

- `test_feature_specific_case`: Integration tests
- `test_functionality_scenario`: Scenario-based tests

### Test Requirements

- Tests must be deterministic
- Use helper functions from `tests/lib.rs`
- Include both positive and negative test cases
- Clean up resources properly

This comprehensive test suite ensures Mini-Isolate functions correctly across various scenarios and system configurations while maintaining security and performance standards.