/// HTTP/2 Protocol Support
///
/// This module provides HTTP/2 protocol utilities for both MITM and proxy modes:
/// - ALPN negotiation (protocol detection)
/// - Frame parsing (read HTTP/2 frames)
/// - Header decompression (HPACK)
/// - Format transcoding (HTTP/2 ↔ HTTP/1.1)
/// - Frame forwarding (bidirectional tunneling with stream mapping)
///
/// These utilities are shared between:
/// - scred-mitm: ALPN detection, upstream h2 handling, transcode to h1 for client
/// - scred-proxy: ALPN detection, upstream h2 multiplexing, transcode to h1 for client

pub mod alpn;
pub mod frame;
pub mod h2_reader;
pub mod transcode;
pub mod stream_state;
pub mod stream_manager;
pub mod per_stream_redactor;
pub mod upstream_pool;
pub mod flow_controller;
pub mod hpack;
pub mod frame_encoder;
pub mod upstream_wiring;
pub mod frame_forwarder;
pub mod header_redactor;
pub mod h2_proxy_bridge;
pub mod server_push;
pub mod stream_priority;
pub mod stream_reset;
pub mod connection_error;
pub mod header_validation;
pub mod stream_state_machine;
pub mod upstream_response_transcoder;
pub mod h2_upstream_client;
pub mod hpack_encoder;
