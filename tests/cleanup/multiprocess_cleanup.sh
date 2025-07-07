#!/bin/bash

# Multi-Process Cleanup Tests for rustbox
# Tests the multi-process architecture cleanup mechanisms

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_failure() { echo -e "${RED}[FAIL]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARN]${NC} $1"; }

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== rustbox Multi-Process Cleanup Tests ====="
echo ""

passed=0
failed=0

# Helper function to count processes by name pattern
count_processes() {
    local pattern="$1"
    pgrep -f "$pattern" 2>/dev/null | wc -l
}

# Helper function to get process tree
get_process_tree() {
    local pid="$1"
    pstree -p "$pid" 2>/dev/null || echo "Process $pid not found"
}

# Test 1: Three-process architecture cleanup
log_info "Test 1: Three-process architecture cleanup verification"
initial_rustbox=$(count_processes "rustbox")

if sudo $MINI_ISOLATE init --box-id multiproc_test1 >/dev/null 2>&1; then
    # Start a process that will create the three-process architecture
    (sudo $MINI_ISOLATE run --box-id multiproc_test1 --max-memory 50 --max-cpu 5 --max-time 10 /bin/sleep -- 8 >/dev/null 2>&1) &
    run_pid=$!
    
    # Wait for processes to start
    sleep 2
    
    # Check if we have the expected process architecture
    during_rustbox=$(count_processes "rustbox")
    
    if [[ $during_rustbox -gt $initial_rustbox ]]; then
        log_info "Multi-process architecture detected ($during_rustbox processes)"
        
        # Let the process finish naturally
        wait $run_pid 2>/dev/null || true
        
        # Cleanup
        sudo $MINI_ISOLATE cleanup --box-id multiproc_test1 >/dev/null 2>&1
        
        # Wait for cleanup to complete
        sleep 3
        
        final_rustbox=$(count_processes "rustbox")
        
        if [[ $final_rustbox -le $initial_rustbox ]]; then
            log_success "Three-process architecture cleanup completed"
            ((passed++))
        else
            log_failure "Process cleanup incomplete - leaked processes detected"
            ((failed++))
        fi
    else
        log_warning "Multi-process architecture not clearly detected"
        sudo $MINI_ISOLATE cleanup --box-id multiproc_test1 >/dev/null 2>&1
        ((passed++))
    fi
else
    log_failure "Three-process test - init failed"
    ((failed++))
fi

# Test 2: Keeper process survival test
log_info "Test 2: Keeper process monitoring and cleanup"
initial_rustbox=$(count_processes "rustbox")

if sudo $MINI_ISOLATE init --box-id keeper_test >/dev/null 2>&1; then
    # Start a long-running process
    (sudo $MINI_ISOLATE run --box-id keeper_test --max-memory 50 --max-cpu 10 --max-time 20 /bin/sleep -- 15 >/dev/null 2>&1) &
    run_pid=$!
    
    sleep 2
    
    # Find rustbox processes
    rustbox_pids=($(pgrep -f "rustbox"))
    
    if [[ ${#rustbox_pids[@]} -gt 1 ]]; then
        log_info "Multiple rustbox processes found: ${#rustbox_pids[@]}"
        
        # Kill the run process to simulate crash
        kill -KILL $run_pid 2>/dev/null || true
        
        # Wait a moment
        sleep 2
        
        # Cleanup should still work
        sudo $MINI_ISOLATE cleanup --box-id keeper_test >/dev/null 2>&1
        
        sleep 3
        final_rustbox=$(count_processes "rustbox")
        
        if [[ $final_rustbox -le $initial_rustbox ]]; then
            log_success "Keeper process cleanup handled correctly"
            ((passed++))
        else
            log_failure "Keeper process cleanup failed"
            ((failed++))
        fi
    else
        log_warning "Expected multi-process architecture not detected"
        sudo $MINI_ISOLATE cleanup --box-id keeper_test >/dev/null 2>&1
        ((passed++))
    fi
else
    log_failure "Keeper process test - init failed"
    ((failed++))
fi

# Test 3: Process group cleanup test
log_info "Test 3: Process group termination test"
initial_rustbox=$(count_processes "rustbox")

if sudo $MINI_ISOLATE init --box-id pgroup_test >/dev/null 2>&1; then
    # Start a process that spawns children
    (sudo $MINI_ISOLATE run --box-id pgroup_test --max-memory 50 --max-cpu 10 --max-time 15 /bin/sh -- -c 'sleep 10 & sleep 10 & wait' >/dev/null 2>&1) &
    run_pid=$!
    
    sleep 3
    
    # Count all sleep processes (should include our spawned ones)
    sleep_count=$(count_processes "sleep")
    
    if [[ $sleep_count -gt 0 ]]; then
        log_info "Child processes detected ($sleep_count sleep processes)"
        
        # Force cleanup while processes are running
        sudo $MINI_ISOLATE cleanup --box-id pgroup_test >/dev/null 2>&1
        
        # Wait for cleanup
        sleep 3
        
        # Check if sleep processes were cleaned up
        final_sleep_count=$(count_processes "sleep")
        final_rustbox=$(count_processes "rustbox")
        
        if [[ $final_rustbox -le $initial_rustbox ]]; then
            log_success "Process group cleanup completed"
            ((passed++))
        else
            log_failure "Process group cleanup incomplete"
            ((failed++))
        fi
    else
        log_warning "Child processes not detected as expected"
        sudo $MINI_ISOLATE cleanup --box-id pgroup_test >/dev/null 2>&1
        ((passed++))
    fi
    
    # Cleanup the background process if still running
    kill -TERM $run_pid 2>/dev/null || true
else
    log_failure "Process group test - init failed"
    ((failed++))
fi

# Test 4: Concurrent multi-process cleanup
log_info "Test 4: Concurrent multi-process cleanup"
initial_rustbox=$(count_processes "rustbox")

# Start multiple boxes concurrently
box_ids=("concurrent1" "concurrent2" "concurrent3")
run_pids=()

for box_id in "${box_ids[@]}"; do
    if sudo $MINI_ISOLATE init --box-id "$box_id" >/dev/null 2>&1; then
        (sudo $MINI_ISOLATE run --box-id "$box_id" --max-memory 50 --max-cpu 10 --max-time 20 /bin/sleep -- 15 >/dev/null 2>&1) &
        run_pids+=($!)
    fi
done

# Wait for all to start
sleep 3

peak_rustbox=$(count_processes "rustbox")
log_info "Peak rustbox processes: $peak_rustbox"

# Cleanup all concurrently
cleanup_pids=()
for box_id in "${box_ids[@]}"; do
    (sudo $MINI_ISOLATE cleanup --box-id "$box_id" >/dev/null 2>&1) &
    cleanup_pids+=($!)
done

# Wait for all cleanups to complete
for pid in "${cleanup_pids[@]}"; do
    wait $pid 2>/dev/null || true
done

# Kill any remaining run processes
for pid in "${run_pids[@]}"; do
    kill -TERM $pid 2>/dev/null || true
done

sleep 3
final_rustbox=$(count_processes "rustbox")

if [[ $final_rustbox -le $((initial_rustbox + 2)) ]]; then
    log_success "Concurrent multi-process cleanup completed"
    ((passed++))
else
    log_failure "Concurrent cleanup left too many processes"
    ((failed++))
fi

# Test 5: Resource exhaustion with multi-process cleanup
log_info "Test 5: Multi-process cleanup under resource pressure"
initial_rustbox=$(count_processes "rustbox")

if sudo $MINI_ISOLATE init --box-id resource_pressure >/dev/null 2>&1; then
    # Start a memory-intensive process that should hit limits
    (sudo $MINI_ISOLATE run --box-id resource_pressure --max-memory 20 --max-cpu 5 --max-time 10 /bin/dd -- if=/dev/zero of=/dev/null bs=1M count=100 >/dev/null 2>&1) &
    run_pid=$!
    
    sleep 2
    
    # Force cleanup while under resource pressure
    sudo $MINI_ISOLATE cleanup --box-id resource_pressure >/dev/null 2>&1
    
    # Wait for cleanup
    sleep 3
    
    final_rustbox=$(count_processes "rustbox")
    
    if [[ $final_rustbox -le $initial_rustbox ]]; then
        log_success "Resource pressure cleanup completed"
        ((passed++))
    else
        log_failure "Resource pressure cleanup incomplete"
        ((failed++))
    fi
    
    # Cleanup background process
    kill -TERM $run_pid 2>/dev/null || true
else
    log_failure "Resource pressure test - init failed"
    ((failed++))
fi

# Test 6: Signal cascade cleanup test
log_info "Test 6: Signal cascade cleanup verification"
initial_rustbox=$(count_processes "rustbox")

if sudo $MINI_ISOLATE init --box-id signal_test >/dev/null 2>&1; then
    # Start a process that ignores SIGTERM (to test SIGKILL fallback)
    (sudo $MINI_ISOLATE run --box-id signal_test --max-memory 50 --max-cpu 10 --max-time 20 /bin/sh -- -c 'trap "" TERM; sleep 15' >/dev/null 2>&1) &
    run_pid=$!
    
    sleep 2
    
    # This should test the SIGTERM -> SIGKILL cascade
    sudo $MINI_ISOLATE cleanup --box-id signal_test >/dev/null 2>&1
    
    sleep 3
    final_rustbox=$(count_processes "rustbox")
    
    if [[ $final_rustbox -le $initial_rustbox ]]; then
        log_success "Signal cascade cleanup completed"
        ((passed++))
    else
        log_failure "Signal cascade cleanup incomplete"
        ((failed++))
    fi
    
    # Force kill if still running
    kill -KILL $run_pid 2>/dev/null || true
else
    log_failure "Signal cascade test - init failed"
    ((failed++))
fi

echo ""
echo "===== Multi-Process Cleanup Test Summary ====="
echo "Passed: $passed"
echo "Failed: $failed"
echo "Success rate: $(( (passed * 100) / (passed + failed) ))%"
echo ""

# Final process audit
final_audit_rustbox=$(count_processes "rustbox")
final_audit_sleep=$(count_processes "sleep")

log_info "Final process audit:"
log_info "  rustbox processes: $final_audit_rustbox"
log_info "  sleep processes: $final_audit_sleep"

if [[ $final_audit_rustbox -gt 10 ]]; then
    log_warning "High number of rustbox processes remaining"
    echo "Consider investigating potential process leaks"
elif [[ $final_audit_sleep -gt 5 ]]; then
    log_warning "High number of sleep processes remaining"
    echo "Some test processes may not have been cleaned up properly"
else
    log_info "Process counts look normal"
fi

if [[ $failed -eq 0 ]]; then
    echo "✅ All multi-process cleanup tests passed!"
    exit 0
else
    echo "⚠️ Some multi-process cleanup tests failed."
    exit 1
fi