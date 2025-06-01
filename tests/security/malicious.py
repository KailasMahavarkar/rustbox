#!/usr/bin/env python3
import os
import sys

# Try to read sensitive files
files_to_try = ["/etc/passwd", "/etc/shadow", "/proc/version"]

for file_path in files_to_try:
    try:
        with open(file_path, 'r') as f:
            content = f.read()
            print(f"Successfully read {file_path}: {len(content)} bytes")
    except Exception as e:
        print(f"Cannot read {file_path}: {e}")

# Try to write to system directories
try:
    with open("/tmp/malicious.txt", "w") as f:
        f.write("Malicious content")
    print("Successfully wrote to /tmp")
except Exception as e:
    print(f"Cannot write to /tmp: {e}")

# Try to execute system commands
try:
    result = os.system("whoami")
    print(f"Command execution result: {result}")
except Exception as e:
    print(f"Cannot execute commands: {e}")