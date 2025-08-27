#include <stdio.h>
#include <unistd.h>

int main() {
    printf("Hello from C program in rustbox!\n");
    printf("Process ID: %d\n", getpid());
    printf("User ID: %d\n", getuid());
    return 0;
}