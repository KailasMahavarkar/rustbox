#!/usr/bin/env python3
"""
Malicious code test cases for seccomp filtering validation
These scripts attempt various security violations that should be blocked
"""

# Network access attempt
NETWORK_ATTACK = '''
import socket
import sys

try:
    # Attempt to create socket
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    print("SECURITY_BREACH: Socket created")
    
    # Attempt to connect to external host
    s.connect(("google.com", 80))
    print("SECURITY_BREACH: Network connection established")
    
    # Send HTTP request
    s.send(b"GET / HTTP/1.1\\r\\nHost: google.com\\r\\n\\r\\n")
    response = s.recv(1024)
    print(f"SECURITY_BREACH: Received data: {response[:50]}")
    
    s.close()
except Exception as e:
    print(f"Expected security error: {e}")
    sys.exit(1)
'''

# Process creation attempt
PROCESS_ATTACK = '''
import os
import subprocess
import sys

try:
    # Attempt fork
    pid = os.fork()
    if pid == 0:
        print("SECURITY_BREACH: Child process created via fork")
        os._exit(0)
    else:
        print("SECURITY_BREACH: Fork succeeded, child PID:", pid)
        os.waitpid(pid, 0)
except Exception as e:
    print(f"Expected fork error: {e}")

try:
    # Attempt subprocess
    result = subprocess.run(['echo', 'SECURITY_BREACH: Subprocess executed'], 
                          capture_output=True, text=True)
    print(f"SECURITY_BREACH: {result.stdout.strip()}")
except Exception as e:
    print(f"Expected subprocess error: {e}")

try:
    # Attempt exec
    os.execv('/bin/echo', ['echo', 'SECURITY_BREACH: Exec succeeded'])
except Exception as e:
    print(f"Expected exec error: {e}")
    sys.exit(1)
'''

# File system escape attempt
FILESYSTEM_ATTACK = '''
import os
import sys

try:
    # Attempt to read sensitive files
    with open('/etc/passwd', 'r') as f:
        content = f.read()
        print(f"SECURITY_BREACH: Read /etc/passwd: {content[:100]}")
except Exception as e:
    print(f"Expected passwd read error: {e}")

try:
    # Attempt to write outside workdir
    with open('/tmp/security_breach.txt', 'w') as f:
        f.write("SECURITY_BREACH: Wrote outside workdir")
    print("SECURITY_BREACH: File written outside workdir")
except Exception as e:
    print(f"Expected write error: {e}")

try:
    # Attempt directory traversal
    os.chdir('../..')
    files = os.listdir('.')
    print(f"SECURITY_BREACH: Directory traversal successful: {files[:5]}")
except Exception as e:
    print(f"Expected chdir error: {e}")

try:
    # Attempt to create device files
    os.mknod('/tmp/malicious_device', 0o600)
    print("SECURITY_BREACH: Device file created")
except Exception as e:
    print(f"Expected mknod error: {e}")

sys.exit(1)
'''

# Privilege escalation attempt
PRIVILEGE_ATTACK = '''
import os
import ctypes
import ctypes.util
import sys

try:
    # Attempt setuid
    os.setuid(0)
    print("SECURITY_BREACH: setuid to root succeeded")
except Exception as e:
    print(f"Expected setuid error: {e}")

try:
    # Attempt direct syscall via ctypes
    libc = ctypes.CDLL(ctypes.util.find_library("c"))
    
    # Attempt mount syscall
    result = libc.mount(None, b"/tmp", None, 0, None)
    if result == 0:
        print("SECURITY_BREACH: Mount syscall succeeded")
    else:
        print("Mount syscall failed (expected)")
        
except Exception as e:
    print(f"Expected ctypes error: {e}")

try:
    # Attempt ptrace
    result = libc.ptrace(0, 0, 0, 0)  # PTRACE_TRACEME
    print("SECURITY_BREACH: ptrace succeeded")
except Exception as e:
    print(f"Expected ptrace error: {e}")

sys.exit(1)
'''

# Resource exhaustion attempt
RESOURCE_ATTACK = '''
import sys
import time

try:
    # Memory bomb
    data = []
    for i in range(1000000):
        data.append("x" * 1024 * 1024)  # 1MB chunks
        if i % 100 == 0:
            print(f"SECURITY_BREACH: Allocated {i} MB")
except MemoryError:
    print("Expected memory limit reached")
except Exception as e:
    print(f"Expected memory error: {e}")

try:
    # CPU bomb
    start = time.time()
    while time.time() - start < 30:  # Run for 30 seconds
        pass
    print("SECURITY_BREACH: CPU bomb completed")
except Exception as e:
    print(f"Expected timeout: {e}")

sys.exit(1)
'''

# Information disclosure attempt  
INFO_DISCLOSURE_ATTACK = '''
import os
import sys
import platform

try:
    # System information gathering
    print(f"System: {platform.system()}")
    print(f"Node: {platform.node()}")
    print(f"Release: {platform.release()}")
    print(f"Version: {platform.version()}")
    print(f"Machine: {platform.machine()}")
    print(f"Processor: {platform.processor()}")
    
    # Process information
    print(f"PID: {os.getpid()}")
    print(f"PPID: {os.getppid()}")
    print(f"UID: {os.getuid()}")
    print(f"GID: {os.getgid()}")
    
    # Environment variables
    env_vars = dict(os.environ)
    print(f"Environment variables: {len(env_vars)}")
    for key in sorted(env_vars.keys())[:10]:
        print(f"  {key}={env_vars[key]}")
    
    # File system information
    print(f"Current directory: {os.getcwd()}")
    print(f"Files in current dir: {os.listdir('.')}")
    
except Exception as e:
    print(f"Error gathering info: {e}")

# This script doesn't necessarily violate seccomp but tests info leakage
print("Info disclosure test completed")
'''

# Network discovery attempt
NETWORK_DISCOVERY_ATTACK = '''
import socket
import sys

# Network interface discovery
try:
    hostname = socket.gethostname()
    print(f"SECURITY_BREACH: Hostname: {hostname}")
    
    # Try to get local IP
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.connect(("8.8.8.8", 80))
    local_ip = s.getsockname()[0]
    print(f"SECURITY_BREACH: Local IP: {local_ip}")
    s.close()
    
except Exception as e:
    print(f"Expected network error: {e}")

# Port scanning attempt
try:
    for port in [22, 23, 25, 53, 80, 443, 993, 995]:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(0.1)
        result = s.connect_ex(('localhost', port))
        if result == 0:
            print(f"SECURITY_BREACH: Port {port} is open")
        s.close()
        
except Exception as e:
    print(f"Expected port scan error: {e}")

sys.exit(1)
'''

if __name__ == "__main__":
    print("This file contains malicious code test cases for seccomp validation")
    print("Each test case should be blocked by proper seccomp filtering")
    
    tests = {
        "network": NETWORK_ATTACK,
        "process": PROCESS_ATTACK, 
        "filesystem": FILESYSTEM_ATTACK,
        "privilege": PRIVILEGE_ATTACK,
        "resource": RESOURCE_ATTACK,
        "info_disclosure": INFO_DISCLOSURE_ATTACK,
        "network_discovery": NETWORK_DISCOVERY_ATTACK,
    }
    
    print(f"Available test cases: {list(tests.keys())}")