use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[test]
fn test_all_patterns_redaction() {
    // Look for test_cases.csv in multiple locations
    let mut csv_path = PathBuf::from("test_cases.csv");
    
    if !csv_path.exists() {
        csv_path = PathBuf::from("../../test_cases.csv");
    }
    
    if !csv_path.exists() {
        csv_path = PathBuf::from("../../../test_cases.csv");
    }
    
    if !csv_path.exists() {
        eprintln!("⚠️  test_cases.csv not found at {:?}, skipping integration test", csv_path);
        return;
    }

    let file = File::open(&csv_path).expect("Failed to open test_cases.csv");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    // Skip header
    let _ = lines.next();
    
    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut failed_tests = Vec::new();

    use scred_redactor::analyzer::ZigAnalyzer;

    for (line_num, line) in lines.enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 {
            continue;
        }

        let input = parts[0].trim();
        let expected_pattern = parts[1].trim();
        let description = if parts.len() > 2 { parts[2].trim() } else { "" };

        total_tests += 1;

        let (redacted_output, _events) = ZigAnalyzer::redact_optimized(input);

        // For "no_secrets" pattern, output should be same as input
        if expected_pattern == "no_secrets" {
            if redacted_output == input {
                passed_tests += 1;
            } else {
                failed_tests.push((
                    line_num + 2, // +2 for header and 1-indexing
                    expected_pattern.to_string(),
                    input.to_string(),
                    format!("Expected no redaction, but got: {}", redacted_output),
                ));
            }
        } else {
            // For all other patterns, output should be DIFFERENT (something was redacted)
            if redacted_output != input {
                passed_tests += 1;
            } else {
                failed_tests.push((
                    line_num + 2,
                    expected_pattern.to_string(),
                    input.to_string(),
                    format!("NOT REDACTED - pattern {} not detected", expected_pattern),
                ));
            }
        }
    }

    println!("\n📋 Pattern Redaction Test Results:");
    println!("  Total tests: {}", total_tests);
    println!("  Passed: {} ✅", passed_tests);
    println!("  Failed: {} ❌", failed_tests.len());

    if !failed_tests.is_empty() {
        println!("\n❌ Failed Tests (first 30):");
        for (line_num, pattern, input, message) in failed_tests.iter().take(30) {
            println!("\n  Line {}: Pattern '{}'", line_num, pattern);
            println!("    Input:  {}", input);
            println!("    Issue:  {}", message);
        }
        
        if failed_tests.len() > 30 {
            println!("\n  ... and {} more failures", failed_tests.len() - 30);
        }
        
        panic!("test_cases.csv: {}/{} tests failed", failed_tests.len(), total_tests);
    }
    
    println!("✅ All {} test cases passed!", total_tests);
}
