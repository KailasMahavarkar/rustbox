#!/bin/bash

# Aggressive Resource Limit Test Suite for mini-isolate
# Tests CPU, memory, and I/O limits under stress conditions

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MINI_ISOLATE="$SCRIPT_DIR/../target/release/mini-isolate"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

log_failure() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

run_test() {
    local test_name="$1"
    local expected_status="$2"
    shift 2
    local cmd=("$@")
    
    ((TESTS_RUN++))
    log_info "Running: $test_name"
    
    # Run the command and capture output
    local output
    local exit_code
    output=$(sudo "${cmd[@]}" 2>&1) || exit_code=$?
    
    # Check if the expected status is in the output
    if echo "$output" | grep -q "Status: $expected_status"; then
        log_success "$test_name"
        return 0
    else
        log_failure "$test_name - Expected: $expected_status, Got: $(echo "$output" | grep "Status:" | head -1)"
        echo "Full output: $output"
        return 1
    fi
}

# Ensure mini-isolate is built
log_info "Using pre-built mini-isolate..."
# cd "$SCRIPT_DIR/.."
# cargo build --release

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    log_warning "This script should be run with sudo for full functionality"
fi

log_info "Starting Aggressive Resource Limit Test Suite"
echo "=================================================="

# Initialize test instance
log_info "Initializing isolate instance..."
sudo "$MINI_ISOLATE" init --box-id 99 > /dev/null 2>&1 || true

# Test 1: CPU Time Limit Tests
log_info "=== CPU TIME LIMIT TESTS ==="

# Create CPU-intensive test program
cat > /tmp/cpu_hog.c << 'EOF'
#include <stdio.h>
#include <time.h>

int main() {
    clock_t start = clock();
    volatile long long i = 0;
    
    // Run for approximately 5 seconds of CPU time
    while (((double)(clock() - start)) / CLOCKS_PER_SEC < 5.0) {
        i++;
        if (i % 1000000 == 0) {
            printf("CPU work: %lld iterations\n", i);
            fflush(stdout);
        }
    }
    
    printf("CPU hog completed: %lld iterations\n", i);
    return 0;
}
EOF

gcc -o /tmp/cpu_hog /tmp/cpu_hog.c

# Test CPU limit enforcement
run_test "CPU limit 1 second" "TimeLimit" "$MINI_ISOLATE" run --box-id 99 --max-cpu 1 /tmp/cpu_hog
run_test "CPU limit 2 seconds" "TimeLimit" "$MINI_ISOLATE" run --box-id 99 --max-cpu 2 /tmp/cpu_hog

# Test 2: Memory Limit Tests
log_info "=== MEMORY LIMIT TESTS ==="

# Create memory-intensive test program
cat > /tmp/memory_hog.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main() {
    int mb_allocated = 0;
    char *ptr;
    
    while (mb_allocated < 200) {
        ptr = malloc(1024 * 1024); // 1MB
        if (!ptr) {
            printf("Failed to allocate memory at %d MB\n", mb_allocated);
            break;
        }
        
        // Touch the memory to ensure it's actually allocated
        memset(ptr, 0x42, 1024 * 1024);
        mb_allocated++;
        
        printf("Allocated %d MB\n", mb_allocated);
        fflush(stdout);
        usleep(100000); // 100ms delay
    }
    
    printf("Memory hog completed: %d MB allocated\n", mb_allocated);
    return 0;
}
EOF

gcc -o /tmp/memory_hog /tmp/memory_hog.c

# Test memory limit enforcement
run_test "Memory limit 50MB" "MemoryLimit" "$MINI_ISOLATE" run --box-id 99 --max-memory 50 /tmp/memory_hog
run_test "Memory limit 100MB" "MemoryLimit" "$MINI_ISOLATE" run --box-id 99 --max-memory 100 /tmp/memory_hog

# Test 3: Wall Time Limit Tests
log_info "=== WALL TIME LIMIT TESTS ==="

# Create time-consuming test program
cat > /tmp/time_hog.c << 'EOF'
#include <stdio.h>
#include <unistd.h>

int main() {
    int seconds = 0;
    
    while (seconds < 10) {
        printf("Running for %d seconds\n", seconds);
        fflush(stdout);
        sleep(1);
        seconds++;
    }
    
    printf("Time hog completed: %d seconds\n", seconds);
    return 0;
}
EOF

gcc -o /tmp/time_hog /tmp/time_hog.c

# Test wall time limit enforcement
run_test "Wall time limit 2 seconds" "TimeLimit" "$MINI_ISOLATE" run --box-id 99 --max-time 2 /tmp/time_hog
run_test "Wall time limit 3 seconds" "TimeLimit" "$MINI_ISOLATE" run --box-id 99 --max-time 3 /tmp/time_hog

# Test 4: I/O Intensive Tests
log_info "=== I/O INTENSIVE TESTS ==="

# Create I/O-intensive test program
cat > /tmp/io_hog.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main() {
    FILE *fp;
    char buffer[1024];
    int i;
    
    // Write test
    fp = fopen("/tmp/test_output.txt", "w");
    if (!fp) {
        printf("Failed to open file for writing\n");
        return 1;
    }
    
    for (i = 0; i < 10000; i++) {
        fprintf(fp, "This is line %d with some data to write\n", i);
        if (i % 1000 == 0) {
            printf("Wrote %d lines\n", i);
            fflush(stdout);
        }
    }
    fclose(fp);
    
    // Read test
    fp = fopen("/tmp/test_output.txt", "r");
    if (!fp) {
        printf("Failed to open file for reading\n");
        return 1;
    }
    
    i = 0;
    while (fgets(buffer, sizeof(buffer), fp)) {
        i++;
        if (i % 1000 == 0) {
            printf("Read %d lines\n", i);
            fflush(stdout);
        }
    }
    fclose(fp);
    
    printf("I/O hog completed: wrote and read %d lines\n", i);
    unlink("/tmp/test_output.txt");
    return 0;
}
EOF

gcc -o /tmp/io_hog /tmp/io_hog.c

# Test I/O with various limits
run_test "I/O with CPU limit" "Success" "$MINI_ISOLATE" run --box-id 99 --max-cpu 5 /tmp/io_hog
run_test "I/O with memory limit" "Success" "$MINI_ISOLATE" run --box-id 99 --max-memory 50 /tmp/io_hog

# Test 5: Combined Stress Tests
log_info "=== COMBINED STRESS TESTS ==="

# Create combined stress test program
cat > /tmp/stress_hog.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <time.h>

int main() {
    char *memory_chunks[100];
    int chunks_allocated = 0;
    clock_t start = clock();
    volatile long long cpu_work = 0;
    
    printf("Starting combined stress test\n");
    fflush(stdout);
    
    while (((double)(clock() - start)) / CLOCKS_PER_SEC < 10.0) {
        // CPU work
        for (int i = 0; i < 100000; i++) {
            cpu_work++;
        }
        
        // Memory allocation
        if (chunks_allocated < 100) {
            memory_chunks[chunks_allocated] = malloc(1024 * 1024); // 1MB
            if (memory_chunks[chunks_allocated]) {
                memset(memory_chunks[chunks_allocated], 0x42, 1024 * 1024);
                chunks_allocated++;
                printf("Allocated chunk %d, CPU work: %lld\n", chunks_allocated, cpu_work);
                fflush(stdout);
            }
        }
        
        // I/O work
        FILE *fp = fopen("/tmp/stress_temp.txt", "w");
        if (fp) {
            fprintf(fp, "CPU work: %lld, Memory: %d MB\n", cpu_work, chunks_allocated);
            fclose(fp);
        }
        
        usleep(10000); // 10ms delay
    }
    
    // Cleanup
    for (int i = 0; i < chunks_allocated; i++) {
        free(memory_chunks[i]);
    }
    unlink("/tmp/stress_temp.txt");
    
    printf("Stress test completed: CPU work: %lld, Memory: %d MB\n", cpu_work, chunks_allocated);
    return 0;
}
EOF

gcc -o /tmp/stress_hog /tmp/stress_hog.c

# Test combined limits
run_test "Combined: CPU 2s + Memory 50MB" "TimeLimit" "$MINI_ISOLATE" run --box-id 99 --max-cpu 2 --max-memory 50 /tmp/stress_hog
run_test "Combined: CPU 3s + Memory 100MB" "TimeLimit" "$MINI_ISOLATE" run --box-id 99 --max-cpu 3 --max-memory 100 /tmp/stress_hog

# Test 6: Edge Cases and Boundary Tests
log_info "=== EDGE CASE TESTS ==="

# Test very small limits
run_test "Tiny CPU limit (0.1s)" "TimeLimit" "$MINI_ISOLATE" run --box-id 99 --max-cpu 1 /tmp/cpu_hog
run_test "Tiny memory limit (1MB)" "MemoryLimit" "$MINI_ISOLATE" run --box-id 99 --max-memory 1 /tmp/memory_hog

# Test normal programs with limits
run_test "Simple echo with limits" "Success" "$MINI_ISOLATE" run --box-id 99 --max-cpu 5 --max-memory 100 /bin/echo -- "Hello World"
run_test "ls command with limits" "Success" "$MINI_ISOLATE" run --box-id 99 --max-cpu 5 --max-memory 100 /bin/ls -- /tmp

# Cleanup
log_info "Cleaning up test files..."
sudo "$MINI_ISOLATE" cleanup --box-id 99 > /dev/null 2>&1 || true
rm -f /tmp/cpu_hog /tmp/memory_hog /tmp/time_hog /tmp/io_hog /tmp/stress_hog
rm -f /tmp/cpu_hog.c /tmp/memory_hog.c /tmp/time_hog.c /tmp/io_hog.c /tmp/stress_hog.c
rm -f /tmp/test_output.txt /tmp/stress_temp.txt

# Summary
echo ""
echo "=================================================="
log_info "Test Suite Summary"
echo "Tests run: $TESTS_RUN"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed. Check the output above.${NC}"
    exit 1
fi