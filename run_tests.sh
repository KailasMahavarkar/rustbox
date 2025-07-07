#!/bin/bash

# rustbox Comprehensive Test Runner
# Uses the organized test directory structure with category-based testing

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TESTS_DIR="$SCRIPT_DIR/tests"
VERBOSE=false
RUN_PRIVILEGED=false
SPECIFIC_CATEGORY=""
SPECIFIC_TEST=""

# Helper functions
log_header() {
    echo -e "\n${BOLD}${CYAN}=== $1 ===${NC}"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_failure() {
    echo -e "${RED}[FAILURE]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

show_help() {
    cat << EOF
${BOLD}rustbox Comprehensive Test Runner${NC}

${BOLD}USAGE:${NC}
    $0 [OPTIONS] [CATEGORY] [TEST_NAME]

${BOLD}OPTIONS:${NC}
    -h, --help              Show this help message
    -v, --verbose           Enable verbose output
    -s, --sudo              Run tests requiring sudo privileges
    --build                 Build rustbox before running tests

${BOLD}TEST CATEGORIES:${NC}
    core                    Basic functionality validation
    resource                Resource limit enforcement tests
    stress                  Load and scalability testing  
    security                Isolation and security validation
    integration             End-to-end workflow testing
    performance             Performance benchmarks
    all                     Run all test categories

${BOLD}EXAMPLES:${NC}
    $0                      # Show help (interactive mode)
    $0 all                  # Run all tests (requires sudo)
    $0 core                 # Run only core functionality tests
    $0 security isolation   # Run specific isolation test in security category
    $0 -v stress parallel   # Run parallel stress test with verbose output
    $0 --build all          # Build rustbox then run all tests

${BOLD}NOTES:${NC}
    - Most tests require sudo privileges for namespace and cgroup operations
    - Use specific categories for focused testing during development
    - All tests use the organized structure in tests/ directory
    - Failed tests don't stop execution; all results are reported
EOF
}

check_dependencies() {
    log_info "Checking dependencies..."
    
    # Check if rustbox binary exists
    local binary_path="$SCRIPT_DIR/target/release/rustbox"
    if [ ! -f "$binary_path" ]; then
        log_warning "rustbox binary not found at $binary_path"
        
        # Check if cargo is available for building
        if command -v cargo &> /dev/null; then
            log_info "Building rustbox (release)..."
            if cargo build --release; then
                log_success "Build completed successfully"
            else
                log_failure "Build failed - cannot run tests"
                return 1
            fi
        else
            log_failure "rustbox binary not found and cargo not available"
            log_info "Please build rustbox first: cargo build --release"
            return 1
        fi
    else
        log_success "rustbox binary found"
    fi
    
    # Check if tests directory exists and contains test categories
    if [[ ! -d "$TESTS_DIR" ]]; then
        log_failure "Tests directory not found at $TESTS_DIR"
        log_info "Tests directory structure may not be properly set up"
        return 1
    fi
    
    # Check sudo access if privileged tests requested  
    if [ "$RUN_PRIVILEGED" = true ]; then
        if ! sudo -n true 2>/dev/null; then
            log_warning "Sudo access not available - will prompt when needed"
        else
            log_info "Sudo access confirmed"
        fi
    fi
    
    log_success "Dependencies check passed"
    return 0
}

run_category_tests() {
    local category="$1"
    local test_filter="$2"
    local category_dir="$TESTS_DIR/$category"
    local tests_run=0
    local tests_passed=0
    local tests_failed=0
    
    log_header "RUNNING $category TESTS"
    
    if [[ ! -d "$category_dir" ]]; then
        log_warning "Category directory not found: $category_dir"
        return 1
    fi
    
    log_info "Executing tests in $category_dir..."
    
    for test_file in "$category_dir"/*.sh; do
        if [[ ! -f "$test_file" ]]; then
            continue
        fi
        
        local test_basename=$(basename "$test_file" .sh)
        
        # Filter by test name if specified
        if [[ -n "$test_filter" && "$test_basename" != *"$test_filter"* ]]; then
            continue
        fi
        
        log_info "Running $(basename "$test_file")..."
        ((tests_run++))
        
        # Execute test with appropriate output handling
        if $VERBOSE; then
            if sudo bash "$test_file"; then
                log_success "$(basename "$test_file") passed"
                ((tests_passed++))
            else
                log_failure "$(basename "$test_file") failed"
                ((tests_failed++))
            fi
        else
            if sudo bash "$test_file" >/dev/null 2>&1; then
                log_success "$(basename "$test_file") passed"
                ((tests_passed++))
            else
                log_failure "$(basename "$test_file") failed"
                ((tests_failed++))
            fi
        fi
        echo ""
    done
    
    # Category summary
    echo -e "${BOLD}Category Summary for $category:${NC}"
    echo "  Tests run: $tests_run"
    echo "  Passed: $tests_passed"
    echo "  Failed: $tests_failed"
    if [[ $tests_run -gt 0 ]]; then
        echo "  Success rate: $(( (tests_passed * 100) / tests_run ))%"
    fi
    echo ""
    
    return $tests_failed
}

run_quick_validation() {
    log_header "QUICK VALIDATION"
    log_info "Running quick core functionality check..."
    
    # Run a quick core test to verify basic functionality
    local core_dir="$TESTS_DIR/core"
    if [[ -d "$core_dir" ]]; then
        for test_file in "$core_dir"/*.sh; do
            if [[ -f "$test_file" && $(basename "$test_file") == "basic_execution.sh" ]]; then
                if sudo bash "$test_file" >/dev/null 2>&1; then
                    log_success "Quick validation passed - rustbox is functional"
                    return 0
                else
                    log_failure "Quick validation failed - basic functionality not working"
                    log_warning "Check core tests with: sudo $0 core"
                    return 1
                fi
            fi
        done
    fi
    
    log_warning "No basic execution test found - skipping quick validation"
    return 0
}

run_all_tests() {
    log_header "COMPREHENSIVE TEST SUITE"
    log_info "Running all test categories in recommended order..."
    
    local categories=("core" "resource" "security" "stress" "integration" "performance")
    local failed_categories=()
    local total_categories=${#categories[@]}
    local passed_categories=0
    
    for category in "${categories[@]}"; do
        log_info "[$((passed_categories + ${#failed_categories[@]} + 1))/$total_categories] Testing $category category..."
        
        if run_category_tests "$category"; then
            ((passed_categories++))
        else
            failed_categories+=("$category")
        fi
        echo ""
    done
    
    # Summary
    log_header "COMPREHENSIVE TEST RESULTS"
    echo -e "${BOLD}Total Categories:${NC} $total_categories"
    echo -e "${GREEN}Passed:${NC} $passed_categories"
    echo -e "${RED}Failed:${NC} ${#failed_categories[@]}"
    
    if [ ${#failed_categories[@]} -eq 0 ]; then
        echo -e "\n${BOLD}${GREEN}🎉 ALL TEST CATEGORIES PASSED!${NC}"
        echo -e "${GREEN}rustbox is working correctly across all functionality${NC}"
        return 0
    else
        echo -e "\n${BOLD}${RED}❌ ${#failed_categories[@]} CATEGORIES FAILED:${NC}"
        for category in "${failed_categories[@]}"; do
            echo -e "${RED}  - $category${NC}"
        done
        echo -e "\n${YELLOW}Run specific categories with detailed output:${NC}"
        for category in "${failed_categories[@]}"; do
            echo -e "${YELLOW}  sudo $0 $category${NC}"
        done
        return 1
    fi
}

show_available_tests() {
    log_header "AVAILABLE TESTS"
    log_info "Scanning test directory structure..."
    echo ""
    
    cd "$TESTS_DIR"
    for category_dir in */; do
        if [ -d "$category_dir" ]; then
            local category=$(basename "$category_dir")
            echo -e "${BOLD}${BLUE}📁 $category/${NC}"
            
            for test_file in "$category_dir"*.sh; do
                if [ -f "$test_file" ]; then
                    local test_name=$(basename "$test_file" .sh)
                    echo -e "   ${GREEN}✓${NC} $test_name"
                fi
            done
            
            for util_file in "$category_dir"*.py; do
                if [ -f "$util_file" ]; then
                    local util_name=$(basename "$util_file")
                    echo -e "   ${YELLOW}🔧${NC} $util_name"
                fi
            done
            echo ""
        fi
    done
    
    echo -e "${BOLD}Usage Examples:${NC}"
    echo -e "  $0 core                    # Run all core tests"
    echo -e "  $0 security isolation      # Run specific isolation test"
    echo -e "  $0 -v stress parallel      # Run parallel stress test with verbose output"
    echo -e "  $0 all                     # Run complete test suite"
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
        -s|--sudo)
            RUN_PRIVILEGED=true
            shift
            ;;
        --build)
            log_info "Building rustbox..."
            if cargo build --release; then
                log_success "Build completed"
            else
                log_failure "Build failed"
                exit 1
            fi
            shift
            ;;
        core|resource|stress|security|integration|performance|all)
            SPECIFIC_CATEGORY="$1"
            shift
            ;;
        *)
            if [ -n "$SPECIFIC_CATEGORY" ]; then
                SPECIFIC_TEST="$1"
                shift
            else
                echo "Unknown option: $1"
                echo "Use -h or --help for usage information"
                exit 1
            fi
            ;;
    esac
done

# Main execution
main() {
    log_header "rustbox TEST RUNNER"
    
    # Check dependencies first
    check_dependencies || exit 1
    
    # If no category specified, show available tests and require user choice
    if [ -z "$SPECIFIC_CATEGORY" ]; then
        show_available_tests
        echo ""
        log_info "Please specify a test category or 'all' to run complete suite"
        exit 0
    fi
    
    # Execute based on category selection
    case "$SPECIFIC_CATEGORY" in
        all)
            # Always run with sudo for comprehensive testing
            RUN_PRIVILEGED=true
            run_all_tests
            exit $?
            ;;
        quick)
            run_quick_validation
            exit $?
            ;;
        *)
            # Always require sudo for individual category tests
            RUN_PRIVILEGED=true
            run_category_tests "$SPECIFIC_CATEGORY" "$SPECIFIC_TEST"
            exit $?
            ;;
    esac
}

# Run main function
main "$@"