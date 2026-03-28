#!/bin/bash
# Benchmark script for scred-proxy throughput testing
# Uses wrk for realistic HTTP load testing

set -e

# Configuration
PROXY_URL="${PROXY_URL:-http://localhost:9999}"
UPSTREAM_URL="${UPSTREAM_URL:-http://localhost:8080}"
DURATION="${DURATION:-10}"
CONNECTIONS="${CONNECTIONS:-4}"
THREADS="${THREADS:-2}"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "=== scred-proxy Throughput Benchmark ==="
echo "Proxy URL: $PROXY_URL"
echo "Upstream URL: $UPSTREAM_URL"
echo "Duration: ${DURATION}s"
echo "Connections: $CONNECTIONS"
echo "Threads: $THREADS"
echo ""

# Verify services are up
echo "Checking services..."
if ! curl -f -s "$UPSTREAM_URL/status/200" > /dev/null 2>&1; then
    echo -e "${RED}ERROR: Upstream not responding at $UPSTREAM_URL${NC}"
    exit 1
fi

if ! curl -f -s "$PROXY_URL/status/200" > /dev/null 2>&1; then
    echo -e "${RED}ERROR: Proxy not responding at $PROXY_URL${NC}"
    exit 1
fi

echo -e "${GREEN}✓ All services healthy${NC}"
echo ""

# Benchmark 1: Simple GET requests (no secrets)
echo "Benchmark 1: Simple GET /status/200"
echo "Command: wrk -t $THREADS -c $CONNECTIONS -d ${DURATION}s $PROXY_URL/status/200"
echo ""

# Install wrk if not present
if ! command -v wrk &> /dev/null; then
    echo "Installing wrk..."
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        apt-get update && apt-get install -y wrk || brew install wrk || true
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        brew install wrk || true
    fi
fi

# Run benchmark with wrk
wrk_output=$(wrk -t $THREADS -c $CONNECTIONS -d ${DURATION}s "$PROXY_URL/status/200" 2>&1)
echo "$wrk_output"
echo ""

# Extract throughput (requests per second)
rps=$(echo "$wrk_output" | grep "Requests/sec" | awk '{print $2}')
if [ -z "$rps" ]; then
    echo "WARNING: Could not extract RPS from wrk output"
    rps=0
fi

# Estimate throughput in MB/s based on average response size
# Typical HTTP response for /status/200 is ~500 bytes
avg_response_size=500  # bytes
throughput_bytes_per_sec=$(echo "$rps * $avg_response_size" | bc -l)
throughput_mb_s=$(echo "scale=2; $throughput_bytes_per_sec / 1000000" | bc -l)

echo "=========================================="
echo "Requests/sec: $rps"
echo "Est. Response Size: ${avg_response_size} bytes"
echo "Est. Throughput: ${throughput_mb_s} MB/s"
echo "=========================================="
echo ""

# Benchmark 2: POST requests with body (closer to real scenario)
echo "Benchmark 2: POST /post with body"
echo "Command: wrk -t $THREADS -c $CONNECTIONS -d ${DURATION}s -s post.lua $PROXY_URL/post"
echo ""

# Create Lua script for POST requests
cat > /tmp/post.lua << 'EOF'
request = function()
    wrk.method = "POST"
    wrk.headers["Content-Type"] = "application/json"
    wrk.body = '{"secret": "sk-1234567890abcdefghijklmnop", "user": "test@example.com"}'
    return wrk.format(nil)
end
EOF

wrk_output_post=$(wrk -t $THREADS -c $CONNECTIONS -d ${DURATION}s -s /tmp/post.lua "$PROXY_URL/post" 2>&1)
echo "$wrk_output_post"
echo ""

rps_post=$(echo "$wrk_output_post" | grep "Requests/sec" | awk '{print $2}')
if [ -z "$rps_post" ]; then
    echo "WARNING: Could not extract RPS from wrk output"
    rps_post=0
fi

# POST responses are larger (~1KB)
avg_response_size_post=1000  # bytes
throughput_bytes_per_sec_post=$(echo "$rps_post * $avg_response_size_post" | bc -l)
throughput_mb_s_post=$(echo "scale=2; $throughput_bytes_per_sec_post / 1000000" | bc -l)

echo "=========================================="
echo "Requests/sec: $rps_post"
echo "Est. Response Size: ${avg_response_size_post} bytes"
echo "Est. Throughput: ${throughput_mb_s_post} MB/s"
echo "=========================================="
echo ""

# Report final throughput (average of both benchmarks)
final_throughput=$(echo "scale=2; ($throughput_mb_s + $throughput_mb_s_post) / 2" | bc -l)
echo -e "${GREEN}=== FINAL RESULT ===${NC}"
echo "Average Throughput: ${final_throughput} MB/s"
echo ""

# Return the metric for autoresearch
echo "$final_throughput"
