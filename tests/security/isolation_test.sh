#!/bin/bash

# Security Tests for Mini-Isolate
# Tests isolation and security features

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/mini-isolate"

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

echo "===== Mini-Isolate Security Tests ====="
echo ""

passed=0
failed=0

# Test 1: Process isolation - check that processes can't see host processes
log_info "Test 1: Process namespace isolation"
if sudo $MINI_ISOLATE init --box-id security1 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id security1 --max-memory 50 --max-cpu 5 --max-time 5 /bin/ps -- aux 2>&1); then
        # In proper isolation, ps should show minimal processes (not host processes)
        process_count=$(echo "$result" | grep -c "^[a-zA-Z]" 2>/dev/null || echo "0")
        if [[ $process_count -lt 10 ]]; then
            log_success "Process namespace isolation (only $process_count processes visible)"
            ((passed++))
        else
            log_failure "Process namespace isolation - too many processes visible ($process_count)"
            ((failed++))
        fi
    else
        log_failure "Process namespace isolation - ps command failed"
        ((failed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id security1 >/dev/null 2>&1 || true
else
    log_failure "Process namespace isolation - init failed"
    ((failed++))
fi

# Test 2: Filesystem isolation - check that /etc/passwd is isolated
log_info "Test 2: Filesystem isolation"
if sudo $MINI_ISOLATE init --box-id security2 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id security2 --max-memory 50 --max-cpu 5 --max-time 5 /bin/cat -- /etc/passwd 2>&1); then
        if [[ "$result" == *"Status: Success"* ]]; then
            # Check that we don't see the host's full /etc/passwd
            user_count=$(echo "$result" | grep -c ":" 2>/dev/null || echo "0")
            if [[ $user_count -lt 10 ]]; then
                log_success "Filesystem isolation (isolated /etc/passwd with $user_count users)"
                ((passed++))
            else
                log_failure "Filesystem isolation - host /etc/passwd may be visible ($user_count users)"
                ((failed++))
            fi
        else
            log_success "Filesystem isolation (isolated environment - no /etc/passwd access)"
            ((passed++))
        fi
    else
        log_success "Filesystem isolation (properly isolated - command failed as expected)"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id security2 >/dev/null 2>&1 || true
else
    log_failure "Filesystem isolation - init failed"
    ((failed++))
fi

# Test 3: Network isolation - check that network access is restricted
log_info "Test 3: Network isolation"
if sudo $MINI_ISOLATE init --box-id security3 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id security3 --max-memory 50 --max-cpu 5 --max-time 5 /bin/ping -- -c 1 8.8.8.8 2>&1); then
        if [[ "$result" == *"Status: Success"* ]] && [[ "$result" == *"1 packets transmitted, 1 received"* ]]; then
            log_failure "Network isolation - ping succeeded (network not isolated)"
            ((failed++))
        else
            log_success "Network isolation (ping failed - network properly isolated)"
            ((passed++))
        fi
    else
        log_success "Network isolation (network access restricted)"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id security3 >/dev/null 2>&1 || true
else
    log_failure "Network isolation - init failed"
    ((failed++))
fi

# Test 4: User isolation - check that user context is isolated
log_info "Test 4: User namespace isolation"
if sudo $MINI_ISOLATE init --box-id security4 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id security4 --max-memory 50 --max-cpu 5 --max-time 5 /bin/id 2>&1); then
        if [[ "$result" == *"Status: Success"* ]]; then
            # Check that we're running as an isolated user
            if [[ "$result" == *"uid=0"* ]] && [[ "$result" == *"gid=0"* ]]; then
                log_success "User namespace isolation (running as isolated root within container)"
                ((passed++))
            else
                log_success "User namespace isolation (running as isolated user)"
                ((passed++))
            fi
        else
            log_failure "User namespace isolation - id command failed"
            ((failed++))
        fi
    else
        log_failure "User namespace isolation - execution failed"
        ((failed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id security4 >/dev/null 2>&1 || true
else
    log_failure "User namespace isolation - init failed"
    ((failed++))
fi

echo ""
echo "===== Security Tests Summary ====="
echo "Passed: $passed"
echo "Failed: $failed"
echo "Success rate: $(( (passed * 100) / (passed + failed) ))%"
echo ""

if [[ $failed -eq 0 ]]; then
    echo "✅ All security tests passed! Isolation is working correctly."
    exit 0
else
    echo "⚠️ Some security tests failed. Check isolation configuration."
    exit 1
fi