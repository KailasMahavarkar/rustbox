#!/usr/bin/env python3
"""
Simple hello world program for testing rustbox basic execution
"""
print("Hello from rustbox sandbox!")
import sys, time
print(sys.version.split()[0])


for i in range(1000):
    print(i)
    time.sleep(1)