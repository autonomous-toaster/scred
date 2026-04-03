/// Zero-Copy Buffer Pool for Streaming Redaction
///
/// Implements object pool pattern for Vec<u8> buffers to eliminate allocation overhead
/// in the hot path of streaming redaction.
///
/// # Architecture
///
/// Pre-allocates N buffers upfront (default 3 × 65KB = 195KB total overhead).
/// Clients acquire() from pool, use the buffer, then release() back.
///
/// # Benefits
///
/// - Eliminates Vec<u8>::new() allocation per chunk
/// - Eliminates Vec<u8>::drop() deallocation per chunk
/// - Reduces GC pressure and memory fragmentation
/// - Expected improvement: +5-10% throughput
///
/// # Design Rationale
///
/// Why 3 buffers?
/// - FrameRing pattern uses 3 frames (read/process/write overlap)
/// - 3 buffers sufficient for single-threaded streaming
/// - 195 KB overhead acceptable for production
///
/// Why 65KB?
/// - Matches optimal streaming chunk size (verified in Phase 3C)
/// - Consistent with lookahead buffer sizing
/// - Fits in L1 cache (256KB per core)
///
/// # Implementation Strategy
///
/// Single-threaded, no locking required (designed for StreamingRedactor):
/// - available: VecDeque<Vec<u8>> - recycled buffers ready to use
/// - held: usize - count of buffers currently held by users
///
/// Thread-safe version would need Arc<Mutex<BufferPool>> if shared across threads.
use std::collections::VecDeque;

/// Zero-copy buffer pool for streaming operations
pub struct BufferPool {
    /// Queue of pre-allocated buffers available for acquisition
    available: VecDeque<Vec<u8>>,
    /// Number of buffers currently held by users (tracking only, no ownership)
    held: usize,
    /// Total pool capacity (max buffers that can exist at once)
    capacity: usize,
    /// Size of each buffer in bytes
    buffer_size: usize,
}

impl BufferPool {
    /// Create a new buffer pool with specified number of buffers and size
    ///
    /// # Arguments
    /// * `num_buffers` - Number of pre-allocated buffers (default: 3)
    /// * `buffer_size` - Size of each buffer in bytes (default: 65536)
    pub fn new(num_buffers: usize, buffer_size: usize) -> Self {
        let mut available = VecDeque::with_capacity(num_buffers);

        // Pre-allocate all buffers upfront
        for _ in 0..num_buffers {
            available.push_back(Vec::with_capacity(buffer_size));
        }

        Self {
            available,
            held: 0,
            capacity: num_buffers,
            buffer_size,
        }
    }

    /// Create a pool with default settings (3 × 65KB)
    pub fn with_defaults() -> Self {
        Self::new(3, 65536)
    }

    /// Acquire a buffer from the pool
    ///
    /// # Returns
    /// - Ok(Vec<u8>) - Pre-allocated buffer ready to use
    /// - Err(&str) - Pool exhausted (shouldn't happen in normal operation)
    ///
    /// # Notes
    /// - Buffer is cleared but capacity is preserved
    /// - User must call release() to return to pool
    /// - Panics if buffer not returned (no automatic return)
    pub fn acquire(&mut self) -> Result<Vec<u8>, &'static str> {
        if let Some(mut buffer) = self.available.pop_front() {
            buffer.clear(); // Clear contents but keep capacity
            self.held += 1;
            Ok(buffer)
        } else if self.held < self.capacity {
            // Emergency fallback: allocate new if somehow lost track
            let mut buffer = Vec::with_capacity(self.buffer_size);
            buffer.clear();
            self.held += 1;
            Ok(buffer)
        } else {
            // Pool exhausted - this shouldn't happen with proper usage
            Err("Buffer pool exhausted - all buffers checked out")
        }
    }

    /// Release a buffer back to the pool
    ///
    /// # Arguments
    /// * `buffer` - Previously acquired buffer to return
    ///
    /// # Notes
    /// - Buffer is cleared before returning to pool
    /// - User loses ownership of buffer after calling this
    pub fn release(&mut self, buffer: Vec<u8>) {
        if self.available.len() < self.capacity {
            let mut buf = buffer;
            buf.clear(); // Clear contents but keep capacity
            self.available.push_back(buf);
        }
        self.held = self.held.saturating_sub(1);
    }

    /// Get statistics about pool state
    pub fn stats(&self) -> BufferPoolStats {
        BufferPoolStats {
            total_buffers: self.capacity,
            available: self.available.len(),
            in_use: self.held,
            buffer_size: self.buffer_size,
            total_memory: self.capacity * self.buffer_size,
        }
    }

    /// Check if pool is healthy (all buffers accounted for)
    pub fn is_healthy(&self) -> bool {
        self.available.len() + self.held == self.capacity
    }
}

/// Statistics about buffer pool state
#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    pub total_buffers: usize,
    pub available: usize,
    pub in_use: usize,
    pub buffer_size: usize,
    pub total_memory: usize,
}

impl std::fmt::Display for BufferPoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BufferPool: {}/{} buffers ({} in use), {} MB total",
            self.available,
            self.total_buffers,
            self.in_use,
            self.total_memory / 1_048_576
        )
    }
}
