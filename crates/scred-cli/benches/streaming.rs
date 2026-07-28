use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use scred_http::{ConfigurableEngine, PatternSelector};
use scred_redactor::{RedactionConfig, RedactionEngine};
use std::sync::Arc;

/// Build realistic data with varying pattern densities.
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

fn benchmark_cli_text_mode(c: &mut Criterion) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let config_engine = ConfigurableEngine::new(
        engine,
        PatternSelector::All,
        PatternSelector::All,
    );

    let mut group = c.benchmark_group("cli_text_mode");
    group.sample_size(10);

    for size in [1024 * 1024, 10 * 1024 * 1024].iter() {
        let data = build_realistic_data("sparse", *size);
        let text = String::from_utf8_lossy(&data).into_owned();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}mb", size / 1024 / 1024)),
            size,
            |b, _| {
                b.iter(|| {
                    let result = config_engine.detect_and_redact(black_box(&text));
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_cli_pattern_density(c: &mut Criterion) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let config_engine = ConfigurableEngine::new(
        engine,
        PatternSelector::All,
        PatternSelector::All,
    );

    let mut group = c.benchmark_group("cli_pattern_density");
    group.sample_size(10);

    for density in ["none", "sparse", "dense"].iter() {
        let data = build_realistic_data(density, 1024 * 1024);
        let text = String::from_utf8_lossy(&data).into_owned();

        group.bench_with_input(
            BenchmarkId::from_parameter(density),
            density,
            |b, _| {
                b.iter(|| {
                    let result = config_engine.detect_and_redact(black_box(&text));
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_cli_text_mode, benchmark_cli_pattern_density);
criterion_main!(benches);
