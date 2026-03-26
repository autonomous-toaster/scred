use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_detector::{detect_all, redact_text};

fn benchmark_redaction(c: &mut Criterion) {
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
    
    // Pre-compute matches
    let matches = detect_all(&data);
    
    c.bench_function("detection_only", |b| {
        b.iter(|| detect_all(black_box(&data)))
    });
    
    c.bench_function("redaction_only", |b| {
        b.iter(|| redact_text(black_box(&data), black_box(&matches.matches)))
    });
    
    c.bench_function("detection_plus_redaction", |b| {
        b.iter(|| {
            let m = detect_all(black_box(&data));
            redact_text(black_box(&data), black_box(&m.matches))
        })
    });
}

criterion_group!(benches, benchmark_redaction);
criterion_main!(benches);
