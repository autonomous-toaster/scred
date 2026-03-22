//! SCRED HTTP Library
//!
//! Comprehensive HTTP utilities for SCRED proxies and CLI tools.
//!
//! ## HTTP Core
//! - `parser`: HTTP/1.1 request/response parsing
//! - `models`: HttpRequest, HttpResponse data structures
//! - `connect`: HTTP CONNECT tunneling for HTTPS proxies
//!
//! ## Proxy Utilities
//! - `duplex`: Combined AsyncRead + AsyncWrite socket wrapper
//! - `host_identification`: Extract hostnames from HTTP/TLS
//! - `proxy_resolver`: Detect system proxy settings (HTTP_PROXY env vars)
//! - `tcp_relay`: Generic TCP relay with bidirectional redaction
//!
//! ## Configuration & Secrets
//! - `config`: Redaction configuration and pattern selection
//! - `secrets`: Secret filtering rules and configuration
//!
//! ## Logging
//! - `logging`: Structured logging (JSON, compact, pretty)

pub mod config;
pub mod connect;
pub mod dns_resolver;
pub mod duplex;
pub mod fixed_upstream;
pub mod h2;
pub mod header_rewriter;
pub mod host_identification;
pub mod http_line_reader;
pub mod http_proxy_handler;
pub mod location_rewriter;
pub mod logging;
pub mod models;
pub mod parser;
pub mod proxy_resolver;
pub mod response_reader;
pub mod secrets;
pub mod tcp_relay;
pub mod http_headers;
pub mod streaming_request;
pub mod streaming_response;
pub mod chunked_parser;
pub mod upstream_h2_client;

pub const VERSION: &str = "0.1.0";
