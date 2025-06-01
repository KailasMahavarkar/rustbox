import time
import sys

# CPU intensive computation
def cpu_intensive(duration=5):
    start = time.time()
    count = 0
    while time.time() - start < duration:
        count += 1
        # Some computation to keep CPU busy
        _ = sum(i*i for i in range(1000))
    print(f"Completed {count} iterations in {time.time() - start:.2f} seconds")

if __name__ == "__main__":
    duration = int(sys.argv[1]) if len(sys.argv) > 1 else 5
    cpu_intensive(duration)