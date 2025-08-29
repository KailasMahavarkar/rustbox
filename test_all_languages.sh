#!/bin/bash

# Comprehensive language testing script

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

# Test code execution for a language
test_language() {
    local language_id="$1"
    local language_name="$2"
    local source_code="$3"
    local expected_output="$4"
    local stdin="$5"
    
    log_info "Testing $language_name (ID: $language_id)..."
    
    # Submit code
    local submission_response=$(curl -s -X POST "$API_BASE_URL/submissions" \
        -H "Content-Type: application/json" \
        -d "{
            \"source_code\": \"$source_code\",
            \"language_id\": $language_id,
            \"stdin\": \"$stdin\"
        }")
    
    # Extract submission ID
    local submission_id=$(echo "$submission_response" | jq -r '.id')
    
    if [ "$submission_id" = "null" ] || [ -z "$submission_id" ]; then
        log_error "Failed to create submission for $language_name"
        echo "Response: $submission_response"
        return 1
    fi
    
    log_info "Created submission $submission_id for $language_name"
    
    # Wait for processing (poll status)
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        local status_response=$(curl -s "$API_BASE_URL/submissions/$submission_id")
        local status=$(echo "$status_response" | jq -r '.status.name')
        
        if [ "$status" = "Accepted" ] || [ "$status" = "Wrong Answer" ] || [ "$status" = "Time Limit Exceeded" ] || [ "$status" = "Runtime Error (Other)" ] || [ "$status" = "Compilation Error" ]; then
            break
        fi
        
        if [ $attempt -eq $max_attempts ]; then
            log_error "Submission $submission_id for $language_name timed out"
            return 1
        fi
        
        sleep 2
        attempt=$((attempt + 1))
    done
    
    # Check results
    local stdout=$(echo "$status_response" | jq -r '.stdout // ""')
    local stderr=$(echo "$status_response" | jq -r '.stderr // ""')
    local final_status=$(echo "$status_response" | jq -r '.status.name')
    
    log_info "Final status for $language_name: $final_status"
    
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

# Main test function
main() {
    log_info "Starting comprehensive language tests..."
    
    local failed_tests=0
    
    # Test Python
    test_language 1 "Python" "print('Hello, World!')" "Hello, World!" "" || failed_tests=$((failed_tests + 1))
    test_language 1 "Python (Input)" "name = input(); print(f'Hello, {name}!')" "Hello, Test!" "Test" || failed_tests=$((failed_tests + 1))
    test_language 1 "Python (Math)" "print(2 + 2)" "4" "" || failed_tests=$((failed_tests + 1))
    
    # Test C++
    test_language 2 "C++" "#include <iostream>
int main() {
    std::cout << \"Hello, World!\" << std::endl;
    return 0;
}" "Hello, World!" "" || failed_tests=$((failed_tests + 1))
    
    test_language 2 "C++ (Math)" "#include <iostream>
int main() {
    int a = 5, b = 3;
    std::cout << a + b << std::endl;
    return 0;
}" "8" "" || failed_tests=$((failed_tests + 1))
    
    # Test Java
    test_language 3 "Java" "public class Main {
    public static void main(String[] args) {
        System.out.println(\"Hello, World!\");
    }
}" "Hello, World!" "" || failed_tests=$((failed_tests + 1))
    
    test_language 3 "Java (Math)" "public class Main {
    public static void main(String[] args) {
        int a = 10, b = 5;
        System.out.println(a - b);
    }
}" "5" "" || failed_tests=$((failed_tests + 1))
    
    # Test error cases
    log_info "Testing error cases..."
    
    # Test compilation error
    test_language 1 "Python (Syntax Error)" "print('Hello, World!'" "" "" || failed_tests=$((failed_tests + 1))
    
    # Test runtime error
    test_language 1 "Python (Runtime Error)" "print(1/0)" "" "" || failed_tests=$((failed_tests + 1))
    
    # Test time limit
    test_language 1 "Python (Infinite Loop)" "while True: pass" "" "" || failed_tests=$((failed_tests + 1))
    
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