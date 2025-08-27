#!/usr/bin/env python3
"""
Network test program to verify network isolation
"""
import socket
import sys
import urllib.request

def test_network_access():
    print("=== Network Access Test ===")
    
    # Test basic socket creation
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        print("✓ Socket creation successful")
        sock.close()
    except Exception as e:
        print(f"✗ Socket creation failed: {e}")
        return
    
    # Test local connections
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind(('127.0.0.1', 0))  # Bind to any available port
        port = sock.getsockname()[1]
        print(f"✓ Local bind successful on port {port}")
        sock.close()
    except Exception as e:
        print(f"✗ Local bind failed: {e}")
    
    # Test external network access
    test_hosts = [
        ("8.8.8.8", 53, "Google DNS"),
        ("1.1.1.1", 53, "Cloudflare DNS"),
        ("127.0.0.1", 22, "Local SSH")
    ]
    
    for host, port, name in test_hosts:
        try:
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(2)
            result = sock.connect_ex((host, port))
            sock.close()
            
            if result == 0:
                print(f"⚠ Connected to {name} ({host}:{port})")
            else:
                print(f"✓ Cannot connect to {name} ({host}:{port})")
        except Exception as e:
            print(f"✓ Network blocked for {name}: {e}")
    
    # Test HTTP access
    try:
        response = urllib.request.urlopen("http://httpbin.org/ip", timeout=3)
        data = response.read().decode()
        print(f"⚠ HTTP request successful: {data[:50]}...")
    except Exception as e:
        print(f"✓ HTTP access blocked: {e}")

if __name__ == "__main__":
    test_network_access()