#!/bin/bash
# Run baseline throughput test for scred-proxy
# Starts httpbin on localhost:8080, proxy on localhost:9999, and measures throughput

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}=== Starting Baseline Throughput Test ===${NC}"
echo ""

# Check if httpbin is running, if not try to start it
if ! curl -s -f http://localhost:8080/status/200 > /dev/null 2>&1; then
    echo "Starting httpbin on localhost:8080..."
    # Try docker first
    if command -v docker &> /dev/null; then
        docker run -d -p 8080:80 --name httpbin-baseline kennethreitz/httpbin 2>/dev/null || true
        sleep 2
    else
        echo -e "${RED}ERROR: Docker not available and httpbin not running${NC}"
        echo "Please start httpbin manually: docker run -p 8080:80 kennethreitz/httpbin"
        exit 1
    fi
fi

# Give httpbin time to start
echo "Waiting for httpbin to be ready..."
for i in {1..10}; do
    if curl -s -f http://localhost:8080/status/200 > /dev/null 2>&1; then
        echo -e "${GREEN}✓ httpbin ready${NC}"
        break
    fi
    echo "  Attempt $i/10..."
    sleep 1
done

echo ""
echo "Starting scred-proxy on localhost:9999..."
# Start proxy in background
RUST_LOG=warn ./target/release/scred-proxy \
    --upstream http://localhost:8080 \
    --listen-port 9999 &
PROXY_PID=$!

# Wait for proxy to start
sleep 2

# Check if proxy is running
if ! curl -s -f http://localhost:9999/status/200 > /dev/null 2>&1; then
    echo -e "${RED}✗ Proxy failed to start${NC}"
    kill $PROXY_PID 2>/dev/null || true
    exit 1
fi

echo -e "${GREEN}✓ Proxy running (PID: $PROXY_PID)${NC}"
echo ""

# Run benchmark
echo "Running throughput benchmark..."
echo "Target: http://localhost:9999/status/200"
echo "Requests: 100 (sequential to measure baseline)"
echo ""

start_time=$(date +%s%N)

for i in {1..100}; do
    curl -s -f http://localhost:9999/status/200 > /dev/null 2>&1
    if [ $((i % 10)) -eq 0 ]; then
        echo "  $i/100 completed"
    fi
done

end_time=$(date +%s%N)
elapsed_ns=$((end_time - start_time))
elapsed_ms=$((elapsed_ns / 1000000))
elapsed_s=$(echo "scale=3; $elapsed_ms / 1000" | bc -l)

rps=$(echo "scale=2; 100 / $elapsed_s" | bc -l)
# Assume 500 byte response
throughput_mb_s=$(echo "scale=3; $rps * 500 / 1000000" | bc -l)

echo ""
echo "=========================================="
echo "Time elapsed: ${elapsed_ms}ms (${elapsed_s}s)"
echo "Requests/sec: $rps"
echo "Est. Response size: 500 bytes"
echo "Estimated Throughput: ${throughput_mb_s} MB/s"
echo "=========================================="

# Cleanup
echo ""
echo "Cleaning up..."
kill $PROXY_PID 2>/dev/null || true
docker stop httpbin-baseline 2>/dev/null || true
docker rm httpbin-baseline 2>/dev/null || true

echo ""
echo -e "${GREEN}Baseline measurement: ${throughput_mb_s} MB/s${NC}"
echo "$throughput_mb_s"
