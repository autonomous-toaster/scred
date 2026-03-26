//! SIMD-based byte search - replacement for memchr
//! Uses std::simd (portable_simd) to search for bytes at high speed
//! This allows us to remove the memchr dependency

#![cfg_attr(feature = "simd-search", feature(portable_simd))]

#[cfg(all(feature = "simd-search", any(target_arch = "x86_64", target_arch = "aarch64")))]
use std::simd::{cmp::SimdPartialEq, u8x32, Simd};

/// Find first occurrence of a byte in data using SIMD
/// Falls back to scalar search if SIMD not available
#[inline]
pub fn simd_memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    #[cfg(all(feature = "simd-search", any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        simd_memchr_impl(needle, haystack)
    }
    #[cfg(not(all(feature = "simd-search", any(target_arch = "x86_64", target_arch = "aarch64"))))]
    {
        scalar_memchr(needle, haystack)
    }
}

/// SIMD implementation: process 32 bytes at a time
#[cfg(all(feature = "simd-search", any(target_arch = "x86_64", target_arch = "aarch64")))]
fn simd_memchr_impl(needle: u8, haystack: &[u8]) -> Option<usize> {
    use std::simd::cmp::SimdPartialEq;
    
    const CHUNK_SIZE: usize = 32;
    let needle_vec = u8x32::splat(needle);
    
    // Process full 32-byte chunks
    let full_chunks = haystack.len() / CHUNK_SIZE;
    for chunk_idx in 0..full_chunks {
        let start = chunk_idx * CHUNK_SIZE;
        let chunk = &haystack[start..start + CHUNK_SIZE];
        
        // Load 32 bytes and compare all at once
        let data = u8x32::from_slice(chunk);
        let mask = data.simd_eq(needle_vec);
        
        // Check if any lane matched
        if mask.any() {
            // Find the first match in this chunk
            for (i, &byte) in chunk.iter().enumerate() {
                if byte == needle {
                    return Some(start + i);
                }
            }
        }
    }
    
    // Process remaining bytes with scalar
    let remainder_start = full_chunks * CHUNK_SIZE;
    scalar_memchr_offset(needle, &haystack[remainder_start..], remainder_start)
}

/// Scalar fallback: simple byte search
#[inline]
fn scalar_memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

/// Scalar search starting from offset
#[inline]
fn scalar_memchr_offset(needle: u8, haystack: &[u8], offset: usize) -> Option<usize> {
    haystack.iter().position(|&b| b == needle).map(|p| offset + p)
}

/// Find first occurrence of multi-byte prefix
/// More efficient than searching for each byte individually
#[inline]
pub fn simd_find_prefix(needle: &[u8], haystack: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return if needle.is_empty() { Some(0) } else { None };
    }
    
    // Single byte: use optimized memchr
    if needle.len() == 1 {
        return simd_memchr(needle[0], haystack);
    }
    
    // Multi-byte: search for first byte, then validate
    let first_byte = needle[0];
    let mut search_pos = 0;
    
    while let Some(pos) = simd_memchr(first_byte, &haystack[search_pos..]) {
        let absolute_pos = search_pos + pos;
        
        // Check if we have enough bytes for full prefix
        if absolute_pos + needle.len() <= haystack.len() {
            // Validate full prefix
            if &haystack[absolute_pos..absolute_pos + needle.len()] == needle {
                return Some(absolute_pos);
            }
        }
        
        // Continue searching
        search_pos = absolute_pos + 1;
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_memchr_single_byte() {
        let data = b"hello world AKIA test";
        assert_eq!(simd_memchr(b'o', data), Some(4));
        assert_eq!(simd_memchr(b'A', data), Some(12));
        assert_eq!(simd_memchr(b'z', data), None);
    }

    #[test]
    fn test_simd_memchr_large_data() {
        let mut data = vec![b'x'; 1000];
        data[500] = b'A';
        assert_eq!(simd_memchr(b'A', &data), Some(500));
    }

    #[test]
    fn test_simd_find_prefix_single() {
        let data = b"hello world AKIA test";
        assert_eq!(simd_find_prefix(b"o", data), Some(4));
        assert_eq!(simd_find_prefix(b"AKIA", data), Some(12));
    }

    #[test]
    fn test_simd_find_prefix_multi() {
        let data = b"prefix_AKIAIOSFODNN7_suffix";
        assert_eq!(simd_find_prefix(b"AKIA", data), Some(7));
        assert_eq!(simd_find_prefix(b"prefix", data), Some(0));
        assert_eq!(simd_find_prefix(b"_AKIA", data), Some(6));
        assert_eq!(simd_find_prefix(b"notfound", data), None);
    }
}
