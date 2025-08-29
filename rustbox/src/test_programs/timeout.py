#!/usr/bin/env python3
"""
Timeout test program to verify time limits
"""
import time
import signal
import sys

def test_timeouts():
    print("=== Timeout Test ===")
    
    # Test short operation
    print("Testing 2-second operation...")
    start_time = time.time()
    time.sleep(2)
    elapsed = time.time() - start_time
    print(f"✓ 2-second operation completed in {elapsed:.2f}s")
    
    # Test medium operation
    print("Testing 5-second operation...")
    start_time = time.time()
    time.sleep(5)
    elapsed = time.time() - start_time
    print(f"✓ 5-second operation completed in {elapsed:.2f}s")
    
    # Test long operation that should be killed
    print("Testing 30-second operation (should be terminated)...")
    start_time = time.time()
    
    try:
        for i in range(30):
            print(f"Second {i+1}/30")
            time.sleep(1)
        elapsed = time.time() - start_time
        print(f"⚠ 30-second operation completed in {elapsed:.2f}s (not terminated)")
    except KeyboardInterrupt:
        elapsed = time.time() - start_time
        print(f"✓ Operation terminated after {elapsed:.2f}s")

def infinite_loop_test():
    print("=== Infinite Loop Test ===")
    print("Starting infinite loop (should be terminated by timeout)...")
    
    counter = 0
    start_time = time.time()
    
    try:
        while True:
            counter += 1
            if counter % 1000000 == 0:
                elapsed = time.time() - start_time
                print(f"Loop iteration {counter}, elapsed: {elapsed:.2f}s")
    except KeyboardInterrupt:
        elapsed = time.time() - start_time
        print(f"✓ Infinite loop terminated after {elapsed:.2f}s, {counter} iterations")

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "infinite":
        infinite_loop_test()
    else:
        test_timeouts()