#!/bin/bash
# Baseline benchmark for scred-proxy throughput
# Uses environment variables for configuration

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROXY_PORT=9999
UPSTREAM_PORT=8888

cleanup() {
    # Kill proxy
    pkill -f "scred-proxy" 2>/dev/null || true
    # Kill upstream
    pkill -f "upstream_handler.py" 2>/dev/null || true
    sleep 0.5
}

trap cleanup EXIT

echo -e "${YELLOW}=== BASELINE: scred-proxy Throughput ===${NC}"

# Start simple upstream server (Python HTTP server)
cat > /tmp/upstream_handler.py << 'PYEOF'
#!/usr/bin/env python3
import http.server
import socketserver
import json

class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        response = json.dumps({"ok": True, "msg": "x" * 470})  # ~500 bytes total
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.send_header('Content-Length', str(len(response)))
        self.end_headers()
        self.wfile.write(response.encode())
    
    def log_message(self, *args):
        pass

socketserver.TCPServer(("127.0.0.1", 8888), Handler).serve_forever()
PYEOF

echo "Starting upstream server..."
python3 /tmp/upstream_handler.py >/dev/null 2>&1 &
sleep 1

# Wait for upstream
for i in {1..10}; do
    if curl -s "http://127.0.0.1:8888/" >/dev/null 2>&1; then
        echo -e "${GREEN}✓ Upstream ready${NC}"
        break
    fi
    sleep 0.2
done

# Start proxy with env config
echo "Starting scred-proxy..."
export SCRED_PROXY_UPSTREAM_URL="http://127.0.0.1:8888"
export SCRED_PROXY_LISTEN_PORT="$PROXY_PORT"
export RUST_LOG="error"
./target/release/scred-proxy >/dev/null 2>&1 &
PROXY_PID=$!
sleep 2

# Wait for proxy
for i in {1..10}; do
    if curl -s "http://127.0.0.1:$PROXY_PORT/" >/dev/null 2>&1; then
        echo -e "${GREEN}✓ scred-proxy ready${NC}"
        break
    fi
    if [ $i -eq 10 ]; then
        echo -e "${RED}✗ Proxy failed to start${NC}"
        exit 1
    fi
    sleep 0.2
done

echo ""
echo "Running benchmark (200 sequential requests)..."

# Warm up
for i in {1..5}; do
    curl -s "http://127.0.0.1:$PROXY_PORT/" >/dev/null 2>&1 || true
done

# Benchmark
start_ns=$(date +%s%N)
success=0

for i in {1..200}; do
    if curl -s -f "http://127.0.0.1:$PROXY_PORT/" >/dev/null 2>&1; then
        success=$((success + 1))
    fi
    if [ $((i % 50)) -eq 0 ]; then
        echo "  $i/200"
    fi
done

end_ns=$(date +%s%N)
elapsed_ns=$((end_ns - start_ns))
elapsed_ms=$((elapsed_ns / 1000000))
elapsed_s=$(echo "scale=3; $elapsed_ms / 1000" | bc -l)

if [ $success -lt 150 ]; then
    echo -e "${RED}✗ Only $success/200 succeeded${NC}"
    exit 1
fi

# Calculate throughput
rps=$(echo "scale=1; $success / $elapsed_s" | bc -l)
throughput_mb_s=$(echo "scale=3; $rps * 500 / 1000000" | bc -l)

echo ""
echo "=========================================="
echo "Success: $success/200"
echo "Time: ${elapsed_ms}ms"
echo "RPS: $rps"
echo "Throughput: ${throughput_mb_s} MB/s"
echo "=========================================="
echo ""

echo -e "${GREEN}BASELINE: ${throughput_mb_s} MB/s${NC}"
echo "METRIC throughput_mb_s=$throughput_mb_s"
