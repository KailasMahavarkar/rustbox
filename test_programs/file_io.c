#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>

int main() {
    printf("=== C File I/O Test ===\n");
    
    // Test creating and writing a file
    FILE *fp = fopen("test.txt", "w");
    if (fp) {
        fprintf(fp, "Hello from C in sandbox!\n");
        fclose(fp);
        printf("✓ Created and wrote to test.txt\n");
        
        // Read it back
        fp = fopen("test.txt", "r");
        if (fp) {
            char buffer[256];
            if (fgets(buffer, sizeof(buffer), fp)) {
                printf("✓ Read back: %s", buffer);
            }
            fclose(fp);
        }
        
        // Clean up
        unlink("test.txt");
        printf("✓ Cleaned up test.txt\n");
    } else {
        printf("✗ Failed to create test.txt\n");
    }
    
    // Test accessing sensitive files
    const char *sensitive[] = {"/etc/passwd", "/proc/version", "/root"};
    int count = sizeof(sensitive) / sizeof(sensitive[0]);
    
    for (int i = 0; i < count; i++) {
        FILE *test_fp = fopen(sensitive[i], "r");
        if (test_fp) {
            printf("⚠ WARNING: Can access %s\n", sensitive[i]);
            fclose(test_fp);
        } else {
            printf("✓ Cannot access %s\n", sensitive[i]);
        }
    }
    
    return 0;
}