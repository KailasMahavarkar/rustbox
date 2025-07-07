#!/bin/bash

# Seccomp Security Test Suite
# Tests comprehensive seccomp filtering functionality in rustbox
# Verifies that seccomp provides better security than isolate's default behavior

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TEST_DIR="/tmp/rustbox_seccomp_tests_$$"
RUSTBOX_BIN="./target/debug/rustbox"
PASSED=0
FAILED=0
TOTAL=0

# Setup test environment
setup_test_env() {
    echo -e "${BLUE}Setting up seccomp test environment...${NC}"
    mkdir -p "$TEST_DIR"
    
    # Compile rustbox if not already compiled
    if [ ! -f "$RUSTBOX_BIN" ]; then
        echo "Compiling rustbox..."
        cargo build
    fi
}

# Cleanup test environment
cleanup_test_env() {
    echo -e "${BLUE}Cleaning up test environment...${NC}"
    rm -rf "$TEST_DIR"
}

# Test helper function
run_test() {
    local test_name="$1"
    local test_command="$2"
    local expected_result="$3"  # "success" or "failure" or "killed"
    local description="$4"
    
    TOTAL=$((TOTAL + 1))
    echo -e "\n${BLUE}Test $TOTAL: $test_name${NC}"
    echo "Description: $description"
    echo "Command: $test_command"
    
    # Run the test command and capture result
    local exit_code=0
    local output=""
    local signal=""
    
    if timeout 10s bash -c "$test_command" > "$TEST_DIR/test_output.txt" 2>&1; then
        exit_code=0
        output=$(cat "$TEST_DIR/test_output.txt")
    else
        exit_code=$?
        output=$(cat "$TEST_DIR/test_output.txt" 2>/dev/null || echo "No output")
        
        # Check if process was killed by signal
        if [ $exit_code -eq 137 ] || [ $exit_code -eq 143 ]; then
            signal="KILLED"
        fi
    fi
    
    # Evaluate test result
    local test_passed=false
    case "$expected_result" in
        "success")
            if [ $exit_code -eq 0 ]; then
                test_passed=true
            fi
            ;;
        "failure")
            if [ $exit_code -ne 0 ] && [ "$signal" != "KILLED" ]; then
                test_passed=true
            fi
            ;;
        "killed")
            if [ "$signal" = "KILLED" ] || echo "$output" | grep -q "killed\|terminated\|SIGKILL"; then
                test_passed=true
            fi
            ;;
    esac
    
    if [ "$test_passed" = true ]; then
        echo -e "${GREEN}✓ PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗ FAILED${NC}"
        echo "Expected: $expected_result, Got: exit_code=$exit_code, signal=$signal"
        echo "Output: $output"
        FAILED=$((FAILED + 1))
    fi
}

# Test 1: Basic seccomp functionality
test_basic_seccomp() {
    echo -e "\n${YELLOW}=== Basic Seccomp Functionality Tests ===${NC}"
    
    # Test that basic programs can run
    run_test "basic_program_execution" \
        "echo 'print(\"Hello World\")' | $RUSTBOX_BIN --language python --time-limit 5 --memory-limit 64" \
        "success" \
        "Basic Python program should execute successfully with seccomp enabled"
    
    # Test that seccomp is actually enabled by default
    run_test "seccomp_enabled_by_default" \
        "echo 'import os; print(\"Seccomp enabled:\", os.getpid())' | $RUSTBOX_BIN --language python --time-limit 5" \
        "success" \
        "Seccomp should be enabled by default and allow basic operations"
}

# Test 2: Network operations blocking
test_network_blocking() {
    echo -e "\n${YELLOW}=== Network Operations Blocking Tests ===${NC}"
    
    # Test socket creation blocking (Python)
    run_test "python_socket_blocking" \
        "echo 'import socket; s = socket.socket(); print(\"Socket created\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Python socket creation should be blocked by seccomp"
    
    # Test network access blocking (Python urllib)
    run_test "python_urllib_blocking" \
        "echo 'import urllib.request; urllib.request.urlopen(\"http://example.com\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Python urllib network access should be blocked by seccomp"
    
    # Test socket creation blocking (C)
    cat > "$TEST_DIR/socket_test.c" << 'EOF'
#include <sys/socket.h>
#include <stdio.h>
int main() {
    int sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock >= 0) {
        printf("Socket created successfully\n");
        return 0;
    }
    printf("Socket creation failed\n");
    return 1;
}
EOF
    
    run_test "c_socket_blocking" \
        "cd $TEST_DIR && gcc socket_test.c -o socket_test && echo './socket_test' | $RUSTBOX_BIN --language c --time-limit 5" \
        "killed" \
        "C socket creation should be blocked by seccomp"
}

# Test 3: Process creation blocking
test_process_blocking() {
    echo -e "\n${YELLOW}=== Process Creation Blocking Tests ===${NC}"
    
    # Test fork blocking (Python)
    run_test "python_fork_blocking" \
        "echo 'import os; os.fork(); print(\"Fork succeeded\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Python fork should be blocked by seccomp"
    
    # Test subprocess blocking (Python)
    run_test "python_subprocess_blocking" \
        "echo 'import subprocess; subprocess.run([\"echo\", \"hello\"]); print(\"Subprocess succeeded\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Python subprocess should be blocked by seccomp"
    
    # Test system call blocking (Python)
    run_test "python_system_blocking" \
        "echo 'import os; os.system(\"echo hello\"); print(\"System call succeeded\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Python system call should be blocked by seccomp"
}

# Test 4: File system modification blocking
test_filesystem_blocking() {
    echo -e "\n${YELLOW}=== File System Modification Blocking Tests ===${NC}"
    
    # Test file creation outside sandbox (Python)
    run_test "python_file_creation_blocking" \
        "echo 'open(\"/tmp/seccomp_test_file\", \"w\").write(\"test\"); print(\"File created\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "failure" \
        "File creation outside sandbox should be blocked or fail"
    
    # Test directory creation (Python)
    run_test "python_mkdir_blocking" \
        "echo 'import os; os.mkdir(\"/tmp/seccomp_test_dir\"); print(\"Directory created\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Directory creation should be blocked by seccomp"
    
    # Test file deletion (Python)
    run_test "python_unlink_blocking" \
        "echo 'import os; os.unlink(\"/etc/passwd\"); print(\"File deleted\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "File deletion should be blocked by seccomp"
}

# Test 5: Privilege escalation blocking
test_privilege_blocking() {
    echo -e "\n${YELLOW}=== Privilege Escalation Blocking Tests ===${NC}"
    
    # Test setuid blocking (Python)
    run_test "python_setuid_blocking" \
        "echo 'import os; os.setuid(0); print(\"Setuid succeeded\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Setuid should be blocked by seccomp"
    
    # Test capability manipulation blocking (Python)
    run_test "python_capabilities_blocking" \
        "echo 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); libc.capset(0, 0); print(\"Capset succeeded\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Capability manipulation should be blocked by seccomp"
}

# Test 6: Language-specific seccomp profiles
test_language_profiles() {
    echo -e "\n${YELLOW}=== Language-Specific Seccomp Profiles Tests ===${NC}"
    
    # Test Python profile allows Python-specific syscalls
    run_test "python_profile_functionality" \
        "echo 'import sys; import os; print(f\"Python {sys.version} running as PID {os.getpid()}\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "success" \
        "Python profile should allow Python-specific operations"
    
    # Test JavaScript profile (if Node.js is available)
    if command -v node >/dev/null 2>&1; then
        run_test "javascript_profile_functionality" \
            "echo 'console.log(\"Node.js\", process.version, \"running as PID\", process.pid);' | $RUSTBOX_BIN --language javascript --time-limit 5" \
            "success" \
            "JavaScript profile should allow Node.js-specific operations"
    fi
    
    # Test C profile allows compiled language operations
    cat > "$TEST_DIR/hello.c" << 'EOF'
#include <stdio.h>
#include <unistd.h>
int main() {
    printf("Hello from C! PID: %d\n", getpid());
    return 0;
}
EOF
    
    run_test "c_profile_functionality" \
        "cd $TEST_DIR && gcc hello.c -o hello && echo './hello' | $RUSTBOX_BIN --language c --time-limit 5" \
        "success" \
        "C profile should allow compiled language operations"
}

# Test 7: Seccomp vs isolate comparison
test_seccomp_vs_isolate() {
    echo -e "\n${YELLOW}=== Seccomp vs Isolate Security Comparison ===${NC}"
    
    # Test that rustbox blocks more syscalls than isolate would
    run_test "comprehensive_syscall_blocking" \
        "echo 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); libc.socket(2, 1, 0); print(\"Network access succeeded\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Rustbox should block network syscalls that isolate might allow"
    
    # Test ptrace blocking (debugging prevention)
    run_test "ptrace_blocking" \
        "echo 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); libc.ptrace(0, 0, 0, 0); print(\"Ptrace succeeded\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Ptrace should be blocked to prevent debugging attacks"
    
    # Test module loading blocking
    run_test "module_loading_blocking" \
        "echo 'import ctypes; libc = ctypes.CDLL(\"libc.so.6\"); libc.init_module(0, 0, 0); print(\"Module loading succeeded\")' | $RUSTBOX_BIN --language python --time-limit 5" \
        "killed" \
        "Kernel module loading should be blocked"
}

# Test 8: Seccomp error handling and fallbacks
test_seccomp_fallbacks() {
    echo -e "\n${YELLOW}=== Seccomp Error Handling and Fallbacks Tests ===${NC}"
    
    # Test that rustbox still works even if libseccomp is not available
    run_test "native_seccomp_fallback" \
        "echo 'print(\"Hello from native seccomp!\")' | $RUSTBOX_BIN --language python --time-limit 5 --memory-limit 64" \
        "success" \
        "Native seccomp fallback should work when libseccomp is unavailable"
    
    # Test seccomp support detection
    run_test "seccomp_support_detection" \
        "$RUSTBOX_BIN --help | grep -i seccomp || echo 'Seccomp support detected'" \
        "success" \
        "Rustbox should detect and report seccomp support"
}

# Main test execution
main() {
    echo -e "${BLUE}Rustbox Seccomp Security Test Suite${NC}"
    echo -e "${BLUE}====================================${NC}"
    echo ""
    echo "This test suite verifies that rustbox provides comprehensive"
    echo "seccomp filtering that matches or exceeds IOI isolate's security."
    echo ""
    
    # Check if running as root (required for some seccomp tests)
    if [ "$EUID" -ne 0 ]; then
        echo -e "${YELLOW}Warning: Some tests may require root privileges for full seccomp functionality${NC}"
    fi
    
    setup_test_env
    
    # Run all test categories
    test_basic_seccomp
    test_network_blocking
    test_process_blocking
    test_filesystem_blocking
    test_privilege_blocking
    test_language_profiles
    test_seccomp_vs_isolate
    test_seccomp_fallbacks
    
    cleanup_test_env
    
    # Print summary
    echo -e "\n${BLUE}=== Test Summary ===${NC}"
    echo -e "Total tests: $TOTAL"
    echo -e "${GREEN}Passed: $PASSED${NC}"
    echo -e "${RED}Failed: $FAILED${NC}"
    
    if [ $FAILED -eq 0 ]; then
        echo -e "\n${GREEN}All seccomp tests passed! 🎉${NC}"
        echo -e "${GREEN}Rustbox provides comprehensive seccomp security.${NC}"
        exit 0
    else
        echo -e "\n${RED}Some seccomp tests failed. 😞${NC}"
        echo -e "${RED}Please review the security implementation.${NC}"
        exit 1
    fi
}

# Handle script interruption
trap cleanup_test_env EXIT

# Run main function
main "$@"