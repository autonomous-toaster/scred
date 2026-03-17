use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_detector::{detect_all, detect_simple_prefix, detect_validation};

fn benchmark_simple_prefix(c: &mut Criterion) {
    c.bench_function("detect_aws_akia_10kb", |b| {
        b.iter(|| {
            let data = black_box({
                let mut v = Vec::new();
                for _ in 0..500 {
                    v.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
                }
                v
            });
            detect_simple_prefix(&data)
        });
    });
}

fn benchmark_validation(c: &mut Criterion) {
    c.bench_function("detect_github_token_10kb", |b| {
        b.iter(|| {
            let data = black_box({
                let mut v = Vec::new();
                for _ in 0..500 {
                    v.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
                }
                v
            });
            detect_validation(&data)
        });
    });
}

fn benchmark_all_patterns(c: &mut Criterion) {
    c.bench_function("detect_all_mixed_10kb", |b| {
        b.iter(|| {
            let data = black_box({
                let mut v = Vec::new();
                for _ in 0..100 {
                    v.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ghp_1234567890abcdefghij ");
                    v.extend_from_slice(b"sk-proj-1234567890abcdefghij ASIA1234567890ABCD ");
                }
                v
            });
            detect_all(&data)
        });
    });
}

fn benchmark_large_text(c: &mut Criterion) {
    c.bench_function("detect_all_1mb_realistic", |b| {
        b.iter(|| {
            let mut data = Vec::new();
            for _ in 0..1000 {
                data.extend_from_slice(b"User: alice@example.com ");
                data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
                data.extend_from_slice(b"ghp_1234567890abcdefghij ");
                data.extend_from_slice(b"sk-proj-1234567890abcd ");
                data.extend_from_slice(b"some normal text ");
            }
            detect_all(black_box(&data))
        });
    });
}

criterion_group!(
    benches,
    benchmark_simple_prefix,
    benchmark_validation,
    benchmark_all_patterns,
    benchmark_large_text
);
criterion_main!(benches);
