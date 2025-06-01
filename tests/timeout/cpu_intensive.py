#!/usr/bin/env python3
import time

print("Starting CPU intensive task")
start_time = time.time()

# CPU intensive loop
counter = 0
while time.time() - start_time < 60:  # Run for 60 seconds
    counter += 1
    if counter % 1000000 == 0:
        elapsed = time.time() - start_time
        print(f"Running for {elapsed:.1f} seconds, counter: {counter}")

print(f"Completed after {time.time() - start_time:.1f} seconds")
print(f"Final counter: {counter}")