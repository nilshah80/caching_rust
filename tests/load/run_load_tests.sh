#!/bin/bash
#
# Load Test Runner Script
#
# This script runs load tests and benchmark comparisons for the caching services.
#
# Usage:
#   ./run_load_tests.sh [command] [options]
#
# Commands:
#   quick     - Run quick test suite (~15 min)
#   full      - Run full test suite (~90 min)
#   service   - Run service stress tests only
#   redis     - Run Redis stress tests only
#   compare   - Run benchmark comparison (Rust vs Go)
#   single    - Run a single test by name
#
# Options:
#   --rust-url URL    Rust service URL (default: http://localhost:8080)
#   --go-url URL      Go service URL (default: http://localhost:8081)
#   --concurrency N   Number of concurrent workers (default: 50)
#   --duration N      Test duration in seconds (default: 60)
#   --output DIR      Output directory for results (default: ./results)
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Default values
RUST_URL="${RUST_URL:-http://localhost:8080}"
GO_URL="${GO_URL:-http://localhost:8081}"
CONCURRENCY="${CONCURRENCY:-50}"
DURATION="${DURATION:-60}"
OUTPUT_DIR="${OUTPUT_DIR:-./results}"
COMMAND="${1:-quick}"

# Parse arguments
shift || true
while [[ $# -gt 0 ]]; do
    case $1 in
        --rust-url)
            RUST_URL="$2"
            shift 2
            ;;
        --go-url)
            GO_URL="$2"
            shift 2
            ;;
        --concurrency)
            CONCURRENCY="$2"
            shift 2
            ;;
        --duration)
            DURATION="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        *)
            TEST_NAME="$1"
            shift
            ;;
    esac
done

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Logging
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="$OUTPUT_DIR/load_test_${TIMESTAMP}.log"

log() {
    echo -e "$1" | tee -a "$LOG_FILE"
}

header() {
    log ""
    log "${BLUE}============================================================${NC}"
    log "${BLUE} $1${NC}"
    log "${BLUE}============================================================${NC}"
    log ""
}

success() {
    log "${GREEN}✓ $1${NC}"
}

error() {
    log "${RED}✗ $1${NC}"
}

warning() {
    log "${YELLOW}! $1${NC}"
}

info() {
    log "${CYAN}→ $1${NC}"
}

# Check prerequisites
check_prerequisites() {
    header "Checking Prerequisites"

    # Check if Rust is installed
    if command -v cargo &> /dev/null; then
        success "Rust/Cargo is installed"
    else
        error "Rust/Cargo is not installed"
        exit 1
    fi

    # Check if services are running
    info "Checking Rust service at $RUST_URL..."
    if curl -s "$RUST_URL/health" > /dev/null 2>&1; then
        success "Rust service is healthy"
    else
        error "Rust service is not available at $RUST_URL"
        exit 1
    fi

    if [[ "$COMMAND" == "compare" ]]; then
        info "Checking Go service at $GO_URL..."
        if curl -s "$GO_URL/api/v1/health" > /dev/null 2>&1; then
            success "Go service is healthy"
        else
            error "Go service is not available at $GO_URL"
            exit 1
        fi
    fi
}

# Build load test tools
build_tools() {
    header "Building Load Test Tools"

    cd "$(dirname "$0")"

    info "Building load test binaries..."
    cargo build --release 2>&1 | tee -a "$LOG_FILE"

    if [[ $? -eq 0 ]]; then
        success "Build successful"
    else
        error "Build failed"
        exit 1
    fi
}

# Run load tests
run_load_test() {
    local test_type=$1
    local url=$2

    header "Running $test_type Tests"

    cd "$(dirname "$0")"

    ./target/release/load_test --url "$url" "$test_type" 2>&1 | tee -a "$LOG_FILE"

    if [[ $? -eq 0 ]]; then
        success "$test_type tests completed"
    else
        warning "$test_type tests had failures"
    fi
}

# Run benchmark comparison
run_comparison() {
    header "Running Benchmark Comparison"

    cd "$(dirname "$0")"

    ./target/release/benchmark_compare \
        --rust-url "$RUST_URL" \
        --go-url "$GO_URL" \
        --concurrency "$CONCURRENCY" \
        --duration "$DURATION" \
        --warmup 10 \
        2>&1 | tee -a "$LOG_FILE"

    # Move results to output directory
    mv benchmark_comparison_*.json "$OUTPUT_DIR/" 2>/dev/null || true
}

# Collect system info
collect_system_info() {
    header "System Information"

    log "Date: $(date)"
    log "Host: $(hostname)"
    log "OS: $(uname -s) $(uname -r)"
    log "CPU: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || cat /proc/cpuinfo | grep 'model name' | head -1 | cut -d: -f2)"
    log "Memory: $(sysctl -n hw.memsize 2>/dev/null | awk '{print $1/1024/1024/1024 " GB"}' || free -h | grep Mem | awk '{print $2}')"
    log ""
    log "Rust URL: $RUST_URL"
    log "Go URL: $GO_URL"
    log "Concurrency: $CONCURRENCY"
    log "Duration: ${DURATION}s"
}

# Print summary
print_summary() {
    header "Test Summary"

    log "Log file: $LOG_FILE"
    log "Results directory: $OUTPUT_DIR"
    log ""

    # List result files
    if ls "$OUTPUT_DIR"/*.json 1> /dev/null 2>&1; then
        log "Result files:"
        for f in "$OUTPUT_DIR"/*.json; do
            log "  - $f"
        done
    fi

    log ""
    log "${GREEN}Load tests completed!${NC}"
}

# Main
main() {
    header "Redis Caching Service Load Tests"

    collect_system_info
    check_prerequisites
    build_tools

    case $COMMAND in
        quick)
            run_load_test "quick" "$RUST_URL"
            ;;
        full|all)
            run_load_test "all" "$RUST_URL"
            ;;
        service)
            run_load_test "service" "$RUST_URL"
            ;;
        redis)
            run_load_test "redis" "$RUST_URL"
            ;;
        compare)
            run_comparison
            ;;
        single|test)
            if [[ -z "$TEST_NAME" ]]; then
                error "Please specify a test name"
                log "Available tests: spike, overload, memory, batch, hotkey, dataset, connection, expiration, pressure, pipeline"
                exit 1
            fi
            header "Running Single Test: $TEST_NAME"
            cd "$(dirname "$0")"
            ./target/release/load_test --url "$RUST_URL" test "$TEST_NAME" 2>&1 | tee -a "$LOG_FILE"
            ;;
        list)
            cd "$(dirname "$0")"
            ./target/release/load_test list
            exit 0
            ;;
        *)
            log "Usage: $0 [command] [options]"
            log ""
            log "Commands:"
            log "  quick     - Run quick test suite (~15 min)"
            log "  full      - Run full test suite (~90 min)"
            log "  service   - Run service stress tests only"
            log "  redis     - Run Redis stress tests only"
            log "  compare   - Run benchmark comparison (Rust vs Go)"
            log "  single    - Run a single test by name"
            log "  list      - List available tests"
            log ""
            log "Options:"
            log "  --rust-url URL    Rust service URL (default: http://localhost:8080)"
            log "  --go-url URL      Go service URL (default: http://localhost:8081)"
            log "  --concurrency N   Number of concurrent workers (default: 50)"
            log "  --duration N      Test duration in seconds (default: 60)"
            log "  --output DIR      Output directory for results (default: ./results)"
            exit 0
            ;;
    esac

    # Move result files
    mv load_test_results_*.json "$OUTPUT_DIR/" 2>/dev/null || true

    print_summary
}

main "$@"
