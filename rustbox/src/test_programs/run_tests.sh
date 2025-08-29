#!/bin/bash
# Test runner script for rustbox

RUSTBOX="../target/release/rustbox"
BOX_ID=1

echo "=== RustBox Test Suite ==="
echo "Using rustbox: $RUSTBOX"
echo "Box ID: $BOX_ID"
echo ""

# Check if rustbox binary exists
if [ ! -f "$RUSTBOX" ]; then
    echo "Error: rustbox binary not found at $RUSTBOX"
    echo "Please compile rustbox first"
    exit 1
fi

# Compile C programs if needed
if [ ! -f "hello" ]; then
    echo "Compiling C programs..."
    ./compile_all.sh
    echo ""
fi

# Function to run a test with rustbox
run_test() {
    local test_name=$1
    local program=$2
    local mem_limit=${3:-64}      # Default 64MB
    local time_limit=${4:-10}     # Default 10 seconds
    local extra_args=${5:-}       # Extra arguments
    
    echo "=== Testing: $test_name ==="
    echo "Program: $program"
    echo "Memory limit: ${mem_limit}MB, Time limit: ${time_limit}s"
    
    # Initialize sandbox
    echo "Initializing sandbox..."
    $RUSTBOX init --box-id $BOX_ID
    
    # Run the test
    echo "Running test..."
    $RUSTBOX run --box-id $BOX_ID --mem $mem_limit --time $time_limit $extra_args $program
    
    # Cleanup
    echo "Cleaning up..."
    $RUSTBOX cleanup --box-id $BOX_ID
    echo ""
}

# Basic functionality tests
run_test "Python Hello World" "python3 hello.py"
run_test "C Hello World" "./hello"

# File I/O tests  
run_test "Python File I/O" "python3 file_io.py"
run_test "C File I/O" "./file_io"

# Network tests
run_test "Python Network" "python3 network.py"
run_test "C Network" "./network"

# System call tests
run_test "Python System Calls" "python3 syscalls.py"
run_test "C System Calls" "./syscalls"

# Memory tests (with different limits)
run_test "Python Memory (32MB limit)" "python3 memory.py" 32
run_test "C Memory (32MB limit)" "./memory" 32
run_test "Python Memory (128MB limit)" "python3 memory.py" 128
run_test "C Memory (128MB limit)" "./memory" 128

# Timeout tests
run_test "Python Timeout" "python3 timeout.py" 64 15
run_test "C Timeout" "./timeout" 64 15
run_test "Python Infinite Loop" "python3 timeout.py infinite" 64 5
run_test "C Infinite Loop" "./timeout infinite" 64 5

# Process tests
run_test "Python Fork" "python3 fork.py" 64 20 "--processes 5"
run_test "C Fork" "./fork" 64 20 "--processes 5"

echo "=== Test Suite Complete ==="