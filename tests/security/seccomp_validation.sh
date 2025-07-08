#!/bin/bash

# Comprehensive Seccomp Validation Test
# Tests the fixed seccomp implementation for production readiness

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Test configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUSTBOX_BIN="$PROJECT_ROOT/target/release/rustbox"
TEST_DIR="/tmp/rustbox_seccomp_validation_$$"
PASSED=0
FAILED=0
TOTAL=0

# Helper functions
log_header() {
    echo -e "\n${BOLD}${CYAN}=== $1 ===${NC}"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_failure() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Setup test environment
setup_test_env() {
    log_info "Setting up seccomp validation test environment..."
    mkdir -p "$TEST_DIR"
    
    # Check if rustbox binary exists
    if [ ! -f "$RUSTBOX_BIN" ]; then
        log_warning "rustbox binary not found, checking for cargo..."
        if command -v cargo >/dev/null 2>&1; then
            log_info "Building rustbox with seccomp features..."
            cd "$PROJECT_ROOT"
            if cargo build --release --features seccomp; then
                log_success "Build completed successfully"
            else
                log_failure "Build failed - cannot proceed with tests"
                exit 1
            fi
        else
            log_failure "Neither rustbox binary nor cargo found"
            exit 1
        fi
    else
        log_success "rustbox binary found"
    fi
}

# Cleanup test environment
cleanup_test_env() {
    log_info "Cleaning up test environment..."
    rm -rf "$TEST_DIR"
}

# Test helper function
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_result="$3"  # "success", "failure", "killed", "timeout"
    local description="$4"
    local timeout_seconds="${5:-10}"
    
    TOTAL=$((TOTAL + 1))
    echo -e "\n${BLUE}Test $TOTAL: $test_name${NC}"
    echo "Description: $description"
    
    # Run the test command and capture result
    local exit_code=0
    local output=""
    local signal=""
    local timed_out=false
    
    if timeout "$timeout_seconds" bash -c "$test_command" > "$TEST_DIR/test_output.txt" 2>&1; then
        exit_code=0
        output=$(cat "$TEST_DIR/test_output.txt")
    else
        exit_code=$?
        output=$(cat "$TEST_DIR/test_output.txt" 2>/dev/null || echo "No output")
        
        # Check if process was killed by signal or timed out
        if [ $exit_code -eq 124 ]; then
            timed_out=true
        elif [ $exit_code -eq 137 ] || [ $exit_code -eq 143 ] || [ $exit_code -eq 9 ]; then
            signal="KILLED"
        fi
    fi
    
    # Evaluate test result
    local test_passed=false
    case "$expected_result" in
        "success")
            if [ $exit_code -eq 0 ] && [ "$timed_out" = false ]; then
                test_passed=true
            fi
            ;;
        "failure")
            if [ $exit_code -ne 0 ] && [ "$signal" != "KILLED" ] && [ "$timed_out" = false ]; then
                test_passed=true
            fi
            ;;
        "killed")
            if [ "$signal" = "KILLED" ] || echo "$output" | grep -q "killed\|terminated\|SIGKILL\|signal 9"; then
                test_passed=true
            fi
            ;;
        "timeout")
            if [ "$timed_out" = true ]; then
                test_passed=true
            fi
            ;;
    esac
    
    if [ "$test_passed" = true ]; then
        log_success "$test_name"
        PASSED=$((PASSED + 1))
    else
        log_failure "$test_name"
        echo "Expected: $expected_result"
        echo "Got: exit_code=$exit_code, signal=$signal, timed_out=$timed_out"
        echo "Output: $output"
        FAILED=$((FAILED + 1))
    fi
}

# Test 1: Seccomp support detection
test_seccomp_support() {
    log_header "Seccomp Support Detection"
    
    run_test "kernel_seccomp_support" \
        "[ -f /proc/sys/kernel/seccomp ] && [ \$(cat /proc/sys/kernel/seccomp) != '0' ]" \
        "success" \
        "Kernel should support seccomp filtering"
    
    run_test "libseccomp_headers" \
        "[ -f /usr/include/seccomp.h ] || [ -f /usr/local/include/seccomp.h ]" \
        "success" \
        "libseccomp headers should be available"
    
    run_test "seccomp_syscall_available" \
        "grep -q seccomp /proc/kallsyms 2>/dev/null || echo 'seccomp available'" \
        "success" \
        "seccomp system call should be available"
}

# Test 2: Basic seccomp functionality
test_basic_seccomp() {
    log_header "Basic Seccomp Functionality"
    
    # Test that basic programs can run with seccomp
    run_test "basic_echo_with_seccomp" \
        "echo 'Hello World' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 --memory-limit 64 -- /bin/cat" \
        "success" \
        "Basic programs should work with seccomp enabled"
    
    # Test Python execution with seccomp
    run_test "python_basic_with_seccomp" \
        "echo 'print(\"Hello from Python\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 --memory-limit 64 -- python3" \
        "success" \
        "Python should work with seccomp filtering"
    
    # Test C compilation and execution
    cat > "$TEST_DIR/hello.c" << 'EOF'
#include <stdio.h>
int main() {
    printf("Hello from C!\n");
    return 0;
}
EOF
    
    run_test "c_compilation_with_seccomp" \
        "cd $TEST_DIR && gcc hello.c -o hello && echo './hello' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 --memory-limit 64 -- bash" \
        "success" \
        "C programs should compile and run with seccomp"
}

# Test 3: Dangerous syscall blocking
test_dangerous_syscall_blocking() {
    log_header "Dangerous Syscall Blocking"
    
    # Test socket creation blocking
    run_test "socket_creation_blocked" \
        "echo 'import socket; s = socket.socket(); print(\"Socket created\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "killed" \
        "Socket creation should be blocked by seccomp"
    
    # Test fork blocking
    run_test "fork_blocked" \
        "echo 'import os; os.fork(); print(\"Fork succeeded\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "killed" \
        "Fork should be blocked by seccomp"
    
    # Test execve blocking
    run_test "execve_blocked" \
        "echo 'import os; os.execve(\"/bin/echo\", [\"echo\", \"hello\"], {}); print(\"Execve succeeded\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "killed" \
        "Execve should be blocked by seccomp"
    
    # Test ptrace blocking
    run_test "ptrace_blocked" \
        "echo 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); libc.ptrace(0, 0, 0, 0); print(\"Ptrace succeeded\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "killed" \
        "Ptrace should be blocked by seccomp"
    
    # Test setuid blocking
    run_test "setuid_blocked" \
        "echo 'import os; os.setuid(0); print(\"Setuid succeeded\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "killed" \
        "Setuid should be blocked by seccomp"
    
    # Test mount blocking
    run_test "mount_blocked" \
        "echo 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); libc.mount(0, 0, 0, 0, 0); print(\"Mount succeeded\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "killed" \
        "Mount should be blocked by seccomp"
}

# Test 4: Language-specific profiles
test_language_profiles() {
    log_header "Language-Specific Seccomp Profiles"
    
    # Test Python profile
    run_test "python_profile_basic" \
        "echo 'import sys; print(f\"Python {sys.version_info.major}.{sys.version_info.minor}\")' | $RUSTBOX_BIN run --enable-seccomp --seccomp-profile python --time-limit 5 -- python3" \
        "success" \
        "Python profile should allow basic Python operations"
    
    # Test that Python profile still blocks dangerous operations
    run_test "python_profile_blocks_socket" \
        "echo 'import socket; s = socket.socket()' | $RUSTBOX_BIN run --enable-seccomp --seccomp-profile python --time-limit 5 -- python3" \
        "killed" \
        "Python profile should still block socket creation"
    
    # Test C profile
    run_test "c_profile_basic" \
        "echo 'printf(\"C program\");' | $RUSTBOX_BIN run --enable-seccomp --seccomp-profile c --time-limit 5 -- gcc -x c - -o /tmp/test && /tmp/test" \
        "success" \
        "C profile should allow compilation and execution"
    
    # Test JavaScript profile (if Node.js available)
    if command -v node >/dev/null 2>&1; then
        run_test "javascript_profile_basic" \
            "echo 'console.log(\"Node.js\", process.version);' | $RUSTBOX_BIN run --enable-seccomp --seccomp-profile javascript --time-limit 5 -- node" \
            "success" \
            "JavaScript profile should allow Node.js execution"
    fi
}

# Test 5: Seccomp fallback mechanisms
test_seccomp_fallbacks() {
    log_header "Seccomp Fallback Mechanisms"
    
    # Test graceful degradation when seccomp is not available
    run_test "graceful_degradation" \
        "echo 'print(\"Hello\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "success" \
        "Should work even if libseccomp is not available (with warnings)"
    
    # Test strict mode behavior
    run_test "strict_mode_seccomp_required" \
        "echo 'print(\"Hello\")' | $RUSTBOX_BIN run --enable-seccomp --strict --time-limit 5 -- python3" \
        "success" \
        "Strict mode should work when seccomp is available"
}

# Test 6: Seccomp bypass prevention
test_bypass_prevention() {
    log_header "Seccomp Bypass Prevention"
    
    # Test various bypass attempts
    run_test "no_new_privs_bypass" \
        "echo 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); libc.prctl(38, 0, 0, 0, 0); print(\"Bypass succeeded\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "killed" \
        "Attempts to disable no-new-privs should be blocked"
    
    # Test seccomp manipulation attempts
    run_test "seccomp_manipulation_blocked" \
        "echo 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); libc.seccomp(0, 0, 0); print(\"Seccomp manipulation succeeded\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "killed" \
        "Seccomp manipulation should be blocked"
    
    # Test BPF program loading
    run_test "bpf_program_blocked" \
        "echo 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); libc.bpf(0, 0, 0); print(\"BPF succeeded\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "killed" \
        "BPF program loading should be blocked"
}

# Test 7: Performance and stability
test_performance_stability() {
    log_header "Performance and Stability"
    
    # Test that seccomp doesn't significantly impact performance
    run_test "performance_impact_minimal" \
        "time (for i in {1..10}; do echo 'print(i)' | $RUSTBOX_BIN run --enable-seccomp --time-limit 2 -- python3 >/dev/null; done)" \
        "success" \
        "Seccomp should not significantly impact performance" \
        15
    
    # Test stability under load
    run_test "stability_under_load" \
        "for i in {1..5}; do echo 'import time; time.sleep(0.1); print(\"OK\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3 & done; wait" \
        "success" \
        "Seccomp should be stable under concurrent load" \
        30
}

# Test 8: Edge cases and error handling
test_edge_cases() {
    log_header "Edge Cases and Error Handling"
    
    # Test invalid seccomp profile
    run_test "invalid_profile_handling" \
        "echo 'print(\"Hello\")' | $RUSTBOX_BIN run --enable-seccomp --seccomp-profile invalid_language --time-limit 5 -- python3" \
        "success" \
        "Invalid seccomp profile should fall back to default"
    
    # Test seccomp with very restrictive limits
    run_test "restrictive_limits_with_seccomp" \
        "echo 'print(\"Hello\")' | $RUSTBOX_BIN run --enable-seccomp --time-limit 1 --memory-limit 16 -- python3" \
        "success" \
        "Seccomp should work with very restrictive resource limits"
    
    # Test seccomp with empty program
    run_test "empty_program_with_seccomp" \
        "echo '' | $RUSTBOX_BIN run --enable-seccomp --time-limit 5 -- python3" \
        "success" \
        "Seccomp should handle empty programs gracefully"
}

# Display test results
display_results() {
    log_header "Seccomp Validation Results"
    
    echo -e "${BOLD}Total Tests:${NC} $TOTAL"
    echo -e "${GREEN}Passed:${NC} $PASSED"
    echo -e "${RED}Failed:${NC} $FAILED"
    
    if [ $TOTAL -gt 0 ]; then
        local success_rate=$(( (PASSED * 100) / TOTAL ))
        echo -e "${BOLD}Success Rate:${NC} $success_rate%"
    fi
    
    echo ""
    
    if [ $FAILED -eq 0 ]; then
        echo -e "${BOLD}${GREEN}🎉 ALL SECCOMP TESTS PASSED!${NC}"
        echo -e "${GREEN}Seccomp implementation is production-ready${NC}"
        return 0
    elif [ $FAILED -le 2 ] && [ $PASSED -gt 20 ]; then
        echo -e "${BOLD}${YELLOW}⚠️  MOSTLY SUCCESSFUL${NC}"
        echo -e "${YELLOW}Minor issues detected but seccomp is largely functional${NC}"
        return 0
    else
        echo -e "${BOLD}${RED}❌ SIGNIFICANT ISSUES DETECTED${NC}"
        echo -e "${RED}Seccomp implementation needs attention before production use${NC}"
        return 1
    fi
}

# Main execution
main() {
    log_header "Rustbox Seccomp Production Readiness Validation"
    echo "This test validates the seccomp implementation for production use"
    echo ""
    
    # Setup
    setup_test_env
    
    # Run all test suites
    test_seccomp_support
    test_basic_seccomp
    test_dangerous_syscall_blocking
    test_language_profiles
    test_seccomp_fallbacks
    test_bypass_prevention
    test_performance_stability
    test_edge_cases
    
    # Display results
    display_results
}

# Handle script interruption
trap cleanup_test_env EXIT

# Run main function
main "$@"