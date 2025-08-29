#!/bin/bash

# Fixed language testing script with better error handling

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
API_BASE_URL="http://localhost:8000"

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Test a single language with timeout
test_language() {
    local language_id="$1"
    local language_name="$2"
    local source_code="$3"
    local expected_output="$4"
    local stdin="$5"
    local timeout_seconds="${6:-30}"
    
    log_info "Testing $language_name (ID: $language_id) with timeout ${timeout_seconds}s..."
    
    # Submit code
    local submission_response=$(curl -s -X POST "$API_BASE_URL/submissions" \
        -H "Content-Type: application/json" \
        -d "{
            \"source_code\": \"$source_code\",
            \"language_id\": $language_id,
            \"stdin\": \"$stdin\",
            \"time_limit\": 10,
            \"memory_limit\": 256
        }")
    
    # Extract submission ID
    local submission_id=$(echo "$submission_response" | jq -r '.id')
    
    if [ "$submission_id" = "null" ] || [ -z "$submission_id" ]; then
        log_error "Failed to create submission for $language_name"
        echo "Response: $submission_response"
        return 1
    fi
    
    log_info "Created submission $submission_id for $language_name"
    
    # Wait for processing with timeout
    local start_time=$(date +%s)
    local max_attempts=$((timeout_seconds * 2))  # Check every 0.5 seconds
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        local current_time=$(date +%s)
        local elapsed=$((current_time - start_time))
        
        if [ $elapsed -ge $timeout_seconds ]; then
            log_error "Submission $submission_id for $language_name timed out after ${timeout_seconds}s"
            return 1
        fi
        
        local status_response=$(curl -s "$API_BASE_URL/submissions/$submission_id")
        local status=$(echo "$status_response" | jq -r '.status.name')
        
        if [ "$status" = "Accepted" ] || [ "$status" = "Wrong Answer" ] || [ "$status" = "Time Limit Exceeded" ] || [ "$status" = "Runtime Error (Other)" ] || [ "$status" = "Compilation Error" ] || [ "$status" = "Internal Error" ]; then
            break
        fi
        
        sleep 0.5
        attempt=$((attempt + 1))
    done
    
    # Check results
    local stdout=$(echo "$status_response" | jq -r '.stdout // ""')
    local stderr=$(echo "$status_response" | jq -r '.stderr // ""')
    local final_status=$(echo "$status_response" | jq -r '.status.name')
    local wall_time=$(echo "$status_response" | jq -r '.wall_time // 0')
    local memory_peak=$(echo "$status_response" | jq -r '.memory_peak // 0')
    
    log_info "Final status for $language_name: $final_status (${wall_time}ms, ${memory_peak}KB)"
    
    if [ "$final_status" = "Accepted" ]; then
        if [ -n "$expected_output" ] && [ "$stdout" != "$expected_output" ]; then
            log_warning "$language_name output mismatch:"
            log_warning "Expected: '$expected_output'"
            log_warning "Got: '$stdout'"
            return 1
        else
            log_success "$language_name test passed"
            return 0
        fi
    else
        log_error "$language_name test failed with status: $final_status"
        if [ -n "$stderr" ]; then
            log_error "Error output: $stderr"
        fi
        return 1
    fi
}

# Test language compilation
test_compilation() {
    local language_id="$1"
    local language_name="$2"
    local source_code="$3"
    
    log_info "Testing $language_name compilation..."
    
    # Submit code with compilation error
    local submission_response=$(curl -s -X POST "$API_BASE_URL/submissions" \
        -H "Content-Type: application/json" \
        -d "{
            \"source_code\": \"$source_code\",
            \"language_id\": $language_id,
            \"stdin\": \"\",
            \"time_limit\": 10,
            \"memory_limit\": 256
        }")
    
    local submission_id=$(echo "$submission_response" | jq -r '.id')
    
    # Wait for processing
    local max_attempts=20
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        local status_response=$(curl -s "$API_BASE_URL/submissions/$submission_id")
        local status=$(echo "$status_response" | jq -r '.status.name')
        
        if [ "$status" = "Compilation Error" ] || [ "$status" = "Accepted" ] || [ "$status" = "Runtime Error (Other)" ]; then
            break
        fi
        
        sleep 1
        attempt=$((attempt + 1))
    done
    
    local final_status=$(echo "$status_response" | jq -r '.status.name')
    
    if [ "$final_status" = "Compilation Error" ]; then
        log_success "$language_name compilation error test passed"
        return 0
    else
        log_warning "$language_name compilation error test - Expected Compilation Error, got $final_status"
        return 1
    fi
}

# Main test function
main() {
    log_info "Starting fixed language tests..."
    
    local failed_tests=0
    
    # Test basic functionality for each language
    test_language 1 "Python" "print('Hello, World!')" "Hello, World!" "" 30 || failed_tests=$((failed_tests + 1))
    test_language 1 "Python (Input)" "name = input(); print(f'Hello, {name}!')" "Hello, Test!" "Test" 30 || failed_tests=$((failed_tests + 1))
    test_language 1 "Python (Math)" "print(2 + 2)" "4" "" 30 || failed_tests=$((failed_tests + 1))
    
    # Test C++ if available
    if curl -s "$API_BASE_URL/languages/" | jq -e '.[] | select(.id == 2)' >/dev/null; then
        test_language 2 "C++" "#include <iostream>
int main() {
    std::cout << \"Hello, World!\" << std::endl;
    return 0;
}" "Hello, World!" "" 60 || failed_tests=$((failed_tests + 1))
        
        test_compilation 2 "C++" "#include <iostream>
int main() {
    std::cout << \"Hello, World!\" << std::endl
    return 0;
}" || failed_tests=$((failed_tests + 1))
    else
        log_warning "C++ language not available, skipping C++ tests"
    fi
    
    # Test Java if available
    if curl -s "$API_BASE_URL/languages/" | jq -e '.[] | select(.id == 3)' >/dev/null; then
        test_language 3 "Java" "public class Main {
    public static void main(String[] args) {
        System.out.println(\"Hello, World!\");
    }
}" "Hello, World!" "" 60 || failed_tests=$((failed_tests + 1))
        
        test_compilation 3 "Java" "public class Main {
    public static void main(String[] args) {
        System.out.println(\"Hello, World!\")
    }
}" || failed_tests=$((failed_tests + 1))
    else
        log_warning "Java language not available, skipping Java tests"
    fi
    
    # Test error cases
    log_info "Testing error cases..."
    test_language 1 "Python (Runtime Error)" "print(1/0)" "" "" 30 || failed_tests=$((failed_tests + 1))
    test_language 1 "Python (Syntax Error)" "print('Hello, World!'" "" "" 30 || failed_tests=$((failed_tests + 1))
    
    # Summary
    echo ""
    if [ $failed_tests -eq 0 ]; then
        log_success "All language tests passed!"
        exit 0
    else
        log_error "$failed_tests test(s) failed"
        exit 1
    fi
}

# Check if API is running
check_api() {
    if ! curl -f -s "$API_BASE_URL/ping" >/dev/null 2>&1; then
        log_error "API is not running at $API_BASE_URL"
        log_error "Please start the API first using: ./deploy.sh start"
        exit 1
    fi
}

# Check if jq is available
check_jq() {
    if ! command -v jq >/dev/null 2>&1; then
        log_error "jq is required but not installed"
        log_error "Please install jq: sudo apt install jq"
        exit 1
    fi
}

# Run main function
check_api
check_jq
main