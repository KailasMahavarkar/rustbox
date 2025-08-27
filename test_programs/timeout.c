#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <time.h>
#include <string.h>

void test_timeouts() {
    printf("=== C Timeout Test ===\n");
    
    // Test short operation
    printf("Testing 2-second operation...\n");
    time_t start = time(NULL);
    sleep(2);
    time_t elapsed = time(NULL) - start;
    printf("✓ 2-second operation completed in %ld seconds\n", elapsed);
    
    // Test medium operation
    printf("Testing 5-second operation...\n");
    start = time(NULL);
    sleep(5);
    elapsed = time(NULL) - start;
    printf("✓ 5-second operation completed in %ld seconds\n", elapsed);
    
    // Test long operation that should be killed
    printf("Testing 30-second operation (should be terminated)...\n");
    start = time(NULL);
    
    for (int i = 1; i <= 30; i++) {
        printf("Second %d/30\n", i);
        fflush(stdout);
        sleep(1);
    }
    
    elapsed = time(NULL) - start;
    printf("⚠ 30-second operation completed in %ld seconds (not terminated)\n", elapsed);
}

void infinite_loop_test() {
    printf("=== C Infinite Loop Test ===\n");
    printf("Starting infinite loop (should be terminated by timeout)...\n");
    
    unsigned long long counter = 0;
    time_t start = time(NULL);
    time_t last_print = start;
    
    while (1) {
        counter++;
        
        // Print status every second
        time_t now = time(NULL);
        if (now - last_print >= 1) {
            printf("Loop iteration %llu, elapsed: %ld seconds\n", 
                   counter, now - start);
            fflush(stdout);
            last_print = now;
        }
    }
}

int main(int argc, char *argv[]) {
    if (argc > 1 && strcmp(argv[1], "infinite") == 0) {
        infinite_loop_test();
    } else {
        test_timeouts();
    }
    
    return 0;
}