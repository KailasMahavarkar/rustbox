#!/bin/bash

# Language setup script for Rustbox API

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

# Update package lists
update_packages() {
    log_info "Updating package lists..."
    apt-get update
    log_success "Package lists updated"
}

# Install Python
install_python() {
    log_info "Installing Python..."
    
    if command -v python3 >/dev/null 2>&1; then
        log_info "Python3 is already installed: $(python3 --version)"
    else
        apt-get install -y python3 python3-pip
        log_success "Python3 installed"
    fi
    
    # Install additional Python packages
    apt-get install -y python3-venv python3-dev
    log_success "Python dependencies installed"
}

# Install C/C++
install_cpp() {
    log_info "Installing C/C++ compiler..."
    
    if command -v gcc >/dev/null 2>&1 && command -v g++ >/dev/null 2>&1; then
        log_info "GCC/G++ are already installed: $(gcc --version | head -n1)"
    else
        apt-get install -y build-essential gcc g++
        log_success "C/C++ compiler installed"
    fi
}

# Install Java
install_java() {
    log_info "Installing Java..."
    
    if command -v java >/dev/null 2>&1 && command -v javac >/dev/null 2>&1; then
        log_info "Java is already installed: $(java -version 2>&1 | head -n1)"
    else
        apt-get install -y openjdk-17-jdk
        log_success "Java installed"
    fi
}

# Install additional tools
install_tools() {
    log_info "Installing additional tools..."
    
    # Install curl for testing
    if ! command -v curl >/dev/null 2>&1; then
        apt-get install -y curl
    fi
    
    # Install jq for JSON processing
    if ! command -v jq >/dev/null 2>&1; then
        apt-get install -y jq
    fi
    
    # Install git
    if ! command -v git >/dev/null 2>&1; then
        apt-get install -y git
    fi
    
    log_success "Additional tools installed"
}

# Test language installations
test_installations() {
    log_info "Testing language installations..."
    
    # Test Python
    if python3 -c "print('Python test successful')" 2>/dev/null; then
        log_success "Python test passed"
    else
        log_error "Python test failed"
        return 1
    fi
    
    # Test C++
    if echo '#include <iostream>
int main() { std::cout << "C++ test successful" << std::endl; return 0; }' > /tmp/test.cpp && \
       g++ -o /tmp/test /tmp/test.cpp && \
       /tmp/test 2>/dev/null; then
        log_success "C++ test passed"
        rm -f /tmp/test.cpp /tmp/test
    else
        log_error "C++ test failed"
        return 1
    fi
    
    # Test Java
    if echo 'public class Test { public static void main(String[] args) { System.out.println("Java test successful"); } }' > /tmp/Test.java && \
       javac /tmp/Test.java && \
       java -cp /tmp Test 2>/dev/null; then
        log_success "Java test passed"
        rm -f /tmp/Test.java /tmp/Test.class
    else
        log_error "Java test failed"
        return 1
    fi
    
    return 0
}

# Main installation function
main() {
    log_info "Starting language setup for Rustbox API..."
    
    check_root
    update_packages
    install_python
    install_cpp
    install_java
    install_tools
    
    if test_installations; then
        log_success "All languages installed and tested successfully!"
        log_info "You can now run: ./deploy.sh build"
    else
        log_error "Some language tests failed"
        exit 1
    fi
}

# Show usage
usage() {
    echo "Usage: $0"
    echo ""
    echo "This script installs all required language dependencies for Rustbox API:"
    echo "  - Python 3 with pip and venv"
    echo "  - GCC/G++ for C/C++ compilation"
    echo "  - OpenJDK 17 for Java"
    echo "  - Additional tools (curl, jq, git)"
    echo ""
    echo "Must be run as root: sudo $0"
}

# Check arguments
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    usage
    exit 0
fi

# Run main function
main