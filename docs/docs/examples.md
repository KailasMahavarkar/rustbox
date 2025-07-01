# Examples and Usage Guide

This document provides practical examples of using Mini-Isolate in various scenarios.

## Basic Usage Examples

### 1. Simple Program Execution

#### Initialize an isolate
```bash
mini-isolate init --box-id 0 --mem 128 --time 10
```

#### Run a simple command
```bash
mini-isolate run --box-id 0 "/bin/echo" -- "Hello World"
```

#### Run with verbose output
```bash
mini-isolate run --box-id 0 --verbose "/usr/bin/python3" -- "-c" "print('Hello from Python')"
```

### 2. Source File Execution

#### Execute Python script
```bash
# Create a Python script
echo 'print("Hello from Python script")' > hello.py

# Execute it
mini-isolate execute --box-id 0 --source hello.py --verbose
```

#### Execute C program
```bash
# Create a C program
cat > hello.c << 'EOF'
#include <stdio.h>
int main() {
    printf("Hello from C!\n");
    return 0;
}
EOF

# Execute it (automatically compiles and runs)
mini-isolate execute --box-id 0 --source hello.c --verbose
```

#### Execute with input file
```bash
# Create input file
echo "5 3" > input.txt

# Create a program that reads input
cat > sum.py << 'EOF'
a, b = map(int, input().split())
print(f"Sum: {a + b}")
EOF

# Execute with input
mini-isolate execute --box-id 0 --source sum.py --input input.txt --verbose
```

### 3. JSON Output and Automation

```bash
# Run with JSON output
mini-isolate execute --box-id 0 --source hello.py --output result.json

# Check the result
cat result.json
```

Example JSON output:
```json
{
  "exit_code": 0,
  "status": "Success",
  "stdout": "Hello from Python script\n",
  "stderr": "",
  "cpu_time": 0.015,
  "wall_time": 0.045,
  "memory_peak": 8642560,
  "signal": null,
  "success": true,
  "error_message": null
}
```

### 4. Resource Limits

#### Set strict memory limit
```bash
mini-isolate init --box-id strict --mem 32 --time 5 --processes 1
```

#### Test memory limit with a program
```bash
cat > memory_test.py << 'EOF'
# Try to allocate 64MB of memory
data = [0] * (64 * 1024 * 1024 // 8)  # 64MB of integers
print("Memory allocated successfully")
EOF

# This should trigger memory limit (when running with root privileges)
mini-isolate execute --box-id strict --source memory_test.py --verbose
```

### 5. Time Limits

#### Set short time limit
```bash
mini-isolate init --box-id quick --time 1 --wall-time 2
```

#### Test with infinite loop
```bash
cat > infinite.py << 'EOF'
import time
i = 0
while True:
    i += 1
    time.sleep(0.1)
    if i % 10 == 0:
        print(f"Iteration {i}")
EOF

# This should be killed due to time limit
mini-isolate execute --box-id quick --source infinite.py --verbose
```

## Programming Contest Usage

### Setup for Contest Environment

```bash
#!/bin/bash
# Setup script for programming contest

# Initialize isolate instances for multiple contestants
for i in {0..9}; do
    mini-isolate init --box-id $i --mem 256 --time 30 --wall-time 60 --processes 5 --fsize 128
done

echo "Contest environment ready with 10 isolate instances"
```

### Automated Judging Script

```bash
#!/bin/bash
# Automated judging script

PROBLEM_DIR="problems/problem1"
SOLUTION_FILE="$1"
BOX_ID="${2:-0}"

if [ ! -f "$SOLUTION_FILE" ]; then
    echo "Usage: $0 <solution_file> [box_id]"
    exit 1
fi

echo "Testing solution: $SOLUTION_FILE"

# Test against each test case
for test_case in "$PROBLEM_DIR"/test*.txt; do
    test_name=$(basename "$test_case" .txt)
    expected_output="$PROBLEM_DIR/${test_name}.out"
    
    echo "Running test case: $test_name"
    
    # Run the solution
    mini-isolate execute --box-id "$BOX_ID" --source "$SOLUTION_FILE" \
        --input "$test_case" --output "result_${test_name}.json"
    
    exit_code=$?
    
    # Check execution result
    if [ $exit_code -eq 0 ]; then
        # Extract stdout from JSON result
        actual_output=$(python3 -c "
import json
with open('result_${test_name}.json', 'r') as f:
    data = json.load(f)
    print(data['stdout'].rstrip())
")
        
        expected=$(cat "$expected_output" | tr -d '\n')
        actual=$(echo "$actual_output" | tr -d '\n')
        
        if [ "$expected" = "$actual" ]; then
            echo "✓ $test_name: PASSED"
        else
            echo "✗ $test_name: WRONG ANSWER"
            echo "Expected: $expected"
            echo "Actual:   $actual"
        fi
    elif [ $exit_code -eq 2 ]; then
        echo "✗ $test_name: TIME LIMIT EXCEEDED"
    elif [ $exit_code -eq 3 ]; then
        echo "✗ $test_name: MEMORY LIMIT EXCEEDED" 
    else
        echo "✗ $test_name: RUNTIME ERROR (exit code: $exit_code)"
    fi
done

# Cleanup
rm -f result_*.json
```

### Mass Testing Script

```bash
#!/bin/bash
# Test multiple solutions

SOLUTIONS_DIR="solutions"
PROBLEM_DIR="problems/problem1"

for solution in "$SOLUTIONS_DIR"/*.py "$SOLUTIONS_DIR"/*.cpp "$SOLUTIONS_DIR"/*.c; do
    if [ -f "$solution" ]; then
        echo "===================="
        echo "Testing: $(basename "$solution")"
        echo "===================="
        ./judge.sh "$solution"
        echo
    fi
done
```

## Educational Usage

### Safe Code Execution Environment

```bash
#!/bin/bash
# Educational environment setup

# Create a teaching environment
mini-isolate init --box-id classroom --mem 128 --time 10 --processes 3 --fsize 64

# Function to run student code safely
run_student_code() {
    local student_file="$1"
    local student_name="$2"
    
    echo "Running code from $student_name..."
    
    mini-isolate execute --box-id classroom --source "$student_file" \
        --output "${student_name}_result.json" --verbose
    
    # Extract and show results
    python3 -c "
import json
with open('${student_name}_result.json', 'r') as f:
    data = json.load(f)
    print(f'Status: {data[\"status\"]}')
    print(f'Time: {data[\"wall_time\"]:.3f}s')
    if data['success']:
        print('Output:', repr(data['stdout']))
    else:
        print('Error:', data['error_message'] or 'Runtime error')
"
}

# Example usage
echo 'print("Hello from student code!")' > student1.py
run_student_code student1.py "Alice"
```

## Development and Testing

### Local Development Workflow

```bash
#!/bin/bash
# Development workflow script

DEV_BOX="dev"

# Initialize development environment
mini-isolate init --box-id $DEV_BOX --mem 512 --time 30

# Function to test current code
test_code() {
    local source_file="$1"
    
    echo "Testing $source_file..."
    mini-isolate execute --box-id $DEV_BOX --source "$source_file" \
        --verbose --output test_result.json
    
    # Show summary
    if [ $? -eq 0 ]; then
        echo "✓ Test passed"
    else
        echo "✗ Test failed"
    fi
}

# Function to run performance test
perf_test() {
    local source_file="$1"
    local input_file="$2"
    
    echo "Performance testing $source_file with $input_file..."
    
    for i in {1..5}; do
        mini-isolate execute --box-id $DEV_BOX --source "$source_file" \
            --input "$input_file" --output "perf_$i.json" >/dev/null
        
        time=$(python3 -c "
import json
with open('perf_$i.json', 'r') as f:
    print(f'{json.load(f)[\"wall_time\"]:.3f}')
")
        echo "Run $i: ${time}s"
    done
    
    rm -f perf_*.json
}

# Example usage
echo 'print(sum(range(1000000)))' > test_program.py
test_code test_program.py
```

## Security Testing

### Safe Malicious Code Testing

```bash
#!/bin/bash
# Security testing environment

# Create highly restricted environment
mini-isolate init --box-id sandbox --mem 64 --time 5 --wall-time 10 --processes 1 --fsize 32

# Test potentially dangerous code safely
test_dangerous_code() {
    cat > dangerous.py << 'EOF'
# This code attempts various potentially malicious activities
import os
import subprocess
import sys

print("Attempting to read /etc/passwd...")
try:
    with open('/etc/passwd', 'r') as f:
        print("SUCCESS: Read system file")
except:
    print("BLOCKED: Cannot read system file")

print("Attempting to execute shell command...")
try:
    result = subprocess.run(['ps', 'aux'], capture_output=True, text=True)
    print("SUCCESS: Executed shell command")
except:
    print("BLOCKED: Cannot execute shell command")

print("Attempting to create large file...")
try:
    with open('bigfile.txt', 'w') as f:
        f.write('X' * (100 * 1024 * 1024))  # 100MB
    print("SUCCESS: Created large file")
except:
    print("BLOCKED: Cannot create large file")
EOF

    echo "Testing potentially malicious code..."
    mini-isolate execute --box-id sandbox --source dangerous.py --verbose
}

test_dangerous_code
```

## Cleanup and Maintenance

### Cleanup Scripts

```bash
#!/bin/bash
# Cleanup script

# Clean specific instances
mini-isolate cleanup --box-id test
mini-isolate cleanup --box-id dev

# Clean all instances
mini-isolate cleanup --all

# System cleanup
echo "Cleaning temporary files..."
rm -f *.json *.py *.c *.cpp *.out result_*
```

### System Monitoring

```bash
#!/bin/bash
# Monitor Mini-Isolate usage

echo "=== Mini-Isolate System Status ==="
mini-isolate info --cgroups

echo -e "\n=== Active Instances ==="
mini-isolate list

echo -e "\n=== System Resources ==="
df -h /tmp | grep -E "(Filesystem|tmpfs)"
free -h
```

## Best Practices

1. **Always set appropriate resource limits** based on expected usage
2. **Use specific box IDs** for different purposes (contest, dev, testing)
3. **Clean up instances** when done to free resources
4. **Use JSON output** for automated processing and logging
5. **Test with time limits** to prevent infinite loops
6. **Set memory limits** to prevent system overload
7. **Use verbose mode** during development and debugging
8. **Implement proper error handling** in automation scripts
9. **Regular cleanup** of temporary files and instances
10. **Monitor system resources** when running multiple instances