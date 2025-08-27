#!/bin/bash
# Compile all C programs for testing rustbox

echo "=== Compiling C Test Programs ==="

# Array of C programs to compile
programs=("hello" "file_io" "network" "syscalls" "memory" "timeout" "fork")

for program in "${programs[@]}"; do
    echo "Compiling ${program}.c..."
    gcc -o "${program}" "${program}.c" -Wall -Wextra
    if [ $? -eq 0 ]; then
        echo "✓ Successfully compiled ${program}"
    else
        echo "✗ Failed to compile ${program}"
    fi
done

echo ""
echo "=== Making Python programs executable ==="
chmod +x *.py

echo ""
echo "=== Compiled programs ==="
ls -la hello file_io network syscalls memory timeout fork 2>/dev/null || echo "No compiled programs found"

echo ""
echo "=== Python programs ==="
ls -la *.py