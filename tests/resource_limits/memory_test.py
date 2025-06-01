import sys

def memory_intensive(mb=100):
    print(f"Allocating {mb}MB of memory...")
    # Allocate memory in chunks
    data = []
    chunk_size = 1024 * 1024  # 1MB chunks
    
    for i in range(mb):
        chunk = bytearray(chunk_size)
        # Fill with some data to ensure allocation
        for j in range(0, chunk_size, 1024):
            chunk[j:j+10] = b"test_data"
        data.append(chunk)
        
        if i % 10 == 0:
            print(f"Allocated {i+1}MB so far...")
    
    print(f"Successfully allocated {mb}MB")
    input("Press Enter to release memory...")

if __name__ == "__main__":
    mb = int(sys.argv[1]) if len(sys.argv) > 1 else 100
    memory_intensive(mb)