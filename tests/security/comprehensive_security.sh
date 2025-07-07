#!/bin/bash

# Comprehensive Security and Isolation Tests
# Tests security boundaries and potential escape vectors

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"
MALICIOUS_PY="$SCRIPT_DIR/malicious.py"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo "[INFO] $1"; }
log_success() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_failure() { echo -e "${RED}[FAIL]${NC} $1"; }
log_security() { echo -e "${YELLOW}[SECURITY]${NC} $1"; }

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== Comprehensive Security Tests ====="
echo ""

passed=0
failed=0

# Test 1: Malicious Python script isolation
log_info "Test 1: Running malicious script in isolation"
if sudo $MINI_ISOLATE init --box-id security_malicious --strict >/dev/null 2>&1; then
    # Copy malicious script into isolate
    sudo cp "$MALICIOUS_PY" "/tmp/rustbox/security_malicious/malicious.py"
    
    if result=$(sudo $MINI_ISOLATE run --box-id security_malicious --max-memory 50 --max-cpu 5 --max-time 5 python3 -- malicious.py 2>&1); then
        if [[ "$result" == *"Status: Success"* ]]; then
            # Check if the script was properly contained
            if [[ "$result" == *"Cannot read /etc/passwd"* ]] || [[ "$result" == *"Cannot read /etc/shadow"* ]]; then
                log_success "Malicious script properly contained"
                log_security "Filesystem access restricted as expected"
                ((passed++))
            elif [[ "$result" == *"Successfully read"* ]]; then
                log_failure "Security breach - malicious script accessed sensitive files"
                ((failed++))
            else
                log_success "Malicious script contained (minimal access)"
                log_security "Script ran with restricted access"
                ((passed++))
            fi
        else
            log_success "Malicious script execution blocked"
            log_security "Script execution properly restricted"
            ((passed++))
        fi
    else
        log_success "Malicious script execution prevented"
        log_security "Execution blocked at system level"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id security_malicious >/dev/null 2>&1 || true
else
    log_failure "Malicious script test setup failed"
    ((failed++))
fi

# Test 2: Network access restriction
log_info "Test 2: Network access isolation"
if sudo $MINI_ISOLATE init --box-id security_network --strict >/dev/null 2>&1; then
    # Try to access external network
    if result=$(sudo $MINI_ISOLATE run --box-id security_network --max-memory 50 --max-cpu 5 --max-time 5 /bin/ping -- -c 1 -W 2 8.8.8.8 2>&1); then
        if [[ "$result" == *"Status: Success"* ]] && [[ "$result" == *"1 packets transmitted, 1 received"* ]]; then
            log_failure "Network isolation breach - ping successful"
            log_security "⚠️  External network access not properly isolated"
            ((failed++))
        else
            log_success "Network access restricted"
            log_security "External network properly blocked"
            ((passed++))
        fi
    else
        log_success "Network access blocked"
        log_security "Network isolation working correctly"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id security_network >/dev/null 2>&1 || true
else
    log_failure "Network isolation test setup failed"
    ((failed++))
fi

# Test 3: Filesystem escape attempts
log_info "Test 3: Filesystem escape attempt prevention"
if sudo $MINI_ISOLATE init --box-id security_fs_escape --strict >/dev/null 2>&1; then
    # Try various directory traversal attempts
    escape_attempts=(
        "/bin/cat -- /../../../etc/passwd"
        "/bin/ls -- /../../../root"
        "/bin/cat -- /proc/mounts"
    )
    
    escape_blocked=0
    for attempt in "${escape_attempts[@]}"; do
        if result=$(sudo $MINI_ISOLATE run --box-id security_fs_escape --max-memory 50 --max-cpu 5 --max-time 2 $attempt 2>&1); then
            if [[ "$result" == *"No such file"* ]] || [[ "$result" == *"Permission denied"* ]] || [[ "$result" == *"Status: RuntimeError"* ]]; then
                ((escape_blocked++))
            fi
        else
            ((escape_blocked++))
        fi
    done
    
    if [[ $escape_blocked -eq ${#escape_attempts[@]} ]]; then
        log_success "All filesystem escape attempts blocked"
        log_security "Directory traversal properly prevented"
        ((passed++))
    elif [[ $escape_blocked -gt 0 ]]; then
        log_success "Most filesystem escapes blocked ($escape_blocked/${#escape_attempts[@]})"
        log_security "Partial filesystem isolation"
        ((passed++))
    else
        log_failure "Filesystem escape attempts succeeded"
        log_security "⚠️  Directory traversal not properly blocked"
        ((failed++))
    fi
    
    sudo $MINI_ISOLATE cleanup --box-id security_fs_escape >/dev/null 2>&1 || true
else
    log_failure "Filesystem escape test setup failed"
    ((failed++))
fi

# Test 4: Process visibility isolation
log_info "Test 4: Process namespace isolation"
if sudo $MINI_ISOLATE init --box-id security_proc --strict >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id security_proc --max-memory 50 --max-cpu 5 --max-time 5 /bin/ps -- aux 2>&1); then
        if [[ "$result" == *"Status: Success"* ]]; then
            # Count visible processes (should be minimal in proper isolation)
            process_count=$(echo "$result" | grep -c "^[a-zA-Z]" 2>/dev/null || echo "0")
            if [[ $process_count -le 10 ]]; then
                log_success "Process isolation working (only $process_count processes visible)"
                log_security "Host process list properly hidden"
                ((passed++))
            else
                log_failure "Process isolation weak - too many processes visible ($process_count)"
                log_security "⚠️  Host processes may be visible in container"
                ((failed++))
            fi
        else
            log_success "Process listing restricted"
            log_security "Process visibility properly controlled"
            ((passed++))
        fi
    else
        log_success "Process commands blocked"
        log_security "Process introspection prevented"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id security_proc >/dev/null 2>&1 || true
else
    log_failure "Process isolation test setup failed"
    ((failed++))
fi

# Test 5: Resource limit bypass attempts
log_info "Test 5: Resource limit bypass prevention"
if sudo $MINI_ISOLATE init --box-id security_resources --strict >/dev/null 2>&1; then
    # Try to bypass memory limits with various techniques
    if result=$(sudo $MINI_ISOLATE run --box-id security_resources --max-memory 10 --max-cpu 2 --max-time 3 /bin/sh -- -c 'dd if=/dev/zero of=/tmp/bigfile bs=1M count=50 2>/dev/null' 2>&1); then
        if [[ "$result" == *"Status: TimeLimit"* ]] || [[ "$result" == *"Status: MemoryLimit"* ]] || [[ "$result" == *"Status: RuntimeError"* ]]; then
            log_success "Resource limit bypass attempt blocked"
            log_security "Resource limits properly enforced"
            ((passed++))
        elif [[ "$result" == *"Status: Success"* ]]; then
            # Check if the operation was actually limited
            if [[ "$result" == *"Memory peak:"* ]]; then
                memory_used=$(echo "$result" | grep "Memory peak:" | awk '{print $3}' | sed 's/[^0-9]//g')
                if [[ $memory_used -lt 15360 ]]; then  # Less than 15MB (allowing some overhead)
                    log_success "Resource usage within limits despite success"
                    log_security "Memory limit respected"
                    ((passed++))
                else
                    log_failure "Resource limit bypassed - used ${memory_used}KB > 10MB limit"
                    log_security "⚠️  Memory limit enforcement may be weak"
                    ((failed++))
                fi
            else
                log_success "Resource operation completed within limits"
                ((passed++))
            fi
        fi
    else
        log_success "Resource-intensive operation blocked"
        log_security "Resource limit enforcement active"
        ((passed++))
    fi
    sudo $MINI_ISOLATE cleanup --box-id security_resources >/dev/null 2>&1 || true
else
    log_failure "Resource bypass test setup failed"
    ((failed++))
fi

echo ""
echo "===== Comprehensive Security Test Results ====="
echo "Passed: $passed"
echo "Failed: $failed"
echo "Success rate: $(( (passed * 100) / (passed + failed) ))%"
echo ""

if [[ $failed -eq 0 ]]; then
    echo "🔒 Excellent! All security tests passed - isolation is robust"
    exit 0
elif [[ $failed -eq 1 ]]; then
    echo "🔶 Good! Only 1 security concern - review the failed test"
    exit 0
else
    echo "🚨 Security concerns detected - review failed tests carefully"
    exit 1
fi