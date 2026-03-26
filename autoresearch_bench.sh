#!/bin/bash
# Benchmark script for autoresearch
cd "$(dirname "$0")"
cargo bench --bench realistic --quiet 2>&1 | grep -oP 'time:\s*\[[\d.]+ ms [\d.]+ ms \K[0-9.]+(?= ms)'
