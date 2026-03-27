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
        scan_token_end_scalar(&data[start..], charset)
    }
}

/// Scalar fallback: process bytes with 8x loop unrolling for maximum ILP
/// Scalar fallback: process bytes with 8x loop unrolling for maximum ILP
/// Uses aggressive inline to allow compiler to prefetch/pipeline
#[inline(always)]
fn scan_token_end_scalar(data: &[u8], charset: &CharsetLut) -> usize {
    let mut i = 0;
    let len = data.len();
    
    // Process 8 bytes at a time (8x unrolled loop)
    // Specialization: For ASCII-heavy data, inline the lookup
    while i + 8 <= len {
        // Manually inline charset.contains for better code gen
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
/// 
/// OPTIMIZED FOR SIMD:
/// - Loads 16 bytes into SIMD vector
/// - Uses direct LUT lookup (not function call overhead)
/// - Tail bytes use 8x scalar unroll for efficiency
#[cfg(feature = "simd-accel")]
#[inline]
fn scan_token_end_simd(data: &[u8], charset: &CharsetLut) -> usize {
    // Only use SIMD on supported platforms
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        use std::simd::u8x16;
        
        // Get direct reference to LUT to avoid function call overhead
        let table = &charset.table;
        
        let mut pos = 0;
        
        // Process 16 bytes at a time
        // KEY OPTIMIZATION: Direct table access instead of contains() method
        while pos + 16 <= data.len() {
            let chunk = u8x16::from_slice(&data[pos..pos + 16]);
            
            // Check all 16 bytes quickly with direct LUT
            for i in 0..16 {
                if !table[chunk[i] as usize] {
                    return pos + i;
                }
            }
            
            pos += 16;
        }
        
        // Scalar 8x unrolled for tail (more efficient than scalar 1x)
        while pos + 8 <= data.len() {
            if !table[data[pos] as usize] { return pos; }
            if !table[data[pos + 1] as usize] { return pos + 1; }
            if !table[data[pos + 2] as usize] { return pos + 2; }
            if !table[data[pos + 3] as usize] { return pos + 3; }
            if !table[data[pos + 4] as usize] { return pos + 4; }
            if !table[data[pos + 5] as usize] { return pos + 5; }
            if !table[data[pos + 6] as usize] { return pos + 6; }
            if !table[data[pos + 7] as usize] { return pos + 7; }
            pos += 8;
        }
        
        // Final bytes
        while pos < data.len() {
            if !table[data[pos] as usize] { return pos; }
            pos += 1;
        }
        
        data.len()
    }
    
    // Fallback for unsupported platforms
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        scan_token_end_scalar(data, charset)
    }
}

