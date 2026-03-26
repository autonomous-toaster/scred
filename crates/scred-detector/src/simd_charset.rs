//! SIMD-accelerated charset scanning for portable performance
//! 
//! Uses std::simd (nightly) when available, falls back to scalar loop.
//! Targets: x86_64 (SSE2 via portable SIMD), arm64 (NEON), others (scalar).

use crate::simd_core::CharsetLut;

/// Scan buffer for end of token (first byte NOT in charset)
/// Uses SIMD when available, falls back to scalar
#[inline]
pub fn scan_token_end_fast(data: &[u8], charset: &CharsetLut, start: usize) -> usize {
    if start >= data.len() {
        return 0;
    }

    #[cfg(feature = "simd-accel")]
    {
        return scan_token_end_simd(&data[start..], charset);
    }

    #[cfg(not(feature = "simd-accel"))]
    {
        return scan_token_end_scalar(&data[start..], charset);
    }
}

/// Scalar fallback: process bytes with 8x loop unrolling for maximum ILP
#[inline]
fn scan_token_end_scalar(data: &[u8], charset: &CharsetLut) -> usize {
    let mut i = 0;
    let len = data.len();
    
    // Process 8 bytes at a time (8x unrolled loop)
    while i + 8 <= len {
        if !charset.contains(data[i]) {
            return i;
        }
        if !charset.contains(data[i + 1]) {
            return i + 1;
        }
        if !charset.contains(data[i + 2]) {
            return i + 2;
        }
        if !charset.contains(data[i + 3]) {
            return i + 3;
        }
        if !charset.contains(data[i + 4]) {
            return i + 4;
        }
        if !charset.contains(data[i + 5]) {
            return i + 5;
        }
        if !charset.contains(data[i + 6]) {
            return i + 6;
        }
        if !charset.contains(data[i + 7]) {
            return i + 7;
        }
        i += 8;
    }
    
    // Process remaining bytes
    while i < len {
        if !charset.contains(data[i]) {
            return i;
        }
        i += 1;
    }
    
    len
}

/// SIMD version: process 16 bytes at a time (when available)
/// This is only compiled with --features simd-accel on nightly
#[cfg(feature = "simd-accel")]
#[inline]
fn scan_token_end_simd(data: &[u8], charset: &CharsetLut) -> usize {
    // Only use SIMD on supported platforms
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        use std::simd::u8x16;
        
        let mut pos = 0;
        
        // Process 16 bytes at a time
        while pos + 16 <= data.len() {
            let chunk = u8x16::from_slice(&data[pos..pos + 16]);
            
            // Check each byte: 0 if in charset, 1 if not
            let mut found_boundary = false;
            let mut boundary_pos = 0;
            
            for i in 0..16 {
                if !charset.contains(chunk[i]) {
                    found_boundary = true;
                    boundary_pos = i;
                    break;
                }
            }
            
            if found_boundary {
                return pos + boundary_pos;
            }
            
            pos += 16;
        }
        
        // Scalar fallback for tail bytes
        while pos < data.len() && charset.contains(data[pos]) {
            pos += 1;
        }
        
        pos
    }
    
    // Fallback for unsupported platforms
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        scan_token_end_scalar(data, charset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_token_end_scalar() {
        let charset = CharsetLut::new(b"abcdefghijklmnopqrstuvwxyz");
        let data = b"hello123";
        
        // "hello" = 5 chars in charset, then "1"
        let result = scan_token_end_scalar(data, &charset);
        assert_eq!(result, 5);
    }

    #[test]
    fn test_scan_token_end_empty() {
        let charset = CharsetLut::new(b"abc");
        let result = scan_token_end_scalar(b"", &charset);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_scan_token_end_all_match() {
        let charset = CharsetLut::new(b"abcdefghijklmnopqrstuvwxyz");
        let result = scan_token_end_scalar(b"abcdef", &charset);
        assert_eq!(result, 6);
    }

    #[test]
    fn test_scan_token_end_none_match() {
        let charset = CharsetLut::new(b"abcdefghijklmnopqrstuvwxyz");
        let result = scan_token_end_scalar(b"123", &charset);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_scan_token_end_fast_consistency() {
        let charset = CharsetLut::new(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_");
        let test_cases: Vec<(&[u8], usize)> = vec![
            (b"hello_world123xyz", 17),  // All match (includes _), full length
            (b"AKIAIOSFODNN7EXAMPLE ", 20),  // "AKIAIOSFODNN7EXAMPLE" match, space breaks
            (b"ghp_abcdef123!secret", 13),  // "ghp_abcdef123" match, ! breaks
            (b"123!", 3),  // "123" match, ! breaks
            (b"a", 1),  // Single match
            (b"abc!", 3),  // "abc" match, ! breaks
        ];

        for (data, expected) in test_cases {
            let result = scan_token_end_fast(data, &charset, 0);
            assert_eq!(result, expected, "Failed for input: {:?}", String::from_utf8_lossy(data));
        }
    }

    #[test]
    fn test_scan_token_end_large_buffer() {
        let charset = CharsetLut::new(b"abcdefghijklmnopqrstuvwxyz");
        
        // Create a 1KB buffer of all 'a'
        let mut data = vec![b'a'; 1024];
        data[512] = b'1';  // Break at position 512
        
        let result = scan_token_end_fast(&data, &charset, 0);
        assert_eq!(result, 512);
    }

    #[test]
    fn test_scan_token_end_offset() {
        let charset = CharsetLut::new(b"abc");
        let data = b"xxxabcdef123";
        
        // Start from position 3 (at 'a')
        let result = scan_token_end_fast(data, &charset, 3);
        assert_eq!(result, 3);  // abc = 3 bytes from position 3
    }
}
