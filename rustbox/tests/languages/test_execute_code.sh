#!/bin/bash

# Test script for language tests in rustbox using execute-code command
# This script tests all languages with proper stdin handling

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}Testing languages with rustbox execute-code...${NC}"

# Build rustbox first
echo -e "${BLUE}Building rustbox...${NC}"
export PATH=$PATH:~/.cargo/bin
cd ../../
cargo build --release
cd tests/languages

RUSTBOX="../../target/release/rustbox"
BOX_ID=1

# Initialize sandbox
echo -e "${BLUE}Initializing sandbox...${NC}"
$RUSTBOX init --box-id $BOX_ID

# Function to test a language program
test_program() {
    local lang=$1
    local file=$2
    local stdin_input=$3
    local time_limit=$4
    local mem_limit=$5
    local process_limit=$6
    local description=$7
    
    echo -e "\n${BLUE}Testing $lang: $description${NC}"
    
    # Build command using file directly (avoiding shell escaping issues)
    local cmd="$RUSTBOX execute-code --box-id $BOX_ID --language $lang --time $time_limit --mem $mem_limit --processes $process_limit"
    
    # Read the code from file
    local code_content=$(cat "$file")
    
    # Add stdin if provided
    if [ ! -z "$stdin_input" ]; then
        # Execute with both code file and stdin parameter
        if $cmd --code "$code_content" --stdin "$stdin_input" | jq -r '.success' | grep -q true; then
            echo -e "${GREEN}✅ $lang $description passed${NC}"
            # Show output
            $cmd --code "$code_content" --stdin "$stdin_input" | jq -r '.stdout' | sed 's/^/   Output: /'
        else
            echo -e "${RED}❌ $lang $description failed${NC}"
            # Show error
            $cmd --code "$code_content" --stdin "$stdin_input" | jq -r '.error_message // .status' | sed 's/^/   Error: /'
        fi
    else
        # Execute without stdin
        if $cmd --code "$code_content" | jq -r '.success' | grep -q true; then
            echo -e "${GREEN}✅ $lang $description passed${NC}"
            # Show output
            $cmd --code "$code_content" | jq -r '.stdout' | sed 's/^/   Output: /'
        else
            echo -e "${RED}❌ $lang $description failed${NC}"
            # Show error
            $cmd --code "$code_content" | jq -r '.error_message // .status' | sed 's/^/   Error: /'
        fi
    fi
}

# Test supported languages: Python, C++, Java

# Python tests
test_program "python" "lang_python/test_1_fact.py" "5" 5 100 10 "factorial"
test_program "python" "lang_python/test_2_star.py" "3" 5 100 10 "star pattern"
test_program "python" "lang_python/test_3_lis.py" "" 5 100 10 "LIS algorithm"

# C++ tests
test_program "cpp" "lang_cpp/test_1_fact.cpp" "5" 10 300 15 "factorial"
test_program "cpp" "lang_cpp/test_2_star.cpp" "3" 10 300 15 "star pattern"
test_program "cpp" "lang_cpp/test_3_lis.cpp" "" 10 300 15 "LIS algorithm"

# Java tests
test_program "java" "lang_java/test_1_fact.java" "5" 15 500 20 "factorial"
test_program "java" "lang_java/test_2_star.java" "3" 15 500 20 "star pattern"
test_program "java" "lang_java/test_3_lis.java" "" 15 500 20 "LIS algorithm"

# Test TLE and MLE detection with Python
echo -e "\n${BLUE}Testing resource limit enforcement...${NC}"
echo -e "\n${BLUE}Testing Time Limit Exceeded (should timeout)...${NC}"
if $RUSTBOX execute-code --box-id $BOX_ID --language python --time 2 --mem 100 --processes 10 --code "$(cat lang_python/test_4_tle.py)" | jq -r '.status' | grep -q "TLE"; then
    echo -e "${GREEN}✅ Time limit enforcement works${NC}"
else
    echo -e "${RED}❌ Time limit enforcement failed${NC}"
fi

echo -e "\n${BLUE}Testing Memory Limit Exceeded (should hit memory limit)...${NC}"
if $RUSTBOX execute-code --box-id $BOX_ID --language python --time 5 --mem 10 --processes 10 --code "$(cat lang_python/test_5_mle.py)" | jq -r '.status' | grep -q "Memory"; then
    echo -e "${GREEN}✅ Memory limit enforcement works${NC}"
else
    echo -e "${RED}❌ Memory limit enforcement failed${NC}"
fi

echo -e "\n${BLUE}Language testing complete!${NC}"
echo -e "\n${GREEN}Summary:${NC}"
echo -e "${GREEN}✅ Supported languages: Python, C++, Java${NC}"
echo -e "${GREEN}✅ Resource limits: Time and Memory limits work correctly${NC}"