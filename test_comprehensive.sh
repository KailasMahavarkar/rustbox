#!/bin/bash

# Comprehensive E2E Test Suite for codejudge-like System
# Tests all languages, error scenarios, concurrency, and system integration

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SERVER_PORT=8000
API_BASE="http://localhost:${SERVER_PORT}"
SERVER_PID=""
WORKER_PID=""
TEST_RESULTS=()
CLEANUP_PERFORMED=false

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to print colored output
print_status() { echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"; }
print_success() { echo -e "${GREEN}✅ $1${NC}"; }
print_error() { echo -e "${RED}❌ $1${NC}"; }
print_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
print_info() { echo -e "${CYAN}ℹ️  $1${NC}"; }
print_test_header() { echo -e "\n${PURPLE}🧪 $1${NC}"; }

# Cleanup function
cleanup() {
    if [ "$CLEANUP_PERFORMED" = true ]; then
        return
    fi
    
    print_status "🧹 Starting cleanup..."
    CLEANUP_PERFORMED=true
    
    # Kill processes
    [ ! -z "$SERVER_PID" ] && kill -TERM "$SERVER_PID" 2>/dev/null
    [ ! -z "$WORKER_PID" ] && kill -TERM "$WORKER_PID" 2>/dev/null
    pkill -f "python3 -m app.main" 2>/dev/null || true
    
    sleep 2
    
    # Force kill if still running
    [ ! -z "$SERVER_PID" ] && kill -9 "$SERVER_PID" 2>/dev/null
    [ ! -z "$WORKER_PID" ] && kill -9 "$WORKER_PID" 2>/dev/null
    
    print_success "Cleanup completed"
}

trap cleanup EXIT INT TERM

# Test result tracking
record_test() {
    local test_name="$1"
    local result="$2"
    local details="$3"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    if [ "$result" = "PASS" ]; then
        PASSED_TESTS=$((PASSED_TESTS + 1))
        print_success "$test_name"
    else
        FAILED_TESTS=$((FAILED_TESTS + 1))
        print_error "$test_name - $details"
    fi
    
    TEST_RESULTS+=("$test_name: $result${details:+ - $details}")
}

# Wait for service to be ready
wait_for_service() {
    local url=$1
    local service_name=$2
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if curl -s "$url" > /dev/null 2>&1; then
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
    done
    return 1
}

# Submit code and wait for result
submit_and_get_result() {
    local language_id=$1
    local source_code=$2
    local stdin=$3
    local timeout=${4:-10}
    local description=$5
    
    # Submit code
    local response=$(curl -s -X POST "$API_BASE/submissions/" \
        -H "Content-Type: application/json" \
        -d "{
            \"language_id\": $language_id,
            \"source_code\": \"$source_code\",
            \"stdin\": \"$stdin\"
        }" 2>/dev/null)
    
    if [ $? -ne 0 ]; then
        echo "ERROR:Network error"
        return 1
    fi
    
    # Extract submission ID
    local submission_id=$(echo "$response" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    if 'error' in data:
        print('ERROR:' + data.get('message', 'Unknown error'))
        sys.exit(1)
    print(data.get('id', ''))
except Exception as e:
    print('ERROR:JSON parse error')
    sys.exit(1)
" 2>/dev/null)
    
    if [[ "$submission_id" == ERROR:* ]]; then
        echo "$submission_id"
        return 1
    fi
    
    if [ -z "$submission_id" ]; then
        echo "ERROR:No submission ID"
        return 1
    fi
    
    # Poll for results
    local attempts=0
    while [ $attempts -lt $timeout ]; do
        sleep 1
        local result=$(curl -s "$API_BASE/submissions/$submission_id" 2>/dev/null)
        
        if [ $? -ne 0 ]; then
            echo "ERROR:Network error getting result"
            return 1
        fi
        
        # Check if processing is complete (not in queue or processing)
        local status_id=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('status_id', 0))
except:
    print(0)
" 2>/dev/null)
        
        if [ "$status_id" -ne 1 ] && [ "$status_id" -ne 2 ]; then
            echo "$result"
            return 0
        fi
        
        attempts=$((attempts + 1))
    done
    
    echo "ERROR:Timeout waiting for result"
    return 1
}

# Language test cases
test_languages() {
    print_test_header "Testing All Supported Languages"
    
    # Python (1)
    local result=$(submit_and_get_result 1 "print('Hello Python')" "" 10 "Python Hello World")
    if [[ "$result" == ERROR:* ]]; then
        record_test "Python Basic" "FAIL" "${result#ERROR:}"
    else
        local output=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stdout', '').strip())
except:
    print('')
" 2>/dev/null)
        
        if [[ "$output" == *"Hello Python"* ]]; then
            record_test "Python Basic" "PASS"
        else
            record_test "Python Basic" "FAIL" "Output: $output"
        fi
    fi
    
    # Python with input
    result=$(submit_and_get_result 1 "name = input()\\nprint(f'Hello {name}!')" "World" 10 "Python Input")
    if [[ "$result" != ERROR:* ]]; then
        local output=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stdout', '').strip())
except:
    print('')
" 2>/dev/null)
        
        if [[ "$output" == *"Hello World"* ]]; then
            record_test "Python Input" "PASS"
        else
            record_test "Python Input" "FAIL" "Output: $output"
        fi
    else
        record_test "Python Input" "FAIL" "${result#ERROR:}"
    fi
    
    # Python Math
    result=$(submit_and_get_result 1 "import math\\nprint(f'Pi = {math.pi:.2f}')" "" 10 "Python Math")
    if [[ "$result" != ERROR:* ]]; then
        local output=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stdout', '').strip())
except:
    print('')
" 2>/dev/null)
        
        if [[ "$output" == *"Pi = 3.14"* ]]; then
            record_test "Python Math" "PASS"
        else
            record_test "Python Math" "FAIL" "Output: $output"
        fi
    else
        record_test "Python Math" "FAIL" "${result#ERROR:}"
    fi
    
    # C++ (2)
    result=$(submit_and_get_result 2 "#include<iostream>\\nusing namespace std;\\nint main(){cout<<\\\"Hello C++\\\"<<endl;return 0;}" "" 10 "C++ Hello World")
    if [[ "$result" != ERROR:* ]]; then
        local output=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stdout', '').strip())
except:
    print('')
" 2>/dev/null)
        
        if [[ "$output" == *"Hello C++"* ]]; then
            record_test "C++ Basic" "PASS"
        else
            record_test "C++ Basic" "FAIL" "Output: $output"
        fi
    else
        record_test "C++ Basic" "FAIL" "${result#ERROR:}"
    fi
    
    # C (3)
    result=$(submit_and_get_result 3 "#include<stdio.h>\\nint main(){printf(\\\"Hello C\\\");return 0;}" "" 10 "C Hello World")
    if [[ "$result" != ERROR:* ]]; then
        local output=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stdout', '').strip())
except:
    print('')
" 2>/dev/null)
        
        if [[ "$output" == *"Hello C"* ]]; then
            record_test "C Basic" "PASS"
        else
            record_test "C Basic" "FAIL" "Output: $output"
        fi
    else
        record_test "C Basic" "FAIL" "${result#ERROR:}"
    fi
    
    # JavaScript (5)
    result=$(submit_and_get_result 5 "console.log('Hello JavaScript');" "" 10 "JavaScript Hello World")
    if [[ "$result" != ERROR:* ]]; then
        local output=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stdout', '').strip())
except:
    print('')
" 2>/dev/null)
        
        if [[ "$output" == *"Hello JavaScript"* ]]; then
            record_test "JavaScript Basic" "PASS"
        else
            record_test "JavaScript Basic" "FAIL" "Output: $output"
        fi
    else
        record_test "JavaScript Basic" "FAIL" "${result#ERROR:}"
    fi
    
    # Java (4)
    result=$(submit_and_get_result 4 "public class Main{public static void main(String[] args){System.out.println(\\\"Hello Java\\\");}};" "" 15 "Java Hello World")
    if [[ "$result" != ERROR:* ]]; then
        local output=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stdout', '').strip())
except:
    print('')
" 2>/dev/null)
        
        if [[ "$output" == *"Hello Java"* ]]; then
            record_test "Java Basic" "PASS"
        else
            record_test "Java Basic" "FAIL" "Output: $output"
        fi
    else
        record_test "Java Basic" "FAIL" "${result#ERROR:}"
    fi
    
    # Rust (6)
    result=$(submit_and_get_result 6 "fn main(){println!(\\\"Hello Rust\\\");}" "" 15 "Rust Hello World")
    if [[ "$result" != ERROR:* ]]; then
        local output=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stdout', '').strip())
except:
    print('')
" 2>/dev/null)
        
        if [[ "$output" == *"Hello Rust"* ]]; then
            record_test "Rust Basic" "PASS"
        else
            record_test "Rust Basic" "FAIL" "Output: $output"
        fi
    else
        record_test "Rust Basic" "FAIL" "${result#ERROR:}"
    fi
    
    # Go (7)
    result=$(submit_and_get_result 7 "package main\\nimport \\\"fmt\\\"\\nfunc main(){fmt.Println(\\\"Hello Go\\\");}" "" 15 "Go Hello World")
    if [[ "$result" != ERROR:* ]]; then
        local output=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stdout', '').strip())
except:
    print('')
" 2>/dev/null)
        
        if [[ "$output" == *"Hello Go"* ]]; then
            record_test "Go Basic" "PASS"
        else
            record_test "Go Basic" "FAIL" "Output: $output"
        fi
    else
        record_test "Go Basic" "FAIL" "${result#ERROR:}"
    fi
}

# Error scenario tests
test_error_scenarios() {
    print_test_header "Testing Error Scenarios"
    
    # Invalid language ID
    local response=$(curl -s -X POST "$API_BASE/submissions/" \
        -H "Content-Type: application/json" \
        -d '{"language_id": 999, "source_code": "test"}' 2>/dev/null)
    
    if echo "$response" | grep -q "error\|Error"; then
        record_test "Invalid Language ID" "PASS"
    else
        record_test "Invalid Language ID" "FAIL" "Should reject invalid language"
    fi
    
    # Empty source code
    response=$(curl -s -X POST "$API_BASE/submissions/" \
        -H "Content-Type: application/json" \
        -d '{"language_id": 1, "source_code": ""}' 2>/dev/null)
    
    if [ $? -eq 0 ]; then
        record_test "Empty Source Code" "PASS" "Request accepted"
    else
        record_test "Empty Source Code" "FAIL" "Network error"
    fi
    
    # Syntax error (Python)
    result=$(submit_and_get_result 1 "print('hello'\\nprint('world')" "" 10 "Python Syntax Error")
    if [[ "$result" != ERROR:* ]]; then
        local status_id=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('status_id', 0))
except:
    print(0)
" 2>/dev/null)
        
        # Should be some error status (not accepted)
        if [ "$status_id" -ne 3 ]; then
            record_test "Python Syntax Error" "PASS"
        else
            record_test "Python Syntax Error" "FAIL" "Should not be accepted"
        fi
    else
        record_test "Python Syntax Error" "FAIL" "${result#ERROR:}"
    fi
    
    # Runtime error (Python division by zero)
    result=$(submit_and_get_result 1 "print(1/0)" "" 10 "Python Runtime Error")
    if [[ "$result" != ERROR:* ]]; then
        local stderr=$(echo "$result" | python3 -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('stderr', ''))
except:
    print('')
" 2>/dev/null)
        
        if [[ "$stderr" == *"ZeroDivisionError"* ]] || [[ "$stderr" == *"division"* ]]; then
            record_test "Python Runtime Error" "PASS"
        else
            record_test "Python Runtime Error" "FAIL" "No division error in stderr"
        fi
    else
        record_test "Python Runtime Error" "FAIL" "${result#ERROR:}"
    fi
}

# Concurrent submission tests
test_concurrency() {
    print_test_header "Testing Concurrent Submissions"
    
    # Submit 5 jobs simultaneously
    local pids=()
    local results=()
    
    for i in {1..5}; do
        {
            local result=$(submit_and_get_result 1 "import time\\ntime.sleep(1)\\nprint('Job $i done')" "" 15 "Concurrent Job $i")
            echo "$result" > "/tmp/job_$i.result"
        } &
        pids+=($!)
    done
    
    # Wait for all jobs to complete
    local completed=0
    for pid in "${pids[@]}"; do
        if wait $pid; then
            completed=$((completed + 1))
        fi
    done
    
    if [ $completed -eq 5 ]; then
        record_test "Concurrent Submissions" "PASS" "All 5 jobs completed"
    else
        record_test "Concurrent Submissions" "FAIL" "Only $completed/5 jobs completed"
    fi
    
    # Clean up temp files
    rm -f /tmp/job_*.result 2>/dev/null
}

# API endpoint tests
test_api_endpoints() {
    print_test_header "Testing API Endpoints"
    
    # Root endpoint
    if curl -s "$API_BASE/" | grep -q "codejudge"; then
        record_test "Root Endpoint" "PASS"
    else
        record_test "Root Endpoint" "FAIL" "No codejudge in response"
    fi
    
    # Ping endpoint
    if curl -s "$API_BASE/ping" | grep -q "ok"; then
        record_test "Ping Endpoint" "PASS"
    else
        record_test "Ping Endpoint" "FAIL" "No ok in response"
    fi
    
    # Languages endpoint
    local langs_response=$(curl -s "$API_BASE/languages/" 2>/dev/null)
    if echo "$langs_response" | grep -q "Python\|C++\|Java"; then
        record_test "Languages Endpoint" "PASS"
    else
        record_test "Languages Endpoint" "FAIL" "No languages found"
    fi
    
    # System info endpoint
    if curl -s "$API_BASE/system/info" | grep -q "version\|uptime"; then
        record_test "System Info Endpoint" "PASS"
    else
        record_test "System Info Endpoint" "FAIL" "No system info"
    fi
    
    # Queue status endpoint
    if curl -s "$API_BASE/system/queue" > /dev/null 2>&1; then
        record_test "Queue Status Endpoint" "PASS"
    else
        record_test "Queue Status Endpoint" "FAIL" "Endpoint error"
    fi
}

# Generate test report
generate_report() {
    print_test_header "Test Results Summary"
    
    echo -e "${CYAN}📊 Test Statistics:${NC}"
    echo -e "  Total Tests: $TOTAL_TESTS"
    echo -e "  Passed: ${GREEN}$PASSED_TESTS${NC}"
    echo -e "  Failed: ${RED}$FAILED_TESTS${NC}"
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "\n${GREEN}🎉 ALL TESTS PASSED!${NC}"
    else
        echo -e "\n${RED}❌ Some tests failed${NC}"
        echo -e "\n${YELLOW}Failed Tests:${NC}"
        for result in "${TEST_RESULTS[@]}"; do
            if [[ "$result" == *": FAIL"* ]]; then
                echo -e "  ${RED}•${NC} $result"
            fi
        done
    fi
    
    local success_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))
    echo -e "\n${BLUE}Success Rate: ${success_rate}%${NC}"
}

# Main execution
main() {
    print_status "🚀 Starting Comprehensive End-to-End Test Suite"
    print_info "Testing codejudge-like System with Rustbox - All Languages & Scenarios"
    echo
    
    # Check environment
    if [ ! -d "venv" ] || [ ! -f "app/main.py" ]; then
        print_error "Run from rustbox-api directory with venv setup"
        exit 1
    fi
    
    source venv/bin/activate
    
    # Start services
    print_status "🖥️  Starting server on port $SERVER_PORT..."
    PORT=$SERVER_PORT python3 -m app.main > server.log 2>&1 &
    SERVER_PID=$!
    
    print_status "⚙️  Starting worker..."
    SERVICE_MODE=worker python3 -m app.main > worker.log 2>&1 &
    WORKER_PID=$!
    
    print_info "Server PID: $SERVER_PID, Worker PID: $WORKER_PID"
    
    # Wait for services
    print_status "⏳ Waiting for services to start..."
    if ! wait_for_service "$API_BASE/ping" "Server"; then
        print_error "Server failed to start"
        exit 1
    fi
    
    sleep 5  # Extra time for worker to register
    
    print_success "Services are ready!"
    echo
    
    # Run test suites
    test_api_endpoints
    test_languages
    test_error_scenarios
    test_concurrency
    
    # Generate final report
    echo
    generate_report
    
    print_status "📋 Check logs: server.log, worker.log"
    print_status "Test completed. Services will be cleaned up on exit."
}

# Check if running from correct directory
if [ ! -f "app/main.py" ]; then
    print_error "Please run this script from the rustbox-api root directory"
    exit 1
fi

# Run main function
main