#!/usr/bin/env python3
"""
Fork/process test program to verify process isolation and limits
"""
import os
import sys
import time
import multiprocessing

def child_worker(worker_id):
    """Worker function for child processes"""
    print(f"Child {worker_id}: PID {os.getpid()}, Parent PID {os.getppid()}")
    time.sleep(2)
    print(f"Child {worker_id}: Completed work")
    return worker_id

def test_fork_operations():
    print("=== Fork/Process Test ===")
    print(f"Main process PID: {os.getpid()}")
    
    # Test basic fork
    try:
        pid = os.fork()
        if pid == 0:
            # Child process
            print(f"✓ Fork successful - Child PID: {os.getpid()}")
            print(f"Child parent PID: {os.getppid()}")
            time.sleep(1)
            sys.exit(0)
        else:
            # Parent process
            print(f"✓ Fork successful - Parent created child PID: {pid}")
            os.waitpid(pid, 0)
            print("✓ Child process completed")
    except OSError as e:
        print(f"✓ Fork blocked: {e}")
    
    # Test multiple forks to check process limits
    print("\nTesting multiple process creation...")
    children = []
    max_processes = 20
    
    for i in range(max_processes):
        try:
            pid = os.fork()
            if pid == 0:
                # Child process
                print(f"Child {i}: PID {os.getpid()}")
                time.sleep(5)  # Keep child alive for a bit
                sys.exit(i)
            else:
                # Parent process
                children.append(pid)
                print(f"✓ Created child {i}: PID {pid}")
        except OSError as e:
            print(f"✓ Process creation blocked at {i} processes: {e}")
            break
    
    # Wait for all children
    for pid in children:
        try:
            os.waitpid(pid, 0)
        except:
            pass
    
    print(f"✓ Created {len(children)} child processes total")

def test_multiprocessing():
    print("\n=== Multiprocessing Test ===")
    
    try:
        # Test multiprocessing module
        pool_size = 4
        print(f"Testing multiprocessing with {pool_size} workers...")
        
        with multiprocessing.Pool(pool_size) as pool:
            tasks = list(range(pool_size))
            results = pool.map(child_worker, tasks)
            print(f"✓ Multiprocessing completed: {results}")
    
    except Exception as e:
        print(f"✓ Multiprocessing blocked: {e}")

if __name__ == "__main__":
    test_fork_operations()
    test_multiprocessing()