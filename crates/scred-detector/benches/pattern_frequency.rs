use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_detector::detect_all;
use std::collections::HashMap;

fn benchmark_pattern_frequency(c: &mut Criterion) {
    // Create realistic benchmark data
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

    c.bench_function("pattern_frequency_analysis", |b| {
        b.iter(|| {
            let result = detect_all(black_box(&data));

            // Analyze which patterns were found
            let mut pattern_counts: HashMap<u16, usize> = HashMap::new();
            for m in &result.matches {
                *pattern_counts.entry(m.pattern_id).or_insert(0) += 1;
            }

            // Print top patterns
            let mut vec: Vec<_> = pattern_counts.into_iter().collect();
            vec.sort_by_key(|&(_, count)| std::cmp::Reverse(count));

            for (pattern_id, count) in vec.iter().take(10) {
                println!("Pattern {}: {} matches", pattern_id, count);
            }

            result
        })
    });
}

criterion_group!(benches, benchmark_pattern_frequency);
criterion_main!(benches);
