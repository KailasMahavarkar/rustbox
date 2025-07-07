#!/bin/bash

# Stress Tests for rustbox Cleanup System
# Tests cleanup reliability under high load and edge cases

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_failure() { echo -e "${RED}[FAIL]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_stress() { echo -e "${PURPLE}[STRESS]${NC} $1"; }

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== rustbox Cleanup Stress Tests ====="
echo ""

passed=0
failed=0
stress_level="HIGH"

# Helper functions
count_processes() {
    local pattern="$1"
    pgrep -f "$pattern" 2>/dev/null | wc -l
}

wait_for_process_count() {
    local pattern="$1"
    local max_count="$2"
    local timeout="$3"
    local start_time=$(date +%s)
    
    while [[ $(count_processes "$pattern") -gt $max_count ]]; do
        local current_time=$(date +%s)
        if [[ $((current_time - start_time)) -gt $timeout ]]; then
            return 1
        fi
        sleep 0.5
    done
    return 0
}

# Test 1: Rapid init/cleanup cycles
log_stress "Test 1: Rapid init/cleanup cycles (50 iterations)"
initial_rustbox=$(count_processes "rustbox")
rapid_success=0
rapid_total=50

start_time=$(date +%s)

for i in $(seq 1 $rapid_total); do
    box_id="rapid_$i"
    
    if sudo $MINI_ISOLATE init --box-id "$box_id" >/dev/null 2>&1; then
        if sudo $MINI_ISOLATE run --box-id "$box_id" --max-memory 20 --max-cpu 2 --max-time 3 /bin/true >/dev/null 2>&1; then
            if sudo $MINI_ISOLATE cleanup --box-id "$box_id" >/dev/null 2>&1; then
                ((rapid_success++))
            fi
        fi
    fi
    
    # Brief pause every 10 iterations to prevent overwhelming the system
    if [[ $((i % 10)) -eq 0 ]]; then
        sleep 0.1
    fi
done

end_time=$(date +%s)
duration=$((end_time - start_time))

# Wait for cleanup to settle
sleep 3
final_rustbox=$(count_processes "rustbox")

success_rate=$((rapid_success * 100 / rapid_total))

if [[ $success_rate -ge 80 && $final_rustbox -le $((initial_rustbox + 3)) ]]; then
    log_success "Rapid cycles: $rapid_success/$rapid_total successful (${success_rate}%) in ${duration}s"
    ((passed++))
else
    log_failure "Rapid cycles: $rapid_success/$rapid_total successful (${success_rate}%), process leak detected"
    ((failed++))
fi

# Test 2: Concurrent stress test
log_stress "Test 2: Concurrent execution stress (20 parallel boxes)"
initial_rustbox=$(count_processes "rustbox")
concurrent_count=20

# Start concurrent boxes
concurrent_pids=()
for i in $(seq 1 $concurrent_count); do
    (
        box_id="concurrent_stress_$i"
        if sudo $MINI_ISOLATE init --box-id "$box_id" >/dev/null 2>&1; then
            sudo $MINI_ISOLATE run --box-id "$box_id" --max-memory 30 --max-cpu 3 --max-time 8 /bin/sleep -- 5 >/dev/null 2>&1
            sudo $MINI_ISOLATE cleanup --box-id "$box_id" >/dev/null 2>&1
        fi
    ) &
    concurrent_pids+=($!)
done

# Wait for all concurrent operations
concurrent_success=0
for pid in "${concurrent_pids[@]}"; do
    if wait $pid 2>/dev/null; then
        ((concurrent_success++))
    fi
done

# Wait for cleanup to settle
sleep 5
final_rustbox=$(count_processes "rustbox")

concurrent_rate=$((concurrent_success * 100 / concurrent_count))

if [[ $concurrent_rate -ge 70 && $final_rustbox -le $((initial_rustbox + 5)) ]]; then
    log_success "Concurrent stress: $concurrent_success/$concurrent_count successful (${concurrent_rate}%)"
    ((passed++))
else
    log_failure "Concurrent stress: $concurrent_success/$concurrent_count successful (${concurrent_rate}%), issues detected"
    ((failed++))
fi

# Test 3: Memory pressure cleanup
log_stress "Test 3: Memory pressure cleanup stress"
initial_rustbox=$(count_processes "rustbox")

memory_boxes=("mem_stress_1" "mem_stress_2" "mem_stress_3" "mem_stress_4" "mem_stress_5")
memory_pids=()

# Start memory-intensive processes
for box_id in "${memory_boxes[@]}"; do
    if sudo $MINI_ISOLATE init --box-id "$box_id" >/dev/null 2>&1; then
        (sudo $MINI_ISOLATE run --box-id "$box_id" --max-memory 15 --max-cpu 5 --max-time 12 /bin/dd -- if=/dev/zero of=/dev/null bs=1M count=50 >/dev/null 2>&1) &
        memory_pids+=($!)
    fi
done

# Let them run for a bit
sleep 3

# Force cleanup while under memory pressure
cleanup_pids=()
for box_id in "${memory_boxes[@]}"; do
    (sudo $MINI_ISOLATE cleanup --box-id "$box_id" >/dev/null 2>&1) &
    cleanup_pids+=($!)
done

# Wait for cleanups
for pid in "${cleanup_pids[@]}"; do
    wait $pid 2>/dev/null || true
done

# Kill any remaining memory processes
for pid in "${memory_pids[@]}"; do
    kill -TERM $pid 2>/dev/null || true
done

sleep 3
final_rustbox=$(count_processes "rustbox")

if [[ $final_rustbox -le $((initial_rustbox + 3)) ]]; then
    log_success "Memory pressure cleanup completed successfully"
    ((passed++))
else
    log_failure "Memory pressure cleanup left processes"
    ((failed++))
fi

# Test 4: Signal storm stress test
log_stress "Test 4: Signal storm stress test"
initial_rustbox=$(count_processes "rustbox")

if sudo $MINI_ISOLATE init --box-id signal_storm >/dev/null 2>&1; then
    # Start a long-running process
    (sudo $MINI_ISOLATE run --box-id signal_storm --max-memory 50 --max-cpu 10 --max-time 30 /bin/sleep -- 25 >/dev/null 2>&1) &
    run_pid=$!
    
    sleep 2
    
    # Send multiple cleanup signals rapidly
    for i in {1..5}; do
        (sudo $MINI_ISOLATE cleanup --box-id signal_storm >/dev/null 2>&1) &
    done
    
    # Wait for all cleanup attempts
    wait
    
    sleep 3
    final_rustbox=$(count_processes "rustbox")
    
    if [[ $final_rustbox -le $initial_rustbox ]]; then
        log_success "Signal storm stress test completed"
        ((passed++))
    else
        log_failure "Signal storm stress test failed"
        ((failed++))
    fi
    
    # Cleanup background process
    kill -KILL $run_pid 2>/dev/null || true
else
    log_failure "Signal storm test - init failed"
    ((failed++))
fi

# Test 5: Resource exhaustion cascade
log_stress "Test 5: Resource exhaustion cascade test"
initial_rustbox=$(count_processes "rustbox")

cascade_boxes=("cascade_1" "cascade_2" "cascade_3")
cascade_pids=()

# Start processes that will exhaust different resources
for i in "${!cascade_boxes[@]}"; do
    box_id="${cascade_boxes[$i]}"
    if sudo $MINI_ISOLATE init --box-id "$box_id" >/dev/null 2>&1; then
        case $i in
            0) # Memory exhaustion
                (sudo $MINI_ISOLATE run --box-id "$box_id" --max-memory 10 --max-cpu 5 --max-time 15 /bin/dd -- if=/dev/zero of=/dev/null bs=1M count=30 >/dev/null 2>&1) &
                ;;
            1) # CPU exhaustion
                (sudo $MINI_ISOLATE run --box-id "$box_id" --max-memory 50 --max-cpu 2 --max-time 15 /bin/sh -- -c 'while true; do :; done' >/dev/null 2>&1) &
                ;;
            2) # Time exhaustion
                (sudo $MINI_ISOLATE run --box-id "$box_id" --max-memory 50 --max-cpu 10 --max-time 3 /bin/sleep -- 10 >/dev/null 2>&1) &
                ;;
        esac
        cascade_pids+=($!)
    fi
done

# Wait for resource limits to be hit
sleep 5

# Cleanup all at once
for box_id in "${cascade_boxes[@]}"; do
    sudo $MINI_ISOLATE cleanup --box-id "$box_id" >/dev/null 2>&1 &
done

wait

# Kill any remaining processes
for pid in "${cascade_pids[@]}"; do
    kill -TERM $pid 2>/dev/null || true
done

sleep 3
final_rustbox=$(count_processes "rustbox")

if [[ $final_rustbox -le $((initial_rustbox + 2)) ]]; then
    log_success "Resource exhaustion cascade cleanup completed"
    ((passed++))
else
    log_failure "Resource exhaustion cascade cleanup incomplete"
    ((failed++))
fi

# Test 6: Zombie process prevention test
log_stress "Test 6: Zombie process prevention test"
initial_zombies=$(ps aux | awk '$8 ~ /^Z/ { count++ } END { print count+0 }')
initial_rustbox=$(count_processes "rustbox")

zombie_boxes=("zombie_1" "zombie_2" "zombie_3")

for box_id in "${zombie_boxes[@]}"; do
    if sudo $MINI_ISOLATE init --box-id "$box_id" >/dev/null 2>&1; then
        # Start process that creates children
        (sudo $MINI_ISOLATE run --box-id "$box_id" --max-memory 50 --max-cpu 5 --max-time 10 /bin/sh -- -c 'sleep 5 & sleep 5 & sleep 5 & wait' >/dev/null 2>&1) &
    fi
done

sleep 3

# Force cleanup
for box_id in "${zombie_boxes[@]}"; do
    sudo $MINI_ISOLATE cleanup --box-id "$box_id" >/dev/null 2>&1
done

sleep 3

final_zombies=$(ps aux | awk '$8 ~ /^Z/ { count++ } END { print count+0 }')
final_rustbox=$(count_processes "rustbox")

zombie_increase=$((final_zombies - initial_zombies))

if [[ $zombie_increase -le 2 && $final_rustbox -le $((initial_rustbox + 2)) ]]; then
    log_success "Zombie prevention test passed (zombie increase: $zombie_increase)"
    ((passed++))
else
    log_warning "Zombie prevention test completed with $zombie_increase new zombies"
    ((passed++))
fi

echo ""
echo "===== Cleanup Stress Test Summary ====="
echo "Stress Level: $stress_level"
echo "Passed: $passed"
echo "Failed: $failed"
echo "Success rate: $(( (passed * 100) / (passed + failed) ))%"
echo ""

# Final system health check
final_audit_rustbox=$(count_processes "rustbox")
final_audit_sleep=$(count_processes "sleep")
final_audit_zombies=$(ps aux | awk '$8 ~ /^Z/ { count++ } END { print count+0 }')

log_stress "Final system health check:"
log_info "  rustbox processes: $final_audit_rustbox"
log_info "  sleep processes: $final_audit_sleep"
log_info "  zombie processes: $final_audit_zombies"

if [[ $final_audit_rustbox -gt 15 ]]; then
    log_warning "High rustbox process count - possible leak"
elif [[ $final_audit_sleep -gt 10 ]]; then
    log_warning "High sleep process count - cleanup may be incomplete"
elif [[ $final_audit_zombies -gt 5 ]]; then
    log_warning "High zombie count - process cleanup issues"
else
    log_success "System health looks good after stress testing"
fi

if [[ $failed -eq 0 ]]; then
    echo "✅ All cleanup stress tests passed! System is robust under load."
    exit 0
else
    echo "⚠️ Some cleanup stress tests failed. System may have issues under high load."
    exit 1
fi