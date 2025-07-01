#!/bin/bash

# Comprehensive Test Runner for mini-isolate
# Organizes and runs all test suites in logical order
# Entry-level friendly with clear documentation and sudo handling

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_TYPE="release"
VERBOSE=false
SUDO_AVAILABLE=false
RUN_PRIVILEGED=false
RUN_UNIT_ONLY=false
RUN_INTEGRATION_ONLY=false

# Helper functions
log_header() {
    echo -e "\n${BOLD}${CYAN}=== $1 ===${NC}"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((PASSED_TESTS++))
}

log_failure() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((FAILED_TESTS++))
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
    ((SKIPPED_TESTS++))
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

show_help() {
    cat << EOF
${BOLD}Mini-Isolate Test Runner${NC}

${BOLD}USAGE:${NC}
    $0 [OPTIONS]

${BOLD}OPTIONS:${NC}
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -d, --debug             Use debug build instead of release
    -s, --sudo              Run tests requiring sudo privileges
    -u, --unit-only         Run only unit tests
    -i, --integration-only  Run only integration tests
    --privileged-only       Run only tests requiring root privileges

${BOLD}TEST CATEGORIES:${NC}
    1. Unit Tests           - Library functionality tests (no privileges needed)
    2. Integration Tests    - Component interaction tests (some may need sudo)
    3. Security Tests       - Filesystem and isolation security (needs sudo)
    4. System Tests         - Full system validation (needs sudo)
    5. Performance Tests    - Resource limit validation (needs sudo)

${BOLD}EXAMPLES:${NC}
    $0                      # Run all tests that don't require sudo
    $0 -s                   # Run all tests including sudo-required ones
    $0 -u                   # Run only unit tests
    $0 -i                   # Run only integration tests
    $0 -v -s                # Run all tests with verbose output

${BOLD}NOTES:${NC}
    - Tests are run in logical order: Unit → Integration → Security → System
    - Sudo tests are skipped unless -s flag is provided
    - Failed tests don't stop execution; all results are reported at the end
    - Use -v for detailed output of test execution
EOF
}

check_sudo() {
    if sudo -n true 2>/dev/null; then
        SUDO_AVAILABLE=true
        log_info "Sudo access available"
    else
        SUDO_AVAILABLE=false
        log_warning "Sudo access not available - privileged tests will be skipped"
    fi
}

check_dependencies() {
    log_info "Checking dependencies..."
    
    # Check if cargo is available
    if ! command -v cargo &> /dev/null; then
        log_failure "Cargo not found. Please install Rust."
        return 1
    fi
    
    # Check if mini-isolate binary exists or can be built
    local binary_path="$SCRIPT_DIR/target/$BUILD_TYPE/mini-isolate"
    if [ ! -f "$binary_path" ]; then
        log_info "Building mini-isolate ($BUILD_TYPE)..."
        if [ "$BUILD_TYPE" = "release" ]; then
            if cargo build --release; then
                log_success "Build completed successfully"
            else
                log_failure "Build failed"
                return 1
            fi
        else
            if cargo build; then
                log_success "Build completed successfully"
            else
                log_failure "Build failed"
                return 1
            fi
        fi
    fi
    
    log_success "Dependencies check passed"
    return 0
}

run_unit_tests() {
    log_header "UNIT TESTS"
    log_info "Running library unit tests..."
    
    ((TOTAL_TESTS++))
    if $VERBOSE; then
        if cargo test --lib; then
            log_success "Unit tests passed"
        else
            log_failure "Unit tests failed"
        fi
    else
        if cargo test --lib --quiet >/dev/null 2>&1; then
            log_success "Unit tests passed"
        else
            log_failure "Unit tests failed"
        fi
    fi
}

run_integration_tests() {
    log_header "INTEGRATION TESTS"
    
    # Resource Limits Tests
    log_info "Running resource limits tests..."
    ((TOTAL_TESTS++))
    if $VERBOSE; then
        if cargo test --test resource_limits; then
            log_success "Resource limits tests passed"
        else
            log_failure "Resource limits tests failed"
        fi
    else
        if cargo test --test resource_limits --quiet; then
            log_success "Resource limits tests passed"
        else
            log_failure "Resource limits tests failed"
        fi
    fi
    
    # Namespace Tests (basic functionality)
    log_info "Running namespace isolation tests (basic)..."
    ((TOTAL_TESTS++))
    if $VERBOSE; then
        if cargo test --test namespace_isolation_tests; then
            log_success "Namespace tests passed"
        else
            log_failure "Namespace tests failed"
        fi
    else
        if cargo test --test namespace_isolation_tests --quiet; then
            log_success "Namespace tests passed"
        else
            log_failure "Namespace tests failed"
        fi
    fi
}

run_security_tests() {
    log_header "SECURITY TESTS"
    
    if [ "$RUN_PRIVILEGED" = true ] && [ "$SUDO_AVAILABLE" = true ]; then
        # Filesystem Security Tests (need sudo for proper isolation)
        log_info "Running filesystem security tests..."
        ((TOTAL_TESTS++))
        if $VERBOSE; then
            if sudo -E cargo test --test filesystem_security_tests; then
                log_success "Filesystem security tests passed"
            else
                log_failure "Filesystem security tests failed"
            fi
        else
            if sudo -E cargo test --test filesystem_security_tests --quiet; then
                log_success "Filesystem security tests passed"
            else
                log_failure "Filesystem security tests failed"
            fi
        fi
        
        # Namespace Tests (privileged functionality)
        log_info "Running namespace isolation tests (privileged)..."
        ((TOTAL_TESTS++))
        if $VERBOSE; then
            if sudo -E cargo test --test namespace_isolation_tests -- --ignored; then
                log_success "Privileged namespace tests passed"
            else
                log_failure "Privileged namespace tests failed"
            fi
        else
            if sudo -E cargo test --test namespace_isolation_tests --quiet -- --ignored; then
                log_success "Privileged namespace tests passed"
            else
                log_failure "Privileged namespace tests failed"
            fi
        fi
    else
        log_skip "Security tests (require sudo - use -s flag to enable)"
        ((TOTAL_TESTS += 2))
        ((SKIPPED_TESTS += 2))
    fi
}

run_system_tests() {
    log_header "SYSTEM TESTS"
    
    # Quick validation test (basic functionality)
    log_info "Running quick validation tests..."
    ((TOTAL_TESTS++))
    if [ -f "$SCRIPT_DIR/quick_test.sh" ]; then
        if [ "$RUN_PRIVILEGED" = true ] && [ "$SUDO_AVAILABLE" = true ]; then
            if $VERBOSE; then
                if bash "$SCRIPT_DIR/quick_test.sh"; then
                    log_success "Quick validation tests passed"
                else
                    log_failure "Quick validation tests failed"
                fi
            else
                if bash "$SCRIPT_DIR/quick_test.sh" >/dev/null 2>&1; then
                    log_success "Quick validation tests passed"
                else
                    log_failure "Quick validation tests failed"
                fi
            fi
        else
            log_skip "Quick validation tests (require sudo)"
            ((SKIPPED_TESTS++))
        fi
    else
        log_skip "Quick validation tests (script not found)"
        ((SKIPPED_TESTS++))
    fi
}

run_performance_tests() {
    log_header "PERFORMANCE TESTS"
    
    if [ "$RUN_PRIVILEGED" = true ] && [ "$SUDO_AVAILABLE" = true ]; then
        # Aggressive resource test
        log_info "Running aggressive resource tests..."
        ((TOTAL_TESTS++))
        if [ -f "$SCRIPT_DIR/tests/aggressive_resource_test.sh" ]; then
            # Create necessary test files first
            create_test_files
            
            if $VERBOSE; then
                if sudo bash "$SCRIPT_DIR/tests/aggressive_resource_test.sh"; then
                    log_success "Aggressive resource tests passed"
                else
                    log_failure "Aggressive resource tests failed"
                fi
            else
                if sudo bash "$SCRIPT_DIR/tests/aggressive_resource_test.sh" >/dev/null 2>&1; then
                    log_success "Aggressive resource tests passed"
                else
                    log_failure "Aggressive resource tests failed"
                fi
            fi
        else
            log_skip "Aggressive resource tests (script not found)"
            ((SKIPPED_TESTS++))
        fi
    else
        log_skip "Performance tests (require sudo - use -s flag to enable)"
        ((TOTAL_TESTS++))
        ((SKIPPED_TESTS++))
    fi
}

create_test_files() {
    log_info "Creating test files for performance tests..."
    
    # Create CPU intensive test
    cat > /tmp/cpu_hog.c << 'EOF'
#include <stdio.h>
#include <time.h>

int main() {
    time_t start = time(NULL);
    volatile long long i = 0;
    
    // Run for approximately 10 seconds of CPU time
    while (time(NULL) - start < 10) {
        for (int j = 0; j < 1000000; j++) {
            i++;
        }
    }
    
    printf("CPU test completed: %lld iterations\n", i);
    return 0;
}
EOF
    
    # Compile CPU test
    if gcc -o /tmp/cpu_hog /tmp/cpu_hog.c 2>/dev/null; then
        chmod +x /tmp/cpu_hog
    else
        log_warning "Failed to compile CPU test - using alternative"
        # Create shell-based CPU hog
        cat > /tmp/cpu_hog << 'EOF'
#!/bin/bash
end_time=$(($(date +%s) + 10))
i=0
while [ $(date +%s) -lt $end_time ]; do
    i=$((i + 1))
done
echo "CPU test completed: $i iterations"
EOF
        chmod +x /tmp/cpu_hog
    fi
    
    # Create memory test
    cat > /tmp/memory_hog.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main() {
    size_t size = 100 * 1024 * 1024; // 100MB
    char *buffer = malloc(size);
    
    if (buffer == NULL) {
        printf("Failed to allocate memory\n");
        return 1;
    }
    
    // Fill memory to ensure it's actually used
    memset(buffer, 'A', size);
    
    printf("Allocated and filled %zu bytes\n", size);
    
    // Hold memory for a while
    sleep(5);
    
    free(buffer);
    return 0;
}
EOF
    
    # Compile memory test
    if gcc -o /tmp/memory_hog /tmp/memory_hog.c 2>/dev/null; then
        chmod +x /tmp/memory_hog
    else
        log_warning "Failed to compile memory test - using alternative"
        # Create shell-based memory hog
        cat > /tmp/memory_hog << 'EOF'
#!/bin/bash
# Allocate memory using dd
dd if=/dev/zero of=/tmp/memory_buffer bs=1M count=100 2>/dev/null
sleep 5
rm -f /tmp/memory_buffer
echo "Memory test completed"
EOF
        chmod +x /tmp/memory_hog
    fi
    
    log_success "Test files created"
}

cleanup_test_files() {
    log_info "Cleaning up test files..."
    rm -f /tmp/cpu_hog /tmp/cpu_hog.c /tmp/memory_hog /tmp/memory_hog.c /tmp/memory_buffer
}

show_summary() {
    log_header "TEST SUMMARY"
    
    echo -e "${BOLD}Total Tests:${NC} $TOTAL_TESTS"
    echo -e "${GREEN}Passed:${NC} $PASSED_TESTS"
    echo -e "${RED}Failed:${NC} $FAILED_TESTS"
    echo -e "${YELLOW}Skipped:${NC} $SKIPPED_TESTS"
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "\n${BOLD}${GREEN}🎉 ALL TESTS PASSED!${NC}"
        if [ $SKIPPED_TESTS -gt 0 ]; then
            echo -e "${YELLOW}Note: $SKIPPED_TESTS tests were skipped (use -s for sudo tests)${NC}"
        fi
    else
        echo -e "\n${BOLD}${RED}❌ $FAILED_TESTS TESTS FAILED${NC}"
        echo -e "${YELLOW}Check the output above for details${NC}"
    fi
    
    # Calculate success rate
    local attempted=$((TOTAL_TESTS - SKIPPED_TESTS))
    if [ $attempted -gt 0 ]; then
        local success_rate=$((PASSED_TESTS * 100 / attempted))
        echo -e "${BOLD}Success Rate:${NC} $success_rate% ($PASSED_TESTS/$attempted)"
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -d|--debug)
            BUILD_TYPE="debug"
            shift
            ;;
        -s|--sudo)
            RUN_PRIVILEGED=true
            shift
            ;;
        -u|--unit-only)
            RUN_UNIT_ONLY=true
            shift
            ;;
        -i|--integration-only)
            RUN_INTEGRATION_ONLY=true
            shift
            ;;
        --privileged-only)
            RUN_PRIVILEGED=true
            RUN_UNIT_ONLY=false
            RUN_INTEGRATION_ONLY=false
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
    esac
done

# Main execution
main() {
    log_header "MINI-ISOLATE COMPREHENSIVE TEST RUNNER"
    
    # Check sudo availability if privileged tests requested
    if [ "$RUN_PRIVILEGED" = true ]; then
        check_sudo
    fi
    
    # Check dependencies
    check_dependencies || exit 1
    
    # Run tests based on options
    if [ "$RUN_UNIT_ONLY" = true ]; then
        run_unit_tests
    elif [ "$RUN_INTEGRATION_ONLY" = true ]; then
        run_integration_tests
    else
        # Run all test categories
        run_unit_tests
        run_integration_tests
        
        if [ "$RUN_PRIVILEGED" = true ]; then
            run_security_tests
            run_system_tests
            run_performance_tests
        else
            log_info "Skipping privileged tests (use -s flag to enable)"
            # Still count them for summary
            ((TOTAL_TESTS += 5))
            ((SKIPPED_TESTS += 5))
        fi
    fi
    
    # Cleanup
    if [ "$RUN_PRIVILEGED" = true ]; then
        cleanup_test_files
    fi
    
    # Show summary
    show_summary
    
    # Exit with appropriate code
    if [ $FAILED_TESTS -eq 0 ]; then
        exit 0
    else
        exit 1
    fi
}

# Run main function
main "$@"