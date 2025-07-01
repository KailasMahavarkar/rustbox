#!/bin/bash

# Quick Resource Limit Validation Test
echo "=== Mini-Isolate Resource Limit Validation ==="
echo

# Build if needed
if [ ! -f "./target/release/mini-isolate" ]; then
    echo "[INFO] Building mini-isolate..."
    cargo build --release
fi

ISOLATE="sudo ./target/release/mini-isolate"
PASS=0
FAIL=0

test_result() {
    if [ $1 -eq 0 ]; then
        echo "✅ PASS: $2"
        ((PASS++))
    else
        echo "❌ FAIL: $2"
        ((FAIL++))
    fi
}

echo "[INFO] Testing basic execution..."
$ISOLATE run --box-id 0 /bin/echo -- "Hello World" > /dev/null 2>&1
test_result $? "Basic command execution"

echo "[INFO] Testing wall time limit..."
timeout 5 $ISOLATE run --box-id 0 --max-time 1 /bin/sleep -- "3" > /dev/null 2>&1
if [ $? -eq 2 ]; then
    test_result 0 "Wall time limit enforcement"
else
    test_result 1 "Wall time limit enforcement"
fi

echo "[INFO] Testing CPU time limit..."
timeout 10 $ISOLATE run --box-id 0 --max-cpu 1 /tmp/cpu_test > /dev/null 2>&1
if [ $? -eq 2 ]; then
    test_result 0 "CPU time limit enforcement"
else
    test_result 1 "CPU time limit enforcement"
fi

echo "[INFO] Testing memory limit..."
timeout 5 $ISOLATE run --box-id 0 --max-memory 5 /tmp/memory_hog > /dev/null 2>&1
if [ $? -eq 1 ]; then
    test_result 0 "Memory limit enforcement"
else
    test_result 1 "Memory limit enforcement"
fi

echo "[INFO] Testing combined limits..."
timeout 5 $ISOLATE run --box-id 0 --max-cpu 1 --max-memory 20 --max-time 10 /tmp/cpu_test > /dev/null 2>&1
if [ $? -eq 2 ]; then
    test_result 0 "Combined resource limits"
else
    test_result 1 "Combined resource limits"
fi

echo
echo "=== RESULTS ==="
echo "✅ Passed: $PASS"
echo "❌ Failed: $FAIL"
echo "Total: $((PASS + FAIL))"

if [ $FAIL -eq 0 ]; then
    echo
    echo "🎉 ALL TESTS PASSED! Resource limits working correctly."
    exit 0
else
    echo
    echo "⚠️  Some tests failed. Check implementation."
    exit 1
fi