use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_detector::{detect_all, detect_simple_prefix, detect_validation};

fn benchmark_realistic_mixed_data(c: &mut Criterion) {
    c.bench_function("detect_all_realistic_1mb", |b| {
        b.iter(|| {
            let mut data = Vec::new();
            // Mix realistic HTTP logs with secrets
            for i in 0..1000 {
                data.extend_from_slice(format!("IP: 192.168.1.{} ", i % 256).as_bytes());
                data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
                data.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
                data.extend_from_slice(format!("Response time: {}ms\n", i * 10).as_bytes());
                data.extend_from_slice(b"sk-proj-abcdefghijklmnopqrstuvwxyz0123456 ");
                data.extend_from_slice(b"normal text content here ");
                data.extend_from_slice(b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U ");
            }
            detect_all(black_box(&data))
        });
    });
}

criterion_group!(benches, benchmark_realistic_mixed_data);
criterion_main!(benches);
