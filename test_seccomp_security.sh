#!/bin/bash

# Comprehensive Seccomp Security Test Suite
# Tests rustbox seccomp filtering against various attack vectors

set -e

RUSTBOX_BIN="./target/debug/rustbox"
TEST_DIR="/tmp/rustbox_security_test"
RESULTS_FILE="/tmp/seccomp_test_results.txt"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Logging function
log_test() {
    local test_name="$1"
    local expected="$2"
    local actual="$3"
    local details="$4"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if [ "$expected" = "$actual" ]; then
        echo -e "${GREEN}✓ PASS${NC}: $test_name"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        echo "PASS: $test_name - $details" >> "$RESULTS_FILE"
    else
        echo -e "${RED}✗ FAIL${NC}: $test_name"
        echo -e "  Expected: $expected, Got: $actual"
        echo -e "  Details: $details"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        echo "FAIL: $test_name - Expected: $expected, Got: $actual - $details" >> "$RESULTS_FILE"
    fi
}

# Setup test environment
setup_test_env() {
    echo -e "${YELLOW}Setting up test environment...${NC}"
    rm -rf "$TEST_DIR"
    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"
    
    # Clear previous results
    echo "Seccomp Security Test Results - $(date)" > "$RESULTS_FILE"
    echo "================================================" >> "$RESULTS_FILE"
    
    # Build rustbox if needed
    if [ ! -f "$RUSTBOX_BIN" ]; then
        echo "Building rustbox..."
        cd /mnt/c/Users/Admin/Desktop/mini-isolate
        cargo build
        cd "$TEST_DIR"
    fi
}

# Test 1: Basic seccomp functionality
test_basic_seccomp() {
    echo -e "\n${YELLOW}Testing basic seccomp functionality...${NC}"
    
    # Test that allowed syscalls work
    local output
    output=$(timeout 5 "$RUSTBOX_BIN" run --enable-seccomp --max-time=3 -- /bin/echo "Hello World" 2>&1)
    local exit_code=$?
    
    if [ $exit_code -eq 0 ] && echo "$output" | grep -q "Hello World"; then
        log_test "Basic allowed syscalls" "SUCCESS" "SUCCESS" "echo command executed successfully"
    else
        log_test "Basic allowed syscalls" "SUCCESS" "FAILED" "echo command failed: $output"
    fi
}

# Test 2: Network syscall blocking
test_network_blocking() {
    echo -e "\n${YELLOW}Testing network syscall blocking...${NC}"
    
    # Create a test program that tries to create a socket
    cat > socket_test.c << 'EOF'
#include <sys/socket.h>
#include <stdio.h>
#include <stdlib.h>

int main() {
    int sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock == -1) {
        printf("Socket creation failed (expected)\n");
        return 1;
    } else {
        printf("Socket creation succeeded (SECURITY VIOLATION)\n");
        return 0;
    }
}
EOF
    
    # Compile the test
    if gcc -o socket_test socket_test.c 2>/dev/null; then
        local output
        output=$(timeout 5 "$RUSTBOX_BIN" run --enable-seccomp --max-time=3 -- ./socket_test 2>&1)
        local exit_code=$?
        
        # Should be killed by seccomp (exit code 137 or similar)
        if [ $exit_code -ne 0 ] && ! echo "$output" | grep -q "SECURITY VIOLATION"; then
            log_test "Network syscall blocking" "BLOCKED" "BLOCKED" "Socket creation properly blocked"
        else
            log_test "Network syscall blocking" "BLOCKED" "ALLOWED" "Socket creation was not blocked: $output"
        fi
    else
        log_test "Network syscall blocking" "BLOCKED" "SKIPPED" "Could not compile test program"
    fi
}

# Test 3: Fork/exec blocking
test_process_creation_blocking() {
    echo -e "\n${YELLOW}Testing process creation blocking...${NC}"
    
    # Create a test program that tries to fork
    cat > fork_test.c << 'EOF'
#include <unistd.h>
#include <stdio.h>
#include <sys/wait.h>

int main() {
    pid_t pid = fork();
    if (pid == -1) {
        printf("Fork failed (expected)\n");
        return 1;
    } else if (pid == 0) {
        printf("Child process created (SECURITY VIOLATION)\n");
        return 0;
    } else {
        printf("Parent process - fork succeeded (SECURITY VIOLATION)\n");
        wait(NULL);
        return 0;
    }
}
EOF
    
    if gcc -o fork_test fork_test.c 2>/dev/null; then
        local output
        output=$(timeout 5 "$RUSTBOX_BIN" run --enable-seccomp --max-time=3 -- ./fork_test 2>&1)
        local exit_code=$?
        
        if [ $exit_code -ne 0 ] && ! echo "$output" | grep -q "SECURITY VIOLATION"; then
            log_test "Process creation blocking" "BLOCKED" "BLOCKED" "Fork properly blocked"
        else
            log_test "Process creation blocking" "BLOCKED" "ALLOWED" "Fork was not blocked: $output"
        fi
    else
        log_test "Process creation blocking" "BLOCKED" "SKIPPED" "Could not compile test program"
    fi
}

# Test 4: File system modification blocking
test_filesystem_modification_blocking() {
    echo -e "\n${YELLOW}Testing filesystem modification blocking...${NC}"
    
    # Create a test program that tries to create a file
    cat > file_test.c << 'EOF'
#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>

int main() {
    int fd = open("/tmp/security_test_file", O_CREAT | O_WRONLY, 0644);
    if (fd == -1) {
        printf("File creation failed (expected)\n");
        return 1;
    } else {
        printf("File creation succeeded (SECURITY VIOLATION)\n");
        close(fd);
        return 0;
    }
}
EOF
    
    if gcc -o file_test file_test.c 2>/dev/null; then
        local output
        output=$(timeout 5 "$RUSTBOX_BIN" run --enable-seccomp --max-time=3 -- ./file_test 2>&1)
        local exit_code=$?
        
        if [ $exit_code -ne 0 ] && ! echo "$output" | grep -q "SECURITY VIOLATION"; then
            log_test "Filesystem modification blocking" "BLOCKED" "BLOCKED" "File creation properly blocked"
        else
            log_test "Filesystem modification blocking" "BLOCKED" "ALLOWED" "File creation was not blocked: $output"
        fi
    else
        log_test "Filesystem modification blocking" "BLOCKED" "SKIPPED" "Could not compile test program"
    fi
}

# Test 5: Mount syscall blocking
test_mount_blocking() {
    echo -e "\n${YELLOW}Testing mount syscall blocking...${NC}"
    
    cat > mount_test.c << 'EOF'
#include <sys/mount.h>
#include <stdio.h>

int main() {
    int result = mount("/dev/null", "/tmp", "tmpfs", 0, NULL);
    if (result == -1) {
        printf("Mount failed (expected)\n");
        return 1;
    } else {
        printf("Mount succeeded (SECURITY VIOLATION)\n");
        return 0;
    }
}
EOF
    
    if gcc -o mount_test mount_test.c 2>/dev/null; then
        local output
        output=$(timeout 5 "$RUSTBOX_BIN" run --enable-seccomp --max-time=3 -- ./mount_test 2>&1)
        local exit_code=$?
        
        if [ $exit_code -ne 0 ] && ! echo "$output" | grep -q "SECURITY VIOLATION"; then
            log_test "Mount syscall blocking" "BLOCKED" "BLOCKED" "Mount properly blocked"
        else
            log_test "Mount syscall blocking" "BLOCKED" "ALLOWED" "Mount was not blocked: $output"
        fi
    else
        log_test "Mount syscall blocking" "BLOCKED" "SKIPPED" "Could not compile test program"
    fi
}

# Test 6: Privilege escalation blocking
test_privilege_escalation_blocking() {
    echo -e "\n${YELLOW}Testing privilege escalation blocking...${NC}"
    
    cat > priv_test.c << 'EOF'
#include <unistd.h>
#include <stdio.h>

int main() {
    int result = setuid(0);
    if (result == -1) {
        printf("Setuid failed (expected)\n");
        return 1;
    } else {
        printf("Setuid succeeded (SECURITY VIOLATION)\n");
        return 0;
    }
}
EOF
    
    if gcc -o priv_test priv_test.c 2>/dev/null; then
        local output
        output=$(timeout 5 "$RUSTBOX_BIN" run --enable-seccomp --max-time=3 -- ./priv_test 2>&1)
        local exit_code=$?
        
        if [ $exit_code -ne 0 ] && ! echo "$output" | grep -q "SECURITY VIOLATION"; then
            log_test "Privilege escalation blocking" "BLOCKED" "BLOCKED" "Setuid properly blocked"
        else
            log_test "Privilege escalation blocking" "BLOCKED" "ALLOWED" "Setuid was not blocked: $output"
        fi
    else
        log_test "Privilege escalation blocking" "BLOCKED" "SKIPPED" "Could not compile test program"
    fi
}

# Test 7: Ptrace blocking (debugging prevention)
test_ptrace_blocking() {
    echo -e "\n${YELLOW}Testing ptrace blocking...${NC}"
    
    cat > ptrace_test.c << 'EOF'
#include <sys/ptrace.h>
#include <stdio.h>

int main() {
    long result = ptrace(PTRACE_TRACEME, 0, NULL, NULL);
    if (result == -1) {
        printf("Ptrace failed (expected)\n");
        return 1;
    } else {
        printf("Ptrace succeeded (SECURITY VIOLATION)\n");
        return 0;
    }
}
EOF
    
    if gcc -o ptrace_test ptrace_test.c 2>/dev/null; then
        local output
        output=$(timeout 5 "$RUSTBOX_BIN" run --enable-seccomp --max-time=3 -- ./ptrace_test 2>&1)
        local exit_code=$?
        
        if [ $exit_code -ne 0 ] && ! echo "$output" | grep -q "SECURITY VIOLATION"; then
            log_test "Ptrace blocking" "BLOCKED" "BLOCKED" "Ptrace properly blocked"
        else
            log_test "Ptrace blocking" "BLOCKED" "ALLOWED" "Ptrace was not blocked: $output"
        fi
    else
        log_test "Ptrace blocking" "BLOCKED" "SKIPPED" "Could not compile test program"
    fi
}

# Test 8: Language-specific profile testing
test_language_profiles() {
    echo -e "\n${YELLOW}Testing language-specific profiles...${NC}"
    
    # Test Python profile
    local python_output
    python_output=$(timeout 5 "$RUSTBOX_BIN" run --enable-seccomp --language=python --max-time=3 -- python3 -c "print('Python works')" 2>&1)
    local python_exit=$?
    
    if [ $python_exit -eq 0 ] && echo "$python_output" | grep -q "Python works"; then
        log_test "Python language profile" "SUCCESS" "SUCCESS" "Python execution successful"
    else
        log_test "Python language profile" "SUCCESS" "FAILED" "Python execution failed: $python_output"
    fi
    
    # Test that dangerous operations still fail in language profiles
    cat > python_socket_test.py << 'EOF'
import socket
try:
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    print("Socket creation succeeded (SECURITY VIOLATION)")
except:
    print("Socket creation failed (expected)")
EOF
    
    local python_socket_output
    python_socket_output=$(timeout 5 "$RUSTBOX_BIN" run --enable-seccomp --language=python --max-time=3 -- python3 python_socket_test.py 2>&1)
    local python_socket_exit=$?
    
    if [ $python_socket_exit -ne 0 ] && ! echo "$python_socket_output" | grep -q "SECURITY VIOLATION"; then
        log_test "Python profile security" "BLOCKED" "BLOCKED" "Dangerous operations blocked in Python profile"
    else
        log_test "Python profile security" "BLOCKED" "ALLOWED" "Dangerous operations not blocked: $python_socket_output"
    fi
}

# Test 9: Resource exhaustion protection
test_resource_exhaustion_protection() {
    echo -e "\n${YELLOW}Testing resource exhaustion protection...${NC}"
    
    # Test memory limit
    cat > memory_bomb.c << 'EOF'
#include <stdlib.h>
#include <stdio.h>

int main() {
    void *ptr;
    int count = 0;
    while (1) {
        ptr = malloc(1024 * 1024); // 1MB
        if (ptr == NULL) {
            printf("Memory allocation failed after %d MB\n", count);
            break;
        }
        count++;
        if (count > 1000) { // More than 1GB
            printf("Memory bomb succeeded (SECURITY VIOLATION)\n");
            break;
        }
    }
    return 0;
}
EOF
    
    if gcc -o memory_bomb memory_bomb.c 2>/dev/null; then
        local output
        output=$(timeout 10 "$RUSTBOX_BIN" run --enable-seccomp --max-memory=100M --max-time=5 -- ./memory_bomb 2>&1)
        local exit_code=$?
        
        if [ $exit_code -ne 0 ] && ! echo "$output" | grep -q "SECURITY VIOLATION"; then
            log_test "Memory exhaustion protection" "BLOCKED" "BLOCKED" "Memory bomb properly limited"
        else
            log_test "Memory exhaustion protection" "BLOCKED" "ALLOWED" "Memory bomb not limited: $output"
        fi
    else
        log_test "Memory exhaustion protection" "BLOCKED" "SKIPPED" "Could not compile test program"
    fi
}

# Test 10: Time limit enforcement
test_time_limit_enforcement() {
    echo -e "\n${YELLOW}Testing time limit enforcement...${NC}"
    
    cat > time_bomb.c << 'EOF'
#include <stdio.h>
#include <unistd.h>

int main() {
    printf("Starting infinite loop...\n");
    while (1) {
        // Infinite loop
        usleep(1000);
    }
    printf("Loop ended (SECURITY VIOLATION)\n");
    return 0;
}
EOF
    
    if gcc -o time_bomb time_bomb.c 2>/dev/null; then
        local start_time=$(date +%s)
        local output
        output=$(timeout 10 "$RUSTBOX_BIN" run --enable-seccomp --max-time=2 -- ./time_bomb 2>&1)
        local exit_code=$?
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        if [ $exit_code -ne 0 ] && [ $duration -le 5 ]; then
            log_test "Time limit enforcement" "BLOCKED" "BLOCKED" "Time bomb properly limited in ${duration}s"
        else
            log_test "Time limit enforcement" "BLOCKED" "ALLOWED" "Time bomb not limited, ran for ${duration}s: $output"
        fi
    else
        log_test "Time limit enforcement" "BLOCKED" "SKIPPED" "Could not compile test program"
    fi
}

# Generate final report
generate_report() {
    echo -e "\n${YELLOW}Security Test Summary${NC}"
    echo "===================="
    echo -e "Total tests: $TOTAL_TESTS"
    echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
    echo -e "${RED}Failed: $FAILED_TESTS${NC}"
    
    local pass_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))
    echo -e "Pass rate: ${pass_rate}%"
    
    echo "" >> "$RESULTS_FILE"
    echo "SUMMARY:" >> "$RESULTS_FILE"
    echo "Total tests: $TOTAL_TESTS" >> "$RESULTS_FILE"
    echo "Passed: $PASSED_TESTS" >> "$RESULTS_FILE"
    echo "Failed: $FAILED_TESTS" >> "$RESULTS_FILE"
    echo "Pass rate: ${pass_rate}%" >> "$RESULTS_FILE"
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "\n${GREEN}🎉 All security tests passed! Rustbox seccomp filtering is production-ready.${NC}"
        echo "RESULT: PRODUCTION READY" >> "$RESULTS_FILE"
    else
        echo -e "\n${RED}⚠️  Some security tests failed. Review and fix before production use.${NC}"
        echo "RESULT: NOT PRODUCTION READY" >> "$RESULTS_FILE"
    fi
    
    echo -e "\nDetailed results saved to: $RESULTS_FILE"
}

# Main execution
main() {
    echo -e "${YELLOW}Rustbox Seccomp Security Test Suite${NC}"
    echo "===================================="
    
    setup_test_env
    
    test_basic_seccomp
    test_network_blocking
    test_process_creation_blocking
    test_filesystem_modification_blocking
    test_mount_blocking
    test_privilege_escalation_blocking
    test_ptrace_blocking
    test_language_profiles
    test_resource_exhaustion_protection
    test_time_limit_enforcement
    
    generate_report
}

# Run the tests
main "$@"