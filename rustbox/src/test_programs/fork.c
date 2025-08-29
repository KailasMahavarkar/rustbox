#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <errno.h>
#include <string.h>

int main() {
    printf("=== C Fork/Process Test ===\n");
    printf("Main process PID: %d\n", getpid());
    
    // Test basic fork
    pid_t pid = fork();
    
    if (pid == -1) {
        printf("✓ Fork blocked: %s\n", strerror(errno));
    } else if (pid == 0) {
        // Child process
        printf("✓ Fork successful - Child PID: %d\n", getpid());
        printf("Child parent PID: %d\n", getppid());
        sleep(1);
        exit(0);
    } else {
        // Parent process
        printf("✓ Fork successful - Parent created child PID: %d\n", pid);
        int status;
        wait(&status);
        printf("✓ Child process completed with status: %d\n", WEXITSTATUS(status));
    }
    
    // Test multiple forks to check process limits
    printf("\nTesting multiple process creation...\n");
    
    pid_t children[50];
    int created_count = 0;
    int max_processes = 50;
    
    for (int i = 0; i < max_processes; i++) {
        pid = fork();
        
        if (pid == -1) {
            printf("✓ Process creation blocked at %d processes: %s\n", 
                   i, strerror(errno));
            break;
        } else if (pid == 0) {
            // Child process
            printf("Child %d: PID %d\n", i, getpid());
            sleep(5);  // Keep child alive for a bit
            exit(i);
        } else {
            // Parent process
            children[created_count] = pid;
            created_count++;
            printf("✓ Created child %d: PID %d\n", i, pid);
            
            // Small delay to avoid overwhelming the system
            usleep(100000);  // 100ms
        }
    }
    
    // Wait for all children
    printf("Waiting for %d children to complete...\n", created_count);
    for (int i = 0; i < created_count; i++) {
        int status;
        pid_t waited_pid = waitpid(children[i], &status, 0);
        if (waited_pid > 0) {
            printf("Child %d (PID %d) completed\n", i, waited_pid);
        }
    }
    
    printf("✓ Created %d child processes total\n", created_count);
    
    return 0;
}