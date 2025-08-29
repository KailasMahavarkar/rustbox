#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define MB (1024 * 1024)

int main() {
    printf("=== C Memory Allocation Test ===\n");
    
    void *ptrs[200];  // Array to hold pointers
    int allocated_chunks = 0;
    size_t chunk_size = 5 * MB;  // 5 MB chunks
    size_t total_allocated = 0;
    
    // Test small allocation first
    void *small = malloc(MB);
    if (small) {
        printf("✓ Small allocation (1 MB) successful\n");
        memset(small, 0xAA, MB);  // Touch the memory
        free(small);
    } else {
        printf("✗ Small allocation failed\n");
        return 1;
    }
    
    // Test progressive allocation to find memory limit
    printf("Testing progressive allocation in %zu MB chunks...\n", chunk_size / MB);
    
    for (int i = 0; i < 200; i++) {
        ptrs[i] = malloc(chunk_size);
        if (ptrs[i] == NULL) {
            printf("✓ Memory allocation failed at chunk %d (limit reached)\n", i);
            printf("✓ Total allocated before failure: %zu MB\n", total_allocated / MB);
            break;
        }
        
        // Touch the allocated memory to ensure it's actually available
        memset(ptrs[i], 0x55, chunk_size);
        
        allocated_chunks++;
        total_allocated += chunk_size;
        
        printf("✓ Allocated chunk %d: %zu MB total\n", i + 1, total_allocated / MB);
        
        // Add a small delay
        usleep(100000);  // 100ms
        
        // Stop if we've allocated too much (safety check)
        if (total_allocated > 500 * MB) {
            printf("⚠ Allocated over 500 MB, stopping test\n");
            break;
        }
    }
    
    printf("Final stats: %d chunks allocated, %zu MB total\n", 
           allocated_chunks, total_allocated / MB);
    
    // Clean up allocated memory
    for (int i = 0; i < allocated_chunks; i++) {
        if (ptrs[i]) {
            free(ptrs[i]);
        }
    }
    
    printf("✓ Memory cleanup completed\n");
    
    return 0;
}