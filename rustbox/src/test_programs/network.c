#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

int main() {
    printf("=== C Network Test ===\n");
    
    // Test socket creation
    int sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock < 0) {
        printf("✗ Socket creation failed\n");
        return 1;
    }
    printf("✓ Socket creation successful\n");
    
    // Test local bind
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = 0;  // Let system choose port
    
    if (bind(sock, (struct sockaddr*)&addr, sizeof(addr)) == 0) {
        printf("✓ Local bind successful\n");
    } else {
        printf("✗ Local bind failed\n");
    }
    
    close(sock);
    
    // Test external connections
    struct {
        const char *ip;
        int port;
        const char *name;
    } test_targets[] = {
        {"8.8.8.8", 53, "Google DNS"},
        {"127.0.0.1", 22, "Local SSH"},
        {"127.0.0.1", 80, "Local HTTP"}
    };
    
    int num_targets = sizeof(test_targets) / sizeof(test_targets[0]);
    
    for (int i = 0; i < num_targets; i++) {
        sock = socket(AF_INET, SOCK_STREAM, 0);
        if (sock < 0) continue;
        
        memset(&addr, 0, sizeof(addr));
        addr.sin_family = AF_INET;
        addr.sin_port = htons(test_targets[i].port);
        inet_pton(AF_INET, test_targets[i].ip, &addr.sin_addr);
        
        if (connect(sock, (struct sockaddr*)&addr, sizeof(addr)) == 0) {
            printf("⚠ Connected to %s (%s:%d)\n", 
                   test_targets[i].name, test_targets[i].ip, test_targets[i].port);
        } else {
            printf("✓ Cannot connect to %s (%s:%d)\n", 
                   test_targets[i].name, test_targets[i].ip, test_targets[i].port);
        }
        
        close(sock);
    }
    
    return 0;
}