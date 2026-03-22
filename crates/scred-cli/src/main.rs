use std::io::{self, Read, Write};
use std::env;
use std::time::Instant;

use scred_redactor::{get_all_patterns, analyzer::ZigAnalyzer};

mod env_mode;
mod env_detection;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Parse flags
    let verbose = args.iter().any(|arg| arg == "-v" || arg == "--verbose");
    let env_mode_forced = args.iter().any(|arg| arg == "--env-mode" || arg == "--env");
    let text_mode_forced = args.iter().any(|arg| arg == "--text-mode");
    let auto_detect_enabled = !args.iter().any(|arg| arg == "--auto-detect=off");
    let detect_only = args.iter().any(|arg| arg == "--detect-only");
    
    // Handle special commands
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" | "help" => {
                print_help();
                return;
            }
            "--list-patterns" => {
                list_patterns();
                return;
            }
            "--describe" => {
                if args.len() > 2 {
                    describe_pattern(&args[2]);
                } else {
                    eprintln!("Usage: scred --describe <pattern-name>");
                    std::process::exit(1);
                }
                return;
            }
            "--version" => {
                println!("SCRED 2.0.0 - Secret Redaction Engine");
                return;
            }
            _ => {}
        }
    }

    // Determine which mode to use
    let use_env_mode = if text_mode_forced {
        false
    } else if env_mode_forced {
        true
    } else if auto_detect_enabled {
        // Auto-detect based on first chunk of input
        run_with_auto_detect(verbose, detect_only)
    } else {
        false
    };

    if use_env_mode {
        run_env_redacting_stream(verbose);
    } else {
        run_redacting_stream(verbose);
    }
}

fn print_help() {
    println!("SCRED - Secret Redaction Engine v2.0.0");
    println!();
    println!("Usage: scred [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -v, --verbose           Show statistics and detected patterns");
    println!("  --env-mode, --env       Force environment variable mode");
    println!("  --text-mode             Force text/pattern mode");
    println!("  --auto-detect=off       Disable auto-detection");
    println!("  --detect-only           Show detection result and exit (debug)");
    println!("  --list-patterns         List all 53 secret detection patterns");
    println!("  --describe <NAME>       Show details for a specific pattern");
    println!("  --version               Show version information");
    println!("  --help, -h              Show this help message");
    println!();
    println!("Auto-Detection:");
    println!("  scred automatically detects environment variable format");
    println!("  when input contains KEY=VALUE patterns with secret keywords.");
    println!();
    println!("Examples:");
    println!("  env | scred > redacted_env.txt               # Auto-detects env-mode");
    println!("  scred < ~/.aws/credentials > redacted_creds # Auto-detects env-mode");
    println!("  cat secrets.txt | scred > redacted.txt      # Uses pattern mode");
    println!("  env | scred -v 2>&1                         # Shows detection decision");
    println!("  env | scred --detect-only                   # Debug: show detection score");
    println!();
}

fn list_patterns() {
    let patterns = get_all_patterns();
    println!("SCRED Pattern Library - {} patterns\n", patterns.len());
    println!("{:<30} {:<15} {}", "Pattern Name", "Prefix", "Min Length");
    println!("{}", "=".repeat(70));
    
    for pattern in patterns {
        println!("{:<30} {:<15} {}", pattern.name, pattern.prefix, pattern.min_len);
    }
}

fn describe_pattern(name: &str) {
    let patterns = get_all_patterns();
    
    if let Some(pattern) = patterns.iter().find(|p| p.name == name) {
        println!("Pattern: {}", pattern.name);
        println!("Prefix: {}", pattern.prefix);
        println!("Min Length: {}", pattern.min_len);
    } else {
        eprintln!("Pattern '{}' not found", name);
        eprintln!("\nUse 'scred --list-patterns' to see all patterns");
        std::process::exit(1);
    }
}

fn run_redacting_stream(verbose: bool) {
    let start = Instant::now();

    // Stream input in 64KB chunks
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut chunk = vec![0u8; CHUNK_SIZE];
    let mut total_read = 0;
    let mut total_written = 0;

    loop {
        match io::stdin().read(&mut chunk) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let input_str = String::from_utf8_lossy(&chunk[..n]);
                let (redacted, pattern_count) = ZigAnalyzer::redact_optimized(&input_str);
                
                io::stdout().write_all(redacted.as_bytes()).ok();
                
                total_read += n;
                total_written += redacted.len();
                
                if verbose {
                    eprintln!("[chunk: {} bytes → {} patterns]", n, pattern_count);
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                std::process::exit(1);
            }
        }
    }

    if verbose {
        let elapsed = start.elapsed();
        let throughput = if elapsed.as_secs_f64() > 0.0 {
            total_read as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64()
        } else {
            0.0
        };
        eprintln!("\n[redacting-stream]");
        eprintln!("  Bytes: {} → {} (char-preserved)", total_read, total_written);
        eprintln!("  Time: {:.2}s", elapsed.as_secs_f64());
        eprintln!("  Throughput: {:.1} MB/s", throughput);
    }
}

fn run_env_redacting_stream(verbose: bool) {
    let start = Instant::now();

    // Stream input in 64KB chunks
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut chunk = vec![0u8; CHUNK_SIZE];
    let mut total_read = 0;
    let mut total_written = 0;

    loop {
        match io::stdin().read(&mut chunk) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let input_str = String::from_utf8_lossy(&chunk[..n]);
                
                // Process line by line with env-aware redaction
                let mut output = String::new();
                for line in input_str.lines() {
                    let redacted = env_mode::redact_env_line(line, |s| {
                        ZigAnalyzer::redact_optimized(s).0
                    });
                    output.push_str(&redacted);
                    output.push('\n');
                }
                
                io::stdout().write_all(output.as_bytes()).ok();
                
                total_read += n;
                total_written += output.len();
                
                if verbose {
                    eprintln!("[env-chunk: {} bytes → {}", n, output.len());
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                std::process::exit(1);
            }
        }
    }

    if verbose {
        let elapsed = start.elapsed();
        let throughput = if elapsed.as_secs_f64() > 0.0 {
            total_read as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64()
        } else {
            0.0
        };
        eprintln!("\n[redacting-env-stream]");
        eprintln!("  Bytes: {} → {}", total_read, total_written);
        eprintln!("  Time: {:.2}s", elapsed.as_secs_f64());
        eprintln!("  Throughput: {:.1} MB/s", throughput);
    }
}

fn run_with_auto_detect(verbose: bool, detect_only: bool) -> bool {
    let start = Instant::now();
    
    // Read first 512 bytes for detection (much faster than 4KB)
    // Typical env files are 0.1-10 KB, so 512B is sufficient for first few lines
    const DETECTION_BUFFER_SIZE: usize = 512;
    let mut buffer = vec![0u8; DETECTION_BUFFER_SIZE];
    
    let n = match io::stdin().read(&mut buffer) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error reading input: {}", e);
            std::process::exit(1);
        }
    };
    
    buffer.truncate(n);
    
    // Detect format
    let detection = env_detection::detect_format(&buffer);
    
    if verbose || detect_only {
        eprintln!("[auto-detect: {} (score: {:.2})]", detection.reason, detection.score);
    }
    
    if detect_only {
        println!("Detection: {:?}", detection.mode);
        println!("Score: {:.2}", detection.score);
        println!("Reason: {}", detection.reason);
        std::process::exit(0);
    }
    
    // Decide which mode based on detection
    let use_env_mode = detection.mode == env_detection::DetectionMode::EnvFormat;
    
    // Process the detected chunk
    if use_env_mode {
        process_env_chunk_and_stream(&buffer, verbose);
    } else {
        process_text_chunk_and_stream(&buffer, verbose);
    }
    
    use_env_mode
}

fn process_env_chunk_and_stream(initial_buffer: &[u8], verbose: bool) {
    let start = Instant::now();
    let mut total_read = initial_buffer.len();
    let mut total_written = 0;
    
    // Process the initial buffer chunk
    let input_str = String::from_utf8_lossy(initial_buffer);
    for line in input_str.lines() {
        let redacted = env_mode::redact_env_line(line, |s| {
            ZigAnalyzer::redact_optimized(s).0
        });
        io::stdout().write_all(redacted.as_bytes()).ok();
        io::stdout().write_all(b"\n").ok();
        total_written += redacted.len() + 1;
    }
    
    // Continue with remaining stream
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut chunk = vec![0u8; CHUNK_SIZE];
    
    loop {
        match io::stdin().read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let input_str = String::from_utf8_lossy(&chunk[..n]);
                for line in input_str.lines() {
                    let redacted = env_mode::redact_env_line(line, |s| {
                        ZigAnalyzer::redact_optimized(s).0
                    });
                    io::stdout().write_all(redacted.as_bytes()).ok();
                    io::stdout().write_all(b"\n").ok();
                    total_written += redacted.len() + 1;
                }
                total_read += n;
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    if verbose {
        let elapsed = start.elapsed();
        let throughput = if elapsed.as_secs_f64() > 0.0 {
            total_read as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64()
        } else {
            0.0
        };
        eprintln!("\n[redacting-env-stream (auto-detected)]");
        eprintln!("  Bytes: {} → {}", total_read, total_written);
        eprintln!("  Time: {:.2}s", elapsed.as_secs_f64());
        eprintln!("  Throughput: {:.1} MB/s", throughput);
    }
}

fn process_text_chunk_and_stream(initial_buffer: &[u8], verbose: bool) {
    let start = Instant::now();
    let mut total_read = initial_buffer.len();
    let mut total_written = 0;
    
    // Process the initial buffer chunk
    let input_str = String::from_utf8_lossy(initial_buffer);
    let (redacted, _) = ZigAnalyzer::redact_optimized(&input_str);
    io::stdout().write_all(redacted.as_bytes()).ok();
    total_written += redacted.len();
    
    // Continue with remaining stream
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut chunk = vec![0u8; CHUNK_SIZE];
    
    loop {
        match io::stdin().read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let input_str = String::from_utf8_lossy(&chunk[..n]);
                let (redacted, _) = ZigAnalyzer::redact_optimized(&input_str);
                io::stdout().write_all(redacted.as_bytes()).ok();
                total_read += n;
                total_written += redacted.len();
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    if verbose {
        let elapsed = start.elapsed();
        let throughput = if elapsed.as_secs_f64() > 0.0 {
            total_read as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64()
        } else {
            0.0
        };
        eprintln!("\n[redacting-stream (auto-detected)]");
        eprintln!("  Bytes: {} → {} (char-preserved)", total_read, total_written);
        eprintln!("  Time: {:.2}s", elapsed.as_secs_f64());
        eprintln!("  Throughput: {:.1} MB/s", throughput);
    }
}
