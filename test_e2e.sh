#!/bin/bash

# End-to-end testing script

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

# Test complete workflow
test_workflow() {
    log_info "Testing complete submission workflow..."
    
    # 1. Get system info
    log_info "Step 1: Getting system information..."
    local system_info=$(curl -s "$API_BASE_URL/system/info")
    local language_count=$(echo "$system_info" | jq '.languages | length')
    log_success "Found $language_count supported languages"
    
    # 2. Create a submission
    log_info "Step 2: Creating a code submission..."
    local submission_response=$(curl -s -X POST "$API_BASE_URL/submissions" \
        -H "Content-Type: application/json" \
        -d '{
            "source_code": "print(\"E2E Test Success!\")",
            "language_id": 1,
            "stdin": "",
            "time_limit": 5,
            "memory_limit": 128
        }')
    
    local submission_id=$(echo "$submission_response" | jq -r '.id')
    log_success "Created submission $submission_id"
    
    # 3. Check submission status
    log_info "Step 3: Monitoring submission status..."
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        local status_response=$(curl -s "$API_BASE_URL/submissions/$submission_id")
        local status=$(echo "$status_response" | jq -r '.status.name')
        
        log_info "Attempt $attempt: Status = $status"
        
        if [ "$status" = "Accepted" ] || [ "$status" = "Wrong Answer" ] || [ "$status" = "Time Limit Exceeded" ] || [ "$status" = "Runtime Error (Other)" ] || [ "$status" = "Compilation Error" ]; then
            break
        fi
        
        if [ $attempt -eq $max_attempts ]; then
            log_error "Submission processing timed out"
            return 1
        fi
        
        sleep 2
        attempt=$((attempt + 1))
    done
    
    # 4. Verify results
    log_info "Step 4: Verifying submission results..."
    local stdout=$(echo "$status_response" | jq -r '.stdout // ""')
    local stderr=$(echo "$status_response" | jq -r '.stderr // ""')
    local wall_time=$(echo "$status_response" | jq -r '.wall_time // 0')
    local memory_peak=$(echo "$status_response" | jq -r '.memory_peak // 0')
    
    log_info "Output: '$stdout'"
    log_info "Wall time: ${wall_time}ms"
    log_info "Memory peak: ${memory_peak}KB"
    
    if [ "$status" = "Accepted" ] && [ "$stdout" = "E2E Test Success!" ]; then
        log_success "E2E test completed successfully!"
        return 0
    else
        log_error "E2E test failed - Status: $status, Output: '$stdout'"
        if [ -n "$stderr" ]; then
            log_error "Error: $stderr"
        fi
        return 1
    fi
}

# Test system health
test_system_health() {
    log_info "Testing system health..."
    
    local health_response=$(curl -s "$API_BASE_URL/system/health")
    local status=$(echo "$health_response" | jq -r '.status')
    local database_connected=$(echo "$health_response" | jq -r '.database_connected')
    local redis_connected=$(echo "$health_response" | jq -r '.redis_connected')
    local rustbox_available=$(echo "$health_response" | jq -r '.rustbox_available')
    
    log_info "System status: $status"
    log_info "Database connected: $database_connected"
    log_info "Redis connected: $redis_connected"
    log_info "Rustbox available: $rustbox_available"
    
    if [ "$status" = "healthy" ] && [ "$database_connected" = "true" ] && [ "$redis_connected" = "true" ] && [ "$rustbox_available" = "true" ]; then
        log_success "System health check passed"
        return 0
    else
        log_error "System health check failed"
        return 1
    fi
}

# Test system statistics
test_system_stats() {
    log_info "Testing system statistics..."
    
    local stats_response=$(curl -s "$API_BASE_URL/system/stats")
    local total_submissions=$(echo "$stats_response" | jq -r '.total_submissions')
    local active_submissions=$(echo "$stats_response" | jq -r '.active_submissions')
    local queue_size=$(echo "$stats_response" | jq -r '.queue_size')
    local worker_count=$(echo "$stats_response" | jq -r '.worker_count')
    
    log_info "Total submissions: $total_submissions"
    log_info "Active submissions: $active_submissions"
    log_info "Queue size: $queue_size"
    log_info "Worker count: $worker_count"
    
    log_success "System statistics retrieved successfully"
    return 0
}

# Test batch operations
test_batch_operations() {
    log_info "Testing batch operations..."
    
    local batch_response=$(curl -s -X POST "$API_BASE_URL/submissions/batch" \
        -H "Content-Type: application/json" \
        -d '{
            "submissions": [
                {
                    "source_code": "print(\"Batch Test 1\")",
                    "language_id": 1,
                    "stdin": ""
                },
                {
                    "source_code": "print(\"Batch Test 2\")",
                    "language_id": 1,
                    "stdin": ""
                },
                {
                    "source_code": "print(\"Batch Test 3\")",
                    "language_id": 1,
                    "stdin": ""
                }
            ]
        }')
    
    local batch_size=$(echo "$batch_response" | jq '. | length')
    
    if [ "$batch_size" = "3" ]; then
        log_success "Batch operation test passed - Created $batch_size submissions"
        return 0
    else
        log_error "Batch operation test failed - Expected 3, got $batch_size"
        return 1
    fi
}

# Main test function
main() {
    log_info "Starting end-to-end tests..."
    
    local failed_tests=0
    
    # Run all tests
    test_system_health || failed_tests=$((failed_tests + 1))
    test_system_stats || failed_tests=$((failed_tests + 1))
    test_workflow || failed_tests=$((failed_tests + 1))
    test_batch_operations || failed_tests=$((failed_tests + 1))
    
    # Summary
    echo ""
    if [ $failed_tests -eq 0 ]; then
        log_success "All end-to-end tests passed!"
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