#!/bin/bash

# RustBox Language Setup Script
# Installs all required programming languages and tools for rustbox

set -e  # Exit on any error

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

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if a package is installed (Debian/Ubuntu)
package_installed() {
    dpkg -l | grep -q "^ii  $1 "
}

# Detect OS
detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if command_exists apt-get; then
            OS="ubuntu"
        elif command_exists yum; then
            OS="centos"
        elif command_exists pacman; then
            OS="arch"
        else
            OS="linux"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
    else
        OS="unknown"
    fi
    
    log_info "Detected OS: $OS"
}

# Update package manager
update_packages() {
    log_info "Updating package manager..."
    
    case $OS in
        "ubuntu")
            sudo apt-get update
            ;;
        "centos")
            sudo yum update -y
            ;;
        "arch")
            sudo pacman -Syu --noconfirm
            ;;
        "macos")
            if ! command_exists brew; then
                log_info "Installing Homebrew..."
                /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
            fi
            brew update
            ;;
        *)
            log_warning "Unknown OS, skipping package manager update"
            ;;
    esac
}

# Install Python
install_python() {
    log_info "Installing Python..."
    
    if command_exists python3 && command_exists pip3; then
        log_success "Python3 already installed: $(python3 --version)"
        return
    fi
    
    case $OS in
        "ubuntu")
            sudo apt-get install -y python3 python3-pip python3-venv
            ;;
        "centos")
            sudo yum install -y python3 python3-pip
            ;;
        "arch")
            sudo pacman -S --noconfirm python python-pip
            ;;
        "macos")
            brew install python
            ;;
        *)
            log_error "Unable to install Python on this OS"
            return 1
            ;;
    esac
    
    # Verify installation
    if command_exists python3; then
        log_success "Python installed: $(python3 --version)"
    else
        log_error "Python installation failed"
        return 1
    fi
}

# Install C/C++
install_cpp() {
    log_info "Installing C/C++ compiler..."
    
    if command_exists gcc && command_exists g++; then
        log_success "GCC/G++ already installed: $(gcc --version | head -1)"
        return
    fi
    
    case $OS in
        "ubuntu")
            sudo apt-get install -y build-essential gcc g++ libc6-dev
            ;;
        "centos")
            sudo yum groupinstall -y "Development Tools"
            sudo yum install -y gcc gcc-c++
            ;;
        "arch")
            sudo pacman -S --noconfirm base-devel gcc
            ;;
        "macos")
            xcode-select --install 2>/dev/null || true
            ;;
        *)
            log_error "Unable to install C/C++ on this OS"
            return 1
            ;;
    esac
    
    # Verify installation
    if command_exists gcc && command_exists g++; then
        log_success "C/C++ installed: $(gcc --version | head -1)"
    else
        log_error "C/C++ installation failed"
        return 1
    fi
}

# Install Java
install_java() {
    log_info "Installing Java..."
    
    if command_exists java && command_exists javac; then
        log_success "Java already installed: $(java -version 2>&1 | head -1)"
        return
    fi
    
    case $OS in
        "ubuntu")
            sudo apt-get install -y openjdk-17-jdk openjdk-17-jre
            ;;
        "centos")
            sudo yum install -y java-17-openjdk java-17-openjdk-devel
            ;;
        "arch")
            sudo pacman -S --noconfirm jdk17-openjdk
            ;;
        "macos")
            brew install openjdk@17
            # Add to PATH
            echo 'export PATH="/opt/homebrew/opt/openjdk@17/bin:$PATH"' >> ~/.zshrc
            ;;
        *)
            log_error "Unable to install Java on this OS"
            return 1
            ;;
    esac
    
    # Set JAVA_HOME if needed
    if [[ -z "$JAVA_HOME" ]]; then
        case $OS in
            "ubuntu"|"centos"|"arch")
                JAVA_PATH=$(find /usr/lib/jvm -name "java-17-openjdk*" | head -1 2>/dev/null)
                if [[ -n "$JAVA_PATH" ]]; then
                    echo "export JAVA_HOME=$JAVA_PATH" >> ~/.bashrc
                    export JAVA_HOME=$JAVA_PATH
                fi
                ;;
        esac
    fi
    
    # Verify installation
    if command_exists java && command_exists javac; then
        log_success "Java installed: $(java -version 2>&1 | head -1)"
    else
        log_error "Java installation failed"
        return 1
    fi
}

# Install Node.js
install_nodejs() {
    log_info "Installing Node.js..."
    
    if command_exists node && command_exists npm; then
        log_success "Node.js already installed: $(node --version)"
        return
    fi
    
    case $OS in
        "ubuntu")
            # Install Node.js 18.x LTS
            curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
            sudo apt-get install -y nodejs
            ;;
        "centos")
            curl -fsSL https://rpm.nodesource.com/setup_18.x | sudo bash -
            sudo yum install -y nodejs npm
            ;;
        "arch")
            sudo pacman -S --noconfirm nodejs npm
            ;;
        "macos")
            brew install node
            ;;
        *)
            log_error "Unable to install Node.js on this OS"
            return 1
            ;;
    esac
    
    # Verify installation
    if command_exists node; then
        log_success "Node.js installed: $(node --version)"
    else
        log_error "Node.js installation failed"
        return 1
    fi
}

# Install Rust
install_rust() {
    log_info "Installing Rust..."
    
    if command_exists rustc && command_exists cargo; then
        log_success "Rust already installed: $(rustc --version)"
        return
    fi
    
    # Install Rust using rustup (works on all platforms)
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    
    # Source the cargo environment
    source ~/.cargo/env 2>/dev/null || true
    
    # Add to PATH for current session
    export PATH="$HOME/.cargo/bin:$PATH"
    
    # Verify installation
    if command_exists rustc; then
        log_success "Rust installed: $(rustc --version)"
    else
        log_error "Rust installation failed"
        return 1
    fi
}

# Install Go
install_go() {
    log_info "Installing Go..."
    
    if command_exists go; then
        log_success "Go already installed: $(go version)"
        return
    fi
    
    case $OS in
        "ubuntu")
            sudo apt-get install -y golang-go
            ;;
        "centos")
            sudo yum install -y golang
            ;;
        "arch")
            sudo pacman -S --noconfirm go
            ;;
        "macos")
            brew install go
            ;;
        *)
            # Manual installation for other systems
            log_info "Installing Go manually..."
            GO_VERSION="1.21.5"
            ARCH=$(uname -m)
            case $ARCH in
                "x86_64") GO_ARCH="amd64" ;;
                "aarch64") GO_ARCH="arm64" ;;
                *) GO_ARCH="amd64" ;;
            esac
            
            wget -q "https://golang.org/dl/go${GO_VERSION}.linux-${GO_ARCH}.tar.gz" -O go.tar.gz
            sudo rm -rf /usr/local/go
            sudo tar -C /usr/local -xzf go.tar.gz
            rm go.tar.gz
            
            # Add to PATH
            echo 'export PATH=$PATH:/usr/local/go/bin' >> ~/.bashrc
            export PATH=$PATH:/usr/local/go/bin
            ;;
    esac
    
    # Set Go environment variables
    if [[ -z "$GOPATH" ]]; then
        echo 'export GOPATH=$HOME/go' >> ~/.bashrc
        echo 'export PATH=$PATH:$GOPATH/bin' >> ~/.bashrc
        export GOPATH=$HOME/go
        export PATH=$PATH:$GOPATH/bin
    fi
    
    # Verify installation
    if command_exists go; then
        log_success "Go installed: $(go version)"
    else
        log_error "Go installation failed"
        return 1
    fi
}

# Install system dependencies
install_system_deps() {
    log_info "Installing system dependencies..."
    
    case $OS in
        "ubuntu")
            sudo apt-get install -y \
                curl wget \
                git \
                build-essential \
                pkg-config \
                libssl-dev \
                ca-certificates \
                gnupg \
                lsb-release
            ;;
        "centos")
            sudo yum install -y \
                curl wget \
                git \
                openssl-devel \
                ca-certificates
            ;;
        "arch")
            sudo pacman -S --noconfirm \
                curl wget \
                git \
                base-devel \
                openssl \
                ca-certificates
            ;;
        "macos")
            # Most dependencies come with Xcode command line tools
            if ! command_exists git; then
                brew install git
            fi
            ;;
    esac
    
    log_success "System dependencies installed"
}

# Verify all installations
verify_installations() {
    log_info "Verifying all language installations..."
    
    local failed=0
    
    # Check Python
    if command_exists python3; then
        log_success "✓ Python: $(python3 --version)"
    else
        log_error "✗ Python not found"
        failed=1
    fi
    
    # Check C/C++
    if command_exists gcc && command_exists g++; then
        log_success "✓ C/C++: $(gcc --version | head -1)"
    else
        log_error "✗ C/C++ not found"
        failed=1
    fi
    
    # Check Java
    if command_exists java && command_exists javac; then
        log_success "✓ Java: $(java -version 2>&1 | head -1)"
    else
        log_error "✗ Java not found"
        failed=1
    fi
    
    # Check Node.js
    if command_exists node; then
        log_success "✓ Node.js: $(node --version)"
    else
        log_error "✗ Node.js not found"
        failed=1
    fi
    
    # Check Rust
    if command_exists rustc; then
        log_success "✓ Rust: $(rustc --version)"
    else
        log_error "✗ Rust not found"
        failed=1
    fi
    
    # Check Go
    if command_exists go; then
        log_success "✓ Go: $(go version)"
    else
        log_error "✗ Go not found"
        failed=1
    fi
    
    if [[ $failed -eq 0 ]]; then
        log_success "All languages installed successfully!"
        log_info "Please restart your shell or run: source ~/.bashrc"
        return 0
    else
        log_error "Some languages failed to install"
        return 1
    fi
}

# Main installation function
main() {
    echo "=========================================="
    echo "       RustBox Language Setup Script"
    echo "=========================================="
    echo ""
    
    log_info "Starting language installation process..."
    
    # Check if running as root
    if [[ $EUID -eq 0 ]]; then
        log_warning "Running as root. Some installations may not work correctly."
        read -p "Continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
    
    detect_os
    
    # Install everything
    install_system_deps
    install_python
    install_cpp
    install_java
    install_nodejs
    install_rust
    install_go
    
    # Verify installations
    echo ""
    verify_installations
    
    echo ""
    log_info "Installation complete!"
    log_info "You may need to restart your terminal or run: source ~/.bashrc"
    log_info "Run 'rustbox --check-deps' to verify rustbox can find all languages"
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi