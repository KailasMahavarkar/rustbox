#!/usr/bin/env python3
import sys

# Allocate memory in chunks to trigger limit
try:
    # Allocate 128MB of memory
    chunk_size = 1024 * 1024  # 1MB chunks
    chunks = []
    for i in range(128):
        chunk = b'x' * chunk_size
        chunks.append(chunk)
        if i % 10 == 0:
            print(f"Allocated {i+1}MB", file=sys.stderr, flush=True)
    
    print(f"Successfully allocated 128MB of memory")
except MemoryError:
    print("Memory allocation failed - limit exceeded")
    sys.exit(1)