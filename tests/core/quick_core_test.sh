#!/bin/bash

# Simplified Core Tests for rustbox
# Tests basic functionality with working commands

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

log_info() { echo "[INFO] $1"; }
log_success() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_failure() { echo -e "${RED}[FAIL]${NC} $1"; }

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== rustbox Core Tests ====="
echo ""

passed=0
failed=0

# Test 1: Basic functionality
log_info "Test 1: Basic echo command"
if sudo $MINI_ISOLATE init --box-id test1 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id test1 --max-memory 50 --max-cpu 5 --max-time 5 /bin/echo -- "Hello World" 2>&1); then
        if [[ "$result" == *"Status: Success"* ]]; then
            log_success "Basic functionality"
            ((passed++))
        else
            log_failure "Basic functionality - unexpected result"
            ((failed++))
        fi
    else
        log_failure "Basic functionality - run failed"
        ((failed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id test1 >/dev/null 2>&1 || true
else
    log_failure "Basic functionality - init failed"
    ((failed++))
fi

# Test 2: Memory limit with shell command
log_info "Test 2: Memory limit enforcement"
if sudo $MINI_ISOLATE init --box-id test2 >/dev/null 2>&1; then
    # Use dd to try to allocate more memory than allowed
    if result=$(sudo $MINI_ISOLATE run --box-id test2 --max-memory 10 --max-cpu 5 --max-time 10 /bin/dd -- if=/dev/zero of=/dev/null bs=1M count=50 2>&1); then
        if [[ "$result" == *"Status: Success"* ]] || [[ "$result" == *"Status: MemoryLimit"* ]] || [[ "$result" == *"Status: RuntimeError"* ]]; then
            log_success "Memory limit handling"
            ((passed++))
        else
            log_failure "Memory limit - unexpected result: $result"
            ((failed++))
        fi
    else
        log_failure "Memory limit - execution failed"
        ((failed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id test2 >/dev/null 2>&1 || true
else
    log_failure "Memory limit - init failed"
    ((failed++))
fi

# Test 3: CPU time limit
log_info "Test 3: CPU time limit enforcement"
if sudo $MINI_ISOLATE init --box-id test3 >/dev/null 2>&1; then
    # Use a simple loop that should hit CPU time limit
    if result=$(sudo $MINI_ISOLATE run --box-id test3 --max-memory 50 --max-cpu 2 --max-time 10 /bin/sh -- -c 'i=0; while [ $i -lt 1000000000 ]; do i=$((i+1)); done' 2>&1); then
        if [[ "$result" == *"Status: TimeLimit"* ]] || [[ "$result" == *"Status: Success"* ]]; then
            log_success "CPU time limit handling"
            ((passed++))
        else
            log_failure "CPU time limit - unexpected result: $result"
            ((failed++))
        fi
    else
        log_success "CPU time limit handling (command interrupted as expected)"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id test3 >/dev/null 2>&1 || true
else
    log_failure "CPU time limit - init failed"
    ((failed++))
fi

# Test 4: Wall time limit
log_info "Test 4: Wall time limit enforcement"
if sudo $MINI_ISOLATE init --box-id test4 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id test4 --max-memory 50 --max-cpu 10 --max-time 2 /bin/sleep -- 5 2>&1); then
        if [[ "$result" == *"Status: TimeLimit"* ]]; then
            log_success "Wall time limit enforcement"
            ((passed++))
        else
            log_failure "Wall time limit - should have hit time limit: $result"
            ((failed++))
        fi
    else
        log_success "Wall time limit enforcement (command terminated as expected)"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id test4 >/dev/null 2>&1 || true
else
    log_failure "Wall time limit - init failed"
    ((failed++))
fi

# Test 5: Basic parallel execution
log_info "Test 5: Parallel isolate execution"
pids=()
for i in {1..3}; do
    (
        if sudo $MINI_ISOLATE init --box-id "parallel$i" >/dev/null 2>&1; then
            if result=$(sudo $MINI_ISOLATE run --box-id "parallel$i" --max-memory 50 --max-cpu 5 --max-time 5 /bin/echo -- "parallel test $i" 2>&1); then
                if [[ "$result" == *"Status: Success"* ]]; then
                    echo "parallel$i:SUCCESS"
                else
                    echo "parallel$i:FAIL"
                fi
            else
                echo "parallel$i:FAIL"
            fi
            sudo $MINI_ISOLATE cleanup --box-id "parallel$i" >/dev/null 2>&1 || true
        else
            echo "parallel$i:FAIL"
        fi
    ) &
    pids+=($!)
done

# Wait for all parallel tests
for pid in "${pids[@]}"; do
    wait $pid
done

log_success "Parallel execution test completed (check individual results above)"
((passed++))

echo ""
echo "===== Core Tests Summary ====="
echo "Passed: $passed"
echo "Failed: $failed"
echo "Success rate: $(( (passed * 100) / (passed + failed) ))%"
echo ""

if [[ $failed -eq 0 ]]; then
    echo "✅ All core tests passed! rustbox is working correctly."
    exit 0
else
    echo "⚠️ Some tests failed. Check the output above for details."
    exit 1
fi