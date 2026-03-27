//! Frame Ring Buffer
//!
//! Implements video transcoding-inspired frame ring buffer pattern for efficient streaming.
//! Pre-allocates N frames of fixed size and rotates through them, providing:
//! - Zero allocation in hot path (all memory pre-allocated)
//! - Cache-friendly memory layout (contiguous buffers)
//! - Predictable memory usage (bounded to N × frame_size)
//!
//! # Example
//! ```ignore
//! let mut ring = FrameRing::<64 * 1024, 3>::new();
//! let frame = ring.get_read_frame();
//! // Fill frame with data...
//! ring.mark_ready_and_rotate();
//! let output = ring.get_output_frame();
//! // Process and write output...
//! ring.mark_written_and_rotate();
//! ```

/// Frame state in the ring buffer pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameState {
    /// Available for reading input
    ReadingInput,
    /// Contains data ready to process
    ReadyToProcess,
    /// Contains processed output ready to write
    ReadyToOutput,
    /// Available for writing
    Available,
}

/// Ring buffer of pre-allocated frames for streaming processing
/// 
/// Typical usage (3 frames in flight: read, process, write):
/// 1. Get read frame, fill with input data
/// 2. Mark ready, get process frame, redact it
/// 3. Mark ready, get output frame, write results
/// 4. Mark available, rotate to next frame
pub struct FrameRing<const FRAME_SIZE: usize, const NUM_FRAMES: usize> {
    /// Pre-allocated frames
    frames: [Vec<u8>; NUM_FRAMES],
    /// Current state of each frame
    states: [FrameState; NUM_FRAMES],
    /// Index of frame being filled (reader)
    read_idx: usize,
    /// Index of frame being processed
    process_idx: usize,
    /// Index of frame being output (writer)
    write_idx: usize,
}

impl<const FRAME_SIZE: usize, const NUM_FRAMES: usize> FrameRing<FRAME_SIZE, NUM_FRAMES> {
    /// Create a new frame ring with pre-allocated frames
    pub fn new() -> Self {
        // Pre-allocate all frames with exact capacity
        let mut frames: Vec<Vec<u8>> = Vec::with_capacity(NUM_FRAMES);
        for _ in 0..NUM_FRAMES {
            frames.push(Vec::with_capacity(FRAME_SIZE));
        }

        // Convert to array (we know exactly NUM_FRAMES, so safe)
        let frames_array: [Vec<u8>; NUM_FRAMES] = frames
            .try_into()
            .expect("frames vec should have exactly NUM_FRAMES elements");

        Self {
            frames: frames_array,
            states: [FrameState::Available; NUM_FRAMES],
            read_idx: 0,
            process_idx: 1 % NUM_FRAMES,
            write_idx: 2 % NUM_FRAMES,
        }
    }

    /// Get mutable reference to read frame (for filling with input)
    pub fn get_read_frame(&mut self) -> &mut Vec<u8> {
        &mut self.frames[self.read_idx]
    }

    /// Mark read frame as ready and rotate to next frame
    pub fn mark_ready_and_rotate_read(&mut self) {
        self.states[self.read_idx] = FrameState::ReadyToProcess;
        self.read_idx = (self.read_idx + 1) % NUM_FRAMES;
    }

    /// Get mutable reference to process frame (for redacting)
    pub fn get_process_frame(&mut self) -> &mut Vec<u8> {
        &mut self.frames[self.process_idx]
    }

    /// Mark process frame as ready for output and rotate
    pub fn mark_process_done_and_rotate(&mut self) {
        self.states[self.process_idx] = FrameState::ReadyToOutput;
        self.process_idx = (self.process_idx + 1) % NUM_FRAMES;
    }

    /// Get reference to output frame (for writing)
    pub fn get_output_frame(&self) -> &Vec<u8> {
        &self.frames[self.write_idx]
    }

    /// Mark output frame as written and available for next input
    pub fn mark_written_and_rotate(&mut self) {
        self.states[self.write_idx] = FrameState::Available;
        self.write_idx = (self.write_idx + 1) % NUM_FRAMES;
    }

    /// Get current state of a frame by index
    pub fn get_state(&self, idx: usize) -> FrameState {
        self.states[idx]
    }

    /// Total capacity of all frames
    pub fn total_capacity(&self) -> usize {
        FRAME_SIZE * NUM_FRAMES
    }

    /// Get frame size
    pub fn frame_size(&self) -> usize {
        FRAME_SIZE
    }

    /// Get number of frames
    pub fn num_frames(&self) -> usize {
        NUM_FRAMES
    }

    /// Clear all frames (resets to initial state)
    pub fn clear(&mut self) {
        for frame in &mut self.frames {
            frame.clear();
        }
        for state in &mut self.states {
            *state = FrameState::Available;
        }
        self.read_idx = 0;
        self.process_idx = 1 % NUM_FRAMES;
        self.write_idx = 2 % NUM_FRAMES;
    }
}

impl<const FRAME_SIZE: usize, const NUM_FRAMES: usize> Default for FrameRing<FRAME_SIZE, NUM_FRAMES> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_ring_creation() {
        let ring: FrameRing<1024, 3> = FrameRing::new();
        assert_eq!(ring.num_frames(), 3);
        assert_eq!(ring.frame_size(), 1024);
        assert_eq!(ring.total_capacity(), 3072);
    }

    #[test]
    fn test_frame_ring_no_allocation() {
        let mut ring: FrameRing<{ 64 * 1024 }, 3> = FrameRing::new();
        
        // Frames are pre-allocated, so mutations should not allocate
        let frame = ring.get_read_frame();
        assert!(frame.capacity() >= 64 * 1024);
    }

    #[test]
    fn test_frame_rotation() {
        let mut ring: FrameRing<100, 3> = FrameRing::new();
        
        // Fill frame 0
        let f0 = ring.get_read_frame();
        f0.extend_from_slice(b"frame0");
        ring.mark_ready_and_rotate_read();
        
        // Fill frame 1
        let f1 = ring.get_read_frame();
        f1.extend_from_slice(b"frame1");
        ring.mark_ready_and_rotate_read();
        
        // Fill frame 2
        let f2 = ring.get_read_frame();
        f2.extend_from_slice(b"frame2");
        ring.mark_ready_and_rotate_read();
        
        // Next should wrap to frame 0 again
        let f0_again = ring.get_read_frame();
        assert_eq!(f0_again.as_slice(), b"frame0");
    }

    #[test]
    fn test_frame_pipeline() {
        let mut ring: FrameRing<256, 3> = FrameRing::new();
        
        // Fill frame 0 with input
        let f0 = ring.get_read_frame();
        f0.extend_from_slice(b"input");
        ring.mark_ready_and_rotate_read();
        
        // Frame indices have rotated:
        // read_idx: 0 → 1
        // process_idx: 1 → next get_process_frame() will return frame 1
        // write_idx: 2 (unchanged)
        
        // Get next frame to read (should be frame 1, which is empty)
        let f1 = ring.get_read_frame();
        assert!(f1.is_empty(), "Frame 1 should be empty");
        f1.extend_from_slice(b"input2");
        ring.mark_ready_and_rotate_read();
        
        // Now read_idx is 2, and frames are: f0=input, f1=input2, f2=empty
        // This verifies rotation works correctly
    }
}
