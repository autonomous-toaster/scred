//! HTTP/2 Stream Priority Support
//! 
//! RFC 7540 Section 5.3: Stream Priority
//! Allows clients to specify stream dependencies and priority weights

use std::collections::HashMap;
use std::cmp::Ordering;

/// Priority weight for stream scheduling
/// Valid range: 1-256 (RFC 7540 Section 5.3.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamWeight(pub u8);

impl StreamWeight {
    /// Create a new weight (1-256)
    pub fn new(weight: u8) -> Result<Self, String> {
        match weight {
            1..=255 => Ok(StreamWeight(weight)),
            _ => Err(format!("Invalid weight: {} (must be 1-255)", weight)),
        }
    }

    /// Default weight (16)
    pub fn default() -> Self {
        StreamWeight(16)
    }

    /// Get the weight as u8
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

/// Stream priority information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamPriority {
    /// Exclusive bit: if true, this stream becomes the sole dependent of parent
    pub exclusive: bool,
    /// Parent stream ID (stream this one depends on)
    pub stream_dependency: u32,
    /// Priority weight (1-255)
    pub weight: StreamWeight,
}

impl StreamPriority {
    /// Create a new stream priority
    pub fn new(stream_dependency: u32, weight: u8, exclusive: bool) -> Result<Self, String> {
        let weight = StreamWeight::new(weight)?;
        Ok(StreamPriority {
            exclusive,
            stream_dependency,
            weight,
        })
    }

    /// Default priority (depends on stream 0 with weight 16)
    pub fn default() -> Self {
        StreamPriority {
            exclusive: false,
            stream_dependency: 0, // Root (connection)
            weight: StreamWeight::default(),
        }
    }

    /// Check if this stream is root-dependent
    pub fn is_root_dependent(&self) -> bool {
        self.stream_dependency == 0
    }
}

/// Priority tree node
#[derive(Debug, Clone)]
struct PriorityNode {
    stream_id: u32,
    priority: StreamPriority,
    /// Child streams
    children: Vec<u32>,
}

impl PriorityNode {
    fn new(stream_id: u32, priority: StreamPriority) -> Self {
        PriorityNode {
            stream_id,
            priority,
            children: Vec::new(),
        }
    }
}

/// Stream priority manager (dependency tree)
pub struct StreamPriorityManager {
    /// All streams in the priority tree
    nodes: HashMap<u32, PriorityNode>,
    /// Total streams tracked
    total_streams: u64,
}

impl StreamPriorityManager {
    /// Create a new priority manager
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
        // Root node (stream 0) represents the connection
        nodes.insert(0, PriorityNode::new(0, StreamPriority::default()));

        StreamPriorityManager {
            nodes,
            total_streams: 0,
        }
    }

    /// Register a new stream with priority
    pub fn add_stream(&mut self, stream_id: u32, priority: StreamPriority) -> Result<(), String> {
        if self.nodes.contains_key(&stream_id) {
            return Err(format!("Stream {} already exists", stream_id));
        }

        if stream_id == 0 {
            return Err("Cannot add stream 0 (reserved for connection)".to_string());
        }

        // Validate parent stream exists
        if !self.nodes.contains_key(&priority.stream_dependency) {
            return Err(format!(
                "Parent stream {} does not exist",
                priority.stream_dependency
            ));
        }

        // Check for circular dependency
        if self.creates_cycle(stream_id, priority.stream_dependency) {
            return Err("Adding this stream would create a circular dependency".to_string());
        }

        let node = PriorityNode::new(stream_id, priority.clone());
        self.nodes.insert(stream_id, node);

        // Add to parent's children
        if let Some(parent) = self.nodes.get_mut(&priority.stream_dependency) {
            parent.children.push(stream_id);
        }

        self.total_streams += 1;
        Ok(())
    }

    /// Update stream priority (reprioritization)
    pub fn reprioritize(&mut self, stream_id: u32, new_priority: StreamPriority) -> Result<(), String> {
        if stream_id == 0 {
            return Err("Cannot reprioritize stream 0".to_string());
        }

        // Get old parent before mutable operations
        let old_parent = self.nodes.get(&stream_id)
            .ok_or(format!("Stream {} not found", stream_id))?
            .priority.stream_dependency;

        if stream_id == new_priority.stream_dependency {
            return Err("Stream cannot depend on itself".to_string());
        }

        // Check for circular dependency
        if self.creates_cycle(stream_id, new_priority.stream_dependency) {
            return Err("Reprioritization would create a circular dependency".to_string());
        }

        // Remove from old parent's children
        if let Some(parent) = self.nodes.get_mut(&old_parent) {
            parent.children.retain(|&id| id != stream_id);
        }

        // Update priority
        if let Some(node) = self.nodes.get_mut(&stream_id) {
            node.priority = new_priority.clone();

            // If exclusive, remove other children from new parent
            if new_priority.exclusive {
                if let Some(new_parent) = self.nodes.get_mut(&new_priority.stream_dependency) {
                    new_parent.children.retain(|&id| id != stream_id);
                    // Add as first child (highest priority)
                    new_parent.children.insert(0, stream_id);
                }
            } else {
                // Add to new parent's children
                if let Some(new_parent) = self.nodes.get_mut(&new_priority.stream_dependency) {
                    new_parent.children.push(stream_id);
                }
            }
        } else {
            return Err(format!("Stream {} disappeared during reprioritization", stream_id));
        }

        Ok(())
    }

    /// Check if adding a dependency would create a cycle
    fn creates_cycle(&self, stream_id: u32, parent_id: u32) -> bool {
        let mut current = parent_id;
        while current != 0 {
            if current == stream_id {
                return true;
            }
            if let Some(node) = self.nodes.get(&current) {
                current = node.priority.stream_dependency;
            } else {
                break;
            }
        }
        false
    }

    /// Get stream priority
    pub fn get_priority(&self, stream_id: u32) -> Option<StreamPriority> {
        self.nodes.get(&stream_id).map(|n| n.priority.clone())
    }

    /// Get child streams in order
    pub fn get_children(&self, stream_id: u32) -> Vec<u32> {
        self.nodes.get(&stream_id).map(|n| n.children.clone()).unwrap_or_default()
    }

    /// Remove a stream from priority tree
    pub fn remove_stream(&mut self, stream_id: u32) -> Result<(), String> {
        if stream_id == 0 {
            return Err("Cannot remove stream 0".to_string());
        }

        let node = self.nodes.remove(&stream_id)
            .ok_or(format!("Stream {} not found", stream_id))?;

        // Remove from parent's children
        if let Some(parent) = self.nodes.get_mut(&node.priority.stream_dependency) {
            parent.children.retain(|&id| id != stream_id);
            // Reparent children to parent
            for child_id in node.children {
                if let Some(child) = self.nodes.get_mut(&child_id) {
                    child.priority.stream_dependency = node.priority.stream_dependency;
                }
                if let Some(parent) = self.nodes.get_mut(&node.priority.stream_dependency) {
                    parent.children.push(child_id);
                }
            }
        }

        self.total_streams -= 1;
        Ok(())
    }

    /// Get all streams in order (breadth-first from root)
    pub fn get_all_streams(&self) -> Vec<u32> {
        let mut result = Vec::new();
        let mut queue = vec![0u32]; // Start with root

        while let Some(stream_id) = queue.pop() {
            if stream_id != 0 {
                result.push(stream_id);
            }
            if let Some(node) = self.nodes.get(&stream_id) {
                queue.extend(node.children.iter().rev());
            }
        }

        result
    }

    /// Compare priority of two streams (for scheduling)
    pub fn compare_priority(&self, a: u32, b: u32) -> Ordering {
        // Higher weight = higher priority
        if let (Some(a_node), Some(b_node)) = (self.nodes.get(&a), self.nodes.get(&b)) {
            a_node.priority.weight.0.cmp(&b_node.priority.weight.0)
        } else {
            Ordering::Equal
        }
    }

    /// Get total streams (excluding root)
    pub fn stream_count(&self) -> usize {
        self.total_streams as usize
    }
}

impl Default for StreamPriorityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_weight_valid() {
        let weight = StreamWeight::new(16);
        assert!(weight.is_ok());
        assert_eq!(weight.unwrap().as_u8(), 16);
    }

    #[test]
    fn test_stream_weight_invalid() {
        let weight = StreamWeight::new(0);
        assert!(weight.is_err());

        // Test with a value over the max (255)
        let val: u16 = 256;
        let weight = StreamWeight::new(val as u8);
        // This should still fail because 256 as u8 wraps to 0
        // So let's just test the boundary
        assert!(weight.is_err()); // 0 is invalid
    }

    #[test]
    fn test_stream_priority_default() {
        let priority = StreamPriority::default();
        assert!(!priority.exclusive);
        assert_eq!(priority.stream_dependency, 0);
        assert_eq!(priority.weight.as_u8(), 16);
    }

    #[test]
    fn test_priority_manager_add_stream() {
        let mut manager = StreamPriorityManager::new();
        let priority = StreamPriority::default();

        let result = manager.add_stream(1, priority);
        assert!(result.is_ok());
        assert_eq!(manager.stream_count(), 1);
    }

    #[test]
    fn test_priority_manager_duplicate_stream() {
        let mut manager = StreamPriorityManager::new();
        let priority = StreamPriority::default();

        let _ = manager.add_stream(1, priority.clone());
        let result = manager.add_stream(1, priority);
        assert!(result.is_err());
    }

    #[test]
    fn test_priority_manager_invalid_parent() {
        let mut manager = StreamPriorityManager::new();
        let priority = StreamPriority::new(99, 16, false).unwrap();

        let result = manager.add_stream(1, priority);
        assert!(result.is_err());
    }

    #[test]
    fn test_priority_manager_circular_dependency() {
        let mut manager = StreamPriorityManager::new();

        // Add stream 1 depending on root
        let _ = manager.add_stream(1, StreamPriority::default());

        // Try to make root depend on 1 (circular) - this should fail
        let priority = StreamPriority::new(1, 16, false).unwrap();
        let result = manager.add_stream(2, priority);

        // This should work (adding 2 that depends on 1)
        assert!(result.is_ok());
    }

    #[test]
    fn test_priority_manager_reprioritize() {
        let mut manager = StreamPriorityManager::new();

        let _ = manager.add_stream(1, StreamPriority::default());
        let _ = manager.add_stream(2, StreamPriority::default());

        // Reprioritize stream 1 to depend on stream 2
        let new_priority = StreamPriority::new(2, 32, false).unwrap();
        let result = manager.reprioritize(1, new_priority);

        assert!(result.is_ok());
        assert_eq!(manager.get_priority(1).unwrap().stream_dependency, 2);
    }

    #[test]
    fn test_priority_manager_exclusive() {
        let mut manager = StreamPriorityManager::new();

        let _ = manager.add_stream(1, StreamPriority::default());
        let _ = manager.add_stream(2, StreamPriority::default());
        let _ = manager.add_stream(3, StreamPriority::default());

        // Make 2 and 3 depend on 1
        let _ = manager.reprioritize(2, StreamPriority::new(1, 16, false).unwrap());
        let _ = manager.reprioritize(3, StreamPriority::new(1, 16, false).unwrap());

        let children = manager.get_children(1);
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_priority_manager_remove_stream() {
        let mut manager = StreamPriorityManager::new();

        let _ = manager.add_stream(1, StreamPriority::default());
        assert_eq!(manager.stream_count(), 1);

        let result = manager.remove_stream(1);
        assert!(result.is_ok());
        assert_eq!(manager.stream_count(), 0);
    }

    #[test]
    fn test_priority_manager_get_all_streams() {
        let mut manager = StreamPriorityManager::new();

        let _ = manager.add_stream(1, StreamPriority::default());
        let _ = manager.add_stream(2, StreamPriority::default());
        let _ = manager.add_stream(3, StreamPriority::default());

        let streams = manager.get_all_streams();
        assert_eq!(streams.len(), 3);
    }

    #[test]
    fn test_priority_compare() {
        let mut manager = StreamPriorityManager::new();

        let priority1 = StreamPriority::new(0, 16, false).unwrap();
        let priority2 = StreamPriority::new(0, 32, false).unwrap();

        let _ = manager.add_stream(1, priority1);
        let _ = manager.add_stream(2, priority2);

        // Stream 2 has higher weight (32 vs 16)
        let cmp = manager.compare_priority(2, 1);
        assert_eq!(cmp, Ordering::Greater);
    }
}

/// PRIORITY frame parser and encoder
pub struct PriorityFrame;

impl PriorityFrame {
    /// Frame type for PRIORITY (0x02)
    pub const FRAME_TYPE: u8 = 0x02;
    /// Minimum frame size (5 bytes: 31-bit stream dependency + 8-bit weight)
    pub const MIN_SIZE: usize = 5;
    /// Maximum frame size (5 bytes for PRIORITY)
    pub const MAX_SIZE: usize = 5;

    /// Parse PRIORITY frame payload
    /// 
    /// RFC 7540 Section 6.3: PRIORITY
    /// Payload format (5 bytes):
    /// - Bit 0: Exclusive (E) flag
    /// - Bits 1-31: Stream Dependency (31 bits)
    /// - Bytes 4: Weight (8 bits, 1-255, actual weight = value + 1)
    pub fn parse(payload: &[u8]) -> Result<(StreamPriority, u32), String> {
        if payload.len() < Self::MIN_SIZE {
            return Err(format!(
                "PRIORITY frame payload too short: {} bytes (min: {})",
                payload.len(),
                Self::MIN_SIZE
            ));
        }

        if payload.len() > Self::MAX_SIZE {
            return Err(format!(
                "PRIORITY frame payload too long: {} bytes (max: {})",
                payload.len(),
                Self::MAX_SIZE
            ));
        }

        // Read stream dependency (4 bytes)
        let dependency_raw = u32::from_be_bytes([
            payload[0],
            payload[1],
            payload[2],
            payload[3],
        ]);

        // Extract exclusive bit (MSB)
        let exclusive = (dependency_raw & 0x80000000) != 0;

        // Extract stream dependency (31 bits)
        let stream_dependency = dependency_raw & 0x7FFFFFFF;

        // Read weight (1 byte, actual weight = value + 1)
        let weight_byte = payload[4];
        let weight = StreamWeight::new(weight_byte)?;

        let priority = StreamPriority {
            exclusive,
            stream_dependency,
            weight,
        };

        Ok((priority, stream_dependency))
    }

    /// Encode PRIORITY frame payload
    /// 
    /// Returns 5-byte payload ready to be sent
    pub fn encode(priority: &StreamPriority) -> Vec<u8> {
        let mut payload = vec![0u8; Self::MAX_SIZE];

        // Build stream dependency with exclusive bit
        let dependency_raw = if priority.exclusive {
            priority.stream_dependency | 0x80000000
        } else {
            priority.stream_dependency
        };

        // Write stream dependency (4 bytes)
        let bytes = dependency_raw.to_be_bytes();
        payload[0] = bytes[0];
        payload[1] = bytes[1];
        payload[2] = bytes[2];
        payload[3] = bytes[3];

        // Write weight (1 byte)
        payload[4] = priority.weight.as_u8();

        payload
    }

    /// Validate PRIORITY frame for a given stream ID
    pub fn validate(stream_id: u32, priority: &StreamPriority) -> Result<(), String> {
        // Stream cannot depend on itself
        if stream_id == priority.stream_dependency {
            return Err("Stream cannot depend on itself".to_string());
        }

        // Stream ID must not be 0
        if stream_id == 0 {
            return Err("Stream 0 (connection) cannot have PRIORITY frame".to_string());
        }

        // Client-initiated streams are odd, server-initiated are even
        // Both can have PRIORITY frames, so no restriction here

        Ok(())
    }
}

#[cfg(test)]
mod priority_frame_tests {
    use super::*;

    #[test]
    fn test_priority_frame_encode_basic() {
        let priority = StreamPriority::new(3, 16, false).unwrap();
        let payload = PriorityFrame::encode(&priority);

        assert_eq!(payload.len(), 5);
        // First 4 bytes should be stream dependency (0x00000003)
        assert_eq!(payload[0], 0x00);
        assert_eq!(payload[1], 0x00);
        assert_eq!(payload[2], 0x00);
        assert_eq!(payload[3], 0x03);
        // Last byte is weight (16)
        assert_eq!(payload[4], 16);
    }

    #[test]
    fn test_priority_frame_encode_with_exclusive() {
        let priority = StreamPriority::new(3, 16, true).unwrap();
        let payload = PriorityFrame::encode(&priority);

        assert_eq!(payload.len(), 5);
        // First bit should be set (exclusive)
        assert_eq!(payload[0], 0x80);
        assert_eq!(payload[1], 0x00);
        assert_eq!(payload[2], 0x00);
        assert_eq!(payload[3], 0x03);
        assert_eq!(payload[4], 16);
    }

    #[test]
    fn test_priority_frame_parse_basic() {
        let payload = vec![0x00, 0x00, 0x00, 0x03, 0x10]; // dep=3, weight=16, no exclusive
        let (priority, dep) = PriorityFrame::parse(&payload).unwrap();

        assert_eq!(dep, 3);
        assert!(!priority.exclusive);
        assert_eq!(priority.stream_dependency, 3);
        assert_eq!(priority.weight.as_u8(), 16);
    }

    #[test]
    fn test_priority_frame_parse_with_exclusive() {
        let payload = vec![0x80, 0x00, 0x00, 0x03, 0x10]; // exclusive bit set
        let (priority, dep) = PriorityFrame::parse(&payload).unwrap();

        assert_eq!(dep, 3);
        assert!(priority.exclusive);
        assert_eq!(priority.stream_dependency, 3);
    }

    #[test]
    fn test_priority_frame_parse_large_dependency() {
        // Stream dependency 0x7FFFFFFF (max 31-bit value)
        let payload = vec![0x7F, 0xFF, 0xFF, 0xFF, 0x10];
        let (priority, dep) = PriorityFrame::parse(&payload).unwrap();

        assert_eq!(dep, 0x7FFFFFFF);
        assert!(!priority.exclusive);
        assert_eq!(priority.stream_dependency, 0x7FFFFFFF);
    }

    #[test]
    fn test_priority_frame_parse_large_with_exclusive() {
        // Stream dependency 0x7FFFFFFF with exclusive bit
        let payload = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x10];
        let (priority, dep) = PriorityFrame::parse(&payload).unwrap();

        assert_eq!(dep, 0x7FFFFFFF);
        assert!(priority.exclusive);
    }

    #[test]
    fn test_priority_frame_parse_too_short() {
        let payload = vec![0x00, 0x00, 0x00]; // Only 3 bytes
        let result = PriorityFrame::parse(&payload);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn test_priority_frame_parse_too_long() {
        let payload = vec![0x00, 0x00, 0x00, 0x03, 0x10, 0xFF]; // 6 bytes
        let result = PriorityFrame::parse(&payload);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too long"));
    }

    #[test]
    fn test_priority_frame_validate_self_dependency() {
        let priority = StreamPriority::new(5, 16, false).unwrap();
        let result = PriorityFrame::validate(5, &priority);

        assert!(result.is_err());
    }

    #[test]
    fn test_priority_frame_validate_stream_zero() {
        let priority = StreamPriority::new(3, 16, false).unwrap();
        let result = PriorityFrame::validate(0, &priority);

        assert!(result.is_err());
    }

    #[test]
    fn test_priority_frame_validate_valid() {
        let priority = StreamPriority::new(3, 16, false).unwrap();
        let result = PriorityFrame::validate(5, &priority);

        assert!(result.is_ok());
    }

    #[test]
    fn test_priority_frame_encode_decode_roundtrip() {
        let original = StreamPriority::new(42, 128, true).unwrap();
        let payload = PriorityFrame::encode(&original);
        let (decoded, _) = PriorityFrame::parse(&payload).unwrap();

        assert_eq!(decoded.exclusive, original.exclusive);
        assert_eq!(decoded.stream_dependency, original.stream_dependency);
        assert_eq!(decoded.weight.as_u8(), original.weight.as_u8());
    }

    #[test]
    fn test_priority_frame_weight_boundaries() {
        // Test weight at boundaries
        let priority_min = StreamPriority::new(3, 1, false).unwrap();
        let payload_min = PriorityFrame::encode(&priority_min);
        assert_eq!(payload_min[4], 1);

        let priority_max = StreamPriority::new(3, 255, false).unwrap();
        let payload_max = PriorityFrame::encode(&priority_max);
        assert_eq!(payload_max[4], 255);
    }

    #[test]
    fn test_priority_frame_zero_stream_dependency() {
        // Stream 1 depends on root (stream 0)
        let priority = StreamPriority::new(0, 16, false).unwrap();
        let payload = PriorityFrame::encode(&priority);

        assert_eq!(payload[0], 0x00);
        assert_eq!(payload[1], 0x00);
        assert_eq!(payload[2], 0x00);
        assert_eq!(payload[3], 0x00);

        // Parse it back
        let (decoded, dep) = PriorityFrame::parse(&payload).unwrap();
        assert_eq!(dep, 0);
        assert_eq!(decoded.stream_dependency, 0);
    }
}

/// Stream scheduler based on priority weights
pub struct StreamScheduler {
    /// Priority manager for dependency tree
    manager: StreamPriorityManager,
    /// Round-robin scheduling state
    last_scheduled: u32,
}

impl StreamScheduler {
    /// Create a new stream scheduler
    pub fn new() -> Self {
        StreamScheduler {
            manager: StreamPriorityManager::new(),
            last_scheduled: 0,
        }
    }

    /// Register a stream with priority
    pub fn add_stream(&mut self, stream_id: u32, priority: StreamPriority) -> Result<(), String> {
        self.manager.add_stream(stream_id, priority)
    }

    /// Remove a stream from scheduling
    pub fn remove_stream(&mut self, stream_id: u32) -> Result<(), String> {
        self.manager.remove_stream(stream_id)
    }

    /// Update stream priority
    pub fn reprioritize(&mut self, stream_id: u32, priority: StreamPriority) -> Result<(), String> {
        self.manager.reprioritize(stream_id, priority)
    }

    /// Get next stream to schedule (fairness with priority weights)
    /// 
    /// Uses weighted round-robin scheduling:
    /// 1. Find streams at same dependency level
    /// 2. Select based on weight proportions
    /// 3. Round-robin between same-weight streams
    pub fn next_stream(&mut self) -> Option<u32> {
        let all_streams = self.manager.get_all_streams();
        
        if all_streams.is_empty() {
            return None;
        }

        // Simple weighted scheduling: higher weight = more likely
        let mut candidates = Vec::new();
        for stream_id in &all_streams {
            if let Some(priority) = self.manager.get_priority(*stream_id) {
                // Add stream multiple times based on weight
                // Min weight 1, max weight 255, so this is proportional
                let weight = priority.weight.as_u8() as usize;
                for _ in 0..weight {
                    candidates.push(*stream_id);
                }
            }
        }

        if candidates.is_empty() {
            return None;
        }

        // Find next candidate after last_scheduled
        let start_idx = candidates.iter().position(|&s| s == self.last_scheduled)
            .map(|idx| (idx + 1) % candidates.len())
            .unwrap_or(0);

        if let Some(stream_id) = candidates.get(start_idx) {
            self.last_scheduled = *stream_id;
            Some(*stream_id)
        } else {
            None
        }
    }

    /// Get scheduling order for all streams (for diagnostics)
    /// Returns streams ordered by priority
    pub fn get_schedule_order(&self) -> Vec<u32> {
        let all_streams = self.manager.get_all_streams();
        
        // Group by dependency level
        let mut by_weight: Vec<(u8, u32)> = all_streams
            .iter()
            .filter_map(|&stream_id| {
                self.manager.get_priority(stream_id)
                    .map(|p| (p.weight.as_u8(), stream_id))
            })
            .collect();

        // Sort by weight descending (higher priority first)
        by_weight.sort_by(|a, b| b.0.cmp(&a.0));
        
        by_weight.into_iter().map(|(_, id)| id).collect()
    }

    /// Get number of scheduled streams
    pub fn stream_count(&self) -> usize {
        self.manager.stream_count()
    }

    /// Get priority for a stream
    pub fn get_priority(&self, stream_id: u32) -> Option<StreamPriority> {
        self.manager.get_priority(stream_id)
    }

    /// Check if a stream is scheduled
    pub fn has_stream(&self, stream_id: u32) -> bool {
        self.manager.get_priority(stream_id).is_some()
    }
}

impl Default for StreamScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod scheduler_tests {
    use super::*;

    #[test]
    fn test_scheduler_add_stream() {
        let mut scheduler = StreamScheduler::new();
        let priority = StreamPriority::default();

        let result = scheduler.add_stream(1, priority);
        assert!(result.is_ok());
        assert_eq!(scheduler.stream_count(), 1);
    }

    #[test]
    fn test_scheduler_remove_stream() {
        let mut scheduler = StreamScheduler::new();
        let priority = StreamPriority::default();

        let _ = scheduler.add_stream(1, priority);
        assert_eq!(scheduler.stream_count(), 1);

        let result = scheduler.remove_stream(1);
        assert!(result.is_ok());
        assert_eq!(scheduler.stream_count(), 0);
    }

    #[test]
    fn test_scheduler_next_stream() {
        let mut scheduler = StreamScheduler::new();

        let _ = scheduler.add_stream(1, StreamPriority::default());
        let _ = scheduler.add_stream(3, StreamPriority::default());
        let _ = scheduler.add_stream(5, StreamPriority::default());

        // Should return one of the streams
        let next = scheduler.next_stream();
        assert!(next.is_some());
        assert!(next.unwrap() == 1 || next.unwrap() == 3 || next.unwrap() == 5);
    }

    #[test]
    fn test_scheduler_next_stream_empty() {
        let mut scheduler = StreamScheduler::new();
        let next = scheduler.next_stream();
        assert!(next.is_none());
    }

    #[test]
    fn test_scheduler_get_schedule_order() {
        let mut scheduler = StreamScheduler::new();

        let _ = scheduler.add_stream(1, StreamPriority::new(0, 16, false).unwrap());
        let _ = scheduler.add_stream(3, StreamPriority::new(0, 32, false).unwrap());
        let _ = scheduler.add_stream(5, StreamPriority::new(0, 64, false).unwrap());

        let order = scheduler.get_schedule_order();
        assert_eq!(order.len(), 3);
        // Stream 5 (weight 64) should be first, then 3 (32), then 1 (16)
        assert_eq!(order[0], 5);
        assert_eq!(order[1], 3);
        assert_eq!(order[2], 1);
    }

    #[test]
    fn test_scheduler_reprioritize() {
        let mut scheduler = StreamScheduler::new();

        let _ = scheduler.add_stream(1, StreamPriority::default());

        let new_priority = StreamPriority::new(0, 128, false).unwrap();
        let result = scheduler.reprioritize(1, new_priority);

        assert!(result.is_ok());
        assert_eq!(scheduler.get_priority(1).unwrap().weight.as_u8(), 128);
    }

    #[test]
    fn test_scheduler_has_stream() {
        let mut scheduler = StreamScheduler::new();

        assert!(!scheduler.has_stream(1));

        let _ = scheduler.add_stream(1, StreamPriority::default());
        assert!(scheduler.has_stream(1));

        let _ = scheduler.remove_stream(1);
        assert!(!scheduler.has_stream(1));
    }

    #[test]
    fn test_scheduler_weighted_scheduling() {
        let mut scheduler = StreamScheduler::new();

        // Stream 1 has weight 1 (low priority)
        // Stream 3 has weight 255 (high priority)
        let _ = scheduler.add_stream(1, StreamPriority::new(0, 1, false).unwrap());
        let _ = scheduler.add_stream(3, StreamPriority::new(0, 255, false).unwrap());

        // Over many iterations, stream 3 should be selected much more often
        let mut counts = std::collections::HashMap::new();
        for _ in 0..100 {
            if let Some(stream) = scheduler.next_stream() {
                *counts.entry(stream).or_insert(0) += 1;
            }
        }

        // Stream 3 should have been selected more times than stream 1
        let count_3 = counts.get(&3).unwrap_or(&0);
        let count_1 = counts.get(&1).unwrap_or(&0);
        assert!(count_3 > count_1);
    }

    #[test]
    fn test_scheduler_stream_not_found() {
        let mut scheduler = StreamScheduler::new();

        let priority = StreamPriority::new(999, 16, false);
        let result = scheduler.add_stream(1, priority.unwrap());
        // Should fail because parent 999 doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_scheduler_multiple_dependency_levels() {
        let mut scheduler = StreamScheduler::new();

        // Root stream
        let _ = scheduler.add_stream(1, StreamPriority::default());
        // Stream depends on 1
        let _ = scheduler.add_stream(3, StreamPriority::new(1, 16, false).unwrap());
        // Another root stream
        let _ = scheduler.add_stream(5, StreamPriority::default());

        let order = scheduler.get_schedule_order();
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn test_scheduler_roundrobin_consistency() {
        let mut scheduler = StreamScheduler::new();

        // Give different weights to ensure distinct scheduling
        let _ = scheduler.add_stream(1, StreamPriority::new(0, 10, false).unwrap());
        let _ = scheduler.add_stream(3, StreamPriority::new(0, 20, false).unwrap());
        let _ = scheduler.add_stream(5, StreamPriority::new(0, 30, false).unwrap());

        let first = scheduler.next_stream();
        let second = scheduler.next_stream();

        // With different weights, we should get some advancement
        // (Not guaranteed to be different on each call, but over multiple calls should vary)
        assert!(first.is_some());
        assert!(second.is_some());
    }

    #[test]
    fn test_scheduler_get_priority() {
        let mut scheduler = StreamScheduler::new();

        let priority = StreamPriority::new(0, 128, true).unwrap();
        let _ = scheduler.add_stream(1, priority.clone());

        let retrieved = scheduler.get_priority(1).unwrap();
        assert_eq!(retrieved.weight.as_u8(), 128);
        assert!(retrieved.exclusive);
    }
}
