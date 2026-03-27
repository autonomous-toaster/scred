use scred_readctor_framering::redact_text;
use std::time::Instant;

fn main() {
    println!("════════════════════════════════════════════════════════════════");
    println!("    SCRED Phase 3a: Throughput Benchmark - Pattern Detection");
    println!("════════════════════════════════════════════════════════════════\n");

    // Generate test data
    let test_data = generate_test_data();
    let data_size_bytes = test_data.len();
    let data_size_mb = data_size_bytes / (1024 * 1024);
    println!("Test Data Size: {} MB ({} bytes)", data_size_mb, data_size_bytes);
    println!("Content: Mix of AWS keys, GitHub tokens, API keys, JWT tokens\n");

    // Pattern detection benchmark
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("PATTERN DETECTION THROUGHPUT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut times = vec![];

    for run in 1..=5 {
        let start = Instant::now();
        let _result = redact_text(&test_data);
        let elapsed = start.elapsed();
        times.push(elapsed);

        let mb_per_sec = if elapsed.as_secs_f64() > 0.0 {
            data_size_mb as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        println!("Run {}: {:?} ({:.2} MB/s)", run, elapsed, mb_per_sec);
    }

    // Statistics
    let avg_time = times.iter().map(|t| t.as_secs_f64()).sum::<f64>() / times.len() as f64;
    let avg_mb_s = data_size_mb as f64 / avg_time;
    let min_mb_s = data_size_mb as f64 / times.iter().max().unwrap().as_secs_f64();
    let max_mb_s = data_size_mb as f64 / times.iter().min().unwrap().as_secs_f64();

    println!("\n┌─ Summary ─────────────────────────────────────────────────┐");
    println!("│ Average: {:.2} MB/s", avg_mb_s);
    println!("│ Min:     {:.2} MB/s", min_mb_s);
    println!("│ Max:     {:.2} MB/s", max_mb_s);
    println!("│ Target:  65-75 MB/s (currently 35-40 MB/s baseline)", );
    println!("└───────────────────────────────────────────────────────────┘\n");

    if avg_mb_s >= 35.0 {
        println!("✅ Baseline confirmed ({:.2} MB/s)", avg_mb_s);
    } else {
        println!("⚠️  Below expected baseline");
    }

    println!("════════════════════════════════════════════════════════════════");
}

fn generate_test_data() -> String {
    let mut data = String::new();

    let aws_keys = ["AKIAIOSFODNN7EXAMPLE",
        "ASIAIOSFODNN7EXAMPLE"];

    let github_tokens = ["ghp_1234567890abcdefghijklmnopqrstuvwxyz",
        "gho_1234567890abcdefghijklmnopqrstuvwxyz"];

    let openai_keys = ["sk-proj-1234567890abcdefghijklmnopqrstuvwxyz"];

    let jwt_tokens = ["eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U"];

    let lorem = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. ";

    // Generate ~50 MB
    let iteration_count = 50 * 1024;

    for i in 0..iteration_count {
        data.push_str(lorem);

        if i % 5 == 0 {
            data.push_str("AWS_KEY=");
            data.push_str(aws_keys[i as usize % aws_keys.len()]);
            data.push('\n');
        }

        if i % 7 == 0 {
            data.push_str("GITHUB_TOKEN=");
            data.push_str(github_tokens[i as usize % github_tokens.len()]);
            data.push('\n');
        }

        if i % 11 == 0 {
            data.push_str("OPENAI_KEY=");
            data.push_str(openai_keys[i as usize % openai_keys.len()]);
            data.push('\n');
        }

        if i % 13 == 0 {
            data.push_str("JWT=");
            data.push_str(jwt_tokens[i as usize % jwt_tokens.len()]);
            data.push('\n');
        }
    }

    data
}
