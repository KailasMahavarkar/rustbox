#!/usr/bin/env python3
import os

try:
    # Try to write a large file
    filename = "large_file.txt"
    size_bytes = 64 * 1024 * 1024  # 64MB
    
    with open(filename, "wb") as f:
        chunk = b"x" * (1024 * 1024)  # 1MB chunk
        for i in range(64):
            f.write(chunk)
            if i % 10 == 0:
                print(f"Written {i+1}MB")
    
    print(f"Successfully created {size_bytes // (1024*1024)}MB file")
    print(f"File size: {os.path.getsize(filename)} bytes")
    
except Exception as e:
    print(f"File creation failed: {e}")
    exit(1)