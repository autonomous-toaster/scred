/// Task 4: Performance Baseline Benchmarking
/// 
/// Measures current 10-pattern redaction performance
/// Establishes baseline for FFI integration comparison
/// 
/// Benchmark Goals:
/// - Throughput (MB/s) per 64KB chunk
/// - Latency (ms) per redaction operation
/// - Memory per pattern
/// - FFI overhead estimate
/// - Acceptable performance with 270 patterns (< 10% regression)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use scred_redactor::{RedactionEngine, RedactionConfig};

/// Generate test data with embedded secrets
fn generate_test_data_with_secrets(size_kb: usize) -> String {
    let mut result = String::with_capacity(size_kb * 1024);
    let test_secrets = vec![
        "AWS Key: AKIAIOSFODNN7EXAMPLE",
        "GitHub: ghp_1234567890123456789012345678901234567890",
        "OpenAI: sk-proj-1234567890123456789012345678901234567890",
        "GitLab: glpat-1234567890123456",
        "JWT: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
        "Slack: xoxb-1234567890123-1234567890123-1234567890abcdefghij",
    ];
    
    let mut idx = 0;
    while result.len() < size_kb * 1024 {
        result.push_str(&format!(
            "Request #{}: {} | Content-Type: application/json | Authorization: Bearer token123\n",
            idx,
            test_secrets[idx % test_secrets.len()]
        ));
        idx += 1;
    }
    
    result.truncate(size_kb * 1024);
    result
}

/// Generate test data without secrets (control)
fn generate_test_data_no_secrets(size_kb: usize) -> String {
    let mut result = String::with_capacity(size_kb * 1024);
    let filler = "This is normal application log data without any secrets. Just regular text. ";
    
    while result.len() < size_kb * 1024 {
        result.push_str(filler);
    }
    
    result.truncate(size_kb * 1024);
    result
}

/// Benchmark: 64KB chunk redaction (minimal, realistic unit)
fn benchmark_64kb_chunk(c: &mut Criterion) {
    let engine = RedactionEngine::new(RedactionConfig::default());
    let test_data_with = black_box(generate_test_data_with_secrets(64));
    let test_data_without = black_box(generate_test_data_no_secrets(64));
    
    let mut group = c.benchmark_group("64kb_chunk");
    group.significance_level(0.1);
    group.sample_size(50);  // Reduce sample size for stability
    
    group.bench_function("with_patterns", |b| {
        b.iter(|| {
            engine.redact(&test_data_with)
        });
    });
    
    group.bench_function("without_patterns", |b| {
        b.iter(|| {
            engine.redact(&test_data_without)
        });
    });
    
    group.finish();
}

/// Benchmark: Various chunk sizes (throughput analysis)
fn benchmark_throughput_by_size(c: &mut Criterion) {
    let engine = RedactionEngine::new(RedactionConfig::default());
    let mut group = c.benchmark_group("throughput_by_size");
    group.significance_level(0.1);
    group.sample_size(20);
    
    for size_kb in [16, 64, 256, 1024].iter() {
        let test_data = black_box(generate_test_data_with_secrets(*size_kb));
        
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}kb", size_kb)),
            size_kb,
            |b, _| {
                b.iter(|| {
                    engine.redact(&test_data)
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark: Patterns per second (pattern matching rate)
fn benchmark_patterns_per_second(c: &mut Criterion) {
    let engine = RedactionEngine::new(RedactionConfig::default());
    let test_data = black_box(generate_test_data_with_secrets(256));  // 256KB with ~3400 lines
    
    let mut group = c.benchmark_group("patterns_per_second");
    group.significance_level(0.1);
    group.sample_size(30);
    
    group.bench_function("all_patterns", |b| {
        b.iter_batched(
            || engine.redact(&test_data),
            |result| {
                // Count patterns detected
                (result.matches.len(), result.warnings.len())
            },
            criterion::BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

/// Benchmark: Cold vs Warm Engine Performance
fn benchmark_engine_warmup(c: &mut Criterion) {
    let test_data = black_box(generate_test_data_with_secrets(64));
    let mut group = c.benchmark_group("engine_warmup");
    group.significance_level(0.1);
    group.sample_size(30);
    
    // Cold start: create new engine each time
    group.bench_function("cold_engine_creation", |b| {
        b.iter(|| {
            let engine = RedactionEngine::new(RedactionConfig::default());
            engine.redact(&test_data)
        });
    });
    
    // Warm: reuse engine
    let engine = RedactionEngine::new(RedactionConfig::default());
    group.bench_function("warm_engine_reuse", |b| {
        b.iter(|| {
            engine.redact(&test_data)
        });
    });
    
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(5))
        .warm_up_time(std::time::Duration::from_secs(2));
    targets = 
        benchmark_64kb_chunk,
        benchmark_throughput_by_size,
        benchmark_patterns_per_second,
        benchmark_engine_warmup
);

criterion_main!(benches);
