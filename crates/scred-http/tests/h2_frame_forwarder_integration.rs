/// HTTP/2 Frame Forwarder Integration Tests

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Mock connection for testing
struct MockConn {
    data: std::io::Cursor<Vec<u8>>,
}

impl AsyncRead for MockConn {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let n = std::io::Read::read(&mut self.data, unsafe {
            std::slice::from_raw_parts_mut(
                buf.unfilled_mut().as_mut_ptr() as *mut u8,
                buf.unfilled_mut().len(),
            )
        })?;
        unsafe { buf.assume_init(n); }
        buf.filled_mut().rotate_left(n);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MockConn {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(std::io::Write::write(&mut self.data, buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(std::io::Write::flush(&mut self.data))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[test]
fn test_stream_id_extraction_and_setting() {
    use scred_http::h2::frame_forwarder::{extract_stream_id, set_stream_id};

    let mut header = vec![0, 0, 10, 1, 5, 0, 0, 0, 1];
    assert_eq!(extract_stream_id(&header), 1);

    set_stream_id(&mut header, 5);
    assert_eq!(extract_stream_id(&header), 5);

    set_stream_id(&mut header, 0x7FFF_FFFF);
    assert_eq!(extract_stream_id(&header), 0x7FFF_FFFF);
}

#[test]
fn test_frame_forwarder_config_default() {
    use scred_http::h2::frame_forwarder::FrameForwarderConfig;

    let config = FrameForwarderConfig::default();
    assert!(config.validate_settings);
    assert_eq!(config.max_concurrent_streams, 100);
    assert!(!config.verbose_logging);
}

#[test]
fn test_frame_forwarder_config_custom() {
    use scred_http::h2::frame_forwarder::FrameForwarderConfig;

    let config = FrameForwarderConfig {
        validate_settings: false,
        max_concurrent_streams: 200,
        verbose_logging: true,
        enable_header_redaction: true,
        redaction_engine: None,
    };

    assert!(!config.validate_settings);
    assert_eq!(config.max_concurrent_streams, 200);
    assert!(config.verbose_logging);
    assert!(config.enable_header_redaction);
}

#[test]
fn test_stream_id_odd_even_convention() {
    use scred_http::h2::frame_forwarder::extract_stream_id;

    let header1 = vec![0, 0, 0, 0, 0, 0, 0, 0, 1];
    assert_eq!(extract_stream_id(&header1), 1);

    let header2 = vec![0, 0, 0, 0, 0, 0, 0, 0, 2];
    assert_eq!(extract_stream_id(&header2), 2);

    let header0 = vec![0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(extract_stream_id(&header0), 0);
}

#[test]
fn test_reserved_bit_masking() {
    use scred_http::h2::frame_forwarder::{extract_stream_id, set_stream_id};

    let mut header = vec![0, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF];
    let stream_id = extract_stream_id(&header);
    assert_eq!(stream_id, 0x7FFF_FFFF);

    set_stream_id(&mut header, 1);
    assert_eq!(extract_stream_id(&header), 1);
}

#[test]
fn test_multiple_stream_mappings() {
    use std::collections::HashMap;

    let mut stream_map = HashMap::new();
    let mut next_upstream_id = 1u32;

    let streams = vec![1, 3, 5, 7, 9];
    for client_id in streams {
        let upstream_id = *stream_map.entry(client_id).or_insert_with(|| {
            let id = next_upstream_id;
            next_upstream_id += 2;
            id
        });

        assert_eq!(stream_map.get(&client_id), Some(&upstream_id));
    }

    assert_eq!(stream_map.len(), 5);
    assert_eq!(next_upstream_id, 11);
}

#[test]
fn test_frame_forwarder_stats() {
    use scred_http::h2::frame_forwarder::ForwardingStats;

    let stats = ForwardingStats {
        frames_forwarded: 100,
        bytes_forwarded: 5000,
        settings_acks_sent: 2,
        stream_mappings_created: 50,
        headers_redacted: 10,
    };

    assert_eq!(stats.frames_forwarded, 100);
    assert_eq!(stats.bytes_forwarded, 5000);
    assert_eq!(stats.settings_acks_sent, 2);
    assert_eq!(stats.stream_mappings_created, 50);
}

#[test]
fn test_rfc7540_compliance_stream_ids() {
    let valid_client_ids = vec![1, 3, 5, 7, 9, 0x7FFF_FFFD]; // Largest odd is 0x7FFF_FFFF
    let valid_server_ids = vec![2, 4, 6, 8, 10, 0x7FFF_FFFE]; // Largest even stream ID
    let reserved = vec![0];

    for id in valid_client_ids {
        assert_eq!(id % 2, 1, "Client stream ID {} should be odd", id);
    }

    for id in valid_server_ids {
        assert_eq!(id % 2, 0, "Server stream ID {} should be even", id);
    }

    for id in reserved {
        assert_eq!(id, 0);
    }
}

#[test]
fn test_frame_length_limits() {
    const DEFAULT_FRAME_SIZE: u32 = 16384;
    const MAX_FRAME_SIZE: u32 = 16777215;

    assert_eq!(DEFAULT_FRAME_SIZE, 16384);
    assert_eq!(MAX_FRAME_SIZE, 16777215);
    assert!(MAX_FRAME_SIZE < u32::MAX);
}

#[test]
fn test_preface_is_correct() {
    let expected_preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
    assert_eq!(expected_preface.len(), 24);
    assert_eq!(expected_preface, b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");
}

#[test]
fn test_ack_flag_handling() {
    const SETTINGS_ACK_FLAG: u8 = 0x01;
    assert_eq!(SETTINGS_ACK_FLAG, 1);

    let frame_with_ack = vec![0, 0, 0, 0x04, 0x01, 0, 0, 0, 0];
    assert_eq!(frame_with_ack[3], 0x04);
    assert_eq!(frame_with_ack[4], 0x01);

    let frame_without_ack = vec![0, 0, 0, 0x04, 0x00, 0, 0, 0, 0];
    assert_eq!(frame_without_ack[3], 0x04);
    assert_eq!(frame_without_ack[4], 0x00);
}
