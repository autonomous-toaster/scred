//! HTTP/2 Server Push (PUSH_PROMISE) support
//! 
//! RFC 7540 Section 6.6: PUSH_PROMISE frame
//! Allows servers to proactively send resources to clients

use crate::h2::frame::{Frame, FrameType};
use std::collections::HashMap;

/// Server push state for a single promised stream
#[derive(Debug, Clone, PartialEq)]
pub enum PushState {
    /// Server promised a resource
    Promised,
    /// Headers received for the promised resource
    HeadersReceived,
    /// Data being received
    ReceivingData,
    /// Push completed successfully
    Completed,
    /// Push rejected (RST_STREAM)
    Rejected,
}

/// Represents a single server-pushed resource
#[derive(Debug, Clone)]
pub struct ServerPush {
    /// Promised stream ID (from PUSH_PROMISE)
    pub promised_stream_id: u32,
    /// Parent stream ID (stream that triggered the push)
    pub parent_stream_id: u32,
    /// Current state of the push
    pub state: PushState,
    /// Promised headers (method, path, etc.)
    pub headers: Vec<(String, String)>,
    /// Response body data
    pub body: Vec<u8>,
    /// Total bytes expected (from Content-Length header if present)
    pub total_size: Option<u64>,
}

impl ServerPush {
    /// Create a new server push
    pub fn new(promised_stream_id: u32, parent_stream_id: u32) -> Self {
        ServerPush {
            promised_stream_id,
            parent_stream_id,
            state: PushState::Promised,
            headers: Vec::new(),
            body: Vec::new(),
            total_size: None,
        }
    }

    /// Add headers to the push
    pub fn add_headers(&mut self, headers: Vec<(String, String)>) {
        self.headers = headers;
        self.state = PushState::HeadersReceived;
        
        // Extract Content-Length if available
        for (name, value) in &self.headers {
            if name.to_lowercase() == "content-length" {
                if let Ok(size) = value.parse::<u64>() {
                    self.total_size = Some(size);
                }
            }
        }
    }

    /// Add body data to the push
    pub fn add_body_data(&mut self, data: &[u8]) -> Result<(), String> {
        if self.state != PushState::HeadersReceived && self.state != PushState::ReceivingData {
            return Err(format!("Cannot add body data in state {:?}", self.state));
        }

        self.body.extend_from_slice(data);
        self.state = PushState::ReceivingData;

        // Check if we've received all data
        if let Some(expected) = self.total_size {
            if self.body.len() as u64 >= expected {
                self.state = PushState::Completed;
            }
        }

        Ok(())
    }

    /// Mark push as completed (END_STREAM received)
    pub fn mark_completed(&mut self) {
        self.state = PushState::Completed;
    }

    /// Mark push as rejected
    pub fn mark_rejected(&mut self) {
        self.state = PushState::Rejected;
    }

    /// Check if push is complete
    pub fn is_complete(&self) -> bool {
        self.state == PushState::Completed
    }

    /// Get promised resource details
    pub fn get_promised_method(&self) -> Option<String> {
        for (name, value) in &self.headers {
            if name == ":method" {
                return Some(value.clone());
            }
        }
        None
    }

    /// Get promised path
    pub fn get_promised_path(&self) -> Option<String> {
        for (name, value) in &self.headers {
            if name == ":path" {
                return Some(value.clone());
            }
        }
        None
    }
}

/// Manager for server push operations
pub struct ServerPushManager {
    /// Map of promised_stream_id -> ServerPush
    pushes: HashMap<u32, ServerPush>,
    /// Counter for total pushes received
    total_pushes: u64,
    /// Counter for completed pushes
    completed_pushes: u64,
    /// Counter for rejected pushes
    rejected_pushes: u64,
}

impl ServerPushManager {
    /// Create a new server push manager
    pub fn new() -> Self {
        ServerPushManager {
            pushes: HashMap::new(),
            total_pushes: 0,
            completed_pushes: 0,
            rejected_pushes: 0,
        }
    }

    /// Register a new server push
    pub fn register_push(&mut self, promised_stream_id: u32, parent_stream_id: u32) -> Result<(), String> {
        if self.pushes.contains_key(&promised_stream_id) {
            return Err(format!("Promised stream {} already exists", promised_stream_id));
        }

        // Promised stream ID must be greater than parent and must be even (server-initiated)
        if promised_stream_id <= parent_stream_id {
            return Err(format!(
                "Invalid promised stream ID: {} (must be > parent {})",
                promised_stream_id, parent_stream_id
            ));
        }

        if promised_stream_id % 2 != 0 {
            return Err(format!("Promised stream ID must be even (server-initiated), got {}", promised_stream_id));
        }

        let push = ServerPush::new(promised_stream_id, parent_stream_id);
        self.pushes.insert(promised_stream_id, push);
        self.total_pushes += 1;

        Ok(())
    }

    /// Add headers to a promised push
    pub fn add_headers_to_push(&mut self, promised_stream_id: u32, headers: Vec<(String, String)>) -> Result<(), String> {
        match self.pushes.get_mut(&promised_stream_id) {
            Some(push) => {
                push.add_headers(headers);
                Ok(())
            }
            None => Err(format!("Promised stream {} not found", promised_stream_id)),
        }
    }

    /// Add body data to a promised push
    pub fn add_body_data(&mut self, promised_stream_id: u32, data: &[u8]) -> Result<(), String> {
        match self.pushes.get_mut(&promised_stream_id) {
            Some(push) => push.add_body_data(data),
            None => Err(format!("Promised stream {} not found", promised_stream_id)),
        }
    }

    /// Mark a push as completed
    pub fn mark_push_completed(&mut self, promised_stream_id: u32) -> Result<(), String> {
        match self.pushes.get_mut(&promised_stream_id) {
            Some(push) => {
                push.mark_completed();
                self.completed_pushes += 1;
                Ok(())
            }
            None => Err(format!("Promised stream {} not found", promised_stream_id)),
        }
    }

    /// Mark a push as rejected
    pub fn mark_push_rejected(&mut self, promised_stream_id: u32) -> Result<(), String> {
        match self.pushes.get_mut(&promised_stream_id) {
            Some(push) => {
                push.mark_rejected();
                self.rejected_pushes += 1;
                Ok(())
            }
            None => Err(format!("Promised stream {} not found", promised_stream_id)),
        }
    }

    /// Get a push by ID
    pub fn get_push(&self, promised_stream_id: u32) -> Option<&ServerPush> {
        self.pushes.get(&promised_stream_id)
    }

    /// Get mutable reference to a push
    pub fn get_push_mut(&mut self, promised_stream_id: u32) -> Option<&mut ServerPush> {
        self.pushes.get_mut(&promised_stream_id)
    }

    /// Get all active pushes
    pub fn get_all_pushes(&self) -> Vec<&ServerPush> {
        self.pushes.values().collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> ServerPushStats {
        ServerPushStats {
            total_pushes: self.total_pushes,
            completed_pushes: self.completed_pushes,
            rejected_pushes: self.rejected_pushes,
            active_pushes: (self.total_pushes - self.completed_pushes - self.rejected_pushes) as usize,
        }
    }

    /// Check if a promised stream is tracked
    pub fn has_push(&self, promised_stream_id: u32) -> bool {
        self.pushes.contains_key(&promised_stream_id)
    }

    /// Get number of active pushes
    pub fn active_push_count(&self) -> usize {
        self.pushes.len()
    }

    /// Clear all pushes
    pub fn clear_all(&mut self) {
        self.pushes.clear();
    }

    /// Remove a completed push
    pub fn remove_push(&mut self, promised_stream_id: u32) -> Option<ServerPush> {
        self.pushes.remove(&promised_stream_id)
    }
}

/// Statistics about server push operations
#[derive(Debug, Clone, PartialEq)]
pub struct ServerPushStats {
    pub total_pushes: u64,
    pub completed_pushes: u64,
    pub rejected_pushes: u64,
    pub active_pushes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_push_creation() {
        let push = ServerPush::new(2, 1);
        assert_eq!(push.promised_stream_id, 2);
        assert_eq!(push.parent_stream_id, 1);
        assert_eq!(push.state, PushState::Promised);
        assert_eq!(push.headers.len(), 0);
        assert_eq!(push.body.len(), 0);
    }

    #[test]
    fn test_server_push_add_headers() {
        let mut push = ServerPush::new(2, 1);
        let headers = vec![
            (":method".to_string(), "GET".to_string()),
            (":path".to_string(), "/style.css".to_string()),
        ];
        push.add_headers(headers);

        assert_eq!(push.state, PushState::HeadersReceived);
        assert_eq!(push.get_promised_method(), Some("GET".to_string()));
        assert_eq!(push.get_promised_path(), Some("/style.css".to_string()));
    }

    #[test]
    fn test_server_push_add_body() {
        let mut push = ServerPush::new(2, 1);
        push.add_headers(vec![(":method".to_string(), "GET".to_string())]);

        let data = b"body content";
        let result = push.add_body_data(data);
        assert!(result.is_ok());
        assert_eq!(push.state, PushState::ReceivingData);
        assert_eq!(push.body, data);
    }

    #[test]
    fn test_server_push_completion() {
        let mut push = ServerPush::new(2, 1);
        push.add_headers(vec![(":method".to_string(), "GET".to_string())]);
        push.add_body_data(b"content").unwrap();
        push.mark_completed();

        assert_eq!(push.state, PushState::Completed);
        assert!(push.is_complete());
    }

    #[test]
    fn test_server_push_rejection() {
        let mut push = ServerPush::new(2, 1);
        push.mark_rejected();
        assert_eq!(push.state, PushState::Rejected);
    }

    #[test]
    fn test_server_push_content_length() {
        let mut push = ServerPush::new(2, 1);
        let headers = vec![
            (":method".to_string(), "GET".to_string()),
            ("content-length".to_string(), "100".to_string()),
        ];
        push.add_headers(headers);
        assert_eq!(push.total_size, Some(100));
    }

    #[test]
    fn test_push_manager_register() {
        let mut manager = ServerPushManager::new();
        let result = manager.register_push(2, 1);
        assert!(result.is_ok());
        assert!(manager.has_push(2));
    }

    #[test]
    fn test_push_manager_invalid_stream_id() {
        let mut manager = ServerPushManager::new();
        
        // Promised ID must be > parent
        let result = manager.register_push(1, 2);
        assert!(result.is_err());

        // Promised ID must be even (server-initiated)
        let result = manager.register_push(3, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_push_manager_duplicate() {
        let mut manager = ServerPushManager::new();
        let _ = manager.register_push(2, 1);
        let result = manager.register_push(2, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_push_manager_add_headers() {
        let mut manager = ServerPushManager::new();
        manager.register_push(2, 1).unwrap();

        let headers = vec![(":method".to_string(), "GET".to_string())];
        let result = manager.add_headers_to_push(2, headers);
        assert!(result.is_ok());
    }

    #[test]
    fn test_push_manager_add_headers_not_found() {
        let mut manager = ServerPushManager::new();
        let headers = vec![(":method".to_string(), "GET".to_string())];
        let result = manager.add_headers_to_push(2, headers);
        assert!(result.is_err());
    }

    #[test]
    fn test_push_manager_stats() {
        let mut manager = ServerPushManager::new();
        manager.register_push(2, 1).unwrap();
        manager.register_push(4, 1).unwrap();
        manager.mark_push_completed(2).unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.total_pushes, 2);
        assert_eq!(stats.completed_pushes, 1);
        assert_eq!(stats.rejected_pushes, 0);
        assert_eq!(stats.active_pushes, 1);
    }

    #[test]
    fn test_push_manager_lifecycle() {
        let mut manager = ServerPushManager::new();
        manager.register_push(2, 1).unwrap();

        let headers = vec![
            (":method".to_string(), "GET".to_string()),
            (":path".to_string(), "/style.css".to_string()),
        ];
        manager.add_headers_to_push(2, headers).unwrap();

        manager.add_body_data(2, b"css content").unwrap();
        manager.mark_push_completed(2).unwrap();

        let push = manager.get_push(2).unwrap();
        assert_eq!(push.state, PushState::Completed);
        assert!(push.is_complete());
    }

    #[test]
    fn test_push_manager_multiple_pushes() {
        let mut manager = ServerPushManager::new();
        manager.register_push(2, 1).unwrap();
        manager.register_push(4, 1).unwrap();
        manager.register_push(6, 3).unwrap();

        assert_eq!(manager.active_push_count(), 3);

        let all_pushes = manager.get_all_pushes();
        assert_eq!(all_pushes.len(), 3);
    }

    #[test]
    fn test_push_manager_remove() {
        let mut manager = ServerPushManager::new();
        manager.register_push(2, 1).unwrap();

        let removed = manager.remove_push(2);
        assert!(removed.is_some());
        assert!(!manager.has_push(2));
    }
}
