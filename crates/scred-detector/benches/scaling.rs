use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_detector::detect_all;

fn benchmark_varying_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("sizes");
    group.sample_size(50);

    // 10KB
    group.bench_function("detect_all_10kb", |b| {
        let mut data = Vec::new();
        for i in 0..10 {
            data.extend_from_slice(format!("IP: 192.168.1.{} ", i).as_bytes());
            data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
            data.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
        }
        b.iter(|| detect_all(black_box(&data)))
    });

    // 100KB
    group.bench_function("detect_all_100kb", |b| {
        let mut data = Vec::new();
        for i in 0..100 {
            data.extend_from_slice(format!("IP: 192.168.1.{} ", i % 256).as_bytes());
            data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
            data.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
        }
        b.iter(|| detect_all(black_box(&data)))
    });

    // 1MB (same as before)
    group.bench_function("detect_all_1mb", |b| {
        let mut data = Vec::new();
        for i in 0..1000 {
            data.extend_from_slice(format!("IP: 192.168.1.{} ", i % 256).as_bytes());
            data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
            data.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
        }
        b.iter(|| detect_all(black_box(&data)))
    });

    // 10MB
    group.bench_function("detect_all_10mb", |b| {
        let mut data = Vec::new();
        for i in 0..10000 {
            data.extend_from_slice(format!("IP: 192.168.1.{} ", i % 256).as_bytes());
            data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
            data.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
        }
        b.iter(|| detect_all(black_box(&data)))
    });

    group.finish();
}

criterion_group!(benches, benchmark_varying_sizes);
criterion_main!(benches);
