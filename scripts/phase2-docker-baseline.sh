#!/bin/bash
# PHASE 2: Benchmark with docker-compose (httpbin + scred-proxy)
# Establishes baseline for Phase 2 optimizations

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${YELLOW}=== PHASE 2: Docker Compose Benchmark ===${NC}"
echo "Upstream: httpbin (realistic)"
echo "Proxy: scred-proxy (all 242 patterns)"
echo "Test: 500 sequential GET requests"
echo ""

# Start docker-compose
echo "Starting docker-compose..."
docker-compose -f docker-compose.autoresearch.yml up -d
sleep 3

# Wait for services to be healthy
echo "Waiting for services to be healthy..."
for i in {1..30}; do
    if curl -s "http://localhost:8080/status/200" >/dev/null 2>&1 && \
       curl -s "http://localhost:9999/status/200" >/dev/null 2>&1; then
        echo "Services ready"
        break
    fi
    echo -n "."
    sleep 1
done

echo ""

# Cleanup function
cleanup() {
    echo "Stopping docker-compose..."
    docker-compose -f docker-compose.autoresearch.yml down -v 2>/dev/null || true
}

trap cleanup EXIT

# Warm up (10 requests)
echo "Warming up..."
for i in {1..10}; do
    curl -s "http://localhost:9999/status/200" >/dev/null 2>&1 || true
done

echo "Running benchmark (500 sequential GET requests)..."

# Benchmark
start_ns=$(date +%s%N)
success=0
failed=0

for i in {1..500}; do
    if curl -s -f "http://localhost:9999/status/200" >/dev/null 2>&1; then
        success=$((success + 1))
    else
        failed=$((failed + 1))
    fi
    
    if [ $((i % 100)) -eq 0 ]; then
        echo "  $i/500 (success: $success, failed: $failed)"
    fi
done

end_ns=$(date +%s%N)
elapsed_ns=$((end_ns - start_ns))
elapsed_ms=$((elapsed_ns / 1000000))
elapsed_s=$(echo "scale=3; $elapsed_ms / 1000" | bc -l)

if [ $success -lt 450 ]; then
    echo -e "${RED}✗ Only $success/500 succeeded (${failed} failed)${NC}"
    exit 1
fi

# Calculate metrics
# Each response is ~200 bytes from httpbin /status/200
response_size_bytes=200
total_bytes=$((success * response_size_bytes))
throughput_bytes_per_sec=$(echo "scale=1; $total_bytes / $elapsed_s" | bc -l)
throughput_mb_s=$(echo "scale=3; $throughput_bytes_per_sec / 1000000" | bc -l)
rps=$(echo "scale=1; $success / $elapsed_s" | bc -l)

echo ""
echo "Results:"
echo "  Success: $success/500"
echo "  Failed: $failed"
echo "  Duration: ${elapsed_ms}ms"
echo "  RPS: $rps"
echo "  Throughput: ${throughput_mb_s} MB/s"
echo ""

echo -e "${GREEN}Result: ${throughput_mb_s} MB/s${NC}"
echo "METRIC throughput_mb_s=$throughput_mb_s"
