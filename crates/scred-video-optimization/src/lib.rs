//! Optimized Streaming Redactor - Video Transcoding Patterns
//!
//! This crate implements the video transcoding optimization experiment:
//! - Frame ring buffer for cache locality (Phase 1)
//! - Parallel pattern batches (Phase 2)
//!
//! Keep it simple: wrap existing scred-redactor with optimizations

pub mod frame_ring_redactor;

pub use frame_ring_redactor::FrameRingRedactor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
