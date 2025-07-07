# IOI Isolate Compatibility Guide

This document explains the compatibility between rustbox and the original IOI isolate sandbox, highlighting similarities, differences, and migration strategies.

## Overview

rustbox is designed to be largely compatible with the IOI isolate sandbox used in programming contests. It provides similar functionality with some differences in implementation and features.

## Command-Line Interface Compatibility

### Supported Commands

| IOI Isolate | rustbox | Compatibility | Notes |
|-------------|--------------|---------------|-------|
| `isolate --init` | `rustbox init` | ✓ Full | Same functionality |
| `isolate --run` | `rustbox run` | ✓ Full | Same behavior |
| `isolate --cleanup` | `rustbox cleanup` | ✓ Full | Same cleanup process |
| `isolate --version` | `rustbox version` | ✓ Full | Shows version info |
| N/A | `rustbox execute` | ➕ Extra | Source file execution |
| N/A | `rustbox list` | ➕ Extra | List active instances |
| N/A | `rustbox info` | ➕ Extra | System information |

### Parameter Compatibility

| IOI Isolate Option | rustbox Option | Compatibility | Notes |
|-------------------|-------------------|---------------|-------|
| `--box-id=ID` | `--box-id ID` | ✓ Full | Same functionality |
| `--mem=SIZE` | `--mem SIZE` | ✓ Full | Memory limit in MB |
| `--time=SEC` | `--time SEC` | ✓ Full | CPU time limit |
| `--wall-time=SEC` | `--wall-time SEC` | ✓ Full | Wall clock limit |
| `--fsize=SIZE` | `--fsize SIZE` | ✓ Full | File size limit in MB |
| `--processes=N` | `--processes N` | ✓ Full | Process count limit |
| `--verbose` | `--verbose` | ✓ Full | Verbose output |
| `--silent` | `--quiet` | ✓ Similar | Minimal output |
| `--dir=PATH` | `--dir PATH` | ⚠️ Planned | Directory binding |
| `--chdir=PATH` | N/A | ❌ Not implemented | Working directory change |
| `--env=VAR=VAL` | `--env VAR=VAL` | ⚠️ Planned | Environment variables |

## Exit Code Compatibility

rustbox follows the same exit code conventions as IOI isolate:

| Exit Code | Meaning | IOI Isolate | rustbox |
|-----------|---------|-------------|--------------|
| 0 | Success | ✓ | ✓ |
| 1 | Runtime Error | ✓ | ✓ |
| 2 | Time Limit Exceeded | ✓ | ✓ |
| 3 | Memory Limit Exceeded | ✓ | ✓ |
| 4 | Output limit exceeded | ✓ | ⚠️ Planned |
| 5 | Compile Error | ✓ | ✓ |
| 6 | File limit exceeded | ✓ | ⚠️ TBD |
| 10+ | System/Internal Error | ✓ | ✓ |

## Migration Examples

### Basic IOI Isolate Usage

#### IOI Isolate
```bash
isolate --init --box-id=0 --mem=128 --time=10
isolate --run --box-id=0 --mem=128 --time=10 -- /usr/bin/python3 solution.py
isolate --cleanup --box-id=0
```

#### rustbox
```bash
rustbox init --box-id 0 --mem 128 --time 10
rustbox run --box-id 0 "/usr/bin/python3" -- "solution.py"
rustbox cleanup --box-id 0
```

### Contest Judge System Migration

#### Original IOI Isolate Script
```bash
#!/bin/bash
# Original judge script

BOX_ID="$1"
SOLUTION="$2"
INPUT="$3"
OUTPUT="$4"

isolate --cleanup --box-id="$BOX_ID" >/dev/null 2>&1
isolate --init --box-id="$BOX_ID" --mem=256 --time=10 --wall-time=20

# Copy solution
cp "$SOLUTION" "/var/local/lib/isolate/$BOX_ID/box/solution.py"

# Run with input
isolate --run --box-id="$BOX_ID" --mem=256 --time=10 --wall-time=20 \
    --stdin="$INPUT" --stdout="$OUTPUT" \
    -- /usr/bin/python3 solution.py

RESULT=$?

isolate --cleanup --box-id="$BOX_ID"
exit $RESULT
```

#### rustbox Migration
```bash
#!/bin/bash
# Migrated judge script

BOX_ID="$1"
SOLUTION="$2" 
INPUT="$3"
OUTPUT="$4"

rustbox cleanup --box-id "$BOX_ID" >/dev/null 2>&1
rustbox init --box-id "$BOX_ID" --mem 256 --time 10 --wall-time 20

# Execute with input/output redirection
rustbox execute --box-id "$BOX_ID" --source "$SOLUTION" \
    --input "$INPUT" --output result.json

RESULT=$?

# Extract stdout to output file
if [ $RESULT -eq 0 ]; then
    python3 -c "
import json
with open('result.json', 'r') as f:
    data = json.load(f)
    print(data['stdout'], end='')
" > "$OUTPUT"
fi

rustbox cleanup --box-id "$BOX_ID"
exit $RESULT
```

### Advanced Usage Patterns

#### Contest Management System

##### IOI Isolate Version
```bash
# Setup multiple boxes for parallel judging
for i in {0..9}; do
    isolate --init --box-id=$i --mem=512 --time=30 --wall-time=60
done

# Judge a submission
judge_submission() {
    local box_id="$1"
    local submission="$2"
    
    isolate --run --box-id="$box_id" --mem=512 --time=30 --wall-time=60 \
        --meta=meta.txt -- /usr/bin/python3 "$submission"
    
    # Parse meta.txt for detailed results
    grep "time:" meta.txt
    grep "max-rss:" meta.txt
}
```

##### rustbox Version
```bash
# Setup multiple boxes for parallel judging  
for i in {0..9}; do
    rustbox init --box-id $i --mem 512 --time 30 --wall-time 60
done

# Judge a submission
judge_submission() {
    local box_id="$1"
    local submission="$2"
    
    rustbox execute --box-id "$box_id" --source "$submission" \
        --output "result_${box_id}.json"
    
    # Parse JSON for detailed results
    python3 -c "
import json
with open('result_${box_id}.json', 'r') as f:
    data = json.load(f)
    print(f'Time: {data[\"cpu_time\"]}s')
    print(f'Memory: {data[\"memory_peak\"]} bytes')
"
}
```

## Key Differences and Enhancements

### 1. Source File Execution
rustbox introduces the `execute` command for direct source file execution:

```bash
# IOI Isolate (requires manual compilation)
gcc solution.c -o solution
isolate --run --box-id=0 -- ./solution

# rustbox (automatic compilation)
rustbox execute --box-id 0 --source solution.c
```

### 2. JSON Output Format
rustbox provides structured JSON output for easier automation:

```bash
rustbox execute --box-id 0 --source solution.py --output result.json
```

Result format:
```json
{
  "exit_code": 0,
  "status": "Success", 
  "stdout": "Hello World\n",
  "stderr": "",
  "cpu_time": 0.015,
  "wall_time": 0.045,
  "memory_peak": 8642560,
  "signal": null,
  "success": true,
  "error_message": null
}
```

### 3. System Information Commands
rustbox provides additional system information:

```bash
rustbox info --cgroups    # Check cgroup support
rustbox list             # List active instances
```

### 4. Enhanced Error Messages
rustbox provides more descriptive error messages and warnings:

```
Warning: rustbox may require root privileges for full functionality
Some features like cgroups may not work without proper permissions
Warning: Cannot create cgroup (permission denied). Resource limits will not be enforced.
```

## Compatibility Testing

### Test Script for Migration Validation
```bash
#!/bin/bash
# Test compatibility between IOI isolate and rustbox

echo "=== Compatibility Test Suite ==="

# Test 1: Basic execution
echo "Testing basic execution..."
echo 'print("Hello World")' > test_basic.py

# Test with IOI isolate (if available)
if command -v isolate >/dev/null; then
    echo "IOI isolate test:"
    isolate --cleanup --box-id=99 >/dev/null 2>&1
    isolate --init --box-id=99 --mem=128 --time=10
    cp test_basic.py /var/local/lib/isolate/99/box/
    isolate --run --box-id=99 --mem=128 --time=10 -- /usr/bin/python3 test_basic.py
    isolate --cleanup --box-id=99
fi

echo "rustbox test:"
rustbox init --box-id 99 --mem 128 --time 10
rustbox execute --box-id 99 --source test_basic.py
rustbox cleanup --box-id 99

# Test 2: Time limit
echo -e "\nTesting time limits..."
cat > test_timeout.py << 'EOF'
import time
time.sleep(10)  # Should be killed before this completes
print("Should not print")
EOF

rustbox init --box-id 98 --time 1
rustbox execute --box-id 98 --source test_timeout.py
echo "Exit code: $?"  # Should be 2 (timeout)
rustbox cleanup --box-id 98

# Cleanup
rm -f test_*.py
```

## Migration Checklist

When migrating from IOI isolate to rustbox:

- [ ] **Command syntax**: Update command-line syntax (dashes vs spaces)
- [ ] **Output parsing**: Switch to JSON output format if needed
- [ ] **File handling**: Utilize the `execute` command for source files
- [ ] **Error handling**: Update error code handling if needed
- [ ] **Privileges**: Ensure proper permissions for cgroup functionality
- [ ] **Testing**: Run compatibility test suite
- [ ] **Documentation**: Update internal documentation and procedures
- [ ] **Scripts**: Update all automation scripts
- [ ] **Monitoring**: Update monitoring and logging systems

## Future Compatibility Plans

Planned features to improve IOI isolate compatibility:

1. **Directory binding** (`--dir` option)
2. **Working directory** (`--chdir` option)  
3. **Environment variables** (`--env` option)
4. **Output size limits**
5. **Meta file output** (similar to IOI isolate meta.txt)
6. **Network isolation** options
7. **User/group configuration**

## Support and Resources

- **Documentation**: See `docs/` folder for detailed guides
- **Examples**: Check `docs/examples.md` for practical usage
- **Issues**: Report compatibility issues on the project repository
- **Contact**: For migration assistance and questions

This compatibility guide ensures smooth transition from IOI isolate to rustbox while highlighting the enhanced features and capabilities of the new system.