// PHASE 5 WAVE 2: PERFORMANCE BENCHMARK BINARY
// Measures throughput for all 18 Wave 2 provider & structure functions

use std::time::Instant;

extern "C" {
    // Provider Functions
    fn validate_gcp_credential(data: *const u8, data_len: usize) -> bool;
    fn validate_azure_credential(credential_type: u8, data: *const u8, data_len: usize) -> bool;
    fn validate_stripe_key(key_type: u8, data: *const u8, data_len: usize) -> bool;
    fn validate_slack_token(token_type: u8, data: *const u8, data_len: usize) -> bool;
    fn validate_sendgrid_key(data: *const u8, data_len: usize) -> bool;
    fn validate_twilio_key(data: *const u8, data_len: usize) -> bool;
    fn validate_mailchimp_key(data: *const u8, data_len: usize) -> bool;
    fn validate_heroku_key(data: *const u8, data_len: usize) -> bool;
    fn validate_digitalocean_token(data: *const u8, data_len: usize) -> bool;
    fn validate_shopify_token(token_type: u8, data: *const u8, data_len: usize) -> bool;

    // Structure Functions
    fn validate_connection_string(service_type: u8, data: *const u8, data_len: usize) -> bool;
    fn validate_mongo_uri(data: *const u8, data_len: usize) -> bool;
    fn validate_redis_url(data: *const u8, data_len: usize) -> bool;
    fn validate_jwt_variant(data: *const u8, data_len: usize) -> bool;
    fn validate_custom_header(header_type: u8, data: *const u8, data_len: usize) -> bool;

    // Charset Functions
    fn validate_extended_base32(data: *const u8, data_len: usize) -> bool;
    fn validate_extended_hex(pattern_type: u8, data: *const u8, data_len: usize) -> bool;
    fn validate_custom_charset(data: *const u8, data_len: usize, charset: *const u8, charset_len: usize) -> bool;
}

#[derive(Clone)]
struct BenchmarkResult {
    function_name: &'static str,
    iterations: usize,
    data_size: usize,
    total_bytes: usize,
    elapsed_ms: u128,
    throughput_mb_s: f64,
}

impl BenchmarkResult {
    fn print(&self) {
        println!(
            "{:30} | {:10} iters | {:6} bytes | {:10.2} MB/s | {:8} ms",
            self.function_name,
            self.iterations,
            self.data_size,
            self.throughput_mb_s,
            self.elapsed_ms
        );
    }
}

fn benchmark<F>(
    function_name: &'static str,
    iterations: usize,
    data_size: usize,
    mut test_fn: F,
) -> BenchmarkResult
where
    F: FnMut() -> bool,
{
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = test_fn();
    }

    let elapsed = start.elapsed();
    let total_bytes = iterations * data_size;
    let elapsed_ms = elapsed.as_millis();
    let elapsed_secs = elapsed.as_secs_f64();
    let throughput_mb_s = (total_bytes as f64 / 1_000_000.0) / elapsed_secs;

    BenchmarkResult {
        function_name,
        iterations,
        data_size,
        total_bytes,
        elapsed_ms,
        throughput_mb_s,
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════════════════════");
    println!("PHASE 5 WAVE 2: PROVIDER & STRUCTURE FUNCTIONS - PERFORMANCE BENCHMARK");
    println!("═══════════════════════════════════════════════════════════════════════════════\n");

    let mut results = Vec::new();
    const ITERATIONS: usize = 100_000;

    // ========================================================================
    // PROVIDER FUNCTIONS (10 functions)
    // ========================================================================

    println!("Provider Functions:");
    println!("{:-^30} | {:-^10} | {:-^6} | {:-^10} | {:-^8}", 
             "Function", "Iterations", "Bytes", "MB/s", "ms");
    println!("{}", "─".repeat(90));

    // GCP Credential
    let json = br#"{"client_email":"sa@project.iam.gserviceaccount.com","private_key":"-----BEGIN RSA PRIVATE KEY-----","project_id":"my-project"}"#;
    let json_len = json.len();
    let result = benchmark(
        "validate_gcp_credential",
        ITERATIONS,
        json_len,
        || unsafe { validate_gcp_credential(json.as_ptr(), json_len) },
    );
    result.print();
    results.push(result);

    // Azure Credential
    let azure_uuid = b"550e8400-e29b-41d4-a716-446655440000";
    let azure_len = azure_uuid.len();
    let result = benchmark(
        "validate_azure_credential",
        ITERATIONS,
        azure_len,
        || unsafe { validate_azure_credential(0, azure_uuid.as_ptr(), azure_len) },
    );
    result.print();
    results.push(result);

    // Stripe Key
    let stripe = b"sk_live_4eC39HqLyjWDarhtT657tSRf";
    let stripe_len = stripe.len();
    let result = benchmark(
        "validate_stripe_key",
        ITERATIONS,
        stripe_len,
        || unsafe { validate_stripe_key(0, stripe.as_ptr(), stripe_len) },
    );
    result.print();
    results.push(result);

    // Slack Token
    let slack = b"xoxb-1234567890123456789012345678";
    let slack_len = slack.len();
    let result = benchmark(
        "validate_slack_token",
        ITERATIONS,
        slack_len,
        || unsafe { validate_slack_token(0, slack.as_ptr(), slack_len) },
    );
    result.print();
    results.push(result);

    // SendGrid Key
    let sendgrid = b"SG.1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXY";
    let sendgrid_len = sendgrid.len();
    let result = benchmark(
        "validate_sendgrid_key",
        ITERATIONS,
        sendgrid_len,
        || unsafe { validate_sendgrid_key(sendgrid.as_ptr(), sendgrid_len) },
    );
    result.print();
    results.push(result);

    // Twilio Key
    let twilio = b"ACaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab";
    let twilio_len = twilio.len();
    let result = benchmark(
        "validate_twilio_key",
        ITERATIONS,
        twilio_len,
        || unsafe { validate_twilio_key(twilio.as_ptr(), twilio_len) },
    );
    result.print();
    results.push(result);

    // Mailchimp Key
    let mailchimp = b"1234567890abcdef1234567890abcdef-us1";
    let mailchimp_len = mailchimp.len();
    let result = benchmark(
        "validate_mailchimp_key",
        ITERATIONS,
        mailchimp_len,
        || unsafe { validate_mailchimp_key(mailchimp.as_ptr(), mailchimp_len) },
    );
    result.print();
    results.push(result);

    // Heroku Key
    let heroku = b"1234567890abcdef1234567890abcdef12345678";
    let heroku_len = heroku.len();
    let result = benchmark(
        "validate_heroku_key",
        ITERATIONS,
        heroku_len,
        || unsafe { validate_heroku_key(heroku.as_ptr(), heroku_len) },
    );
    result.print();
    results.push(result);

    // DigitalOcean Token
    let do_token = b"dop_v1_1234567890abcdefghijklmnopqrst";
    let do_len = do_token.len();
    let result = benchmark(
        "validate_digitalocean_token",
        ITERATIONS,
        do_len,
        || unsafe { validate_digitalocean_token(do_token.as_ptr(), do_len) },
    );
    result.print();
    results.push(result);

    // Shopify Token
    let shopify = b"shpat_1234567890abcdefghijklmnopqrst";
    let shopify_len = shopify.len();
    let result = benchmark(
        "validate_shopify_token",
        ITERATIONS,
        shopify_len,
        || unsafe { validate_shopify_token(0, shopify.as_ptr(), shopify_len) },
    );
    result.print();
    results.push(result);

    // ========================================================================
    // STRUCTURE FUNCTIONS (5 functions)
    // ========================================================================

    println!("\nStructure Functions:");
    println!("{:-^30} | {:-^10} | {:-^6} | {:-^10} | {:-^8}", 
             "Function", "Iterations", "Bytes", "MB/s", "ms");
    println!("{}", "─".repeat(90));

    // Connection String
    let conn = b"postgresql://user:pass@localhost:5432/dbname";
    let conn_len = conn.len();
    let result = benchmark(
        "validate_connection_string",
        ITERATIONS,
        conn_len,
        || unsafe { validate_connection_string(0, conn.as_ptr(), conn_len) },
    );
    result.print();
    results.push(result);

    // MongoDB URI
    let mongo = b"mongodb://user:password@localhost:27017/database";
    let mongo_len = mongo.len();
    let result = benchmark(
        "validate_mongo_uri",
        ITERATIONS,
        mongo_len,
        || unsafe { validate_mongo_uri(mongo.as_ptr(), mongo_len) },
    );
    result.print();
    results.push(result);

    // Redis URL
    let redis = b"redis://localhost:6379";
    let redis_len = redis.len();
    let result = benchmark(
        "validate_redis_url",
        ITERATIONS,
        redis_len,
        || unsafe { validate_redis_url(redis.as_ptr(), redis_len) },
    );
    result.print();
    results.push(result);

    // JWT Variant
    let jwt = b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let jwt_len = jwt.len();
    let result = benchmark(
        "validate_jwt_variant",
        ITERATIONS,
        jwt_len,
        || unsafe { validate_jwt_variant(jwt.as_ptr(), jwt_len) },
    );
    result.print();
    results.push(result);

    // Custom Header
    let header = b"Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
    let header_len = header.len();
    let result = benchmark(
        "validate_custom_header",
        ITERATIONS,
        header_len,
        || unsafe { validate_custom_header(0, header.as_ptr(), header_len) },
    );
    result.print();
    results.push(result);

    // ========================================================================
    // CHARSET FUNCTIONS (3 functions)
    // ========================================================================

    println!("\nCharset Functions:");
    println!("{:-^30} | {:-^10} | {:-^6} | {:-^10} | {:-^8}", 
             "Function", "Iterations", "Bytes", "MB/s", "ms");
    println!("{}", "─".repeat(90));

    // Extended Base32
    let base32 = b"JBSWY3DPEBLW64TMMQ======";
    let base32_len = base32.len();
    let result = benchmark(
        "validate_extended_base32",
        ITERATIONS,
        base32_len,
        || unsafe { validate_extended_base32(base32.as_ptr(), base32_len) },
    );
    result.print();
    results.push(result);

    // Extended Hex
    let hex = b"0x1234567890abcdef";
    let hex_len = hex.len();
    let result = benchmark(
        "validate_extended_hex",
        ITERATIONS,
        hex_len,
        || unsafe { validate_extended_hex(0, hex.as_ptr(), hex_len) },
    );
    result.print();
    results.push(result);

    // Custom Charset
    let custom = b"abc123xyz";
    let charset = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let custom_len = custom.len();
    let charset_len = charset.len();
    let result = benchmark(
        "validate_custom_charset",
        ITERATIONS,
        custom_len,
        || unsafe { validate_custom_charset(custom.as_ptr(), custom_len, charset.as_ptr(), charset_len) },
    );
    result.print();
    results.push(result);

    // ========================================================================
    // SUMMARY
    // ========================================================================

    println!("\n{}", "═".repeat(90));
    println!("SUMMARY");
    println!("{}", "═".repeat(90));

    let total_iterations: usize = results.iter().map(|r| r.iterations).sum();
    let total_bytes: usize = results.iter().map(|r| r.total_bytes).sum();
    let total_ms: u128 = results.iter().map(|r| r.elapsed_ms).sum();
    let avg_throughput: f64 = results.iter().map(|r| r.throughput_mb_s).sum::<f64>() / results.len() as f64;

    println!("\nTotal Functions Benchmarked: {}", results.len());
    println!("Total Iterations:           {}", total_iterations);
    println!("Total Data Processed:       {} MB", total_bytes / 1_000_000);
    println!("Total Time:                 {} ms", total_ms);
    println!("Average Throughput:         {:.2} MB/s", avg_throughput);

    // Top performers
    let mut top = results.clone();
    top.sort_by(|a, b| b.throughput_mb_s.partial_cmp(&a.throughput_mb_s).unwrap());
    
    println!("\nTop 5 Fastest:");
    for (i, result) in top.iter().take(5).enumerate() {
        println!("  {}. {} - {:.2} MB/s", i + 1, result.function_name, result.throughput_mb_s);
    }

    println!("\nTop 5 Slowest:");
    for (i, result) in top.iter().rev().take(5).enumerate() {
        println!("  {}. {} - {:.2} MB/s", i + 1, result.function_name, result.throughput_mb_s);
    }

    println!("\n{}", "═".repeat(90));
    println!("WAVE 2 BENCHMARK COMPLETE");
    println!("{}", "═".repeat(90));
}
