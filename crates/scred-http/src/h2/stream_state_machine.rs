//! HTTP/2 Stream State Machine
//! 
//! RFC 7540 Section 5.1: Stream States and State Transitions

/// HTTP/2 Stream states
/// RFC 7540 Section 5.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamState {
    /// Idle - stream hasn't been created yet
    Idle,
    /// Open - stream is open for sending and receiving
    Open,
    /// HalfClosedLocal - local side closed, remote can still send
    HalfClosedLocal,
    /// HalfClosedRemote - remote side closed, local can still send
    HalfClosedRemote,
    /// Closed - both sides closed
    Closed,
    /// Reserved(Local) - server has initiated a push, local reserved
    ReservedLocal,
    /// Reserved(Remote) - client has sent a request, remote reserved
    ReservedRemote,
}

impl StreamState {
    /// Check if stream is open for sending (local)
    pub fn can_send(&self) -> bool {
        matches!(self, StreamState::Open | StreamState::HalfClosedRemote)
    }

    /// Check if stream is open for receiving (remote)
    pub fn can_receive(&self) -> bool {
        matches!(self, StreamState::Open | StreamState::HalfClosedLocal)
    }

    /// Check if stream is active (not closed or idle)
    pub fn is_active(&self) -> bool {
        !matches!(self, StreamState::Idle | StreamState::Closed)
    }

    /// Check if stream is closed
    pub fn is_closed(&self) -> bool {
        *self == StreamState::Closed
    }

    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            StreamState::Idle => "IDLE",
            StreamState::Open => "OPEN",
            StreamState::HalfClosedLocal => "HALF_CLOSED_LOCAL",
            StreamState::HalfClosedRemote => "HALF_CLOSED_REMOTE",
            StreamState::Closed => "CLOSED",
            StreamState::ReservedLocal => "RESERVED_LOCAL",
            StreamState::ReservedRemote => "RESERVED_REMOTE",
        }
    }
}

/// Stream state machine
pub struct StreamStateMachine {
    /// Current state
    state: StreamState,
    /// Stream ID
    stream_id: u32,
    /// Sent END_STREAM flag
    sent_end_stream: bool,
    /// Received END_STREAM flag
    received_end_stream: bool,
}

impl StreamStateMachine {
    /// Create new stream state machine
    pub fn new(stream_id: u32) -> Self {
        StreamStateMachine {
            state: StreamState::Idle,
            stream_id,
            sent_end_stream: false,
            received_end_stream: false,
        }
    }

    /// Get current state
    pub fn state(&self) -> StreamState {
        self.state
    }

    /// Send headers (client-initiated request)
    pub fn send_headers(&mut self) -> Result<(), String> {
        match self.state {
            StreamState::Idle => {
                self.state = StreamState::Open;
                Ok(())
            }
            _ => Err(format!(
                "Cannot send headers in state: {}",
                self.state.name()
            )),
        }
    }

    /// Receive headers (server response or client request)
    pub fn receive_headers(&mut self) -> Result<(), String> {
        match self.state {
            StreamState::Idle => {
                self.state = StreamState::Open;
                Ok(())
            }
            StreamState::ReservedRemote => {
                self.state = StreamState::HalfClosedLocal;
                Ok(())
            }
            _ => Err(format!(
                "Cannot receive headers in state: {}",
                self.state.name()
            )),
        }
    }

    /// Send data (with optional END_STREAM)
    pub fn send_data(&mut self, end_stream: bool) -> Result<(), String> {
        if !self.state.can_send() {
            return Err(format!(
                "Cannot send data in state: {}",
                self.state.name()
            ));
        }

        if end_stream {
            self.sent_end_stream = true;
            self.state = if self.received_end_stream {
                StreamState::Closed
            } else {
                StreamState::HalfClosedLocal
            };
        }

        Ok(())
    }

    /// Receive data (with optional END_STREAM)
    pub fn receive_data(&mut self, end_stream: bool) -> Result<(), String> {
        if !self.state.can_receive() {
            return Err(format!(
                "Cannot receive data in state: {}",
                self.state.name()
            ));
        }

        if end_stream {
            self.received_end_stream = true;
            self.state = if self.sent_end_stream {
                StreamState::Closed
            } else {
                StreamState::HalfClosedRemote
            };
        }

        Ok(())
    }

    /// Send RST_STREAM (close stream abnormally)
    pub fn reset(&mut self) -> Result<(), String> {
        if self.state == StreamState::Idle {
            return Err("Cannot reset idle stream".to_string());
        }

        self.state = StreamState::Closed;
        Ok(())
    }

    /// Receive RST_STREAM
    pub fn receive_reset(&mut self) -> Result<(), String> {
        if self.state == StreamState::Idle {
            return Err("Cannot reset idle stream".to_string());
        }

        self.state = StreamState::Closed;
        Ok(())
    }

    /// Push promise (server initiates push)
    pub fn push_promise(&mut self) -> Result<(), String> {
        match self.state {
            StreamState::Open | StreamState::HalfClosedRemote => {
                // Parent stream must be open or half-closed remote
                Ok(())
            }
            _ => Err(format!(
                "Cannot send PUSH_PROMISE in state: {}",
                self.state.name()
            )),
        }
    }

    /// Reserve local (for pushed stream)
    pub fn reserve_local(&mut self) -> Result<(), String> {
        match self.state {
            StreamState::Idle => {
                self.state = StreamState::ReservedLocal;
                Ok(())
            }
            _ => Err(format!(
                "Cannot reserve local in state: {}",
                self.state.name()
            )),
        }
    }

    /// Reserve remote (for pushed stream)
    pub fn reserve_remote(&mut self) -> Result<(), String> {
        match self.state {
            StreamState::Idle => {
                self.state = StreamState::ReservedRemote;
                Ok(())
            }
            _ => Err(format!(
                "Cannot reserve remote in state: {}",
                self.state.name()
            )),
        }
    }

    /// Check if stream can receive PRIORITY frame
    pub fn can_receive_priority(&self) -> bool {
        self.state != StreamState::Idle && self.state != StreamState::Closed
    }

    /// Check if stream can receive WINDOW_UPDATE
    pub fn can_receive_window_update(&self) -> bool {
        self.state != StreamState::Idle && self.state != StreamState::Closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_can_send() {
        assert!(!StreamState::Idle.can_send());
        assert!(StreamState::Open.can_send());
        assert!(StreamState::HalfClosedRemote.can_send());
        assert!(!StreamState::HalfClosedLocal.can_send());
        assert!(!StreamState::Closed.can_send());
    }

    #[test]
    fn test_state_can_receive() {
        assert!(!StreamState::Idle.can_receive());
        assert!(StreamState::Open.can_receive());
        assert!(StreamState::HalfClosedLocal.can_receive());
        assert!(!StreamState::HalfClosedRemote.can_receive());
        assert!(!StreamState::Closed.can_receive());
    }

    #[test]
    fn test_state_is_active() {
        assert!(!StreamState::Idle.is_active());
        assert!(StreamState::Open.is_active());
        assert!(!StreamState::Closed.is_active());
    }

    #[test]
    fn test_state_name() {
        assert_eq!(StreamState::Open.name(), "OPEN");
        assert_eq!(StreamState::HalfClosedLocal.name(), "HALF_CLOSED_LOCAL");
        assert_eq!(StreamState::Closed.name(), "CLOSED");
    }

    #[test]
    fn test_normal_request_response_cycle() {
        let mut sm = StreamStateMachine::new(1);

        // Client sends headers
        assert!(sm.send_headers().is_ok());
        assert_eq!(sm.state(), StreamState::Open);

        // Server sends data with END_STREAM (response)
        assert!(sm.receive_data(true).is_ok());
        assert_eq!(sm.state(), StreamState::HalfClosedRemote);

        // Client sends data with END_STREAM (request body)
        assert!(sm.send_data(true).is_ok());
        assert_eq!(sm.state(), StreamState::Closed);
    }

    #[test]
    fn test_send_headers_not_idle() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();

        // Can't send headers again
        let result = sm.send_headers();
        assert!(result.is_err());
    }

    #[test]
    fn test_send_data_before_headers() {
        let mut sm = StreamStateMachine::new(1);

        // Can't send data on idle stream
        let result = sm.send_data(false);
        assert!(result.is_err());
    }

    #[test]
    fn test_receive_data_before_headers() {
        let mut sm = StreamStateMachine::new(1);

        // Can't receive data on idle stream
        let result = sm.receive_data(false);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset_stream() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();

        // Can reset open stream
        assert!(sm.reset().is_ok());
        assert_eq!(sm.state(), StreamState::Closed);
    }

    #[test]
    fn test_reset_idle_stream() {
        let mut sm = StreamStateMachine::new(1);

        // Can't reset idle stream
        let result = sm.reset();
        assert!(result.is_err());
    }

    #[test]
    fn test_push_promise_valid() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();

        // Can send PUSH_PROMISE on open stream
        assert!(sm.push_promise().is_ok());
    }

    #[test]
    fn test_push_promise_half_closed_remote() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();
        let _ = sm.receive_data(true); // Half-closed remote

        // Can send PUSH_PROMISE on half-closed remote stream
        assert!(sm.push_promise().is_ok());
    }

    #[test]
    fn test_push_promise_invalid() {
        let mut sm = StreamStateMachine::new(1);

        // Can't send PUSH_PROMISE on idle stream
        let result = sm.push_promise();
        assert!(result.is_err());
    }

    #[test]
    fn test_reserve_local() {
        let mut sm = StreamStateMachine::new(3); // Even = server-initiated

        // Can reserve local on idle stream
        assert!(sm.reserve_local().is_ok());
        assert_eq!(sm.state(), StreamState::ReservedLocal);
    }

    #[test]
    fn test_reserve_remote() {
        let mut sm = StreamStateMachine::new(4); // Even = server-initiated

        // Can reserve remote on idle stream
        assert!(sm.reserve_remote().is_ok());
        assert_eq!(sm.state(), StreamState::ReservedRemote);
    }

    #[test]
    fn test_reserved_remote_to_half_closed_local() {
        let mut sm = StreamStateMachine::new(2);
        let _ = sm.reserve_remote();

        // Receive headers on reserved remote
        assert!(sm.receive_headers().is_ok());
        assert_eq!(sm.state(), StreamState::HalfClosedLocal);
    }

    #[test]
    fn test_priority_on_open_stream() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();

        // Can receive PRIORITY on open stream
        assert!(sm.can_receive_priority());
    }

    #[test]
    fn test_priority_on_closed_stream() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();
        let _ = sm.send_data(true);
        let _ = sm.receive_data(true);

        // Can't receive PRIORITY on closed stream
        assert!(!sm.can_receive_priority());
    }

    #[test]
    fn test_window_update_on_open_stream() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();

        // Can receive WINDOW_UPDATE on open stream
        assert!(sm.can_receive_window_update());
    }

    #[test]
    fn test_window_update_on_closed_stream() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();
        let _ = sm.send_data(true);
        let _ = sm.receive_data(true);

        // Can't receive WINDOW_UPDATE on closed stream
        assert!(!sm.can_receive_window_update());
    }

    #[test]
    fn test_half_closed_local_no_send() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();
        let _ = sm.send_data(true);

        assert_eq!(sm.state(), StreamState::HalfClosedLocal);

        // Can't send more data
        let result = sm.send_data(false);
        assert!(result.is_err());
    }

    #[test]
    fn test_half_closed_remote_no_receive() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();
        let _ = sm.receive_data(true);

        assert_eq!(sm.state(), StreamState::HalfClosedRemote);

        // Can't receive more data
        let result = sm.receive_data(false);
        assert!(result.is_err());
    }

    #[test]
    fn test_receive_reset_from_open() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();

        // Receive RST_STREAM
        assert!(sm.receive_reset().is_ok());
        assert_eq!(sm.state(), StreamState::Closed);
    }

    #[test]
    fn test_send_and_receive_end_stream_order() {
        let mut sm = StreamStateMachine::new(1);
        let _ = sm.send_headers();

        // Send END_STREAM first
        let _ = sm.send_data(true);
        assert_eq!(sm.state(), StreamState::HalfClosedLocal);

        // Then receive END_STREAM
        let _ = sm.receive_data(true);
        assert_eq!(sm.state(), StreamState::Closed);
    }
}
