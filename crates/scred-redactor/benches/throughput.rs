use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use scred_redactor::streaming::{StreamingConfig, StreamingRedactor};
use scred_redactor::{RedactionConfig, RedactionEngine};
use std::sync::Arc;

/// Build realistic data with varying pattern densities.
/// - `none`: no secrets, just normal log lines
/// - `sparse`: 1 secret per KB of data
/// - `dense`: 10 secrets per KB of data
fn build_realistic_data(density: &str, total_size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(total_size);

    match density {
        "none" => {
            while data.len() < total_size {
                data.extend_from_slice(b"GET /api/v1/users HTTP/1.1\nHost: example.com\nUser-Agent: Mozilla/5.0\nAccept: */*\n\n");
            }
        }
        "sparse" => {
            let mut bytes_since_secret = 0;
            while data.len() < total_size {
                if bytes_since_secret >= 1024 {
                    data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE");
                    bytes_since_secret = 0;
                } else {
                    data.extend_from_slice(b"GET /api/v1/users HTTP/1.1\n");
                    bytes_since_secret += 28;
                }
            }
        }
        "dense" => {
            let mut bytes_since_secret = 0;
            while data.len() < total_size {
                if bytes_since_secret >= 100 {
                    data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE");
                    bytes_since_secret = 0;
                } else {
                    data.extend_from_slice(b"GET /api/v1/users HTTP/1.1\n");
                    bytes_since_secret += 28;
                }
            }
        }
        _ => {}
    }

    data.truncate(total_size);
    data
}

/// Build data with secrets that span chunk boundaries for lookahead testing.
fn build_cross_boundary_data(total_size: usize, chunk_size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(total_size);
    let secret = b"AKIAIOSFODNN7EXAMPLE";
    let filler = b"GET /api/v1/users HTTP/1.1\n";

    while data.len() < total_size {
        // Place a secret that straddles a chunk boundary
        let pos_in_chunk = chunk_size - 5;
        let current_len = data.len();
        let target_pos = (current_len / chunk_size) * chunk_size + pos_in_chunk;

        while data.len() < target_pos && data.len() < total_size {
            data.extend_from_slice(filler);
        }

        if data.len() + secret.len() <= total_size {
            data.extend_from_slice(secret);
        }

        while data.len() < ((data.len() / chunk_size) + 1) * chunk_size && data.len() < total_size {
            data.extend_from_slice(filler);
        }
    }

    data.truncate(total_size);
    data
}

fn benchmark_redact_reader_to_writer(c: &mut Criterion) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    let mut group = c.benchmark_group("redact_reader_to_writer");
    group.sample_size(10);

    for chunk_size in [1024, 65536, 1048576].iter() {
        let config = StreamingConfig {
            chunk_size: *chunk_size,
            lookahead_size: 512,
        };
        let redactor = StreamingRedactor::new(engine.clone(), config);
        let data = build_realistic_data("sparse", 1024 * 1024); // 1MB

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}kb", chunk_size / 1024)),
            chunk_size,
            |b, _| {
                b.iter(|| {
                    let mut reader = std::io::Cursor::new(&data);
                    let mut writer = std::io::Cursor::new(Vec::new());
                    let stats = redactor
                        .redact_reader_to_writer(&mut reader, &mut writer)
                        .unwrap();
                    black_box((writer.into_inner(), stats));
                });
            },
        );
    }

    group.finish();
}

fn benchmark_pattern_density(c: &mut Criterion) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let config = StreamingConfig {
        chunk_size: 65536,
        lookahead_size: 512,
    };
    let redactor = StreamingRedactor::new(engine, config);

    let mut group = c.benchmark_group("pattern_density");
    group.sample_size(10);

    for density in ["none", "sparse", "dense"].iter() {
        let data = build_realistic_data(density, 1024 * 1024);

        group.bench_with_input(
            BenchmarkId::from_parameter(density),
            density,
            |b, _| {
                b.iter(|| {
                    let mut reader = std::io::Cursor::new(&data);
                    let mut writer = std::io::Cursor::new(Vec::new());
                    let stats = redactor
                        .redact_reader_to_writer(&mut reader, &mut writer)
                        .unwrap();
                    black_box((writer.into_inner(), stats));
                });
            },
        );
    }

    group.finish();
}

fn benchmark_cross_boundary(c: &mut Criterion) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let config = StreamingConfig {
        chunk_size: 65536,
        lookahead_size: 512,
    };
    let redactor = StreamingRedactor::new(engine, config);

    let mut group = c.benchmark_group("cross_boundary");
    group.sample_size(10);

    let aligned_data = build_realistic_data("sparse", 1024 * 1024);
    let cross_data = build_cross_boundary_data(1024 * 1024, 65536);

    group.bench_function("aligned", |b| {
        b.iter(|| {
            let mut reader = std::io::Cursor::new(&aligned_data);
            let mut writer = std::io::Cursor::new(Vec::new());
            let stats = redactor
                .redact_reader_to_writer(&mut reader, &mut writer)
                .unwrap();
            black_box((writer.into_inner(), stats));
        });
    });

    group.bench_function("cross_boundary", |b| {
        b.iter(|| {
            let mut reader = std::io::Cursor::new(&cross_data);
            let mut writer = std::io::Cursor::new(Vec::new());
            let stats = redactor
                .redact_reader_to_writer(&mut reader, &mut writer)
                .unwrap();
            black_box((writer.into_inner(), stats));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_redact_reader_to_writer,
    benchmark_pattern_density,
    benchmark_cross_boundary
);
criterion_main!(benches);
