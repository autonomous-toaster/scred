use std::io::{self, Read, Write};
use std::env;
use std::time::Instant;
use std::sync::Arc;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;

use scred_redactor::{get_all_patterns, RedactionEngine, RedactionConfig};
use scred_http::{ConfigurableEngine, PatternSelector, env_detection};
use scred_config::ConfigLoader;
use tracing::{info, debug};

/// Check if stdin is connected to a terminal (TTY)
/// Returns true if stdin is a terminal, false if piped/redirected
#[cfg(unix)]
fn stdin_is_tty() -> bool {
    use libc::isatty;
    unsafe {
        isatty(io::stdin().as_raw_fd()) == 1
    }
}

#[cfg(not(unix))]
fn stdin_is_tty() -> bool {
    // On non-Unix platforms, assume it's not a TTY
    false
}

mod env_mode;

/// Extract flag value from command line arguments
/// E.g., "--detect CRITICAL" returns Some("CRITICAL")
/// E.g., "--detect=CRITICAL" returns Some("CRITICAL")
fn extract_flag_value(args: &[String], flag: &str) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        } else if arg.starts_with(&format!("{}=", flag)) {
            return Some(arg.split('=').nth(1).unwrap_or("").to_string());
        }
    }
    None
}

/// Parse pattern selectors from CLI flags and environment variables
/// Precedence: CLI flags > environment variables > defaults
/// EXITS with error code 1 on invalid selector
fn parse_pattern_selectors(
    detect_flag: Option<&str>,
    redact_flag: Option<&str>,
    _verbose: bool,
) -> (PatternSelector, PatternSelector) {
    // Get environment variable values
    let detect_env = env::var("SCRED_DETECT_PATTERNS").ok();
    let redact_env = env::var("SCRED_REDACT_PATTERNS").ok();

    // CLI flags take precedence over env vars
    // Detect ALL patterns by default (for logging visibility)
    let detect_str = detect_flag
        .or(detect_env.as_deref())
        .unwrap_or("ALL");
    
    // Redact conservatively: only CRITICAL and API_KEYS by default
    // PATTERNS tier (JWT, Bearer, BasicAuth) excluded to reduce log noise
    // Users can explicitly enable: --redact CRITICAL,API_KEYS,PATTERNS
    let redact_str = redact_flag
        .or(redact_env.as_deref())
        .unwrap_or("CRITICAL,API_KEYS");

    // Parse selectors - EXIT on error instead of fallback
    let detect_selector = match PatternSelector::from_str(detect_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR: Invalid SCRED_DETECT_PATTERNS value: '{}'", detect_str);
            eprintln!("Reason: {}", e);
            eprintln!("\nValid tier names: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS");
            eprintln!("Valid patterns: aws-*, github-*, sk-*, etc.");
            eprintln!("Valid regex: regex:^(aws|github)");
            eprintln!("Examples:");
            eprintln!("  scred --detect CRITICAL");
            eprintln!("  scred --detect CRITICAL,API_KEYS");
            eprintln!("  scred --detect 'regex:^sk-'");
            std::process::exit(1);
        }
    };

    let redact_selector = match PatternSelector::from_str(redact_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR: Invalid SCRED_REDACT_PATTERNS value: '{}'", redact_str);
            eprintln!("Reason: {}", e);
            eprintln!("\nValid tier names: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS");
            eprintln!("Valid patterns: aws-*, github-*, sk-*, etc.");
            eprintln!("Valid regex: regex:^(aws|github)");
            eprintln!("Examples:");
            eprintln!("  scred --redact CRITICAL");
            eprintln!("  scred --redact CRITICAL,API_KEYS");
            eprintln!("  scred --redact 'regex:^sk-'");
            std::process::exit(1);
        }
    };

    info!("[cli-config] Detect: {}", detect_selector.description());
    info!("[cli-config] Redact: {}", redact_selector.description());

    (detect_selector, redact_selector)
}

/// Load CLI configuration from file, with fallback to defaults
fn load_cli_config() -> (PatternSelector, PatternSelector) {
    // Try loading from config file
    if let Ok(file_config) = ConfigLoader::load() {
        if let Some(cli_cfg) = file_config.scred_cli {
            let detect_str = cli_cfg.patterns.detect.join(",");
            let redact_str = cli_cfg.patterns.redact.join(",");
            
            let detect = PatternSelector::from_str(&detect_str)
                .unwrap_or_else(|_| PatternSelector::default_detect());
            let redact = PatternSelector::from_str(&redact_str)
                .unwrap_or_else(|_| PatternSelector::default_redact());
            
            info!("[cli-config] Loaded from config file");
            return (detect, redact);
        }
    }
    
    // Fallback to defaults
    (PatternSelector::default_detect(), PatternSelector::default_redact())
}

/// List available pattern tiers
fn list_tiers_command() {
    println!("SCRED Pattern Tiers");
    println!();
    println!("{:<20} {:<10} Redact by Default", "Tier", "Risk");
    println!("{}", "=".repeat(50));
    
    let tiers = [
        ("CRITICAL", "95%", "YES"),
        ("API_KEYS", "80%", "YES"),
        ("INFRASTRUCTURE", "60%", "NO"),
        ("SERVICES", "40%", "NO"),
        ("PATTERNS", "30%", "NO"),
    ];

    for (tier, risk, redact) in &tiers {
        println!("{:<20} {:<10} {}", tier, risk, redact);
    }

    println!();
    println!("Usage:");
    println!("  scred --detect CRITICAL,API_KEYS < input.txt");
    println!("  scred --redact CRITICAL < input.txt");
    println!("  SCRED_DETECT_PATTERNS=all scred < input.txt");
    println!("  scred --list-tiers");
}

fn main() {
    // Initialize logging - DISABLED FOR DEBUGGING
    
    // let log_level = if env::var("SCRED_DEBUG").is_ok() {
    //     "debug"
    // } else if env::var("SCRED_TRACE").is_ok() {
    //     "trace"
    // } else {
    //     "warn"
    // };
    
    // tracing_subscriber::fmt()
    //     .with_max_level(log_level.parse().unwrap_or(tracing::Level::WARN))
    //     .with_target(false)
    //     .with_thread_ids(false)
    //     .with_file(false)
    //     .with_line_number(false)
    //     .init();

    let args: Vec<String> = env::args().collect();
    
    // Parse flags
    let verbose = args.iter().any(|arg| arg == "-v" || arg == "--verbose");
    let env_mode_forced = args.iter().any(|arg| arg == "--env-mode" || arg == "--env");
    let text_mode_forced = args.iter().any(|arg| arg == "--text-mode");
    let auto_detect_enabled = !args.iter().any(|arg| arg == "--auto-detect=off");
    let detect_only_flag = args.iter().any(|arg| arg == "--detect-only");
    
    // New pattern tier flags
    let detect_flag = extract_flag_value(&args, "--detect");
    let redact_flag = extract_flag_value(&args, "--redact");
    
    // Handle special commands
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" | "help" => {
                print_help();
                return;
            }
            "--list-patterns" => {
                let filter_type = extract_flag_value(&args, "--filter-type");
                list_patterns(filter_type.as_deref());
                return;
            }
            "--list-tiers" => {
                list_tiers_command();
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

    // Parse pattern selectors from flags and env vars
    let (detect_selector, redact_selector) = parse_pattern_selectors(
        detect_flag.as_deref(),
        redact_flag.as_deref(),
        verbose,
    );

    // Determine which mode to use
    // FIX: Skip auto-detect if stdin is piped (not a TTY) to avoid blocking on read()
    // Piped input hangs on first read() because Unix pipe doesn't send EOF until
    // parent shell closes write-end, but parent waits for child to exit (deadlock).
    // Solution: When stdin is piped, always use text mode (not auto-detect).
    let use_env_mode = if text_mode_forced {
        debug!("[cli-mode] Text mode forced");
        false
    } else if env_mode_forced {
        debug!("[cli-mode] Env mode forced");
        true
    } else if auto_detect_enabled && stdin_is_tty() {
        // Only auto-detect if stdin is actually a terminal
        // Skip for piped input to avoid deadlock
        debug!("[cli-mode] Auto-detecting (stdin is TTY)");
        run_with_auto_detect(
            verbose,
            detect_only_flag,
            &detect_selector,
            &redact_selector,
        )
    } else {
        debug!("[cli-mode] Using text mode (stdin is piped)");
        false
    };
    
    if use_env_mode {
        run_env_redacting_stream(verbose, &detect_selector, &redact_selector);
    } else {
        run_redacting_stream(verbose, &detect_selector, &redact_selector);
    }
}

fn print_help() {
    println!("SCRED - Secret Redaction Engine v2.0.0");
    println!();
    println!("Usage: scred [OPTIONS]");
    println!();
    println!("Pattern Detection Options:");
    println!("  --detect <TYPES>        Which patterns to detect: fast, structured, regex, all");
    println!("                          (default: fast) - Controls performance");
    println!("  --redact <TYPES>        Which patterns to redact: same as --detect");
    println!("                          (default: fast) - Conservative by default");
    println!();
    println!("Mode Options:");
    println!("  -v, --verbose           Show statistics and detected patterns");
    println!("  --env-mode, --env       Force environment variable mode");
    println!("  --text-mode             Force text/pattern mode");
    println!("  --auto-detect=off       Disable auto-detection");
    println!("  --detect-only           Show detection result and exit (debug)");
    println!();
    println!("Information Options:");
    println!("  --list-patterns         List all secret detection patterns");
    println!("  --filter-type <TYPE>    Filter patterns by type: fast, structured, regex");
    println!("  --describe <NAME>       Show details for a specific pattern");
    println!("  --version               Show version information");
    println!("  --help, -h              Show this help message");
    println!();
    println!("Pattern Type Reference:");
    println!("  fast                    FastPrefix (71 patterns, <5ms) - production");
    println!("  structured              StructuredFormat (1 pattern) - JWT validation");
    println!("  regex                   RegexBased (198 patterns, ~1000ms) - comprehensive");
    println!("  all                     All 270 patterns - development only");
    println!();
    println!();
    println!("Environment Variables:");
    println!("  SCRED_DETECT_PATTERNS   Which patterns to detect (same format as --detect, default: ALL)");
    println!("  SCRED_REDACT_PATTERNS   Which patterns to redact (same format as --redact, default: CRITICAL,API_KEYS)");
    println!();
    println!("Usage Examples:");
    println!("  env | scred > redacted_env.txt                     # Auto-detects env-mode, detects ALL patterns");
    println!("  scred < ~/.aws/credentials > redacted_creds        # Auto-detects env-mode, detects ALL patterns");
    println!("  cat secrets.txt | scred > redacted.txt             # Uses pattern mode, detects ALL patterns");
    println!("  scred --detect CRITICAL --redact CRITICAL < file   # Show/redact only CRITICAL");
    println!("  SCRED_DETECT_PATTERNS=CRITICAL scred < file.txt    # Detect only CRITICAL patterns");
    println!("  scred --list-tiers                                 # Show available tiers");
    println!("  env | scred -v 2>&1                                # Shows detection decision");
    println!();
}

fn list_patterns(filter_type: Option<&str>) {
    
    
    use std::collections::BTreeMap;
    
    // Get all patterns directly from Zig detector (single source of truth)
    let all_patterns = get_all_patterns();
    
    // Filter by pattern type if specified
    let filtered_patterns: Vec<_> = if let Some(filter) = filter_type {
        let filter_lower = filter.to_lowercase();
        all_patterns.into_iter().filter(|p| {
            match (p.pattern_type, filter_lower.as_str()) {
                (0, "fast" | "fastprefix") => true,
                (1, "structured" | "structuredformat") => true,
                (2, "regex" | "regexbased") => true,
                (_, "all") => true,
                _ => false,
            }
        }).collect()
    } else {
        all_patterns
    };
    
    let total = filtered_patterns.len();
    
    // Group patterns by pattern type
    let mut by_type: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    
    for pattern in &filtered_patterns {
        let type_name = match pattern.pattern_type {
            0 => "FastPrefix",
            1 => "StructuredFormat",
            2 => "RegexBased",
            _ => "Unknown",
        };
        
        let type_key = match pattern.pattern_type {
            0 => "0. FastPrefix (26+45 patterns, <5ms total)",
            1 => "1. StructuredFormat (1 pattern, ~1ms)",
            2 => "2. RegexBased (198 patterns, ~1000ms total)",
            _ => "Unknown",
        };
        
        by_type.entry(type_key.to_string())
            .or_default()
            .push((pattern.name.clone(), type_name.to_string()));
    }
    
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║         SCRED Pattern Library - {} patterns                ║", total);
    println!("║                  Grouped by Performance Type                ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
    
    println!("⚡ Use --filter-type to control performance:\n");
    println!("  scred --list-patterns --filter-type fast       # Only FastPrefix");
    println!("  scred --list-patterns --filter-type regex      # Only RegexBased");
    println!("  scred --list-patterns --filter-type all        # All patterns\n");
    
    for (type_key, pattern_list) in &by_type {
        println!("📊 {} - {} patterns", type_key, pattern_list.len());
        
        // Print patterns in 3 columns
        let cols = 3;
        for chunk in pattern_list.chunks(cols) {
            let formatted: Vec<String> = chunk.iter()
                .map(|(name, _)| format!("{:<30}", name))
                .collect();
            println!("   {}", formatted.join("   "));
        }
        println!();
    }
    
    println!("\n📋 Usage Examples:");
    println!("  Detect fast patterns only (production):");
    println!("    SCRED_DETECT_PATTERNS=fast scred < input.txt");
    println!("    scred --detect fast < input.txt");
    println!();
    println!("  Detect fast + structured (balanced):");
    println!("    SCRED_DETECT_PATTERNS=fast,structured scred < input.txt");
    println!();
    println!("  Detect all patterns (development):");
    println!("    SCRED_DETECT_PATTERNS=all scred < input.txt");
    println!("    scred --detect all < input.txt\n");
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

fn run_redacting_stream(verbose: bool, detect_selector: &PatternSelector, redact_selector: &PatternSelector) {
    debug!("[redacting-stream] Starting");
    let start = Instant::now();

    // Create ConfigurableEngine with pattern selectors
    debug!("[redacting-stream] Creating RedactionEngine");
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    debug!("[redacting-stream] RedactionEngine created");
    
    debug!("[redacting-stream] Creating ConfigurableEngine");
    let config_engine = ConfigurableEngine::new(
        engine,
        detect_selector.clone(),
        redact_selector.clone(),
    );
    debug!("[redacting-stream] ConfigurableEngine created");

    // FIX: Read in single shot to avoid pipe deadlock
    // (Unix pipe won't send EOF until parent shell closes write-end,
    //  but parent waits for child to exit - causing deadlock if we loop)
    // Solution: Read once, process, exit. 64KB is sufficient for CLI.
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut chunk = vec![0u8; CHUNK_SIZE];
    let mut total_read = 0;
    let mut total_written = 0;

    debug!("[redacting-stream] About to read from stdin");
    match io::stdin().read(&mut chunk) {
        Ok(n) if n > 0 => {
            let input_str = String::from_utf8_lossy(&chunk[..n]);
            let result = config_engine.detect_and_redact(&input_str);
            
            io::stdout().write_all(result.redacted.as_bytes()).ok();
            io::stdout().flush().ok();
            
            total_read = n;
            total_written = result.redacted.len();
            
            if verbose {
                eprintln!("[redacting-stream]");
                eprintln!("  Patterns detected: {}", result.warnings.len());
            }
        }
        Ok(_) => {
            // EOF or 0 bytes read - just exit cleanly
        }
        Err(e) => {
            eprintln!("Error reading input: {}", e);
            std::process::exit(1);
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

fn run_env_redacting_stream(verbose: bool, detect_selector: &PatternSelector, redact_selector: &PatternSelector) {
    let start = Instant::now();

    // Create ConfigurableEngine with pattern selectors
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let config_engine = ConfigurableEngine::new(
        engine,
        detect_selector.clone(),
        redact_selector.clone(),
    );

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
                    let redacted = env_mode::redact_env_line_configurable(line, &config_engine);
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

fn run_with_auto_detect(
    verbose: bool,
    detect_only_flag: bool,
    detect_selector: &PatternSelector,
    redact_selector: &PatternSelector,
) -> bool {
    let _start = Instant::now();
    
    // Read first 512 bytes for detection (much faster than 4KB)
    // Typical env files are 0.1-10 KB, so 512B is sufficient for first few lines
    const DETECTION_BUFFER_SIZE: usize = 512;
    let mut buffer = vec![0u8; DETECTION_BUFFER_SIZE];
    
    let n = match io::stdin().read(&mut buffer) {
        Ok(bytes) => {
            bytes
        },
        Err(e) => {
            eprintln!("Error reading input: {}", e);
            std::process::exit(1);
        }
    };
    
    buffer.truncate(n);
    
    // Detect format
    let detection = env_detection::detect_format(&buffer);
    
    if verbose || detect_only_flag {
        eprintln!("[auto-detect: {} (score: {:.2})]", detection.reason, detection.score);
    }
    
    if detect_only_flag {
        println!("Detection: {:?}", detection.mode);
        println!("Score: {:.2}", detection.score);
        println!("Reason: {}", detection.reason);
        std::process::exit(0);
    }
    
    // Decide which mode based on detection
    let use_env_mode = detection.mode == env_detection::DetectionMode::EnvFormat;
    
    // Process the detected chunk
    if use_env_mode {
        process_env_chunk_and_stream(&buffer, verbose, detect_selector, redact_selector);
    } else {
        process_text_chunk_and_stream(&buffer, verbose, detect_selector, redact_selector);
    }
    
    use_env_mode
}

fn process_env_chunk_and_stream(
    initial_buffer: &[u8],
    verbose: bool,
    detect_selector: &PatternSelector,
    redact_selector: &PatternSelector,
) {
    let start = Instant::now();
    let mut total_read = initial_buffer.len();
    let mut total_written = 0;

    // Create ConfigurableEngine
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let config_engine = ConfigurableEngine::new(
        engine,
        detect_selector.clone(),
        redact_selector.clone(),
    );
    
    // Process the initial buffer chunk
    let input_str = String::from_utf8_lossy(initial_buffer);
    for line in input_str.lines() {
        let redacted = env_mode::redact_env_line_configurable(line, &config_engine);
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
                    let redacted = env_mode::redact_env_line_configurable(line, &config_engine);
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

fn process_text_chunk_and_stream(
    initial_buffer: &[u8],
    verbose: bool,
    detect_selector: &PatternSelector,
    redact_selector: &PatternSelector,
) {
    let start = Instant::now();
    let mut total_read = initial_buffer.len();
    let mut total_written = 0;

    // Create ConfigurableEngine
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let config_engine = ConfigurableEngine::new(
        engine,
        detect_selector.clone(),
        redact_selector.clone(),
    );
    
    // Process the initial buffer chunk
    let input_str = String::from_utf8_lossy(initial_buffer);
    let result = config_engine.detect_and_redact(&input_str);
    io::stdout().write_all(result.redacted.as_bytes()).ok();
    total_written += result.redacted.len();
    
    // Continue with remaining stream
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut chunk = vec![0u8; CHUNK_SIZE];
    
    loop {
        match io::stdin().read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let input_str = String::from_utf8_lossy(&chunk[..n]);
                let result = config_engine.detect_and_redact(&input_str);
                io::stdout().write_all(result.redacted.as_bytes()).ok();
                total_read += n;
                total_written += result.redacted.len();
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
