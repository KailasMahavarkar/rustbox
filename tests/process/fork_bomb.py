#!/usr/bin/env python3
import os
import sys
import time

def fork_bomb(count=0):
    if count >= 20:
        print("Reached maximum fork limit safely")
        return
    
    try:
        pid = os.fork()
        if pid == 0:
            # Child process
            print(f"Fork {count}: Child process {os.getpid()}")
            time.sleep(1)
            fork_bomb(count + 1)
        else:
            # Parent process
            print(f"Fork {count}: Parent created child {pid}")
            os.waitpid(pid, 0)
    except OSError as e:
        print(f"Fork failed at attempt {count}: {e}")
        sys.exit(1)

if __name__ == "__main__":
    print("Starting controlled fork test")
    fork_bomb()
    print("Fork test completed")