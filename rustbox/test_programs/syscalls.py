#!/usr/bin/env python3
"""
System call test program to verify seccomp filtering
"""
import os
import sys
import subprocess
import time

def test_system_calls():
    print("=== System Call Test ===")
    
    # Test basic allowed system calls
    print(f"Process ID: {os.getpid()}")
    print(f"Parent PID: {os.getppid()}")
    print(f"User ID: {os.getuid()}")
    print(f"Group ID: {os.getgid()}")
    
    # Test file system calls
    try:
        os.getcwd()
        print("✓ getcwd() allowed")
    except Exception as e:
        print(f"✗ getcwd() blocked: {e}")
    
    # Test time-related calls
    try:
        current_time = time.time()
        print(f"✓ time() allowed: {current_time}")
    except Exception as e:
        print(f"✗ time() blocked: {e}")
    
    # Test potentially dangerous system calls
    dangerous_operations = [
        ("chmod", lambda: os.chmod(".", 0o755)),
        ("chown", lambda: os.chown(".", os.getuid(), os.getgid())),
        ("system", lambda: os.system("echo test")),
        ("exec", lambda: os.execv("/bin/true", ["/bin/true"])),
    ]
    
    for name, operation in dangerous_operations:
        try:
            operation()
            print(f"⚠ {name}() executed successfully")
        except PermissionError:
            print(f"✓ {name}() permission denied")
        except OSError as e:
            print(f"✓ {name}() blocked: {e}")
        except Exception as e:
            print(f"✓ {name}() failed: {e}")
    
    # Test subprocess creation
    try:
        result = subprocess.run(["echo", "Hello from subprocess"], 
                              capture_output=True, text=True, timeout=2)
        print(f"⚠ Subprocess allowed: {result.stdout.strip()}")
    except Exception as e:
        print(f"✓ Subprocess blocked: {e}")

if __name__ == "__main__":
    test_system_calls()