import time
import sys

def time_test(seconds=10):
    print(f"Sleeping for {seconds} seconds...")
    time.sleep(seconds)
    print("Sleep completed")

if __name__ == "__main__":
    seconds = int(sys.argv[1]) if len(sys.argv) > 1 else 10
    time_test(seconds)