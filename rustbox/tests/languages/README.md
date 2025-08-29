# Language Tests for Rustbox

This directory contains test programs for different programming languages to verify rustbox functionality.

## Test Structure

Each `lang_*` folder contains 5 test programs:

1. **test_1_fact** - Factorial calculation (tests basic math and recursion)
2. **test_2_star** - Star pattern printing (tests loops and string output)
3. **test_3_lis** - Longest Increasing Subsequence (tests algorithms and arrays)
4. **test_4_tle** - Infinite loop (tests time limit enforcement)
5. **test_5_mle** - Memory exhaustion (tests memory limit enforcement)

## Supported Languages

-   **C++** (`lang_cpp/`) - Standard C++ programs with STL
-   **Java** (`lang_java/`) - Java programs with standard library
-   **Python** (`lang_python/`) - Python 3 programs

## Testing with Rustbox

### Using execute-code (Recommended)

The `execute-code` command provides the best experience with proper stdin handling and automatic compilation:

```bash
# Initialize sandbox once
./target/release/rustbox init --box-id 1

# Python
./target/release/rustbox execute-code --box-id 1 --language python --stdin "5" --time 5 --mem 100 --processes 10 --code "$(cat test.py)"

# C++
./target/release/rustbox execute-code --box-id 1 --language cpp --stdin "5" --time 10 --mem 300 --processes 15 --code "$(cat test.cpp)"

# Java (needs more resources for JVM)
./target/release/rustbox execute-code --box-id 1 --language java --stdin "5" --time 15 --mem 500 --processes 20 --code "$(cat Test.java)"

```

## Test Results

Based on testing with `test_execute_code.sh` using the execute-code command:

-   ✅ **Python**: Works perfectly with full stdin/stdout support
-   ✅ **C++**: Works perfectly with proper process limits (--processes 15)
-   ✅ **Java**: Works perfectly with adequate JVM resources (500MB, 20 processes)

## Known Limitations

1. **Compilation Time**: Compiled languages may need longer time limits for compilation
2. **Threading**: Languages that require thread creation (Node.js, Go) may fail in restricted environments
3. **JVM**: Java requires significant memory and time for JVM startup and compilation
4. **Process Limits**: Some languages hit process creation limits in the sandbox

## Usage Notes

-   Copy test files to `/tmp` before execution to ensure sandbox access
-   Use appropriate time and memory limits based on language requirements
-   For compiled languages, consider separating compilation and execution phases
-   Python is the most reliable for testing core rustbox functionality
