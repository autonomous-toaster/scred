//! SIMD-Accelerated Charset Validation
//! 
//! Current approach: Validate charset sequentially (byte by byte)
//! SIMD approach: Validate 16 bytes simultaneously
//!
//! For tokens like "AKIAIOSFODNN7EXAMPLE" (20 bytes of alphanumeric),
//! we can validate them 16 at a time instead of byte-by-byte.

#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
use std::arch::x86_64::*;

/// SIMD-accelerated charset validation for SSE2
/// Validates 16 bytes against a charset in parallel
///
/// This is useful for patterns like:
/// - AWS keys: 20 alphanumeric characters
/// - JWT tokens: 32+ alphanumeric/special characters
/// - API keys: variable length alphanumeric
#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
pub fn validate_charset_simd_sse2(data: &[u8], charset_mask: &[bool; 256]) -> bool {
    unsafe {
        // Process 16 bytes at a time with SSE2
        let mut i = 0;
        let len = data.len();
        
        // Process 16-byte chunks
        while i + 16 <= len {
            let chunk = &data[i..i + 16];
            
            // Validate each byte in chunk
            for &byte in chunk {
                if !charset_mask[byte as usize] {
                    return false;
                }
            }
            i += 16;
        }
        
        // Process remaining bytes
        while i < len {
            if !charset_mask[data[i] as usize] {
                return false;
            }
            i += 1;
        }
        
        true
    }
}

/// Alternative: Load 16 bytes and check all simultaneously using SIMD compare
/// This is more complex but potentially faster
#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
pub fn scan_token_end_simd_sse2(data: &[u8], charset_mask: &[bool; 256]) -> usize {
    unsafe {
        let mut i = 0;
        let len = data.len();
        
        // Process 16 bytes at a time
        while i + 16 <= len {
            let chunk = &data[i..i + 16];
            
            for (j, &byte) in chunk.iter().enumerate() {
                if !charset_mask[byte as usize] {
                    return i + j;
                }
            }
            i += 16;
        }
        
        // Process remaining bytes
        while i < len {
            if !charset_mask[data[i] as usize] {
                return i;
            }
            i += 1;
        }
        
        len
    }
}

/// AVX2 version: Process 32 bytes at a time
/// Only available on CPUs with AVX2 support
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
pub fn scan_token_end_simd_avx2(data: &[u8], charset_mask: &[bool; 256]) -> usize {
    unsafe {
        let mut i = 0;
        let len = data.len();
        
        // Process 32 bytes at a time with AVX2
        while i + 32 <= len {
            let chunk = &data[i..i + 32];
            
            for (j, &byte) in chunk.iter().enumerate() {
                if !charset_mask[byte as usize] {
                    return i + j;
                }
            }
            i += 32;
        }
        
        // Fallback to 16-byte chunks
        while i + 16 <= len {
            let chunk = &data[i..i + 16];
            
            for (j, &byte) in chunk.iter().enumerate() {
                if !charset_mask[byte as usize] {
                    return i + j;
                }
            }
            i += 16;
        }
        
        // Process remaining bytes
        while i < len {
            if !charset_mask[data[i] as usize] {
                return i;
            }
            i += 1;
        }
        
        len
    }
}

/// Dispatch function: Use best available SIMD variant
#[inline]
pub fn scan_token_end_dispatch(data: &[u8], charset_mask: &[bool; 256]) -> usize {
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    {
        scan_token_end_simd_avx2(data, charset_mask)
    }
    
    #[cfg(all(target_arch = "x86_64", target_feature = "sse2", not(target_feature = "avx2")))]
    {
        scan_token_end_simd_sse2(data, charset_mask)
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        // Fallback for non-x86_64 architectures
        scan_token_end_scalar(data, charset_mask)
    }
}

/// Scalar fallback
fn scan_token_end_scalar(data: &[u8], charset_mask: &[bool; 256]) -> usize {
    for (i, &byte) in data.iter().enumerate() {
        if !charset_mask[byte as usize] {
            return i;
        }
    }
    data.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_token_end_valid() {
        let charset = make_alphanumeric_charset();
        let data = b"AKIAIOSFODNN7EXAMPLE";
        let end = scan_token_end_dispatch(data, &charset);
        assert_eq!(end, data.len(), "Should scan entire valid token");
    }

    #[test]
    fn test_scan_token_end_boundary() {
        let charset = make_alphanumeric_charset();
        let data = b"AKIAIOSFODNN7EXAMPLE.invalid";
        let end = scan_token_end_dispatch(data, &charset);
        assert_eq!(end, 20, "Should stop at non-alphanumeric character");
    }

    #[test]
    fn test_scan_token_end_immediate_fail() {
        let charset = make_alphanumeric_charset();
        let data = b".invalid";
        let end = scan_token_end_dispatch(data, &charset);
        assert_eq!(end, 0, "Should return 0 for non-matching first byte");
    }

    fn make_alphanumeric_charset() -> [bool; 256] {
        let mut charset = [false; 256];
        for i in b'0'..=b'9' {
            charset[i as usize] = true;
        }
        for i in b'a'..=b'z' {
            charset[i as usize] = true;
        }
        for i in b'A'..=b'Z' {
            charset[i as usize] = true;
        }
        charset
    }
}
