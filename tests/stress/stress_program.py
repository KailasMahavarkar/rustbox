#!/usr/bin/env python3
"""
Simple Stress Test Program for Mini-Isolate
Configurable CPU + Memory workload with clear progress reporting
"""

import sys
import time
import gc

def stress_workload(memory_mb, cpu_iterations, work_duration):
    """
    Combined CPU and memory stress workload
    """
    print(f"Starting workload: {memory_mb}MB memory, {cpu_iterations} CPU iterations, {work_duration}s duration")
    
    # Memory allocation
    print("Allocating memory...")
    try:
        buffer = bytearray(memory_mb * 1024 * 1024)
        
        # Touch all pages to ensure real allocation
        page_size = 4096
        for i in range(0, len(buffer), page_size):
            buffer[i] = (i // page_size) % 256
        
        print(f"Successfully allocated {memory_mb}MB")
    except MemoryError as e:
        print(f"Memory allocation failed: {e}")
        return False
    
    # CPU work with memory access
    print("Starting CPU work...")
    start_time = time.time()
    result = 0
    
    iterations_per_second = cpu_iterations // max(1, work_duration)
    
    for second in range(work_duration):
        second_start = time.time()
        
        # CPU intensive work
        for i in range(iterations_per_second):
            result += i * i * i
            
            # Periodically touch memory to keep it active
            if i % 10000 == 0 and len(buffer) > 0:
                idx = (i * page_size) % len(buffer)
                buffer[idx] = (buffer[idx] + 1) % 256
        
        elapsed = time.time() - start_time
        print(f"Progress: {second + 1}/{work_duration}s (elapsed: {elapsed:.1f}s)")
        
        # Sleep to maintain timing if we finished early
        second_elapsed = time.time() - second_start
        if second_elapsed < 1.0:
            time.sleep(1.0 - second_elapsed)
    
    elapsed = time.time() - start_time
    
    # Final memory verification
    checksum = sum(buffer[i] for i in range(0, len(buffer), page_size * 100))
    
    print(f"Workload completed in {elapsed:.2f}s")
    print(f"CPU result: {result}")
    print(f"Memory checksum: {checksum}")
    
    return True

def main():
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <memory_mb> <cpu_iterations> <work_duration_seconds>")
        print("Example: python3 stress_program.py 200 50000000 5")
        return 1
    
    try:
        memory_mb = int(sys.argv[1])
        cpu_iterations = int(sys.argv[2])
        work_duration = int(sys.argv[3])
        
        if memory_mb <= 0 or cpu_iterations <= 0 or work_duration <= 0:
            print("All parameters must be positive")
            return 1
        
        success = stress_workload(memory_mb, cpu_iterations, work_duration)
        return 0 if success else 1
        
    except ValueError as e:
        print(f"Invalid parameters: {e}")
        return 1
    except KeyboardInterrupt:
        print("\nWorkload interrupted")
        return 0
    except Exception as e:
        print(f"Workload failed: {e}")
        return 1

if __name__ == "__main__":
    sys.exit(main())