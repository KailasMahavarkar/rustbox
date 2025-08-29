#!/bin/bash

# Parallel rustbox Test
# Runs 50 isolates concurrently with resource limits using execute-code

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"

if [[ $EUID -ne 0 ]]; then
    echo "❌ Requires sudo"
    exit 1
fi

echo "===== PARALLEL TEST (50 isolates) ====="
echo "Configuration: 50MB memory, 5s CPU time"
echo "Start: $(date)"
echo ""

start_time=$(date +%s)
succeeded=0
failed=0

# Inline stress test code
STRESS_CODE='
import sys
import time
import gc

def stress_workload(memory_mb, cpu_iterations, work_duration):
    print(f"Starting workload: {memory_mb}MB memory, {cpu_iterations} CPU iterations, {work_duration}s duration")
    
    # Memory allocation
    print("Allocating memory...")
    try:
        buffer = bytearray(memory_mb * 1024 * 1024)
        
        # Touch all pages to ensure real allocation
        page_size = 4096
        for i in range(0, len(buffer), page_size):
            buffer[i] = (i // page_size) % 256
        
        print(f"Successfully allocated {memory_mb}MB")
    except MemoryError as e:
        print(f"Memory allocation failed: {e}")
        return False
    
    # CPU work with memory access
    print("Starting CPU work...")
    start_time = time.time()
    result = 0
    
    iterations_per_second = cpu_iterations // max(1, work_duration)
    
    for second in range(work_duration):
        second_start = time.time()
        
        # CPU intensive work
        for i in range(iterations_per_second):
            result += i * i * i
            
            # Periodically touch memory to keep it active
            if i % 10000 == 0 and len(buffer) > 0:
                idx = (i * page_size) % len(buffer)
                buffer[idx] = (buffer[idx] + 1) % 256
        
        elapsed = time.time() - start_time
        print(f"Progress: {second + 1}/{work_duration}s (elapsed: {elapsed:.1f}s)")
        
        # Sleep to maintain timing if we finished early
        second_elapsed = time.time() - second_start
        if second_elapsed < 1.0:
            time.sleep(1.0 - second_elapsed)
    
    elapsed = time.time() - start_time
    
    # Final memory verification
    checksum = sum(buffer[i] for i in range(0, len(buffer), page_size * 100))
    
    print(f"Workload completed in {elapsed:.2f}s")
    print(f"CPU result: {result}")
    print(f"Memory checksum: {checksum}")
    
    return True

# Run the stress test
success = stress_workload(40, 150000, 4)
sys.exit(0 if success else 1)
'

echo "Launching 50 parallel isolates..."

for i in {1..50}; do
    (
        box_id=$((3000 + i))
        
        # Run stress test using execute-code
        result=$(sudo "$MINI_ISOLATE" execute-code \
            --box-id "$box_id" \
            --language python \
            --code "$STRESS_CODE" \
            --mem 50 \
            --cpu 5 \
            --time 10 2>&1)
        
        # Store result
        if [[ "$result" == *'"status": "Success"'* ]]; then
            echo "✅ Test $i: SUCCESS"
            echo "1" > "/tmp/result_$i"
        else
            echo "❌ Test $i: FAILED"
            echo "0" > "/tmp/result_$i"
        fi
        
    ) &
    
    # Batch control - start 10 at a time
    if (( i % 10 == 0 )); then
        sleep 0.5
        echo "Batch $((i/10)) launched..."
    fi
done

echo "All 50 tests launched, waiting for completion..."
wait

# Count results
succeeded=$(cat /tmp/result_* 2>/dev/null | grep -c "1" || echo "0")
succeeded=$(echo "$succeeded" | tr -d '\n')
failed=$((50 - succeeded))

# Cleanup
rm -f /tmp/result_*

end_time=$(date +%s)
duration=$((end_time - start_time))

echo ""
echo "===== PARALLEL TEST RESULTS ====="
echo "Tests completed: 50"
echo "Succeeded: $succeeded"
echo "Failed: $failed"
echo "Success rate: $(( (succeeded * 100) / 50 ))%"
echo "Duration: ${duration}s"
echo "End: $(date)"

if [[ $succeeded -ge 45 ]]; then
    echo "🎉 Excellent! ${succeeded}/50 parallel tests succeeded!"
    exit 0
else
    echo "⚠️ Only ${succeeded}/50 tests succeeded"
    exit 1
fi