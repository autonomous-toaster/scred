use std::time::Instant;
use scred_detector::*;

fn main() {
    // Generate test data
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    let target_size = 10 * 1024 * 1024;
    let mut data = Vec::with_capacity(target_size);
    while data.len() < target_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(target_size);
    
    eprintln!("Profiling individual detectors on 10MB data:");
    
    // Profile each detector
    let start = Instant::now();
    let r1 = detect_simple_prefix(&data);
    let t1 = start.elapsed();
    eprintln!("simple_prefix:  {:6}ms, {} matches, {:.1} MB/s", 
        t1.as_millis(), r1.matches.len(), 10.0 / t1.as_secs_f64());
    
    let start = Instant::now();
    let r2 = detect_validation(&data);
    let t2 = start.elapsed();
    eprintln!("validation:     {:6}ms, {} matches, {:.1} MB/s",
        t2.as_millis(), r2.matches.len(), 10.0 / t2.as_secs_f64());
    
    let start = Instant::now();
    let r3 = detect_jwt(&data);
    let t3 = start.elapsed();
    eprintln!("jwt:            {:6}ms, {} matches, {:.1} MB/s",
        t3.as_millis(), r3.matches.len(), 10.0 / t3.as_secs_f64());
    
    let start = Instant::now();
    let r4 = detect_ssh_keys(&data);
    let t4 = start.elapsed();
    eprintln!("ssh_keys:       {:6}ms, {} matches, {:.1} MB/s",
        t4.as_millis(), r4.matches.len(), 10.0 / t4.as_secs_f64());
    
    let start = Instant::now();
    let r5 = detect_uri_patterns(&data);
    let t5 = start.elapsed();
    eprintln!("uri_patterns:   {:6}ms, {} matches, {:.1} MB/s",
        t5.as_millis(), r5.matches.len(), 10.0 / t5.as_secs_f64());
    
    let total = t1 + t2 + t3 + t4 + t5;
    eprintln!("\nBreakdown:");
    eprintln!("simple_prefix:  {:.1}%", t1.as_secs_f64() * 100.0 / total.as_secs_f64());
    eprintln!("validation:     {:.1}%", t2.as_secs_f64() * 100.0 / total.as_secs_f64());
    eprintln!("jwt:            {:.1}%", t3.as_secs_f64() * 100.0 / total.as_secs_f64());
    eprintln!("ssh_keys:       {:.1}%", t4.as_secs_f64() * 100.0 / total.as_secs_f64());
    eprintln!("uri_patterns:   {:.1}%", t5.as_secs_f64() * 100.0 / total.as_secs_f64());
    eprintln!("\nTotal: {:.0}ms", total.as_millis());
}
