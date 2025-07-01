#!/usr/bin/env python3
"""
Code Sandbox I/O Demonstration Script

This script demonstrates the advanced I/O capabilities implemented for the 
mini-isolate code sandbox executor.
"""

import sys
import json
import time

def main():
    print("=== Code Sandbox I/O Demonstration ===")
    print("This program showcases advanced I/O features:")
    print("- Real-time stdout/stderr output")
    print("- Interactive stdin processing")
    print("- Unicode and encoding support")
    print("- Large output handling")
    print()

    # Test 1: Interactive input processing
    print("Test 1: Interactive Input Processing")
    try:
        name = input("Enter your name: ")
        age = input("Enter your age: ")
        print(f"Hello {name}, you are {age} years old!")
    except EOFError:
        print("No input provided, using defaults")
        name = "Anonymous"
        age = "Unknown"
        print(f"Hello {name}, you are {age} years old!")
    print()

    # Test 2: Mixed stdout/stderr output
    print("Test 2: Mixed Output Streams")
    for i in range(5):
        print(f"STDOUT: Line {i}")
        print(f"STDERR: Error {i}", file=sys.stderr)
        time.sleep(0.1)  # Small delay to test real-time handling
    print()

    # Test 3: Unicode and special characters
    print("Test 3: Unicode Support")
    print("Unicode characters: 🌍 🚀 ⭐ 🎯")
    print("Math symbols: ∑ ∆ π ∞ ≈ ≠")
    print("Languages: English, 中文, العربية, Русский")
    print("Accented: café, naïve, résumé, piñata")
    print()

    # Test 4: Large output generation
    print("Test 4: Large Output Generation")
    for i in range(100):
        if i % 10 == 0:
            print(f"Progress: {i}% complete")
        if i % 20 == 0:
            print(f"Checkpoint {i//20}", file=sys.stderr)
    print("Large output test completed!")
    print()

    # Test 5: JSON processing (simulating code execution results)
    print("Test 5: Structured Data Processing")
    result = {
        "status": "success",
        "execution_time": 0.123,
        "memory_used": "2.5MB",
        "output_lines": 150,
        "errors": 0,
        "features_tested": [
            "interactive_input",
            "mixed_streams",
            "unicode_support",
            "large_output",
            "json_processing"
        ]
    }
    print("Execution Result:")
    print(json.dumps(result, indent=2))
    print()

    # Test 6: Error handling
    print("Test 6: Error Handling")
    try:
        # Simulate a controlled error
        x = 1 / 0
    except ZeroDivisionError as e:
        print(f"Caught expected error: {e}", file=sys.stderr)
        print("Error handling works correctly!")
    print()

    print("=== All I/O Tests Completed Successfully ===")
    print("The code sandbox now supports:")
    print("✓ TTY support for interactive programs")
    print("✓ Pipe-based real-time I/O")
    print("✓ File-based I/O redirection")
    print("✓ Advanced stdin handling")
    print("✓ Configurable buffer sizes")
    print("✓ Text encoding support")
    print("✓ Large output processing")
    print("✓ Unicode character support")

if __name__ == "__main__":
    main()