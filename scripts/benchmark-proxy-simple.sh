#!/bin/bash
# Simple throughput benchmark for scred-proxy using curl and ab (Apache Bench)
# Falls back to custom bash implementation if ab not available

set -e

PROXY_URL="${PROXY_URL:-http://localhost:9999}"
UPSTREAM_URL="${UPSTREAM_URL:-http://localhost:8080}"
DURATION="${DURATION:-10}"
NUM_REQUESTS="${NUM_REQUESTS:-1000}"

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== scred-proxy Throughput Benchmark ==="
echo "Proxy: $PROXY_URL"
echo "Upstream: $UPSTREAM_URL"
echo "Requests: $NUM_REQUESTS over ${DURATION}s"
echo ""

# Verify services are up
echo "Checking services..."
max_retries=5
retry=0
while ! curl -f -s -m 1 "$UPSTREAM_URL/status/200" > /dev/null 2>&1; do
    if [ $retry -ge $max_retries ]; then
        echo -e "${RED}✗ Upstream not responding at $UPSTREAM_URL${NC}"
        exit 1
    fi
    echo "Waiting for upstream... ($retry/$max_retries)"
    sleep 1
    retry=$((retry+1))
done

retry=0
while ! curl -f -s -m 1 "$PROXY_URL/status/200" > /dev/null 2>&1; do
    if [ $retry -ge $max_retries ]; then
        echo -e "${RED}✗ Proxy not responding at $PROXY_URL${NC}"
        exit 1
    fi
    echo "Waiting for proxy... ($retry/$max_retries)"
    sleep 1
    retry=$((retry+1))
done

echo -e "${GREEN}✓ All services healthy${NC}"
echo ""

# Try Apache Bench if available
if command -v ab &> /dev/null; then
    echo "Using Apache Bench (ab)..."
    echo ""
    
    # Benchmark: simple GET
    echo "GET /status/200 (${NUM_REQUESTS} requests, concurrent=4)"
    ab_output=$(ab -n $NUM_REQUESTS -c 4 "$PROXY_URL/status/200" 2>&1)
    echo "$ab_output" | tail -20
    
    # Extract RPS
    rps=$(echo "$ab_output" | grep "Requests per second" | awk '{print $4}')
    echo ""
    echo "Requests/sec: $rps"
    
    # Estimate throughput (assuming 500 byte avg response)
    throughput=$(echo "scale=2; $rps * 500 / 1000000" | bc -l)
    echo "Est. Throughput: ${throughput} MB/s"
else
    echo -e "${YELLOW}Apache Bench not available, using custom bash benchmark${NC}"
    echo ""
    
    # Custom benchmark using parallel curl requests
    echo "Benchmark: ${NUM_REQUESTS} GET requests to /status/200"
    
    # Create temp file for requests
    tmpdir=$(mktemp -d)
    
    # Warm up
    echo "Warming up (10 requests)..."
    for i in {1..10}; do
        curl -s "$PROXY_URL/status/200" > /dev/null 2>&1 &
        wait $!
    done
    
    echo "Running benchmark..."
    start_time=$(date +%s%N)
    
    # Send requests with limited concurrency (4 at a time)
    pids=()
    for i in $(seq 1 $NUM_REQUESTS); do
        curl -s "$PROXY_URL/status/200" > /dev/null 2>&1 &
        pids+=($!)
        
        # Keep max 4 concurrent
        if [ ${#pids[@]} -ge 4 ]; then
            wait -n
            pids=(${pids[@]/$!/})
        fi
        
        # Progress indicator
        if [ $((i % 100)) -eq 0 ]; then
            echo "  Completed: $i/$NUM_REQUESTS"
        fi
    done
    
    # Wait for remaining
    wait
    
    end_time=$(date +%s%N)
    elapsed_ns=$((end_time - start_time))
    elapsed_s=$(echo "scale=3; $elapsed_ns / 1000000000" | bc -l)
    
    rps=$(echo "scale=2; $NUM_REQUESTS / $elapsed_s" | bc -l)
    throughput=$(echo "scale=2; $rps * 500 / 1000000" | bc -l)
    
    echo ""
    echo "=========================================="
    echo "Requests: $NUM_REQUESTS"
    echo "Time: ${elapsed_s}s"
    echo "Requests/sec: $rps"
    echo "Est. Throughput: ${throughput} MB/s"
    echo "=========================================="
fi

echo ""
echo -e "${GREEN}=== BENCHMARK COMPLETE ===${NC}"
echo "Metric: $throughput"
