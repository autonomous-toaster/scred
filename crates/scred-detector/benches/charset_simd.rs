/// Benchmark SIMD vs scalar charset scanning
/// Run with: cargo bench --bench charset_simd [--features simd-accel]
/// 
/// SIMD mode (nightly + feature flag): Uses std::simd for 16-byte chunks
/// Scalar mode (stable): Falls back to 1-byte loop

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use scred_detector::simd_core::CharsetLut;
use scred_detector::simd_charset::scan_token_end_fast;

fn benchmark_charset_scanning(c: &mut Criterion) {
    let charset = CharsetLut::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_");
    
    let mut group = c.benchmark_group("charset_scan");
    group.sample_size(100); // More samples for better statistics
    
    // Test cases with different patterns
    let test_cases = vec![
        ("95% charset hits", {
            let mut v = Vec::new();
            for _ in 0..500 {
                v.extend_from_slice(b"abcdefghijklmnopqrstuvwxyz ");
            }
            v
        }),
        ("5% charset hits", {
            let mut v = Vec::new();
            for _ in 0..500 {
                v.extend_from_slice(b"!!!!!!!!!!!!!!!!!!!!!!!!!!!! ");
            }
            v
        }),
        ("aws keys mixed", {
            let mut v = Vec::new();
            for _ in 0..100 {
                v.extend_from_slice(b"AKIAIOSFODNN7EXAMPLE ");
                v.extend_from_slice(b"some_text ");
            }
            v
        }),
        ("github tokens", {
            let mut v = Vec::new();
            for _ in 0..100 {
                v.extend_from_slice(b"ghp_abcdefghijklmnopqrstuvwxyz0123456789ab ");
            }
            v
        }),
    ];

    for (name, data) in test_cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), &name, |b, _| {
            b.iter(|| {
                let mut total = 0usize;
                for chunk in data.chunks(1024) {
                    total += scan_token_end_fast(black_box(chunk), black_box(&charset), 0);
                }
                black_box(total)
            });
        });
    }

    group.finish();
}

fn benchmark_various_buffer_sizes(c: &mut Criterion) {
    let charset = CharsetLut::new(b"abcdefghijklmnopqrstuvwxyz");
    
    let mut group = c.benchmark_group("buffer_sizes");
    group.sample_size(50);
    
    for size in [16, 32, 64, 128, 256, 512, 1024, 4096].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(format!("{} bytes", size)), size, |b, &size| {
            let data: Vec<u8> = (0..size).map(|i| (b'a' + (i % 26) as u8)).collect();
            
            b.iter(|| {
                scan_token_end_fast(black_box(&data), black_box(&charset), 0)
            });
        });
    }
    
    group.finish();
}

fn benchmark_boundary_detection(c: &mut Criterion) {
    let charset = CharsetLut::new(b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_");
    
    let mut group = c.benchmark_group("boundary_detection");
    group.sample_size(50);
    
    // Test cases: where boundary is at different positions
    let boundary_positions = vec![
        ("boundary at 10%", {
            let mut v = vec![b'a'; 1024];
            v[102] = b'!';  // Break point
            v
        }),
        ("boundary at 50%", {
            let mut v = vec![b'a'; 1024];
            v[512] = b'!';  // Break point
            v
        }),
        ("boundary at 90%", {
            let mut v = vec![b'a'; 1024];
            v[922] = b'!';  // Break point
            v
        }),
        ("no boundary (all match)", {
            vec![b'a'; 1024]
        }),
    ];

    for (label, data) in boundary_positions {
        group.bench_with_input(BenchmarkId::from_parameter(label), &label, |b, _| {
            b.iter(|| {
                scan_token_end_fast(black_box(&data), black_box(&charset), 0)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_charset_scanning,
    benchmark_various_buffer_sizes,
    benchmark_boundary_detection
);
criterion_main!(benches);
