use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_detector::{detect_simple_prefix, detect_validation, detect_jwt};

fn benchmark_individual_methods(c: &mut Criterion) {
    let mut data = Vec::new();
    for i in 0..1000 {
        data.extend_from_slice(format!("IP: 192.168.1.{} ", i % 256).as_bytes());
        data.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
        data.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
        data.extend_from_slice(format!("Response time: {}ms\n", i * 10).as_bytes());
        data.extend_from_slice(b"sk-proj-abcdefghijklmnopqrstuvwxyz0123456 ");
        data.extend_from_slice(b"normal text content here ");
        data.extend_from_slice(b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U ");
    }
    
    c.bench_function("simple_prefix", |b| {
        b.iter(|| detect_simple_prefix(black_box(&data)))
    });
    
    c.bench_function("validation", |b| {
        b.iter(|| detect_validation(black_box(&data)))
    });
    
    c.bench_function("jwt", |b| {
        b.iter(|| detect_jwt(black_box(&data)))
    });
}

criterion_group!(benches, benchmark_individual_methods);
criterion_main!(benches);
