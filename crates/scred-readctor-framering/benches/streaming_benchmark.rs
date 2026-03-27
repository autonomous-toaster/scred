use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use scred_readctor_framering::{RedactionEngine, RedactionConfig, StreamingRedactor, StreamingConfig};
use std::sync::Arc;

fn create_test_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    while data.len() < size {
        data.extend_from_slice(pattern);
    }
    data.truncate(size);
    data
}

fn create_test_data_no_patterns(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let pattern = b"This is normal data with no secrets at all in this line here\n";
    while data.len() < size {
        data.extend_from_slice(pattern);
    }
    data.truncate(size);
    data
}

fn benchmark_streaming_core(c: &mut Criterion) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);

    let mut group = c.benchmark_group("streaming_core");
    group.sample_size(10); // Reduce sample size for large benchmarks

    // Benchmark: 1MB no patterns
    group.bench_function("1mb_no_patterns", |b| {
        let data = black_box(create_test_data_no_patterns(1024 * 1024));
        b.iter(|| {
            let (output, stats) = redactor.redact_buffer(&data);
            assert!(output.len() >= data.len() - 1024); // Allow for lookahead buffer
            black_box((output, stats));
        });
    });

    // Benchmark: 1MB with patterns (50% density)
    group.bench_function("1mb_with_patterns", |b| {
        let data = black_box(create_test_data(1024 * 1024));
        b.iter(|| {
            let (output, stats) = redactor.redact_buffer(&data);
            assert!(output.len() >= data.len() - 1024); // Allow for lookahead buffer
            assert!(stats.patterns_found > 0);
            black_box((output, stats));
        });
    });

    // Benchmark: 10MB no patterns
    group.bench_function("10mb_no_patterns", |b| {
        let data = black_box(create_test_data_no_patterns(10 * 1024 * 1024));
        b.iter(|| {
            let (output, stats) = redactor.redact_buffer(&data);
            assert!(output.len() >= data.len() - 1024); // Allow for lookahead buffer
            black_box((output, stats));
        });
    });

    // Benchmark: 10MB with patterns
    group.bench_function("10mb_with_patterns", |b| {
        let data = black_box(create_test_data(10 * 1024 * 1024));
        b.iter(|| {
            let (output, stats) = redactor.redact_buffer(&data);
            assert!(output.len() >= data.len() - 1024); // Allow for lookahead buffer
            assert!(stats.patterns_found > 0);
            black_box((output, stats));
        });
    });

    group.finish();
}

fn benchmark_chunk_sizes(c: &mut Criterion) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let data = black_box(create_test_data(10 * 1024 * 1024));

    let mut group = c.benchmark_group("chunk_sizes");
    group.sample_size(5);

    for chunk_size in [8192, 16384, 32768, 65536, 131072].iter() {
        let config = StreamingConfig {
            chunk_size: *chunk_size,
            lookahead_size: 512,
        };
        let redactor = StreamingRedactor::new(engine.clone(), config);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}kb", chunk_size / 1024)),
            chunk_size,
            |b, _| {
                b.iter(|| {
                    let (output, _stats) = redactor.redact_buffer(&data);
                    assert!(output.len() >= data.len() - 1024); // Allow for lookahead buffer
                    black_box(output);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_lookahead_sizes(c: &mut Criterion) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let data = black_box(create_test_data(10 * 1024 * 1024));

    let mut group = c.benchmark_group("lookahead_sizes");
    group.sample_size(5);

    for lookahead_size in [128, 256, 512, 1024, 2048].iter() {
        let config = StreamingConfig {
            chunk_size: 65536,
            lookahead_size: *lookahead_size,
        };
        let redactor = StreamingRedactor::new(engine.clone(), config);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}b", lookahead_size)),
            lookahead_size,
            |b, _| {
                b.iter(|| {
                    let (output, _stats) = redactor.redact_buffer(&data);
                    assert!(output.len() >= data.len() - 1024); // Allow for lookahead buffer
                    black_box(output);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_streaming_core,
    benchmark_chunk_sizes,
    benchmark_lookahead_sizes
);
criterion_main!(benches);
