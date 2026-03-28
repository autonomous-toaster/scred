#!/bin/bash

# P1 Benchmarking Script - Simple, reliable version
# Uses environment variables for configuration
# Target: Validate 3-5 MB/s with P1 optimizations

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Configuration (proxy defaults to 0.0.0.0:9999, can't change via env)
LISTEN_ADDR="127.0.0.1"
LISTEN_PORT="9999"
DEBUG_ADDR="127.0.0.1"
DEBUG_PORT="8899"
RESPONSE_SIZE="1000"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}SCRED Proxy P1 Benchmarking - Phase 1 Validation${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo ""

# Cleanup
cleanup() {
    echo -e "\n${BLUE}[*] Cleaning up...${NC}"
    pkill -f "scred-debug-server" 2>/dev/null || true
    pkill -f "scred-proxy" 2>/dev/null || true
    sleep 1
}

trap cleanup EXIT

# Build
echo -e "${BLUE}[*] Building binaries...${NC}"
cargo build -p scred-debug-server --release 2>&1 | tail -1
cargo build -p scred-proxy --release 2>&1 | tail -1

# Start debug server
echo -e "${BLUE}[*] Starting debug server on ${DEBUG_ADDR}:${DEBUG_PORT}...${NC}"
./target/release/scred-debug-server \
    -a "$DEBUG_ADDR" \
    -p "$DEBUG_PORT" \
    -r "$RESPONSE_SIZE" \
    > /tmp/debug.log 2>&1 &
DEBUG_PID=$!
sleep 2

if ! kill -0 $DEBUG_PID 2>/dev/null; then
    echo -e "${RED}[!] Debug server failed${NC}"
    cat /tmp/debug.log
    exit 1
fi
echo -e "${GREEN}[+] Debug server running (PID: $DEBUG_PID)${NC}"

# Start proxy with env vars (proxy defaults to 0.0.0.0:9999)
echo -e "${BLUE}[*] Starting proxy on 0.0.0.0:${LISTEN_PORT}...${NC}"
export SCRED_PROXY_UPSTREAM_URL="http://${DEBUG_ADDR}:${DEBUG_PORT}/"
export RUST_LOG="info"

./target/release/scred-proxy \
    > /tmp/proxy.log 2>&1 &
PROXY_PID=$!
sleep 2

if ! kill -0 $PROXY_PID 2>/dev/null; then
    echo -e "${RED}[!] Proxy failed to start${NC}"
    cat /tmp/proxy.log
    exit 1
fi
echo -e "${GREEN}[+] Proxy running (PID: $PROXY_PID)${NC}"

# Test
echo -e "${BLUE}[*] Testing connectivity...${NC}"
sleep 1

if ! timeout 5 curl -s -x "http://${LISTEN_ADDR}:${LISTEN_PORT}" "http://${DEBUG_ADDR}:${DEBUG_PORT}/" > /dev/null 2>&1; then
    echo -e "${RED}[!] Proxy connection failed${NC}"
    cat /tmp/proxy.log | tail -20
    exit 1
fi
echo -e "${GREEN}[+] Proxy working${NC}"

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Running Benchmark (1000 requests, 10 concurrent)${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo ""

# Run benchmark
BENCH_RESULT="/tmp/bench_result.txt"
hey -n 1000 \
    -c 10 \
    -x "http://${LISTEN_ADDR}:${LISTEN_PORT}" \
    "http://${DEBUG_ADDR}:${DEBUG_PORT}/" \
    2>&1 | tee "$BENCH_RESULT"

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Analysis${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
echo ""

# Parse results - grep differently to avoid flag confusion
RPS=$(grep "Requests/sec:" "$BENCH_RESULT" | awk '{print $2}' | head -1)

if [ ! -z "$RPS" ]; then
    # Response size: ~1000 bytes + ~100 bytes HTTP overhead
    AVG_SIZE=$(echo "scale=0; $RESPONSE_SIZE + 100" | bc)
    MIMPORAL=$(echo "scale=3; $RPS * $AVG_SIZE / 1048576" | bc 2>/dev/null || echo "0")
    
    echo "Results:"
    echo "  Requests/sec: $RPS"
    echo "  Avg response size: ~${AVG_SIZE} bytes"
    echo "  Estimated throughput: ${MIMPORAL} MB/s"
    echo ""
    
    # Target validation
    if (( $(echo "$MIMPORAL > 2.5" | bc -l 2>/dev/null || echo 0) )); then
        echo -e "${GREEN}✓ GOOD: ${MIMPORAL} MB/s >= 2.5 MB/s${NC}"
        if (( $(echo "$MIMPORAL > 3.0" | bc -l 2>/dev/null || echo 0) )); then
            echo -e "${GREEN}✓ TARGET MET: ${MIMPORAL} MB/s >= 3 MB/s${NC}"
        fi
    else
        echo -e "${YELLOW}⚠ Below target: ${MIMPORAL} MB/s < 3 MB/s${NC}"
    fi
else
    echo -e "${RED}[!] Failed to parse benchmark results${NC}"
    cat "$BENCH_RESULT"
fi

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
