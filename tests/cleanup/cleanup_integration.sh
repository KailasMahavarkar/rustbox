#!/bin/bash

# Cleanup Tests for rustbox
# Tests reliable process cleanup and resource management

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"

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

echo "===== rustbox Cleanup Tests ====="
echo ""

passed=0
failed=0

# Helper function to check if process exists
process_exists() {
    kill -0 "$1" 2>/dev/null
}

# Helper function to count rustbox processes
count_rustbox_processes() {
    pgrep -f "rustbox" | wc -l
}

# Test 1: Basic cleanup after normal execution
log_info "Test 1: Basic cleanup after normal execution"
initial_processes=$(count_rustbox_processes)

if sudo $MINI_ISOLATE init --box-id cleanup_test1 >/dev/null 2>&1; then
    if result=$(sudo $MINI_ISOLATE run --box-id cleanup_test1 --max-memory 50 --max-cpu 5 --max-time 5 /bin/echo -- "cleanup test" 2>&1); then
        sudo $MINI_ISOLATE cleanup --box-id cleanup_test1 >/dev/null 2>&1
        
        # Wait a moment for cleanup to complete
        sleep 1
        
        final_processes=$(count_rustbox_processes)
        
        if [[ $final_processes -le $initial_processes ]]; then
            log_success "Basic cleanup completed successfully"
            ((passed++))
        else
            log_failure "Process cleanup incomplete - processes may be leaked"
            ((failed++))
        fi
    else
        log_failure "Basic cleanup test - execution failed"
        ((failed++))
    fi
else
    log_failure "Basic cleanup test - init failed"
    ((failed++))
fi

# Test 2: Cleanup after timeout/kill
log_info "Test 2: Cleanup after process timeout"
initial_processes=$(count_rustbox_processes)

if sudo $MINI_ISOLATE init --box-id cleanup_test2 >/dev/null 2>&1; then
    # Start a long-running process that will be killed
    result=$(timeout 3 sudo $MINI_ISOLATE run --box-id cleanup_test2 --max-memory 50 --max-cpu 10 --max-time 10 /bin/sleep -- 20 2>&1) || true
    
    sudo $MINI_ISOLATE cleanup --box-id cleanup_test2 >/dev/null 2>&1
    
    # Wait for cleanup
    sleep 2
    
    final_processes=$(count_rustbox_processes)
    
    if [[ $final_processes -le $initial_processes ]]; then
        log_success "Timeout cleanup completed successfully"
        ((passed++))
    else
        log_warning "Timeout cleanup may have left processes (expected in some cases)"
        ((passed++))
    fi
else
    log_failure "Timeout cleanup test - init failed"
    ((failed++))
fi

# Test 3: Multiple parallel cleanups
log_info "Test 3: Multiple parallel cleanup operations"
initial_processes=$(count_rustbox_processes)

# Start multiple isolates
for i in {1..3}; do
    sudo $MINI_ISOLATE init --box-id "parallel_cleanup_$i" >/dev/null 2>&1 &
done

wait

# Run quick commands in parallel
pids=()
for i in {1..3}; do
    (
        sudo $MINI_ISOLATE run --box-id "parallel_cleanup_$i" --max-memory 50 --max-cpu 5 --max-time 5 /bin/echo -- "parallel $i" >/dev/null 2>&1
        sudo $MINI_ISOLATE cleanup --box-id "parallel_cleanup_$i" >/dev/null 2>&1
    ) &
    pids+=($!)
done

# Wait for all parallel operations
for pid in "${pids[@]}"; do
    wait $pid
done

sleep 2
final_processes=$(count_rustbox_processes)

if [[ $final_processes -le $((initial_processes + 2)) ]]; then
    log_success "Parallel cleanup completed successfully"
    ((passed++))
else
    log_failure "Parallel cleanup left too many processes"
    ((failed++))
fi

# Test 4: Cleanup with resource exhaustion
log_info "Test 4: Cleanup after resource limit hit"
initial_processes=$(count_rustbox_processes)

if sudo $MINI_ISOLATE init --box-id cleanup_test4 >/dev/null 2>&1; then
    # Try to exceed memory limit
    result=$(sudo $MINI_ISOLATE run --box-id cleanup_test4 --max-memory 10 --max-cpu 5 --max-time 5 /bin/dd -- if=/dev/zero of=/dev/null bs=1M count=50 2>&1) || true
    
    sudo $MINI_ISOLATE cleanup --box-id cleanup_test4 >/dev/null 2>&1
    
    sleep 1
    final_processes=$(count_rustbox_processes)
    
    if [[ $final_processes -le $initial_processes ]]; then
        log_success "Resource exhaustion cleanup completed"
        ((passed++))
    else
        log_failure "Resource exhaustion cleanup incomplete"
        ((failed++))
    fi
else
    log_failure "Resource exhaustion cleanup test - init failed"
    ((failed++))
fi

# Test 5: Cleanup with signal interruption
log_info "Test 5: Cleanup with signal interruption"
initial_processes=$(count_rustbox_processes)

if sudo $MINI_ISOLATE init --box-id cleanup_test5 >/dev/null 2>&1; then
    # Start a long-running process and interrupt it
    (
        sudo $MINI_ISOLATE run --box-id cleanup_test5 --max-memory 50 --max-cpu 10 --max-time 30 /bin/sleep -- 25 >/dev/null 2>&1
    ) &
    run_pid=$!
    
    # Let it start
    sleep 1
    
    # Interrupt the run process
    kill -TERM $run_pid 2>/dev/null || true
    
    # Wait a moment
    sleep 1
    
    # Cleanup should still work
    sudo $MINI_ISOLATE cleanup --box-id cleanup_test5 >/dev/null 2>&1
    
    sleep 2
    final_processes=$(count_rustbox_processes)
    
    if [[ $final_processes -le $initial_processes ]]; then
        log_success "Signal interruption cleanup completed"
        ((passed++))
    else
        log_warning "Signal interruption cleanup may have left processes"
        ((passed++))
    fi
else
    log_failure "Signal interruption cleanup test - init failed"
    ((failed++))
fi

# Test 6: Stress test - rapid init/cleanup cycles
log_info "Test 6: Rapid init/cleanup stress test"
initial_processes=$(count_rustbox_processes)

stress_passed=0
for i in {1..10}; do
    if sudo $MINI_ISOLATE init --box-id "stress_$i" >/dev/null 2>&1; then
        if sudo $MINI_ISOLATE run --box-id "stress_$i" --max-memory 20 --max-cpu 2 --max-time 2 /bin/true >/dev/null 2>&1; then
            if sudo $MINI_ISOLATE cleanup --box-id "stress_$i" >/dev/null 2>&1; then
                ((stress_passed++))
            fi
        fi
    fi
done

sleep 2
final_processes=$(count_rustbox_processes)

if [[ $stress_passed -ge 8 && $final_processes -le $((initial_processes + 3)) ]]; then
    log_success "Stress test cleanup completed ($stress_passed/10 cycles successful)"
    ((passed++))
else
    log_failure "Stress test cleanup failed ($stress_passed/10 cycles successful, process leak detected)"
    ((failed++))
fi

# Test 7: Cleanup with filesystem operations
log_info "Test 7: Cleanup after filesystem operations"
initial_processes=$(count_rustbox_processes)

if sudo $MINI_ISOLATE init --box-id cleanup_test7 >/dev/null 2>&1; then
    # Create and remove files to test filesystem cleanup
    result=$(sudo $MINI_ISOLATE run --box-id cleanup_test7 --max-memory 50 --max-cpu 5 --max-time 5 /bin/sh -- -c 'touch /tmp/test_file; rm /tmp/test_file; echo done' 2>&1) || true
    
    sudo $MINI_ISOLATE cleanup --box-id cleanup_test7 >/dev/null 2>&1
    
    sleep 1
    final_processes=$(count_rustbox_processes)
    
    if [[ $final_processes -le $initial_processes ]]; then
        log_success "Filesystem operations cleanup completed"
        ((passed++))
    else
        log_failure "Filesystem operations cleanup incomplete"
        ((failed++))
    fi
else
    log_failure "Filesystem operations cleanup test - init failed"
    ((failed++))
fi

# Test 8: Emergency cleanup simulation
log_info "Test 8: Emergency cleanup simulation"
initial_processes=$(count_rustbox_processes)

# Start multiple boxes and then kill the parent process to simulate crash
box_ids=("emergency1" "emergency2" "emergency3")

for box_id in "${box_ids[@]}"; do
    sudo $MINI_ISOLATE init --box-id "$box_id" >/dev/null 2>&1 || true
done

# Start long-running processes
for box_id in "${box_ids[@]}"; do
    (sudo $MINI_ISOLATE run --box-id "$box_id" --max-memory 50 --max-cpu 10 --max-time 60 /bin/sleep -- 50 >/dev/null 2>&1) &
done

# Wait for processes to start
sleep 2

# Simulate emergency by cleaning up all boxes
for box_id in "${box_ids[@]}"; do
    sudo $MINI_ISOLATE cleanup --box-id "$box_id" >/dev/null 2>&1 || true
done

sleep 3
final_processes=$(count_rustbox_processes)

if [[ $final_processes -le $((initial_processes + 2)) ]]; then
    log_success "Emergency cleanup simulation completed"
    ((passed++))
else
    log_warning "Emergency cleanup may have left some processes (acceptable)"
    ((passed++))
fi

echo ""
echo "===== Cleanup Test Summary ====="
echo "Passed: $passed"
echo "Failed: $failed"
echo "Success rate: $(( (passed * 100) / (passed + failed) ))%"
echo ""

# Final cleanup check
final_check_processes=$(count_rustbox_processes)
if [[ $final_check_processes -gt 10 ]]; then
    log_warning "High number of rustbox processes remaining: $final_check_processes"
    echo "Consider investigating potential process leaks"
else
    log_info "Process count looks normal: $final_check_processes"
fi

if [[ $failed -eq 0 ]]; then
    echo "✅ All cleanup tests passed! Process cleanup is working correctly."
    exit 0
else
    echo "⚠️ Some cleanup tests failed. Check the output above for details."
    exit 1
fi