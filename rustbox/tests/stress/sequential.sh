#!/bin/bash

# Sequential rustbox Test
# Runs 5 isolates one after another with resource limits using execute-code

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"

if [[ $EUID -ne 0 ]]; then
    echo "❌ Requires sudo"
    exit 1
fi

echo "===== SEQUENTIAL TEST (5 isolates) ====="
echo "Configuration: 50MB memory, 5s CPU time (optimized for speed)"
echo "Start: $(date)"
echo ""

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

# Run the stress test (reduced workload for faster execution)
success = stress_workload(40, 100000, 3)
sys.exit(0 if success else 1)
'

for i in {1..5}; do
    echo "Running test $i..."
    box_id=$((4000 + i))
    
    # Run stress test using execute-code
    result=$(sudo "$MINI_ISOLATE" execute-code \
        --box-id "$box_id" \
        --language python \
        --code "$STRESS_CODE" \
        --mem 50 \
        --cpu 5 \
        --time 8 2>&1)
    
    if [[ "$result" == *'"status": "Success"'* ]]; then
        echo "✅ Test $i: SUCCESS"
        ((succeeded++))
    else
        echo "❌ Test $i: FAILED"
        ((failed++))
    fi
    
    sleep 1  # Brief pause between tests
done

echo ""
echo "===== SEQUENTIAL TEST RESULTS ====="
echo "Succeeded: $succeeded/5"
echo "Failed: $failed/5"
echo "Success rate: $(( (succeeded * 100) / 5 ))%"
echo "End: $(date)"

if [[ $succeeded -eq 5 ]]; then
    echo "🎉 All sequential tests passed!"
    exit 0
else
    echo "⚠️ Some tests failed"
    exit 1
fi