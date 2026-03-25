// Test pattern FFI classification

use std::ffi::CStr;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExportedPattern {
    pub name: [u8; 128],
    pub prefix: [u8; 256],
    pub min_len: usize,
    pub severity: u8,
    pub category: u8,
    pub kind: u8,
    pub origin: u8,
}

#[link(name = "scred_pattern_detector", kind = "static")]
extern "C" {
    pub fn scred_detector_get_pattern_count() -> usize;
    pub fn scred_detector_get_pattern(index: usize, exported: *mut ExportedPattern) -> i32;
}

fn u8_to_severity(val: u8) -> &'static str {
    match val {
        95 => "Critical",
        85 => "High",
        65 => "Medium",
        40 => "Low",
        30 => "Generic",
        _ => "Unknown",
    }
}

fn u8_to_category(val: u8) -> &'static str {
    match val {
        0 => "CloudProvider",
        1 => "PaymentProcessor",
        2 => "CodeHost",
        3 => "Database",
        4 => "Messaging",
        5 => "Infrastructure",
        6 => "Authentication",
        7 => "Monitoring",
        8 => "Development",
        9 => "AI",
        10 => "Other",
        _ => "Unknown",
    }
}

fn u8_to_kind(val: u8) -> &'static str {
    match val {
        0 => "FixedPrefix",
        1 => "StructuredFormat",
        2 => "RegexBased",
        _ => "Unknown",
    }
}

fn main() {
    unsafe {
        let count = scred_detector_get_pattern_count();
        println!("Total patterns: {}\n", count);
        
        println!("{:<35} | {:>8} | {:>18} | {:>16}", 
                 "Pattern Name", "Severity", "Category", "Kind");
        println!("{}", "-".repeat(100));
        
        let mut sample_count = 0;
        for i in 0..count {
            let mut pattern = std::mem::zeroed::<ExportedPattern>();
            let success = scred_detector_get_pattern(i as usize, &mut pattern);
            
            if success != 1 {
                continue;
            }
            
            // Extract C string
            let name_cstr = CStr::from_bytes_until_nul(&pattern.name)
                .unwrap_or(CStr::from_bytes_with_nul(b"<invalid>\0").unwrap());
            let name = name_cstr.to_string_lossy();
            
            println!("{:<35} | {:>8} | {:>18} | {:>16}",
                     name,
                     u8_to_severity(pattern.severity),
                     u8_to_category(pattern.category),
                     u8_to_kind(pattern.kind));
            
            sample_count += 1;
            if sample_count >= 15 {
                println!("\n... and {} more patterns", count - sample_count);
                break;
            }
        }
    }
}
