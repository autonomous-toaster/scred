use scred_redactor::{RedactionEngine, RedactionConfig};

#[test]
fn debug_ffi_call() {
    let engine = RedactionEngine::new(RedactionConfig::default());
    let text = "AKIAIOSFODNN7EXAMPLE";
    let result = engine.redact(text);
    
    println!("Input: {}", text);
    println!("Output: {}", result.redacted);
    println!("Matches: {}", result.matches.len());
    
    // Check if redaction happened
    if result.redacted.contains("xxx") {
        println!("✅ Redaction occurred!");
    } else {
        println!("❌ No redaction occurred");
    }
}
