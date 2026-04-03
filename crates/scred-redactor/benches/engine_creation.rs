use scred_redactor::{RedactionConfig, RedactionEngine};
use std::time::Instant;

fn main() {
    println!("Testing engine creation time (all 244 patterns)\n");

    // Warmup
    for _ in 0..3 {
        let _ = RedactionEngine::new(RedactionConfig::default());
    }

    // Benchmark engine creation
    let iterations = 100;
    let start = Instant::now();
    for i in 0..iterations {
        let _ = RedactionEngine::new(RedactionConfig::default());
        if i == 0 {
            let first_elapsed = start.elapsed();
            println!(
                "First engine creation: {:.2} ms",
                first_elapsed.as_secs_f64() * 1000.0
            );
        }
    }
    let total_elapsed = start.elapsed();

    let avg_ms = total_elapsed.as_secs_f64() * 1000.0 / iterations as f64;
    println!(
        "Average engine creation ({} iterations): {:.2} ms",
        iterations, avg_ms
    );
    println!(
        "Throughput: {:.1} engines/sec",
        iterations as f64 / total_elapsed.as_secs_f64()
    );
}
