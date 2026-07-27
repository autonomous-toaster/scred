use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_detector::detect_all;

/// Build test data with patterns from all 5 detection tiers
fn build_test_data(size_kb: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size_kb * 1024);
    let target = size_kb * 1024;

    // Mix patterns from all 5 tiers
    let patterns: &[&[u8]] = &[
        // Tier 1: Simple prefix (AWS keys, GitHub tokens)
        b"AKIAIOSFODNN7EXAMPLE ",
        b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ",
        // Tier 2: Prefix validation (database URLs, webhook URLs)
        b"postgresql://user:pass@localhost:5432/db ",
        b"https://hooks.slack.com/services/T00/B00/abc123 ",
        // Tier 3: JWT tokens
        b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U ",
        // Tier 4: Multiline markers (SSH keys, PEM)
        b"-----BEGIN OPENSSH PRIVATE KEY-----\n",
        b"-----END OPENSSH PRIVATE KEY-----\n",
        // Tier 5: URI patterns (various API endpoints)
        b"https://api.github.com/repos/owner/repo ",
        b"https://management.azure.com/subscriptions/abc123 ",
    ];

    let mut i = 0;
    while data.len() < target {
        data.extend_from_slice(format!("field_{}: ", i % 100).as_bytes());
        data.extend_from_slice(patterns[i % patterns.len()]);
        data.extend_from_slice(b"normal text here for context\n");
        i += 1;
    }
    data.truncate(target);
    data
}

fn benchmark_varying_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");
    group.sample_size(50);

    // Build data outside b.iter() for each size
    let data_10kb = build_test_data(10);
    let data_100kb = build_test_data(100);
    let data_1mb = build_test_data(1024);
    let data_10mb = build_test_data(10240);

    group.bench_function("detect_all_10kb", |b| {
        b.iter(|| detect_all(black_box(&data_10kb)))
    });

    group.bench_function("detect_all_100kb", |b| {
        b.iter(|| detect_all(black_box(&data_100kb)))
    });

    group.bench_function("detect_all_1mb", |b| {
        b.iter(|| detect_all(black_box(&data_1mb)))
    });

    group.bench_function("detect_all_10mb", |b| {
        b.iter(|| detect_all(black_box(&data_10mb)))
    });

    group.finish();
}

criterion_group!(benches, benchmark_varying_sizes);
criterion_main!(benches);
