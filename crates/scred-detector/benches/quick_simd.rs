use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use scred_detector::simd_core::CharsetLut;
use scred_detector::simd_charset::scan_token_end_fast;

fn benchmark_quick_charset_scan(c: &mut Criterion) {
    let charset = CharsetLut::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_");
    
    let mut group = c.benchmark_group("quick_scan");
    group.sample_size(50); // Fewer samples for speed
    group.measurement_time(std::time::Duration::from_secs(2)); // Shorter per-benchmark time
    
    // AWS keys mixed - the key test case
    let data = {
        let mut v = Vec::new();
        for _ in 0..100 {
            v.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
            v.extend_from_slice(b"some_text ");
        }
        v
    };

    group.bench_function("aws_keys_mixed", |b| {
        b.iter(|| {
            let mut total = 0usize;
            for chunk in data.chunks(1024) {
                total += scan_token_end_fast(black_box(chunk), black_box(&charset), 0);
            }
            black_box(total)
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_quick_charset_scan);
criterion_main!(benches);
