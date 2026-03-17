#!/bin/bash
# Simple Python HTTP server for serving sensitive data
# Used to test response-only redaction scenarios

PORT=${1:-8001}
SERVE_DIR=$(dirname "$0")

echo "Starting fake upstream on port $PORT"
echo "Serving files from: $SERVE_DIR"
echo "Endpoint: http://localhost:$PORT/secrets.json"
echo ""
echo "Press Ctrl+C to stop"
echo ""

cd "$SERVE_DIR"
python3 -m http.server $PORT --directory . --bind 0.0.0.0
