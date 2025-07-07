#!/bin/bash

# Sequential rustbox Test
# Runs 5 isolates one after another with resource limits

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../../target/release/rustbox"
STRESS_PROGRAM="$SCRIPT_DIR/stress_program.py"

if [[ $EUID -ne 0 ]]; then
    echo "❌ Requires sudo"
    exit 1
fi

echo "===== SEQUENTIAL TEST (5 isolates) ====="
echo "Configuration: 100MB memory, 10s CPU time"
echo "Start: $(date)"
echo ""

succeeded=0
failed=0

for i in {1..5}; do
    echo "Running test $i..."
    box_id="seq_test_$i"
    
    # Initialize isolate
    if ! sudo "$MINI_ISOLATE" init --box-id "$box_id" >/dev/null 2>&1; then
        echo "❌ Test $i: Init failed"
        ((failed++))
        continue
    fi
    
    # Copy stress program and run
    sudo cp "$STRESS_PROGRAM" "/tmp/rustbox/$box_id/stress_program.py"
    
    result=$(sudo "$MINI_ISOLATE" run \
        --box-id "$box_id" \
        --max-memory 100 \
        --max-cpu 10 \
        --max-time 15 \
        python3 -- stress_program.py 80 300000 8 2>&1)
    
    # Check result and cleanup
    sudo "$MINI_ISOLATE" cleanup --box-id "$box_id" >/dev/null 2>&1
    
    if [[ "$result" == *"Status: Success"* ]]; then
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