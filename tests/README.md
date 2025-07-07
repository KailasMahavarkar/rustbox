# Mini-Isolate Test Suite

This directory contains comprehensive tests for the Mini-Isolate sandboxing system, organized into logical categories for easy maintenance and execution.

## Test Organization

**Note**: Legacy Rust test directories have been removed as they referenced non-existent functions and were not functional. The current structure focuses on working shell-based tests that actually test the compiled mini-isolate binary.

### 📁 Core Tests (`tests/core/`)
Basic functionality and essential features
- **quick_core_test.sh** - Core functionality validation including basic commands, resource limits, and parallel execution

### 📁 Resource Tests (`tests/resource/`)
Resource limit enforcement and management
- **resource_test.sh** - Memory, CPU, and wall-time limit validation

### 📁 Stress Tests (`tests/stress/`)
Load testing and scalability validation
- **sequential.sh** - Sequential execution of 5 isolates with high resource usage
- **parallel.sh** - Concurrent execution of 50 isolates with resource limits
- **stress_program.py** - Python stress testing utility

### 📁 Security Tests (`tests/security/`)
Isolation and security feature validation
- **isolation_test.sh** - Process, filesystem, network, and user namespace isolation  
- **comprehensive_security.sh** - Advanced security testing including attack vector prevention
- **malicious.py** - Python utility for testing security boundaries


### 📁 Integration Tests (`tests/integration/`)
End-to-end workflow and complex scenario testing
- **end_to_end.sh** - Complete workflows, resource recovery, and concurrent management

### 📁 Performance Tests (`tests/performance/`)
Performance benchmarks and optimization metrics
- **benchmark.sh** - Startup time, execution overhead, memory usage, and throughput testing

## Running Tests

### Quick Start
```bash
# Run all tests in a category
sudo ./run_category.sh core
sudo ./run_category.sh security
sudo ./run_category.sh all

# Run specific test within category
sudo ./run_category.sh stress parallel
sudo ./run_category.sh performance benchmark
```

### Individual Test Execution
```bash
# Core functionality
sudo ./core/quick_core_test.sh

# Resource limits
sudo ./resource/resource_test.sh

# Security isolation
sudo ./security/isolation_test.sh

# Performance benchmarks
sudo ./performance/benchmark.sh

# Stress testing
sudo ./stress/sequential.sh
sudo ./stress/parallel.sh
```

## Test Categories Explained

### Core Tests
Essential functionality that must always work:
- Basic command execution
- Resource limit enforcement (memory, CPU, wall-time)
- Parallel isolate management
- Init and cleanup operations

**Expected Results**: 100% pass rate for stable release

### Resource Tests
Validation of resource management:
- Low memory limit scenarios
- CPU time restrictions
- Wall clock time limits
- Resource exhaustion handling

**Expected Results**: All limits properly enforced

### Stress Tests
System capability under load:
- Sequential high-resource operations
- Concurrent isolate execution
- Resource contention scenarios
- Scaling validation

**Expected Results**: 90%+ success rate at target scale

### Security Tests
Isolation and containment validation:
- Process namespace isolation
- Filesystem access control
- Network isolation
- User context separation

**Expected Results**: All isolation features working correctly

### Integration Tests
Complex workflow validation:
- Multi-step operations
- Error recovery scenarios
- Resource limit recovery
- Concurrent management

**Expected Results**: End-to-end workflows function correctly

### Performance Tests
System performance characteristics:
- Startup time (target: <0.5s)
- Execution overhead (target: <0.2s)
- Memory efficiency (target: <10MB)
- Throughput (target: >2 ops/sec)

**Expected Results**: Performance within acceptable bounds

## Requirements

- **Root Privileges**: All tests require sudo access for namespace and resource management
- **System Dependencies**: 
  - Linux kernel with cgroups v1 support
  - Namespace support
  - Seccomp support
  - Python 3 (for stress tests)
- **Mini-Isolate Binary**: Must be built at `../target/release/mini-isolate`

## Test Output Format

Each test category provides:
- ✅ **PASS** - Test passed successfully
- ❌ **FAIL** - Test failed with error details
- 🤔 **WARNING** - Unexpected but handled result
- 📊 **BENCHMARK** - Performance metric

## Adding New Tests

### Adding to Existing Category
1. Create `.sh` script in appropriate category directory
2. Follow existing naming convention
3. Include header comments explaining purpose
4. Use consistent output formatting
5. Make executable with `chmod +x`

### Creating New Category
1. Create new directory under `tests/`
2. Add category to `CATEGORIES` array in `run_category.sh`
3. Update this README with category description
4. Create category-specific tests

### Test Script Template
```bash
#!/bin/bash

# [Category] Tests for Mini-Isolate
# Description of test purpose

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/mini-isolate"

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== [Category] Tests ====="
# Test implementation here
```

## Test Maintenance

- **Regular Execution**: Run `sudo ./run_category.sh all` before releases
- **Performance Monitoring**: Track benchmark results over time
- **Failure Investigation**: Check specific category when CI/CD fails
- **Test Updates**: Update tests when adding new features

## Troubleshooting

### Common Issues
- **Permission Denied**: Ensure running with sudo
- **Binary Not Found**: Build mini-isolate first with `cargo build --release`
- **Resource Limits**: Ensure system supports cgroups v1
- **Namespace Issues**: Check kernel namespace support

### Debug Mode
Add `-x` flag to any test script for detailed execution tracing:
```bash
sudo bash -x ./core/quick_core_test.sh
```

## Contributing

When adding new tests:
1. Follow existing patterns and conventions
2. Include comprehensive error handling
3. Provide clear success/failure indicators
4. Update this README with new test descriptions
5. Test on clean system before submitting

## Test Status

Last Updated: $(date)
Current Status: All test categories implemented and functional
Recommended Usage: Run `core` and `resource` tests for basic validation, `all` for comprehensive testing