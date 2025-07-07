#!/bin/bash

# File Descriptor Limit Test for rustbox
# Tests file descriptor limit enforcement as part of resource limits

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== File Descriptor Limit Tests ====="
echo ""

passed=0
failed=0

# Create a Python script that tries to open many files
cat > /tmp/fd_test.py << 'EOF'
import sys
import os

def test_fd_limit():
    files = []
    try:
        for i in range(1000):  # Try to open many files
            f = open('/dev/null', 'r')
            files.append(f)
            if i % 10 == 0:
                print(f"Opened {i} file descriptors", flush=True)
    except OSError as e:
        print(f"Hit limit at {len(files)} file descriptors: {e}", flush=True)
        return len(files)
    finally:
        for f in files:
            try:
                f.close()
            except:
                pass
    
    return len(files)

if __name__ == "__main__":
    count = test_fd_limit()
    print(f"Maximum FDs opened: {count}")
EOF

# Test 1: Very low fd_limit (5)
echo "[INFO] Test 1: Very low fd_limit (5)"
if sudo $MINI_ISOLATE init --box-id fd_test_low --fd-limit 5 --time 10 --mem 256 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id fd_test_low --max-time 5 --verbose python3 -- /tmp/fd_test.py 2>&1); then
        if [[ "$result" == *"Hit limit at"* ]] && [[ "$result" == *"Too many open files"* ]]; then
            # Extract the number of FDs opened from the output
            fd_count=$(echo "$result" | grep "Hit limit at" | sed -n 's/.*Hit limit at \([0-9]*\) file descriptors.*/\1/p')
            if [[ -n "$fd_count" && "$fd_count" -lt 10 ]]; then
                echo "✅ Low fd_limit enforced correctly (opened $fd_count FDs)"
                ((passed++))
            else
                echo "❌ Low fd_limit not enforced properly (opened $fd_count FDs)"
                ((failed++))
            fi
        else
            echo "❌ Low fd_limit test unexpected result"
            ((failed++))
        fi
    else
        echo "❌ Low fd_limit test execution failed"
        ((failed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id fd_test_low >/dev/null 2>&1 || true
else
    echo "❌ Low fd_limit test init failed"
    ((failed++))
fi

# Test 2: Medium fd_limit (50)
echo "[INFO] Test 2: Medium fd_limit (50)"
if sudo $MINI_ISOLATE init --box-id fd_test_med --fd-limit 50 --time 10 --mem 256 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id fd_test_med --max-time 5 --verbose python3 -- /tmp/fd_test.py 2>&1); then
        if [[ "$result" == *"Hit limit at"* ]] && [[ "$result" == *"Too many open files"* ]]; then
            # Extract the number of FDs opened from the output
            fd_count=$(echo "$result" | grep "Hit limit at" | sed -n 's/.*Hit limit at \([0-9]*\) file descriptors.*/\1/p')
            if [[ -n "$fd_count" && "$fd_count" -gt 30 && "$fd_count" -lt 60 ]]; then
                echo "✅ Medium fd_limit enforced correctly (opened $fd_count FDs)"
                ((passed++))
            else
                echo "❌ Medium fd_limit not in expected range (opened $fd_count FDs)"
                ((failed++))
            fi
        else
            echo "❌ Medium fd_limit test unexpected result"
            ((failed++))
        fi
    else
        echo "❌ Medium fd_limit test execution failed"
        ((failed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id fd_test_med >/dev/null 2>&1 || true
else
    echo "❌ Medium fd_limit test init failed"
    ((failed++))
fi

# Test 3: FD limit override functionality
echo "[INFO] Test 3: FD limit override (init=20, override=10)"
if sudo $MINI_ISOLATE init --box-id fd_test_override --fd-limit 20 --time 10 --mem 256 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id fd_test_override --max-time 5 --fd-limit 10 --verbose python3 -- /tmp/fd_test.py 2>&1); then
        if [[ "$result" == *"Hit limit at"* ]] && [[ "$result" == *"Too many open files"* ]]; then
            # Extract the number of FDs opened from the output
            fd_count=$(echo "$result" | grep "Hit limit at" | sed -n 's/.*Hit limit at \([0-9]*\) file descriptors.*/\1/p')
            if [[ -n "$fd_count" && "$fd_count" -lt 15 ]]; then
                echo "✅ FD limit override enforced correctly (opened $fd_count FDs)"
                ((passed++))
            else
                echo "❌ FD limit override not working (opened $fd_count FDs, expected <15)"
                ((failed++))
            fi
        else
            echo "❌ FD limit override test unexpected result"
            ((failed++))
        fi
    else
        echo "❌ FD limit override test execution failed"
        ((failed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id fd_test_override >/dev/null 2>&1 || true
else
    echo "❌ FD limit override test init failed"
    ((failed++))
fi

# Test 4: Higher fd_limit allowing more files
echo "[INFO] Test 4: Higher fd_limit (200) - should allow more files"
if sudo $MINI_ISOLATE init --box-id fd_test_high --fd-limit 200 --time 10 --mem 256 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id fd_test_high --max-time 5 --verbose python3 -- /tmp/fd_test.py 2>&1); then
        if [[ "$result" == *"Hit limit at"* ]] && [[ "$result" == *"Too many open files"* ]]; then
            # Extract the number of FDs opened from the output
            fd_count=$(echo "$result" | grep "Hit limit at" | sed -n 's/.*Hit limit at \([0-9]*\) file descriptors.*/\1/p')
            if [[ -n "$fd_count" && "$fd_count" -gt 100 ]]; then
                echo "✅ Higher fd_limit allows more files correctly (opened $fd_count FDs)"
                ((passed++))
            else
                echo "❌ Higher fd_limit not allowing enough files (opened $fd_count FDs, expected >100)"
                ((failed++))
            fi
        else
            echo "❌ Higher fd_limit test unexpected result"
            ((failed++))
        fi
    else
        echo "❌ Higher fd_limit test execution failed"
        ((failed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id fd_test_high >/dev/null 2>&1 || true
else
    echo "❌ Higher fd_limit test init failed"
    ((failed++))
fi

# Cleanup
rm -f /tmp/fd_test.py

echo ""
echo "===== File Descriptor Limit Test Results ====="
echo "Passed: $passed"
echo "Failed: $failed"

if [[ $failed -eq 0 ]]; then
    echo "✅ All fd_limit tests working correctly!"
    exit 0
else
    echo "⚠️ Some fd_limit tests had issues"
    exit 1
fi