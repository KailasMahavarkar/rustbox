#!/bin/bash

# Parallel rustbox Test
# Runs 50 isolates concurrently with resource limits

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"
STRESS_PROGRAM="$SCRIPT_DIR/stress_program.py"

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

echo "Launching 50 parallel isolates..."

for i in {1..50}; do
    (
        box_id="parallel_${i}_$$"
        
        # Initialize isolate
        if ! sudo "$MINI_ISOLATE" init --box-id "$box_id" >/dev/null 2>&1; then
            echo "❌ Test $i: Init failed"
            echo "0" > "/tmp/result_$i"
            exit 1
        fi
        
        # Copy stress program and run
        sudo cp "$STRESS_PROGRAM" "/tmp/rustbox/$box_id/stress_program.py"
        
        result=$(sudo "$MINI_ISOLATE" run \
            --box-id "$box_id" \
            --max-memory 50 \
            --max-cpu 5 \
            --max-time 10 \
            python3 -- stress_program.py 40 150000 4 2>&1)
        
        # Cleanup
        sudo "$MINI_ISOLATE" cleanup --box-id "$box_id" >/dev/null 2>&1
        
        # Store result
        if [[ "$result" == *"Status: Success"* ]]; then
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