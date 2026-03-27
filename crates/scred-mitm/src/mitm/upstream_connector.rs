/// Upstream Server Connector
///
/// Handles connections to upstream servers with automatic HTTP/2 detection.
/// Detects if upstream supports HTTP/2 via ALPN and provides unified interface.

use anyhow::{anyhow, Result};
use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, TlsStream};
use rustls::ServerName;
use tracing::{debug, info, warn};

use scred_http::dns_resolver::DnsResolver;
use scred_http::proxy_resolver::connect_through_proxy;
use scred_http::h2::alpn::{HttpProtocol, alpn_protocols};

/// Information about upstream server connection
#[derive(Debug, Clone)]
pub struct UpstreamConnectionInfo {
    /// Protocol negotiated with upstream server
    pub protocol: HttpProtocol,
}

/// Connect to upstream server with HTTP/2 detection
///
/// # Returns
/// - (TlsStream, UpstreamConnectionInfo) tuple
/// - Stream: Ready for HTTP communication
/// - Info: Contains negotiated protocol (h2 or http/1.1)
pub async fn connect_to_upstream(
    upstream_addr: &str,
    target_host: &str,
) -> Result<(TlsStream<TcpStream>, UpstreamConnectionInfo)> {
    debug!("Connecting to upstream: {} (target: {})", upstream_addr, target_host);

    // Step 1: Establish TCP connection
    let is_upstream_proxy = upstream_addr.contains("://");
    let upstream_tcp = if is_upstream_proxy {
        debug!("Using upstream proxy: {}", upstream_addr);
        connect_through_proxy(upstream_addr, target_host, 443).await?
    } else {
        debug!("Direct connection to upstream: {}:443", target_host);
        DnsResolver::connect_with_retry(&format!("{}:443", target_host)).await?
    };

    info!("TCP connected to upstream");

    // Step 2: Set up TLS ClientConfig with ALPN support
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    let mut client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    // Add ALPN support to advertise both h2 and http/1.1
    client_config.alpn_protocols = alpn_protocols();

    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name = ServerName::try_from(target_host)
        .map_err(|e| anyhow!("Invalid upstream hostname '{}': {}", target_host, e))?;

    // Step 3: Perform TLS handshake with ALPN
    let tls_stream = connector
        .connect(server_name, upstream_tcp)
        .await
        .map_err(|e| anyhow!("Upstream TLS handshake failed: {}", e))?;

    // Step 4: Extract negotiated protocol
    let negotiated_protocol = tls_stream.get_ref().1.alpn_protocol()
        .and_then(|proto| HttpProtocol::from_bytes(proto))
        .unwrap_or(HttpProtocol::Http11);

    info!(
        "Upstream TLS handshake successful, protocol: {}",
        negotiated_protocol
    );

    // Upstream protocol negotiation complete
    if negotiated_protocol.is_h2() {
        info!("Upstream server supports HTTP/2 (h2 ALPN)");
        // HTTP/2 MULTIPLEXING: Available via h2 crate integration
        // Current approach: Transparent HTTP/1.1 fallback for compatibility
        // Future: Direct HTTP/2 multiplexing via h2_upstream_forwarder (if needed)
    }

    let connection_info = UpstreamConnectionInfo {
        protocol: negotiated_protocol,
    };

    Ok((tls_stream, connection_info))
}

