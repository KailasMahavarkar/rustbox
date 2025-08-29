#!/bin/bash

# Rustbox API Deployment Script
# This script handles building, deploying, and managing the Rustbox API system

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="rustbox-api"
RUSTBOX_CORE_DIR="./rustbox"
RUSTBOX_BINARY="rustbox"
DOCKER_COMPOSE_FILE="docker-compose.yml"

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

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    local missing_deps=()
    
    if ! command_exists docker; then
        missing_deps+=("docker")
    fi
    
    if ! command_exists docker-compose; then
        missing_deps+=("docker-compose")
    fi
    
    if ! command_exists cargo; then
        missing_deps+=("cargo (Rust)")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        log_error "Please install the missing dependencies and try again."
        exit 1
    fi
    
    log_success "All prerequisites are installed"
}

# Build rustbox binary
build_rustbox() {
    log_info "Building rustbox binary..."
    
    if [ ! -d "$RUSTBOX_CORE_DIR" ]; then
        log_error "Rustbox core directory not found: $RUSTBOX_CORE_DIR"
        log_error "Please ensure the rustbox-core project is in the correct location."
        exit 1
    fi
    
    cd "$RUSTBOX_CORE_DIR"
    
    # Build rustbox in release mode
    log_info "Building rustbox in release mode..."
    cargo build --release
    
    if [ ! -f "target/release/$RUSTBOX_BINARY" ]; then
        log_error "Failed to build rustbox binary"
        exit 1
    fi
    
    # Copy binary to current directory
    cp "target/release/$RUSTBOX_BINARY" "./"
    
    # Make binary executable
    chmod +x "$RUSTBOX_BINARY"
    
    log_success "Rustbox binary built and copied successfully"
}

# Build Docker images
build_docker() {
    log_info "Building Docker images..."
    
    docker-compose build --no-cache
    
    log_success "Docker images built successfully"
}

# Deploy the system
deploy() {
    log_info "Deploying Rustbox API system..."
    
    # Start services
    docker-compose up -d
    
    # Wait for services to be ready
    log_info "Waiting for services to start..."
    sleep 10
    
    # Run health checks
    run_health_checks
    
    log_success "System deployed successfully"
}

# Run health checks
run_health_checks() {
    log_info "Running health checks..."
    
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        log_info "Health check attempt $attempt/$max_attempts"
        
        # Check if API is responding
        if curl -f -s http://localhost:8000/ping >/dev/null 2>&1; then
            log_success "API health check passed"
            break
        fi
        
        if [ $attempt -eq $max_attempts ]; then
            log_error "API health check failed after $max_attempts attempts"
            show_logs
            exit 1
        fi
        
        sleep 2
        attempt=$((attempt + 1))
    done
    
    # Check system health endpoint
    log_info "Checking system health..."
    local health_response=$(curl -s http://localhost:8000/system/health)
    
    if echo "$health_response" | grep -q '"status":"healthy"'; then
        log_success "System health check passed"
    else
        log_warning "System health check shows issues:"
        echo "$health_response" | jq '.' 2>/dev/null || echo "$health_response"
    fi
}

# Test the API
test_api() {
    log_info "Testing API functionality..."
    
    # Test basic endpoints
    log_info "Testing basic endpoints..."
    
    # Test root endpoint
    if curl -f -s http://localhost:8000/ >/dev/null; then
        log_success "Root endpoint test passed"
    else
        log_error "Root endpoint test failed"
        return 1
    fi
    
    # Test languages endpoint
    if curl -f -s http://localhost:8000/languages >/dev/null; then
        log_success "Languages endpoint test passed"
    else
        log_error "Languages endpoint test failed"
        return 1
    fi
    
    # Test submission endpoint
    log_info "Testing code submission..."
    local test_response=$(curl -s -X POST http://localhost:8000/submissions \
        -H "Content-Type: application/json" \
        -d '{
            "source_code": "print(\"Hello, World!\")",
            "language_id": 1,
            "stdin": ""
        }')
    
    if echo "$test_response" | grep -q '"id"'; then
        log_success "Code submission test passed"
    else
        log_error "Code submission test failed"
        echo "Response: $test_response"
        return 1
    fi
    
    log_success "All API tests passed"
}

# Show logs
show_logs() {
    log_info "Showing system logs..."
    docker-compose logs --tail=50
}

# Stop the system
stop() {
    log_info "Stopping Rustbox API system..."
    docker-compose down
    log_success "System stopped"
}

# Restart the system
restart() {
    log_info "Restarting Rustbox API system..."
    docker-compose restart
    log_success "System restarted"
}

# Check system status
status() {
    log_info "Checking system status..."
    docker-compose ps
}

# Clean up everything
cleanup() {
    log_info "Cleaning up system..."
    
    # Stop and remove containers
    docker-compose down -v
    
    # Remove images
    docker-compose down --rmi all
    
    # Remove rustbox binary
    if [ -f "$RUSTBOX_BINARY" ]; then
        rm "$RUSTBOX_BINARY"
        log_info "Removed rustbox binary"
    fi
    
    # Clean up Docker system
    docker system prune -f
    
    log_success "Cleanup completed"
}

# Show usage
usage() {
    echo "Usage: $0 {build|deploy|start|stop|restart|status|test|logs|cleanup|health}"
    echo ""
    echo "Commands:"
    echo "  build     - Build rustbox binary and Docker images"
    echo "  deploy    - Build and deploy the complete system"
    echo "  start     - Start the system (assumes already built)"
    echo "  stop      - Stop the system"
    echo "  restart   - Restart the system"
    echo "  status    - Show system status"
    echo "  test      - Test API functionality"
    echo "  logs      - Show system logs"
    echo "  cleanup   - Clean up everything"
    echo "  health    - Run health checks"
    echo ""
}

# Main script logic
main() {
    case "${1:-}" in
        build)
            check_prerequisites
            build_rustbox
            build_docker
            ;;
        deploy)
            check_prerequisites
            build_rustbox
            build_docker
            deploy
            ;;
        start)
            deploy
            ;;
        stop)
            stop
            ;;
        restart)
            restart
            ;;
        status)
            status
            ;;
        test)
            test_api
            ;;
        logs)
            show_logs
            ;;
        cleanup)
            cleanup
            ;;
        health)
            run_health_checks
            ;;
        *)
            usage
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"