/// HTTP/2 Flow Control
///
/// Manages flow control windows to prevent deadlock and ensure proper
/// backpressure handling on both connection and stream levels.
///
/// Reference: RFC 9113 Section 5.1.2 (Flow Control)

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Flow control window for HTTP/2
///
/// Default window size per RFC 9113: 65535 bytes
/// Tracks available bytes for sending/receiving data.
pub struct FlowWindow {
    /// Current window size (bytes available)
    available: i32,
    
    /// Initial window size (for reset)
    initial_size: i32,
    
    /// Total bytes consumed
    total_consumed: u64,
}

impl FlowWindow {
    /// Create new flow window with initial size
    pub fn new(initial_size: i32) -> Self {
        Self {
            available: initial_size,
            initial_size,
            total_consumed: 0,
        }
    }

    /// Default window size per RFC 9113 (65535 bytes)
    pub fn default_size() -> i32 {
        65535
    }

    /// Get available bytes
    pub fn available(&self) -> i32 {
        self.available
    }

    /// Consume bytes from window
    /// Returns error if window insufficient
    pub fn consume(&mut self, bytes: u32) -> Result<()> {
        if (bytes as i32) > self.available {
            return Err(anyhow!(
                "Flow control window exhausted: need {}, have {}",
                bytes,
                self.available
            ));
        }

        self.available -= bytes as i32;
        self.total_consumed += bytes as u64;

        debug!(
            "FlowWindow: Consumed {} bytes, {} remaining",
            bytes, self.available
        );

        Ok(())
    }

    /// Increase window size (called on WINDOW_UPDATE)
    pub fn update(&mut self, increment: u32) -> Result<()> {
        let new_size = (self.available as i64) + (increment as i64);

        if new_size > i32::MAX as i64 {
            return Err(anyhow!(
                "Flow control window overflow: current {}, increment {}",
                self.available,
                increment
            ));
        }

        self.available = new_size as i32;

        debug!(
            "FlowWindow: Updated by {}, now {} available",
            increment, self.available
        );

        Ok(())
    }

    /// Get total bytes consumed (for statistics)
    pub fn total_consumed(&self) -> u64 {
        self.total_consumed
    }

    /// Reset window to initial size (on error recovery)
    pub fn reset(&mut self) {
        self.available = self.initial_size;
        debug!("FlowWindow: Reset to {} bytes", self.initial_size);
    }
}

/// Flow control manager for HTTP/2 connection
///
/// Tracks both connection-level and stream-level flow control windows.
pub struct FlowController {
    /// Connection-level window
    connection_window: FlowWindow,

    /// Per-stream windows: HashMap<stream_id, FlowWindow>
    stream_windows: HashMap<u32, FlowWindow>,

    /// Bytes received since last WINDOW_UPDATE (connection level)
    bytes_since_update_conn: u32,

    /// Per-stream bytes received since last WINDOW_UPDATE
    bytes_since_update_stream: HashMap<u32, u32>,

    /// WINDOW_UPDATE threshold: Send UPDATE when consumed > initial_size * threshold
    /// Default: 0.5 (50% of initial window)
    update_threshold: f32,

    /// Statistics: total WINDOW_UPDATE frames sent
    updates_sent: u64,

    /// Statistics: total backpressure events
    backpressure_events: u64,
}

impl FlowController {
    /// Create new flow controller
    pub fn new() -> Self {
        Self {
            connection_window: FlowWindow::new(FlowWindow::default_size()),
            stream_windows: HashMap::new(),
            bytes_since_update_conn: 0,
            bytes_since_update_stream: HashMap::new(),
            update_threshold: 0.5,
            updates_sent: 0,
            backpressure_events: 0,
        }
    }

    /// Create stream window (called when stream created)
    pub fn create_stream(&mut self, stream_id: u32) -> Result<()> {
        if self.stream_windows.contains_key(&stream_id) {
            return Err(anyhow!("Stream {} window already exists", stream_id));
        }

        self.stream_windows
            .insert(stream_id, FlowWindow::new(FlowWindow::default_size()));
        self.bytes_since_update_stream.insert(stream_id, 0);

        debug!("FlowController: Created window for stream {}", stream_id);

        Ok(())
    }

    /// Consume bytes from both connection and stream window
    /// (called when DATA frame received from client)
    pub fn consume_data(
        &mut self,
        stream_id: u32,
        bytes: u32,
    ) -> Result<()> {
        // Consume from connection window
        self.connection_window.consume(bytes)?;
        self.bytes_since_update_conn += bytes;

        // Consume from stream window
        if let Some(window) = self.stream_windows.get_mut(&stream_id) {
            window.consume(bytes)?;
            if let Some(count) = self.bytes_since_update_stream.get_mut(&stream_id) {
                *count += bytes;
            }
        } else {
            return Err(anyhow!("Stream {} window not found", stream_id));
        }

        // Check for backpressure
        if self.connection_window.available() <= 0 {
            warn!(
                "FlowController: Connection window exhausted (backpressure)"
            );
            self.backpressure_events += 1;
        }

        Ok(())
    }

    /// Check if WINDOW_UPDATE needed for connection
    ///
    /// Returns true if enough bytes consumed to warrant update
    pub fn should_update_connection(&self) -> bool {
        let threshold = (FlowWindow::default_size() as f32 * self.update_threshold) as u32;
        self.bytes_since_update_conn >= threshold
    }

    /// Check if WINDOW_UPDATE needed for stream
    pub fn should_update_stream(&self, stream_id: u32) -> bool {
        if let Some(count) = self.bytes_since_update_stream.get(&stream_id) {
            let threshold = (FlowWindow::default_size() as f32 * self.update_threshold) as u32;
            *count >= threshold
        } else {
            false
        }
    }

    /// Generate WINDOW_UPDATE for connection
    ///
    /// Returns increment value to send
    pub fn get_connection_update(&mut self) -> Option<u32> {
        if !self.should_update_connection() {
            return None;
        }

        let increment = self.bytes_since_update_conn;
        self.bytes_since_update_conn = 0;
        self.updates_sent += 1;

        debug!(
            "FlowController: Sending connection WINDOW_UPDATE ({})",
            increment
        );

        Some(increment)
    }

    /// Generate WINDOW_UPDATE for stream
    pub fn get_stream_update(&mut self, stream_id: u32) -> Option<u32> {
        if !self.should_update_stream(stream_id) {
            return None;
        }

        if let Some(count) = self.bytes_since_update_stream.get_mut(&stream_id) {
            let increment = *count;
            *count = 0;
            self.updates_sent += 1;

            debug!(
                "FlowController: Sending stream {} WINDOW_UPDATE ({})",
                stream_id, increment
            );

            Some(increment)
        } else {
            None
        }
    }

    /// Handle WINDOW_UPDATE frame from upstream
    pub fn handle_window_update(
        &mut self,
        stream_id: u32,
        increment: u32,
    ) -> Result<()> {
        if stream_id == 0 {
            // Connection-level update
            self.connection_window.update(increment)?;
        } else {
            // Stream-level update
            if let Some(window) = self.stream_windows.get_mut(&stream_id) {
                window.update(increment)?;
            } else {
                return Err(anyhow!("Stream {} window not found", stream_id));
            }
        }

        Ok(())
    }

    /// Close stream and clean up window
    pub fn close_stream(&mut self, stream_id: u32) {
        self.stream_windows.remove(&stream_id);
        self.bytes_since_update_stream.remove(&stream_id);

        debug!("FlowController: Closed window for stream {}", stream_id);
    }

    /// Get connection window status
    pub fn connection_window_available(&self) -> i32 {
        self.connection_window.available()
    }

    /// Get stream window status
    pub fn stream_window_available(&self, stream_id: u32) -> Option<i32> {
        self.stream_windows
            .get(&stream_id)
            .map(|w| w.available())
    }

    /// Get statistics
    pub fn stats(&self) -> FlowControlStats {
        FlowControlStats {
            connection_window_available: self.connection_window.available(),
            connection_window_consumed: self.connection_window.total_consumed(),
            stream_count: self.stream_windows.len(),
            updates_sent: self.updates_sent,
            backpressure_events: self.backpressure_events,
        }
    }
}

impl Default for FlowController {
    fn default() -> Self {
        Self::new()
    }
}

/// Flow control statistics
#[derive(Debug, Clone)]
pub struct FlowControlStats {
    pub connection_window_available: i32,
    pub connection_window_consumed: u64,
    pub stream_count: usize,
    pub updates_sent: u64,
    pub backpressure_events: u64,
}

impl std::fmt::Display for FlowControlStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FlowControl: {} bytes available, {} consumed, {} streams, {} updates, {} backpressure",
            self.connection_window_available,
            self.connection_window_consumed,
            self.stream_count,
            self.updates_sent,
            self.backpressure_events
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_window_creation() {
        let window = FlowWindow::new(65535);
        assert_eq!(window.available(), 65535);
        assert_eq!(window.total_consumed(), 0);
    }

    #[test]
    fn test_flow_window_consume() {
        let mut window = FlowWindow::new(65535);
        
        window.consume(1000).unwrap();
        assert_eq!(window.available(), 64535);
        assert_eq!(window.total_consumed(), 1000);
    }

    #[test]
    fn test_flow_window_exhaust() {
        let mut window = FlowWindow::new(1000);
        
        let result = window.consume(2000);
        assert!(result.is_err());
        assert_eq!(window.available(), 1000); // Unchanged
    }

    #[test]
    fn test_flow_window_update() {
        let mut window = FlowWindow::new(1000);
        window.consume(500).unwrap();
        
        window.update(500).unwrap();
        assert_eq!(window.available(), 1000);
    }

    #[test]
    fn test_flow_controller_creation() {
        let controller = FlowController::new();
        assert_eq!(controller.connection_window_available(), 65535);
    }

    #[test]
    fn test_flow_controller_stream_creation() {
        let mut controller = FlowController::new();
        
        controller.create_stream(1).unwrap();
        assert_eq!(controller.stream_window_available(1), Some(65535));
    }

    #[test]
    fn test_flow_controller_consume() {
        let mut controller = FlowController::new();
        controller.create_stream(1).unwrap();
        
        controller.consume_data(1, 1000).unwrap();
        assert_eq!(controller.connection_window_available(), 64535);
        assert_eq!(controller.stream_window_available(1), Some(64535));
    }

    #[test]
    fn test_flow_controller_update_threshold() {
        let mut controller = FlowController::new();
        controller.create_stream(1).unwrap();
        
        // Consume 50% of window (32767 bytes)
        let half_window = 32767u32;
        controller.consume_data(1, half_window).unwrap();
        
        assert!(controller.should_update_connection());
        assert!(controller.should_update_stream(1));
        
        let update = controller.get_connection_update();
        assert_eq!(update, Some(half_window));
    }

    #[test]
    fn test_flow_controller_close_stream() {
        let mut controller = FlowController::new();
        controller.create_stream(1).unwrap();
        
        controller.close_stream(1);
        assert_eq!(controller.stream_window_available(1), None);
    }

    #[test]
    fn test_flow_controller_stats() {
        let mut controller = FlowController::new();
        controller.create_stream(1).unwrap();
        
        controller.consume_data(1, 1000).unwrap();
        
        let stats = controller.stats();
        assert_eq!(stats.stream_count, 1);
        assert_eq!(stats.connection_window_consumed, 1000);
    }

    #[test]
    fn test_flow_controller_window_update() {
        let mut controller = FlowController::new();
        
        controller.handle_window_update(0, 5000).unwrap();
        assert_eq!(controller.connection_window_available(), 65535 + 5000);
    }
}
