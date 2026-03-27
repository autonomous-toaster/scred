use scred_readctor_framering::redact_text;
use std::sync::Arc;
use std::thread;

#[test]
fn test_concurrent_redaction_no_crashes() {
    let text = Arc::new(
        "AWS_KEY=AKIAIOSFODNN7EXAMPLE\n\
         GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwxyz\n\
         OPENAI_KEY=sk-proj-1234567890abcdefghijklmnopqrstuvwxyz"
            .to_string(),
    );

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let t = Arc::clone(&text);
            thread::spawn(move || {
                // Should not crash or deadlock
                let _result = redact_text(&t);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_concurrent_redaction_same_result() {
    let text = Arc::new(
        "AWS: AKIAIOSFODNN7EXAMPLE, GitHub: ghp_1234567890abcdefghijklmnopqrstuvwxyz".to_string(),
    );

    let mut results: Vec<String> = vec![];
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let t = Arc::clone(&text);
            thread::spawn(move || redact_text(&t))
        })
        .collect();

    for handle in handles {
        results.push(handle.join().unwrap());
    }

    // All results should be identical
    for window in results.windows(2) {
        assert_eq!(
            window[0], window[1],
            "Concurrent redaction produced different results"
        );
    }
}

#[test]
fn test_concurrent_redaction_under_load() {
    let text = Arc::new(generate_large_test_data());
    let num_threads = 16;
    let iterations_per_thread = 10;

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let t = Arc::clone(&text);
            thread::spawn(move || {
                for _ in 0..iterations_per_thread {
                    let _result = redact_text(&t);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

fn generate_large_test_data() -> String {
    let mut data = String::new();
    let patterns = vec![
        "AKIAIOSFODNN7EXAMPLE",
        "ghp_1234567890abcdefghijklmnopqrstuvwxyz",
        "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz",
    ];

    for _ in 0..1000 {
        data.push_str("Lorem ipsum dolor sit amet. AWS: ");
        data.push_str(patterns[0]);
        data.push('\n');
    }

    data
}
