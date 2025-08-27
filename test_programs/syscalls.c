#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <errno.h>
#include <string.h>

int main() {
    printf("=== C System Call Test ===\n");
    
    // Test basic system calls
    printf("Process ID: %d\n", getpid());
    printf("Parent PID: %d\n", getppid());
    printf("User ID: %d\n", getuid());
    printf("Group ID: %d\n", getgid());
    
    // Test file operations
    if (access(".", F_OK) == 0) {
        printf("✓ access() allowed\n");
    } else {
        printf("✗ access() failed: %s\n", strerror(errno));
    }
    
    // Test potentially dangerous system calls
    
    // Test chmod
    if (chmod(".", 0755) == 0) {
        printf("⚠ chmod() allowed\n");
    } else {
        printf("✓ chmod() blocked: %s\n", strerror(errno));
    }
    
    // Test fork
    pid_t pid = fork();
    if (pid == -1) {
        printf("✓ fork() blocked: %s\n", strerror(errno));
    } else if (pid == 0) {
        // Child process
        printf("⚠ fork() successful - child process\n");
        exit(0);
    } else {
        // Parent process
        printf("⚠ fork() successful - parent process\n");
        wait(NULL);
    }
    
    // Test exec
    pid = fork();
    if (pid == 0) {
        execl("/bin/echo", "echo", "Hello from exec", NULL);
        printf("✓ execl() blocked: %s\n", strerror(errno));
        exit(1);
    } else if (pid > 0) {
        int status;
        wait(&status);
        if (WIFEXITED(status) && WEXITSTATUS(status) == 0) {
            printf("⚠ execl() successful\n");
        } else {
            printf("✓ execl() blocked or failed\n");
        }
    }
    
    // Test system() call
    int result = system("echo 'Hello from system()'");
    if (result == 0) {
        printf("⚠ system() call successful\n");
    } else {
        printf("✓ system() call blocked or failed\n");
    }
    
    return 0;
}