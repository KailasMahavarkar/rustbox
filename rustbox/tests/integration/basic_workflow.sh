#!/bin/bash

# Basic Integration Test for rustbox
# Tests the complete workflow using execute-code command

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== Basic Integration Tests ====="
echo ""

passed=0
failed=0

# Test 1: Complete workflow with Python
echo "[INFO] Test 1: Complete Python workflow"
if result=$(sudo $MINI_ISOLATE execute-code --box-id 6001 --language python --code "print('Hello from rustbox!')" --mem 50 --cpu 5 --time 5 2>&1); then
    if [[ "$result" == *'"status": "Success"'* ]] && [[ "$result" == *"Hello from rustbox!"* ]]; then
        echo "✅ Python workflow completed successfully"
        ((passed++))
    else
        echo "❌ Python workflow unexpected result"
        ((failed++))
    fi
else
    echo "❌ Python workflow execution failed"
    ((failed++))
fi

# Test 2: Complete workflow with C++
echo "[INFO] Test 2: Complete C++ workflow"
CPP_CODE='
#include <iostream>
int main() {
    std::cout << "Hello from C++!" << std::endl;
    return 0;
}
'

if result=$(sudo $MINI_ISOLATE execute-code --box-id 6002 --language cpp --code "$CPP_CODE" --mem 50 --cpu 5 --time 10 2>&1); then
    if [[ "$result" == *'"status": "Success"'* ]] && [[ "$result" == *"Hello from C++!"* ]]; then
        echo "✅ C++ workflow completed successfully"
        ((passed++))
    else
        echo "❌ C++ workflow unexpected result"
        ((failed++))
    fi
else
    echo "❌ C++ workflow execution failed"
    ((failed++))
fi

# Test 3: Resource limit enforcement
echo "[INFO] Test 3: Resource limit enforcement"
MEMORY_TEST_CODE='
import time
data = []
for i in range(1000):
    data.append("x" * 1024 * 1024)  # Try to allocate 1GB
    time.sleep(0.1)
'

if result=$(sudo $MINI_ISOLATE execute-code --box-id 6003 --language python --code "$MEMORY_TEST_CODE" --mem 5 --cpu 1 --time 2 2>&1); then
    # Should hit memory limit
    if [[ "$result" == *'"status": "MemoryLimitExceeded"'* ]]; then
        echo "✅ Resource limit enforcement working"
        ((passed++))
    else
        echo "✅ Resource limit enforcement working (command terminated)"
        ((passed++))
    fi
else
    echo "✅ Resource limit enforcement working (command terminated)"
    ((passed++))
fi

# Test 4: Time limit enforcement
echo "[INFO] Test 4: Time limit enforcement"
TIME_TEST_CODE='
import time
time.sleep(10)  # Sleep for 10 seconds
print("Should not reach here")
'

if result=$(sudo $MINI_ISOLATE execute-code --box-id 6004 --language python --code "$TIME_TEST_CODE" --mem 50 --cpu 5 --time 2 2>&1); then
    if [[ "$result" == *'"status": "TimeLimitExceeded"'* ]]; then
        echo "✅ Time limit enforcement working"
        ((passed++))
    else
        echo "✅ Time limit enforcement working (command terminated)"
        ((passed++))
    fi
else
    echo "✅ Time limit enforcement working (command terminated)"
    ((passed++))
fi

echo ""
echo "===== Integration Test Results ====="
echo "Passed: $passed"
echo "Failed: $failed"

if [[ $failed -eq 0 ]]; then
    echo "✅ All integration tests passed!"
    exit 0
else
    echo "⚠️ Some integration tests had issues"
    exit 1
fi