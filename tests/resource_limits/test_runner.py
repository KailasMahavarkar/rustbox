#!/usr/bin/env python3
"""
Test setup utility for resource limit testing
Provides helper functions to run tests with the new resource limit flags
"""

import subprocess
import sys
import os

def run_mini_isolate(args, timeout=30):
    """Run mini-isolate command with given arguments"""
    try:
        cmd = ["mini-isolate"] + args
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "Command timed out"
    except Exception as e:
        return -1, "", str(e)

def test_cpu_override():
    """Test CPU limit override functionality"""
    print("Testing CPU limit override...")
    
    # Initialize test isolate with 5 second limit
    code, stdout, stderr = run_mini_isolate([
        "init", "--box-id", "cpu_test", "--mem", "128", "--time", "5"
    ])
    
    if code != 0:
        print(f"Failed to initialize isolate: {stderr}")
        return False
    
    # Run CPU test with 10 second override (should succeed)
    test_file = os.path.join(os.path.dirname(__file__), "cpu_test.py")
    code, stdout, stderr = run_mini_isolate([
        "execute", "--box-id", "cpu_test", "--source", test_file,
        "--max-cpu", "10", "8"  # Run for 8 seconds with 10 second limit
    ])
    
    success = code == 0
    print(f"CPU override test: {'PASS' if success else 'FAIL'}")
    
    # Cleanup
    run_mini_isolate(["cleanup", "--box-id", "cpu_test"])
    return success

def test_memory_override():
    """Test memory limit override functionality"""
    print("Testing memory limit override...")
    
    # Initialize test isolate with 64MB limit
    code, stdout, stderr = run_mini_isolate([
        "init", "--box-id", "mem_test", "--mem", "64", "--time", "30"
    ])
    
    if code != 0:
        print(f"Failed to initialize isolate: {stderr}")
        return False
    
    # Run memory test with 256MB override (should succeed)
    test_file = os.path.join(os.path.dirname(__file__), "memory_test.py")
    code, stdout, stderr = run_mini_isolate([
        "run", "--box-id", "mem_test", "--max-memory", "256",
        "python3", test_file, "100"  # Try to allocate 100MB
    ])
    
    success = code == 0
    print(f"Memory override test: {'PASS' if success else 'FAIL'}")
    
    # Cleanup
    run_mini_isolate(["cleanup", "--box-id", "mem_test"])
    return success

def test_time_override():
    """Test time limit override functionality"""
    print("Testing time limit override...")
    
    # Initialize test isolate with 5 second wall time
    code, stdout, stderr = run_mini_isolate([
        "init", "--box-id", "time_test", "--mem", "128", "--time", "10", "--wall-time", "5"
    ])
    
    if code != 0:
        print(f"Failed to initialize isolate: {stderr}")
        return False
    
    # Run time test with 15 second override (should succeed)
    test_file = os.path.join(os.path.dirname(__file__), "time_test.py")
    code, stdout, stderr = run_mini_isolate([
        "execute", "--box-id", "time_test", "--source", test_file,
        "--max-time", "15", "8"  # Sleep for 8 seconds with 15 second limit
    ])
    
    success = code == 0
    print(f"Time override test: {'PASS' if success else 'FAIL'}")
    
    # Cleanup
    run_mini_isolate(["cleanup", "--box-id", "time_test"])
    return success

def show_example_usage():
    """Show example usage of the new resource limit flags"""
    examples = [
        "# Initialize test isolate",
        "mini-isolate init --box-id test --mem 128 --time 5",
        "",
        "# Test CPU limit override (allow 10s instead of 5s)",
        "mini-isolate execute --box-id test --source tests/resource_limits/cpu_test.py --max-cpu 10",
        "",
        "# Test memory limit override (allow 256MB instead of 128MB)",
        "mini-isolate run --box-id test --max-memory 256 python3 -- tests/resource_limits/memory_test.py 200",
        "",
        "# Test time limit override (allow 15s wall time)",
        "mini-isolate execute --box-id test --source tests/resource_limits/time_test.py --max-time 15",
        "",
        "# Test all limits together",
        "mini-isolate run --box-id test --max-cpu 30 --max-memory 512 --max-time 60 python3 -- tests/resource_limits/cpu_test.py 8",
        "",
        "# Clean up",
        "mini-isolate cleanup --box-id test"
    ]
    
    print("Example Usage:")
    print("=" * 50)
    for line in examples:
        print(line)

if __name__ == "__main__":
    print("Mini-Isolate Resource Limit Test Utility")
    print("=" * 45)
    
    if len(sys.argv) > 1 and sys.argv[1] == "examples":
        show_example_usage()
        sys.exit(0)
    
    print("Running automated tests...")
    
    tests = [
        test_cpu_override,
        test_memory_override,
        test_time_override
    ]
    
    passed = 0
    total = len(tests)
    
    for test in tests:
        if test():
            passed += 1
    
    print(f"\nTest Results: {passed}/{total} tests passed")
    
    if passed < total:
        print("Some tests failed. Ensure mini-isolate is compiled and accessible.")
        sys.exit(1)
    else:
        print("All tests passed!")
        sys.exit(0)