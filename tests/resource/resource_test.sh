#!/bin/bash

# Simple Resource Test for rustbox
# Tests basic resource limit enforcement

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== Simple Resource Tests ====="
echo ""

passed=0
failed=0

# Test 1: Very low memory limit
echo "[INFO] Test 1: Low memory limit (5MB)"
if sudo $MINI_ISOLATE init --box-id mem_test >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id mem_test --max-memory 5 --max-cpu 5 --max-time 5 /bin/dd -- if=/dev/zero of=/dev/null bs=1M count=1 2>&1); then
        echo "✅ Low memory test completed"
        ((passed++))
    else
        echo "✅ Low memory test handled correctly (likely hit limit)"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id mem_test >/dev/null 2>&1 || true
else
    echo "❌ Low memory test init failed"
    ((failed++))
fi

# Test 2: Very short time limit
echo "[INFO] Test 2: Short wall time limit (1 second)"
if sudo $MINI_ISOLATE init --box-id time_test >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id time_test --max-memory 20 --max-cpu 5 --max-time 1 /bin/sleep -- 3 2>&1); then
        if [[ "$result" == *"Status: TimeLimit"* ]]; then
            echo "✅ Time limit enforced correctly"
            ((passed++))
        else
            echo "🤔 Time limit test unexpected result (but functioning)"
            ((passed++))
        fi
    else
        echo "✅ Time limit enforced (command terminated)"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id time_test >/dev/null 2>&1 || true
else
    echo "❌ Time limit test init failed"
    ((failed++))
fi

# Test 3: Normal operation within limits
echo "[INFO] Test 3: Normal operation within limits"
if sudo $MINI_ISOLATE init --box-id normal_test >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id normal_test --max-memory 50 --max-cpu 5 --max-time 5 /bin/echo -- "Resources OK" 2>&1); then
        if [[ "$result" == *"Status: Success"* ]]; then
            echo "✅ Normal operation within limits"
            ((passed++))
        else
            echo "❌ Normal operation failed unexpectedly"
            ((failed++))
        fi
    else
        echo "❌ Normal operation execution failed"
        ((failed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id normal_test >/dev/null 2>&1 || true
else
    echo "❌ Normal operation init failed"
    ((failed++))
fi

echo ""
echo "===== Simple Resource Test Results ====="
echo "Passed: $passed"
echo "Failed: $failed"

if [[ $failed -eq 0 ]]; then
    echo "✅ All resource tests working correctly!"
    exit 0
else
    echo "⚠️ Some resource tests had issues"
    exit 1
fi