/// HTTP/2 Frame Forwarder for MITM
///
/// Wraps the shared scred-http frame forwarder for use in the MITM module.
/// Provides a simplified interface for MITM-specific use cases.

use std::sync::Arc;
use anyhow::Result;
use tracing::info;

use scred_redactor::RedactionEngine;
use scred_http::h2::frame_forwarder::{self, FrameForwarderConfig};

/// Handle HTTP/2 MITM proxying with upstream connection
pub async fn handle_upstream_h2_connection<C, U>(
    client_conn: C,
    upstream_conn: U,
    host: &str,
    _redaction_engine: Arc<RedactionEngine>,
    h2_redact_headers: bool,
) -> Result<()>
where
    C: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Unpin,
    U: tokio::io::AsyncReadExt + tokio::io::AsyncWriteExt + Unpin,
{
    info!("MITM H2 Handler: Starting for host: {}", host);

    let config = FrameForwarderConfig {
        validate_settings: true,
        max_concurrent_streams: 100,
        verbose_logging: false,
        enable_header_redaction: h2_redact_headers,
        redaction_engine: Some(Arc::new(RedactionEngine::new(scred_redactor::RedactionConfig::default()))),
    };

    let stats = frame_forwarder::forward_h2_frames(
        client_conn,
        upstream_conn,
        host,
        config,
    )
    .await?;

    info!(
        "MITM H2 Handler: Connection complete. Forwarded {} frames, {} bytes",
        stats.frames_forwarded, stats.bytes_forwarded
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mitm_handler_compiles() {
        // Just test that the handler compiles and is accessible
        // Real testing is done in integration tests
    }
}
