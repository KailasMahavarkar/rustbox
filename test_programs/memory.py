#!/usr/bin/env python3
"""
Memory allocation test program to verify memory limits
"""
import sys
import time
import gc

def test_memory_allocation():
    print("=== Memory Allocation Test ===")
    
    # Get initial memory info
    try:
        import psutil
        process = psutil.Process()
        initial_memory = process.memory_info().rss / 1024 / 1024
        print(f"Initial memory usage: {initial_memory:.2f} MB")
    except ImportError:
        print("psutil not available, using basic memory test")
    
    # Test small allocation
    try:
        small_data = bytearray(1024 * 1024)  # 1 MB
        print("✓ Small allocation (1 MB) successful")
        del small_data
        gc.collect()
    except MemoryError:
        print("✗ Small allocation failed")
        return
    
    # Test medium allocation
    try:
        medium_data = bytearray(10 * 1024 * 1024)  # 10 MB
        print("✓ Medium allocation (10 MB) successful")
        del medium_data
        gc.collect()
    except MemoryError:
        print("✓ Medium allocation blocked by memory limit")
    
    # Test large allocation
    try:
        large_data = bytearray(100 * 1024 * 1024)  # 100 MB
        print("⚠ Large allocation (100 MB) successful")
        del large_data
        gc.collect()
    except MemoryError:
        print("✓ Large allocation blocked by memory limit")
    
    # Test progressive allocation to find limit
    allocations = []
    chunk_size = 5 * 1024 * 1024  # 5 MB chunks
    total_allocated = 0
    
    print(f"Testing progressive allocation in {chunk_size // 1024 // 1024} MB chunks...")
    
    for i in range(50):  # Try up to 250 MB
        try:
            chunk = bytearray(chunk_size)
            allocations.append(chunk)
            total_allocated += chunk_size
            print(f"✓ Allocated chunk {i+1}: {total_allocated // 1024 // 1024} MB total")
            time.sleep(0.1)  # Brief pause
        except MemoryError:
            print(f"✓ Memory limit reached at {total_allocated // 1024 // 1024} MB")
            break
    
    # Clean up
    del allocations
    gc.collect()
    print("✓ Memory cleanup completed")

if __name__ == "__main__":
    test_memory_allocation()