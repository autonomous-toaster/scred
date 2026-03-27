use scred_readctor_framering::*;
use std::time::Instant;

fn main() {
    let config = RedactionConfig { enabled: true };
    
    // Time engine creation
    let start = Instant::now();
    let engine = RedactionEngine::new(config);
    let engine_time = start.elapsed();
    
    println!("Engine creation: {:?}", engine_time);
    
    // Time redaction
    let test_text = "aws: AKIAIOSFODNN7EXAMPLE, github: ghp_abc, stripe: sk_test_123";
    
    let start = Instant::now();
    let _ = engine.redact(test_text);
    let redact_time = start.elapsed();
    
    println!("Redaction of {} bytes: {:?}", test_text.len(), redact_time);
    println!("Per byte: {:.1} ns", redact_time.as_nanos() as f64 / test_text.len() as f64);
    
    // Run redaction multiple times
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = engine.redact(test_text);
    }
    let total_time = start.elapsed();
    
    println!("1000 redactions: {:?} ({:?} avg)", total_time, total_time / 1000);
}
