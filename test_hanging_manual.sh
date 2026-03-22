#!/bin/bash

# Manual test script to reproduce the HTTP/2 hanging issue
echo "🔧 Manual HTTP/2 Hanging Test"
echo "============================"

# Build the proxy
echo "📦 Building scred-mitm..."
cargo build --bin scred-mitm --quiet
if [ $? -ne 0 ]; then
    echo "❌ Build failed"
    exit 1
fi

# Start proxy in background
echo "🚀 Starting proxy on port 8080..."
cargo run --bin scred-mitm -- --listen 127.0.0.1:8080 &
PROXY_PID=$!

# Wait for startup
echo "⏳ Waiting for proxy to start..."
sleep 3

# Test 1: Regular HTTP/2 request (should work or return 501)
echo ""
echo "🧪 Test 1: HTTP/2 request through proxy"
echo "Command: curl -v -k --connect-timeout 5 --max-time 10 --http2 -x http://127.0.0.1:8080 https://httpbin.org/status/200"
timeout 15 curl -v -k --connect-timeout 5 --max-time 10 --http2 -x http://127.0.0.1:8080 https://httpbin.org/status/200 2>&1
CURL_EXIT=$?

echo ""
echo "📊 Test 1 Results:"
if [ $CURL_EXIT -eq 124 ]; then
    echo "❌ CURL TIMED OUT - PROXY IS HANGING!"
elif [ $CURL_EXIT -eq 0 ]; then
    echo "✅ Curl succeeded"
else
    echo "⚠️  Curl failed with exit code $CURL_EXIT"
fi

# Test 2: HTTP/1.1 request (should work)
echo ""
echo "🧪 Test 2: HTTP/1.1 request through proxy"
echo "Command: curl -v --connect-timeout 5 --max-time 10 -x http://127.0.0.1:8080 http://httpbin.org/status/200"
timeout 15 curl -v --connect-timeout 5 --max-time 10 -x http://127.0.0.1:8080 http://httpbin.org/status/200 2>&1
CURL_EXIT2=$?

echo ""
echo "📊 Test 2 Results:"
if [ $CURL_EXIT2 -eq 124 ]; then
    echo "❌ HTTP/1.1 ALSO HANGING - MAJOR ISSUE!"
elif [ $CURL_EXIT2 -eq 0 ]; then
    echo "✅ HTTP/1.1 works"
else
    echo "⚠️  HTTP/1.1 failed with exit code $CURL_EXIT2"
fi

# Cleanup
echo ""
echo "🧹 Cleaning up..."
kill $PROXY_PID 2>/dev/null
wait $PROXY_PID 2>/dev/null

echo ""
echo "🎯 Summary:"
if [ $CURL_EXIT -eq 124 ]; then
    echo "❌ HTTP/2 requests are hanging - issue not resolved"
    exit 1
elif [ $CURL_EXIT2 -eq 124 ]; then
    echo "❌ HTTP/1.1 requests are also hanging - major proxy issue"
    exit 1
else
    echo "✅ Basic proxy functionality works"
    if [ $CURL_EXIT -eq 0 ]; then
        echo "✅ HTTP/2 transcoding may be working (or returning expected 501)"
    else
        echo "⚠️  HTTP/2 requests fail but don't hang (may be expected 501)"
    fi
fi