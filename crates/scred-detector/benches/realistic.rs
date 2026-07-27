use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_detector::detect_all;

/// Build realistic mixed data with all pattern categories
fn build_realistic_data() -> Vec<u8> {
    let mut data = Vec::new();
    // Mix realistic HTTP logs with secrets from all categories
    for i in 0..1000 {
        // AWS keys
        data.extend_from_slice(
            format!("AWS Access Key: AKIAIOSFODNN7EXAMPLE (user {}) ", i).as_bytes(),
        );
        // GitHub tokens
        data.extend_from_slice(b"GitHub PAT: ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
        // OpenAI keys
        data.extend_from_slice(b"OpenAI: sk-proj-abcdefghijklmnopqrstuvwxyz0123456 ");
        // JWTs
        data.extend_from_slice(b"JWT: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U ");
        // SSH keys
        data.extend_from_slice(b"SSH: -----BEGIN OPENSSH PRIVATE KEY-----\n");
        // Database URIs
        data.extend_from_slice(
            format!("DB: postgresql://user:pass@host{}.internal:5432/proddb ", i).as_bytes(),
        );
        // Webhook URLs
        data.extend_from_slice(b"Webhook: https://hooks.slack.com/services/T00/B00/abc123 ");
        // Normal log context
        data.extend_from_slice(format!("Response time: {}ms status=200\n", i * 10).as_bytes());
    }
    data
}

fn benchmark_realistic_mixed_data(c: &mut Criterion) {
    let data = build_realistic_data();

    c.bench_function("detect_all_realistic_1mb", |b| {
        b.iter(|| detect_all(black_box(&data)))
    });
}

criterion_group!(benches, benchmark_realistic_mixed_data);
criterion_main!(benches);
