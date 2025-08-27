#!/bin/bash

# Core Tests for rustbox - Updated for current implementation
# Tests basic functionality using execute-code command

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUSTBOX="$SCRIPT_DIR/../../target/release/rustbox"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo "[INFO] $1"; }
log_success() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_failure() { echo -e "${RED}[FAIL]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARN]${NC} $1"; }

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== rustbox Core Tests (Updated) ====="
echo ""

passed=0
failed=0

# Helper function to run test and check JSON output (no jq dependency)
run_test() {
    local test_name="$1"
    local language="$2" 
    local code="$3"
    local expected_status="$4"
    local box_id="$5"
    local extra_args="$6"
    
    log_info "$test_name"
    
    # Capture stdout (JSON) and stderr separately
    local stdout_file=$(mktemp)
    local stderr_file=$(mktemp)
    
    # Run the command and capture exit code
    sudo $RUSTBOX execute-code --strict --box-id="$box_id" --language="$language" --code="$code" $extra_args >"$stdout_file" 2>"$stderr_file"
    local exit_code=$?
    
    # Check if we got JSON output (exit code 0 or 1 both can produce valid JSON)
    if [[ -s "$stdout_file" && $(head -1 "$stdout_file" | grep -c '{') -gt 0 ]]; then
        local result=$(cat "$stdout_file")
        
        # Parse JSON result using grep (no jq dependency)
        local actual_status=$(echo "$result" | grep -o '"status": "[^"]*"' | cut -d'"' -f4)
        local success_value=$(echo "$result" | grep -o '"success": [^,}]*' | cut -d':' -f2 | tr -d ' ')
        
        if [[ "$success_value" == "true" && "$expected_status" == "Success" ]]; then
            log_success "$test_name"
            ((passed++))
            rm -f "$stdout_file" "$stderr_file"
            return 0
        elif [[ "$actual_status" == "$expected_status" && "$expected_status" != "Success" ]]; then
            log_success "$test_name - correctly enforced $expected_status"
            ((passed++))
            rm -f "$stdout_file" "$stderr_file"
            return 0
        else
            log_failure "$test_name - unexpected result"
            echo "Expected: $expected_status, Got: $actual_status (success: $success_value)"
            ((failed++))
            rm -f "$stdout_file" "$stderr_file"
            return 1
        fi
    else
        log_failure "$test_name - execution failed"
        echo "Error: $(cat "$stderr_file" | head -2)"
        ((failed++))
        rm -f "$stdout_file" "$stderr_file"
        return 1
    fi
}

# Test 1: Basic Python execution
run_test "Test 1: Basic Python execution" \
    "python" \
    'print("Hello from Python")' \
    "Success" \
    1

# Test 2: Basic C++ execution  
run_test "Test 2: Basic C++ compilation and execution" \
    "cpp" \
    '#include <iostream>
int main() { std::cout << "Hello from C++" << std::endl; return 0; }' \
    "Success" \
    2 \
    "--time=10 --processes=50 --mem=256"

# Test 3: Memory limit enforcement
run_test "Test 3: Memory limit enforcement" \
    "python" \
    'data = []
for i in range(1000000):
    data.append([0] * 1000)
print("This should not print")' \
    "Memory Limit Exceeded" \
    3 \
    "--time=10 --mem=50"

# Test 4: CPU time limit enforcement
run_test "Test 4: CPU time limit enforcement" \
    "python" \
    'import time
time.sleep(10)
print("This should not print")' \
    "TLE" \
    4 \
    "--time=2 --mem=128"

# Test 5: Java execution (high resource requirements)
run_test "Test 5: Java execution with proper limits" \
    "java" \
    'public class Main { public static void main(String[] args) { System.out.println("Hello from Java"); } }' \
    "Success" \
    5 \
    "--time=10 --processes=50 --mem=256"

# Test 6: Go execution (high resource requirements)
run_test "Test 6: Go execution with proper limits" \
    "go" \
    'package main
import "fmt"
func main() { fmt.Println("Hello from Go") }' \
    "Success" \
    6 \
    "--time=10 --processes=60 --mem=256"

# Test 7: Dependency checker
log_info "Test 7: Language dependency checker"
if result=$(sudo $RUSTBOX check-deps 2>&1); then
    if echo "$result" | grep -q "All language dependencies are installed"; then
        log_success "Dependency checker working"
        ((passed++))
    else
        log_failure "Dependency checker - unexpected output"
        echo "$result" | head -3
        ((failed++))
    fi
else
    log_failure "Dependency checker failed"
    ((failed++))
fi

# Test 8: Init and cleanup commands
log_info "Test 8: Init and cleanup commands"
if sudo $RUSTBOX init --box-id 99 >/dev/null 2>&1; then
    if sudo $RUSTBOX cleanup --box-id 99 >/dev/null 2>&1; then
        log_success "Init and cleanup commands working"
        ((passed++))
    else
        log_failure "Cleanup command failed"
        ((failed++))
    fi
else
    log_failure "Init command failed"
    ((failed++))
fi

echo ""
echo "===== Core Tests Summary ====="
echo "Passed: $passed"
echo "Failed: $failed" 
if [[ $((passed + failed)) -gt 0 ]]; then
    echo "Success rate: $(( (passed * 100) / (passed + failed) ))%"
fi
echo ""

if [[ $failed -eq 0 ]]; then
    echo "✅ All core tests passed! rustbox is working correctly."
    exit 0
else
    echo "⚠️ Some tests failed. Check the output above for details."
    exit 1
fi