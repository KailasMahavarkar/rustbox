#!/bin/bash

# Test script for language-specific configuration
echo "🧪 Testing Rustbox Language-Specific Configuration"
echo "=================================================="

# Build the project
echo "🔨 Building Rustbox..."
export PATH="$PATH:/home/rook/.cargo/bin"
cargo build --release

echo ""
echo "📋 Testing Python with default language config..."
./target/release/rustbox execute-code --box-id 10 --language python --code "
import sys
print('Python test - should use 128MB memory limit')
print(f'Python version: {sys.version.split()[0]}')
" --config language-limits.json

echo ""
echo "☕ Testing Java with language-specific config..."
./target/release/rustbox execute-code --box-id 11 --language java --code "
public class Test { 
    public static void main(String[] args) { 
        System.out.println(\"Java test - should use 512MB memory limit\");
        System.out.println(\"Java version: \" + System.getProperty(\"java.version\"));
    } 
}" --config language-limits.json

echo ""
echo "🔒 Testing Python with strict security profile..."
./target/release/rustbox execute-code --box-id 12 --language python --code "
print('Python with strict security - maximum isolation')
" --config language-limits.json --profile strict

echo ""
echo "🏁 Testing with contest profile..."
./target/release/rustbox execute-code --box-id 13 --language python --code "
print('Contest mode - strict time limits')
import time
# This should still work within contest limits
for i in range(1000):
    pass
print('Contest test completed')
" --config language-limits.json --profile contest

echo ""
echo "🔧 Testing command-line overrides..."
./target/release/rustbox execute-code --box-id 14 --language python --code "
print('Command-line overrides take precedence over config file')
" --config language-limits.json --mem 32 --time 2

echo ""
echo "✅ Language configuration testing complete!"