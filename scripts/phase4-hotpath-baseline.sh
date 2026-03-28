#!/bin/bash
# PHASE 4: Optimize hot path - measure streaming redaction performance

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}=== PHASE 4: Hot Path Optimization ===${NC}"
echo "Goal: Measure and optimize streaming redaction without network I/O"
echo ""

# Create test binary that benchmarks just the redaction hot path
cat > /tmp/test_hotpath.rs << 'EOF'
use scred_redactor::{RedactionEngine, StreamingRedactor, StreamingConfig, RedactionConfig};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    // Create redaction engine with all patterns
    let config = RedactionConfig { enabled: true };
    let engine = Arc::new(RedactionEngine::new(config));
    let redactor = Arc::new(StreamingRedactor::new(engine, StreamingConfig::default()));
    
    // Create test payload - simulates HTTP response
    let mut payload = String::new();
    for _ in 0..1000 {
        payload.push_str("aws_access_key_id=AKIAIOSFODNN7EXAMPLE&aws_secret_access_key=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY\n");
    }
    
    // Warm up
    let mut lookahead = Vec::new();
    for _ in 0..10 {
        redactor.process_chunk(payload.as_bytes(), &mut lookahead, false);
    }
    
    // Benchmark
    let iterations = 100;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let mut lookahead = Vec::new();
        redactor.process_chunk(payload.as_bytes(), &mut lookahead, true);
    }
    
    let elapsed = start.elapsed();
    let per_chunk_us = elapsed.as_micros() as f64 / iterations as f64;
    let mb_per_s = (payload.len() as f64 * iterations as f64) / (elapsed.as_secs_f64() * 1_000_000.0);
    
    println!("Hot path benchmark:");
    println!("  Payload size: {} bytes", payload.len());
    println!("  Iterations: {}", iterations);
    println!("  Time per chunk: {:.2} µs", per_chunk_us);
    println!("  Throughput: {:.2} MB/s", mb_per_s);
    println!("METRIC throughput_mb_s={:.3}", mb_per_s);
}
EOF

echo "Building hot path benchmark..."
cd /Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred

# Compile test (it will link with existing libraries)
rustc --edition 2021 \
  -L target/release/deps \
  --extern scred_redactor=target/release/libscred_redactor.rlib \
  -L /Users/jean-christophe.saad-dupuy2/.cargo/registry/src/index.crates.io-*/*/target/release \
  /tmp/test_hotpath.rs -o /tmp/test_hotpath 2>&1 | head -20 || echo "Compile warning (expected)"

echo "Running hot path benchmark..."
/tmp/test_hotpath 2>&1 | tail -10 || echo "Note: Direct hotpath test requires full linking setup"
