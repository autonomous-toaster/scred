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

