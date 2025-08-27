#!/usr/bin/env python3
"""
File I/O test program to verify filesystem isolation
"""
import os
import sys

def test_file_operations():
    print("=== File I/O Test ===")
    
    # Test writing to current directory
    try:
        with open("test_write.txt", "w") as f:
            f.write("Hello from sandbox file!\n")
        print("✓ Created test_write.txt")
        
        # Test reading back
        with open("test_write.txt", "r") as f:
            content = f.read()
        print(f"✓ Read back: {content.strip()}")
        
        # Clean up
        os.remove("test_write.txt")
        print("✓ Cleaned up test_write.txt")
        
    except Exception as e:
        print(f"✗ File operation failed: {e}")
    
    # Test trying to access sensitive directories
    sensitive_paths = ["/etc/passwd", "/root", "/proc/version", "/sys"]
    
    for path in sensitive_paths:
        try:
            if os.path.exists(path):
                if os.path.isfile(path):
                    with open(path, "r") as f:
                        content = f.read(100)  # Read first 100 chars
                    print(f"⚠ Accessed {path}: {content[:50]}...")
                else:
                    files = os.listdir(path)[:5]  # List first 5 files
                    print(f"⚠ Listed {path}: {files}")
            else:
                print(f"✓ Cannot access {path} (doesn't exist)")
        except PermissionError:
            print(f"✓ Permission denied for {path}")
        except Exception as e:
            print(f"✓ Cannot access {path}: {e}")

if __name__ == "__main__":
    test_file_operations()