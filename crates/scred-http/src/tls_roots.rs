//! TLS root certificate store configuration
//!
//! Supports standard environment variables for custom CA certificates:
//! - SSL_CERT_FILE: Path to a PEM-encoded CA certificate file
//! - CURL_CA_BUNDLE: Path to a PEM-encoded CA bundle (alias for SSL_CERT_FILE)
//!
//! # Example
//!
//! ```ignore
//! // In docker-compose.yaml:
//! environment:
//!   SSL_CERT_FILE: /etc/ssl/certs/mitmproxy-ca.pem
//! volumes:
//!   - ./data/mitmproxy/mitmproxy-ca-cert.pem:/etc/ssl/certs/mitmproxy-ca.pem:ro
//! ```

use rustls::RootCertStore;
use std::env;
use std::fs::File;
use std::io::BufReader;
use tracing::{info, warn};

/// Build a RootCertStore with system roots plus optional custom CA from environment
///
/// Reads SSL_CERT_FILE or CURL_CA_BUNDLE environment variables to add custom CAs.
/// This allows trusting mitmproxy or other custom CAs without modifying code.
///
/// # Environment Variables
///
/// - `SSL_CERT_FILE`: Path to a PEM-encoded CA certificate file
/// - `CURL_CA_BUNDLE`: Alias for SSL_CERT_FILE (curl-compatible)
///
/// # Example
///
/// ```ignore
/// use scred_http::tls_roots::build_root_cert_store;
///
/// let root_store = build_root_cert_store();
/// let client_config = ClientConfig::builder()
///     .with_safe_defaults()
///     .with_root_certificates(root_store)
///     .with_no_client_auth();
/// ```
pub fn build_root_cert_store() -> RootCertStore {
    info!("[TLS_ROOTS] Building root certificate store...");
    let mut root_store = RootCertStore::empty();

    // Add system roots (Mozilla root program)
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    // Check for custom CA certificate (SSL_CERT_FILE or CURL_CA_BUNDLE)
    let cert_path = env::var("SSL_CERT_FILE")
        .or_else(|_| env::var("CURL_CA_BUNDLE"))
        .ok();

    if let Some(ref path) = cert_path {
        info!("[TLS_ROOTS] Loading custom CA certificate from: {}", path);
        match add_pem_certs(&mut root_store, path) {
            Ok(count) => {
                info!("[TLS_ROOTS] Loaded {} certificates from {}", count, path);
            }
            Err(e) => {
                warn!("[TLS_ROOTS] Failed to load custom CA from {}: {}", path, e);
            }
        }
    } else {
        info!("[TLS_ROOTS] No custom CA certificate configured (set SSL_CERT_FILE or CURL_CA_BUNDLE)");
    }

    root_store
}

/// Load PEM-encoded certificates from a file and add to root store
///
/// # Arguments
///
/// * `root_store` - The root certificate store to add certificates to
/// * `path` - Path to a PEM-encoded certificate file
///
/// # Returns
///
/// Number of certificates successfully added
fn add_pem_certs(root_store: &mut RootCertStore, path: &str) -> std::io::Result<usize> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Parse PEM certificates (rustls-pemfile 2.x)
    let certs = rustls_pemfile::certs(&mut reader)
        .filter_map(|result| result.ok())
        .map(|cert_der| cert_der.to_vec())
        .collect::<Vec<Vec<u8>>>();

    // Add each certificate as a trust anchor
    let mut added = 0;
    for cert_der in certs {
        let cert = rustls::Certificate(cert_der);
        match root_store.add(&cert) {
            Ok(_) => added += 1,
            Err(e) => {
                warn!("Failed to add certificate to root store: {:?}", e);
            }
        }
    }

    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_root_cert_store_system_only() {
        // Without SSL_CERT_FILE, should only have system roots
        env::remove_var("SSL_CERT_FILE");
        env::remove_var("CURL_CA_BUNDLE");

        let store = build_root_cert_store();
        // System roots should be non-empty
        assert!(!store.is_empty());
    }
}
