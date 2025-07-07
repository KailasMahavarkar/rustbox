#!/bin/bash

# Performance Tests for Mini-Isolate
# Tests performance characteristics and benchmarks

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/mini-isolate"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo "[INFO] $1"; }
log_success() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_failure() { echo -e "${RED}[FAIL]${NC} $1"; }
log_benchmark() { echo -e "${YELLOW}[BENCHMARK]${NC} $1"; }

if [[ $EUID -ne 0 ]]; then
    echo "❌ This script requires sudo privileges"
    exit 1
fi

echo "===== Mini-Isolate Performance Tests ====="
echo ""

passed=0
failed=0

# Performance Test 1: Startup time
log_info "Performance Test 1: Isolate startup time"
startup_times=()
for i in {1..5}; do
    start_time=$(date +%s.%N)
    if sudo $MINI_ISOLATE init --box-id "perf_startup_$i" >/dev/null 2>&1; then
        end_time=$(date +%s.%N)
        startup_time=$(echo "$end_time - $start_time" | bc)
        startup_times+=($startup_time)
        sudo $MINI_ISOLATE cleanup --box-id "perf_startup_$i" >/dev/null 2>&1 || true
    else
        log_failure "Startup test iteration $i failed"
        ((failed++))
    fi
done

if [[ ${#startup_times[@]} -eq 5 ]]; then
    avg_startup=$(echo "${startup_times[@]}" | tr ' ' '\n' | awk '{sum+=$1} END {printf "%.3f", sum/NR}')
    log_benchmark "Average startup time: ${avg_startup}s"
    
    # Good performance if startup is under 0.5 seconds
    if (( $(echo "$avg_startup < 0.5" | bc -l) )); then
        log_success "Startup performance (${avg_startup}s < 0.5s)"
        ((passed++))
    elif (( $(echo "$avg_startup < 1.0" | bc -l) )); then
        log_success "Acceptable startup performance (${avg_startup}s < 1.0s)"
        ((passed++))
    else
        log_failure "Slow startup performance (${avg_startup}s >= 1.0s)"
        ((failed++))
    fi
else
    log_failure "Startup performance test failed"
    ((failed++))
fi

# Performance Test 2: Execution overhead
log_info "Performance Test 2: Command execution overhead"
exec_times=()
for i in {1..5}; do
    if sudo $MINI_ISOLATE init --box-id "perf_exec_$i" >/dev/null 2>&1; then
        start_time=$(date +%s.%N)
        result=$(sudo $MINI_ISOLATE run --box-id "perf_exec_$i" --max-memory 50 --max-cpu 5 --max-time 5 /bin/echo -- "performance test" 2>&1)
        end_time=$(date +%s.%N)
        
        if [[ "$result" == *"Status: Success"* ]]; then
            exec_time=$(echo "$end_time - $start_time" | bc)
            exec_times+=($exec_time)
        fi
        
        sudo $MINI_ISOLATE cleanup --box-id "perf_exec_$i" >/dev/null 2>&1 || true
    fi
done

if [[ ${#exec_times[@]} -eq 5 ]]; then
    avg_exec=$(echo "${exec_times[@]}" | tr ' ' '\n' | awk '{sum+=$1} END {printf "%.3f", sum/NR}')
    log_benchmark "Average execution time: ${avg_exec}s"
    
    # Good performance if execution is under 0.2 seconds
    if (( $(echo "$avg_exec < 0.2" | bc -l) )); then
        log_success "Execution performance (${avg_exec}s < 0.2s)"
        ((passed++))
    elif (( $(echo "$avg_exec < 0.5" | bc -l) )); then
        log_success "Acceptable execution performance (${avg_exec}s < 0.5s)"
        ((passed++))
    else
        log_failure "Slow execution performance (${avg_exec}s >= 0.5s)"
        ((failed++))
    fi
else
    log_failure "Execution performance test failed"
    ((failed++))
fi

# Performance Test 3: Memory overhead
log_info "Performance Test 3: Memory usage overhead"
if sudo $MINI_ISOLATE init --box-id "perf_memory" >/dev/null 2>&1; then
    # Get baseline memory usage
    baseline_mem=$(ps -o pid,vsz,rss -p $$ | tail -n 1 | awk '{print $2}')
    
    # Run a simple command and check memory usage
    result=$(sudo $MINI_ISOLATE run --box-id "perf_memory" --max-memory 50 --max-cpu 5 --max-time 5 /bin/echo -- "memory test" 2>&1)
    
    if [[ "$result" == *"Memory peak:"* ]]; then
        memory_used=$(echo "$result" | grep "Memory peak:" | awk '{print $3}' | sed 's/[^0-9]//g')
        log_benchmark "Memory usage: ${memory_used} KB"
        
        # Good if under 10MB (10240 KB)
        if [[ $memory_used -lt 10240 ]]; then
            log_success "Memory efficiency (${memory_used} KB < 10240 KB)"
            ((passed++))
        elif [[ $memory_used -lt 51200 ]]; then  # 50MB
            log_success "Acceptable memory usage (${memory_used} KB < 51200 KB)"
            ((passed++))
        else
            log_failure "High memory usage (${memory_used} KB >= 51200 KB)"
            ((failed++))
        fi
    else
        log_failure "Could not measure memory usage"
        ((failed++))
    fi
    
    sudo $MINI_ISOLATE cleanup --box-id "perf_memory" >/dev/null 2>&1 || true
else
    log_failure "Memory performance test setup failed"
    ((failed++))
fi

# Performance Test 4: Throughput test
log_info "Performance Test 4: Sequential operation throughput"
start_time=$(date +%s.%N)
successful_ops=0

for i in {1..10}; do
    if sudo $MINI_ISOLATE init --box-id "throughput_$i" >/dev/null 2>&1; then
        if result=$(sudo $MINI_ISOLATE run --box-id "throughput_$i" --max-memory 50 --max-cpu 5 --max-time 5 /bin/echo -- "throughput test $i" 2>&1); then
            if [[ "$result" == *"Status: Success"* ]]; then
                ((successful_ops++))
            fi
        fi
        sudo $MINI_ISOLATE cleanup --box-id "throughput_$i" >/dev/null 2>&1 || true
    fi
done

end_time=$(date +%s.%N)
total_time=$(echo "$end_time - $start_time" | bc)
ops_per_sec=$(echo "scale=2; $successful_ops / $total_time" | bc)

log_benchmark "Throughput: ${ops_per_sec} operations/second (${successful_ops}/10 successful)"
log_benchmark "Total time for 10 operations: ${total_time}s"

if [[ $successful_ops -eq 10 ]]; then
    # Good if more than 2 ops/sec
    if (( $(echo "$ops_per_sec > 2.0" | bc -l) )); then
        log_success "Good throughput (${ops_per_sec} ops/sec > 2.0)"
        ((passed++))
    elif (( $(echo "$ops_per_sec > 1.0" | bc -l) )); then
        log_success "Acceptable throughput (${ops_per_sec} ops/sec > 1.0)"
        ((passed++))
    else
        log_failure "Low throughput (${ops_per_sec} ops/sec <= 1.0)"
        ((failed++))
    fi
else
    log_failure "Throughput test - only ${successful_ops}/10 operations successful"
    ((failed++))
fi

echo ""
echo "===== Performance Tests Summary ====="
echo "Passed: $passed"
echo "Failed: $failed"
echo "Success rate: $(( (passed * 100) / (passed + failed) ))%"
echo ""

if [[ $failed -eq 0 ]]; then
    echo "✅ All performance tests passed! Mini-isolate is performing well."
    exit 0
else
    echo "⚠️ Some performance tests failed. Consider optimization."
    exit 1
fi