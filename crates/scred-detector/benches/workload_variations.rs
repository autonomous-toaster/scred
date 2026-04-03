use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_detector::detect_all;

fn workload_no_secrets(c: &mut Criterion) {
    let data = "normal log messages with no secrets at all\n".repeat(2000);
    c.bench_function("no_secrets_200kb", |b| {
        b.iter(|| detect_all(black_box(data.as_bytes())))
    });
}

fn workload_many_matches(c: &mut Criterion) {
    let data = ("AKIAIOSFODNN7EXAMPLE1234567890AB ".repeat(1000) + "\n").repeat(30);
    c.bench_function("many_matches_1mb", |b| {
        b.iter(|| detect_all(black_box(data.as_bytes())))
    });
}

fn workload_mixed_realistic(c: &mut Criterion) {
    let mut data = Vec::new();
    for i in 0..1000 {
        data.extend_from_slice(format!("Log entry {}: ", i).as_bytes());
        if i % 4 == 0 {
            data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
        }
        if i % 5 == 0 {
            data.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
        }
        if i % 6 == 0 {
            data.extend_from_slice(b"sk-proj-abcdefghijklmnopqrstuvwxyz0123456 ");
        }
        data.extend_from_slice(b"normal content\n");
    }
    c.bench_function("mixed_realistic_100kb", |b| {
        b.iter(|| detect_all(black_box(&data)))
    });
}

criterion_group!(
    benches,
    workload_no_secrets,
    workload_many_matches,
    workload_mixed_realistic
);
criterion_main!(benches);
