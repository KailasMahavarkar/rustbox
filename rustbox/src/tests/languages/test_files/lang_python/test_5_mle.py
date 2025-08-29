import sys

def main():
    size = 1
    try:
        while True:
            arr = [0] * size
            size *= 2
            if size <= 0:
                break
    except MemoryError:
        print(f"Memory allocation failed at size = {size}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    main()