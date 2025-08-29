#!/bin/bash

# Basic API functionality test script

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

# Test function
test_endpoint() {
    local endpoint="$1"
    local expected_status="$2"
    local description="$3"
    
    log_info "Testing: $description"
    
    local response=$(curl -s -w "%{http_code}" -o /tmp/response.json "$API_BASE_URL$endpoint")
    local status_code="${response: -3}"
    
    if [ "$status_code" = "$expected_status" ]; then
        log_success "$description - Status: $status_code"
        return 0
    else
        log_error "$description - Expected: $expected_status, Got: $status_code"
        if [ -f /tmp/response.json ]; then
            echo "Response: $(cat /tmp/response.json)"
        fi
        return 1
    fi
}

# Test POST endpoint
test_post_endpoint() {
    local endpoint="$1"
    local data="$2"
    local expected_status="$3"
    local description="$4"
    
    log_info "Testing: $description"
    
    local response=$(curl -s -w "%{http_code}" -o /tmp/response.json \
        -X POST \
        -H "Content-Type: application/json" \
        -d "$data" \
        "$API_BASE_URL$endpoint")
    local status_code="${response: -3}"
    
    if [ "$status_code" = "$expected_status" ]; then
        log_success "$description - Status: $status_code"
        return 0
    else
        log_error "$description - Expected: $expected_status, Got: $status_code"
        if [ -f /tmp/response.json ]; then
            echo "Response: $(cat /tmp/response.json)"
        fi
        return 1
    fi
}

# Main test function
main() {
    log_info "Starting basic API tests..."
    
    local failed_tests=0
    
    # Test basic endpoints
    test_endpoint "/" "200" "Root endpoint" || failed_tests=$((failed_tests + 1))
    test_endpoint "/ping" "200" "Ping endpoint" || failed_tests=$((failed_tests + 1))
    test_endpoint "/system/health" "200" "Health check endpoint" || failed_tests=$((failed_tests + 1))
    test_endpoint "/system/info" "200" "System info endpoint" || failed_tests=$((failed_tests + 1))
    test_endpoint "/languages/" "200" "Languages endpoint" || failed_tests=$((failed_tests + 1))
    test_endpoint "/submissions/" "200" "Submissions list endpoint" || failed_tests=$((failed_tests + 1))
    
    # Test code submission
    test_post_endpoint "/submissions/" '{
        "source_code": "print(\"Hello, World!\")",
        "language_id": 1,
        "stdin": ""
    }' "201" "Python code submission" || failed_tests=$((failed_tests + 1))
    
    # Test invalid submission
    test_post_endpoint "/submissions/" '{
        "source_code": "print(\"Hello, World!\")",
        "language_id": 999,
        "stdin": ""
    }' "400" "Invalid language ID submission" || failed_tests=$((failed_tests + 1))
    
    # Test batch submission
    test_post_endpoint "/submissions/batch" '{
        "submissions": [
            {
                "source_code": "print(\"Test 1\")",
                "language_id": 1,
                "stdin": ""
            },
            {
                "source_code": "print(\"Test 2\")",
                "language_id": 1,
                "stdin": ""
            }
        ]
    }' "201" "Batch submission" || failed_tests=$((failed_tests + 1))
    
    # Summary
    echo ""
    if [ $failed_tests -eq 0 ]; then
        log_success "All basic tests passed!"
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

# Run main function
check_api
main