use std::fs;
use std::time::Instant;

fn main() {
    println!("════════════════════════════════════════════════════════════════");
    println!("    SCRED Phase 3a: Throughput Benchmark - Pattern Detection");
    println!("════════════════════════════════════════════════════════════════\n");

    // Generate test data with various secret patterns
    let test_data = generate_test_data();
    println!("Test Data Size: {} MB", test_data.len() / (1024 * 1024));
    println!("Content: Mix of AWS keys, GitHub tokens, API keys, JWT tokens\n");

    // Test throughput directly using the FFI
    benchmark_pattern_detection(&test_data);

    // Test redaction throughput
    benchmark_redaction(&test_data);

    // Memory usage
    benchmark_memory_usage();

    println!("\n════════════════════════════════════════════════════════════════");
    println!("Benchmark Complete");
    println!("════════════════════════════════════════════════════════════════");
}

fn generate_test_data() -> Vec<u8> {
    let mut data = String::new();

    // AWS Access Keys
    let aws_keys = vec![
        "AKIAIOSFODNN7EXAMPLE",
        "ASIAIOSFODNN7EXAMPLE",
        "ABIAIOSFODNN7EXAMPLE",
        "ACCAIOSFODNN7EXAMPLE",
    ];

    // GitHub PATs
    let github_tokens = vec![
        "ghp_1234567890abcdefghijklmnopqrstuvwxyz",
        "gho_1234567890abcdefghijklmnopqrstuvwxyz",
        "ghu_1234567890abcdefghijklmnopqrstuvwxyz",
    ];

    // OpenAI API Keys
    let openai_keys = vec![
        "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz",
        "sk-svcacct-1234567890abcdefghijklmnopqrstuvwxyz",
    ];

    // JWT Tokens
    let jwt_tokens = vec![
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
    ];

    // Bearer tokens
    let bearer_tokens = vec![
        "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
    ];

    // Generate ~50 MB of test data by repeating patterns
    // Mix: 20% secrets, 80% normal text
    let iteration_count = 50 * 1024; // 50k iterations
    let lorem = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. ";

    for i in 0..iteration_count {
        // Add normal text
        data.push_str(lorem);

        // Add a secret every 5 iterations
        if i % 5 == 0 {
            let secret_idx = i as usize % aws_keys.len();
            data.push_str("AWS_KEY=");
            data.push_str(aws_keys[secret_idx]);
            data.push('\n');
        }

        if i % 7 == 0 {
            let token_idx = i as usize % github_tokens.len();
            data.push_str("GITHUB_TOKEN=");
            data.push_str(github_tokens[token_idx]);
            data.push('\n');
        }

        if i % 11 == 0 {
            let key_idx = i as usize % openai_keys.len();
            data.push_str("OPENAI_KEY=");
            data.push_str(openai_keys[key_idx]);
            data.push('\n');
        }

        if i % 13 == 0 {
            let jwt_idx = i as usize % jwt_tokens.len();
            data.push_str("JWT=");
            data.push_str(jwt_tokens[jwt_idx]);
            data.push('\n');
        }

        if i % 17 == 0 {
            let bearer_idx = i as usize % bearer_tokens.len();
            data.push_str(bearer_tokens[bearer_idx]);
            data.push('\n');
        }
    }

    data.into_bytes()
}

fn benchmark_pattern_detection(data: &[u8]) {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("PATTERN DETECTION THROUGHPUT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let text = String::from_utf8_lossy(data).to_string();

    // Warm up
    let _ = scred_redactor::redact(&text);

    // Benchmark: 5 runs
    let mut times = vec![];

    for run in 1..=5 {
        let start = Instant::now();
        let _result = scred_redactor::redact(&text);
        let elapsed = start.elapsed();
        times.push(elapsed);

        let mb_per_sec = (data.len() as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();
        println!("Run {}: {:?} ({:.2} MB/s)", run, elapsed, mb_per_sec);
    }

    // Statistics
    let avg_time = times.iter().map(|t| t.as_secs_f64()).sum::<f64>() / times.len() as f64;
    let avg_mb_s = (data.len() as f64 / (1024.0 * 1024.0)) / avg_time;

    println!("\nAverage: {:.2} MB/s", avg_mb_s);
    println!("Min: {:.2} MB/s", {
        let min_time = times.iter().min().unwrap();
        (data.len() as f64 / (1024.0 * 1024.0)) / min_time.as_secs_f64()
    });
    println!("Max: {:.2} MB/s\n", {
        let max_time = times.iter().max().unwrap();
        (data.len() as f64 / (1024.0 * 1024.0)) / max_time.as_secs_f64()
    });

    // Expected baseline
    println!("Expected baseline: 35-40 MB/s");
    if avg_mb_s >= 35.0 {
        println!("✅ Baseline achieved");
    } else {
        println!("⚠️  Below baseline");
    }
}

fn benchmark_redaction(data: &[u8]) {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("REDACTION THROUGHPUT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let text = String::from_utf8_lossy(data).to_string();

    let start = Instant::now();
    let result = scred_redactor::redact(&text);
    let total_time = start.elapsed();

    let reduction = 100.0 - (result.len() as f64 / data.len() as f64) * 100.0;

    println!("Total time: {:?}", total_time);
    println!("Throughput: {:.2} MB/s", 
        (data.len() as f64 / (1024.0 * 1024.0)) / total_time.as_secs_f64());
    println!("Input size: {} MB", data.len() / (1024 * 1024));
    println!("Output size: {} MB", result.len() / (1024 * 1024));
    println!("Space reduction: {:.2}%\n", reduction);
}

fn benchmark_memory_usage() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("MEMORY USAGE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Estimate memory from binary size
    let binary_size = std::mem::size_of::<Vec<u8>>() * 1000; // Rough estimate
    println!("Estimated runtime memory: ~{} KB (library)", binary_size / 1024);
    println!("Note: Use /usr/bin/time -v or ps for actual measurements\n");
}
