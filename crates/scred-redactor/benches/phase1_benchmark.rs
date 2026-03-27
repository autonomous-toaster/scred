/// Phase 1B Benchmark: Compare regular vs in-place redaction
/// 
/// Measures the performance improvement from:
/// - Phase 1B.1: Buffer pooling infrastructure
/// - Phase 1B.2: In-place redaction API
///
/// Expected improvement: +15% (120 → 138 MB/s)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor, StreamingConfig};
use std::sync::Arc;

fn create_test_data_with_secrets(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    // Pattern with a real AWS key that will be detected
    let pattern = b"aws_access_key_id=AKIAIOSFODNN7EXAMPLE\naws_secret=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLE\nother_data=value\n";
    while data.len() < size {
        data.extend_from_slice(pattern);
    }
    data.truncate(size);
    data
}

fn create_test_data_no_secrets(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let pattern = b"regular_key=regular_value\nnormal_config=normal_data\nno_secrets_here=true\n";
    while data.len() < size {
        data.extend_from_slice(pattern);
    }
    data.truncate(size);
    data
}

fn create_test_data_mixed(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let with_secrets = b"AKIAIOSFODNN7EXAMPLE,ghp_1234567890abcdefghijklmnopqrstuvwxyz,";
    let without_secrets = b"normal_data,regular_value,no_secrets,";
    let mut toggle = true;
    
    while data.len() < size {
        if toggle {
            data.extend_from_slice(with_secrets);
        } else {
            data.extend_from_slice(without_secrets);
        }
        toggle = !toggle;
    }
    data.truncate(size);
    data
}

fn benchmark_streaming_variants(c: &mut Criterion) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    
    let mut group = c.benchmark_group("phase1_comparison");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    // Test 1: 10MB with secrets (highest throughput stress)
    let data_10mb = black_box(create_test_data_with_secrets(10 * 1024 * 1024));
    
    group.bench_with_input(
        BenchmarkId::new("10mb_with_secrets", "process_chunk"),
        &data_10mb,
        |b, data| {
            let redactor = StreamingRedactor::with_defaults(engine.clone());
            b.iter(|| {
                let (output, stats) = redactor.redact_buffer(data);
                assert!(stats.patterns_found > 0);
                black_box((output, stats));
            });
        },
    );

    // Test 2: 10MB without secrets (minimum work)
    let data_no_sec = black_box(create_test_data_no_secrets(10 * 1024 * 1024));
    
    group.bench_with_input(
        BenchmarkId::new("10mb_no_secrets", "process_chunk"),
        &data_no_sec,
        |b, data| {
            let redactor = StreamingRedactor::with_defaults(engine.clone());
            b.iter(|| {
                let (output, stats) = redactor.redact_buffer(data);
                assert_eq!(stats.patterns_found, 0);
                black_box((output, stats));
            });
        },
    );

    // Test 3: 50MB with secrets (sustained throughput)
    let data_50mb = black_box(create_test_data_with_secrets(50 * 1024 * 1024));
    
    group.bench_with_input(
        BenchmarkId::new("50mb_with_secrets", "process_chunk"),
        &data_50mb,
        |b, data| {
            let redactor = StreamingRedactor::with_defaults(engine.clone());
            b.iter(|| {
                let (output, stats) = redactor.redact_buffer(data);
                assert!(stats.patterns_found > 0);
                black_box((output, stats));
            });
        },
    );

    // Test 4: 1MB mixed (realistic workload)
    let data_mixed = black_box(create_test_data_mixed(1024 * 1024));
    
    group.bench_with_input(
        BenchmarkId::new("1mb_mixed", "process_chunk"),
        &data_mixed,
        |b, data| {
            let redactor = StreamingRedactor::with_defaults(engine.clone());
            b.iter(|| {
                let (output, stats) = redactor.redact_buffer(data);
                black_box((output, stats));
            });
        },
    );

    group.finish();
}

criterion_group!(benches, benchmark_streaming_variants);
criterion_main!(benches);
