/// Host Identification for MITM
///
/// Extracts target host from multiple sources with priority:
/// 1. CONNECT request (primary)
/// 2. TLS SNI (Server Name Indication)
/// 3. HTTP Host header (from decrypted request)
/// 4. Certificate CN from upstream (fallback)
///
/// This is isomorphic to the Zig implementation

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Host identification from various sources
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostIdentification {
    /// Final determined host
    pub host: String,
    /// Final determined port
    pub port: u16,
    /// Source of host identification
    pub source: HostSource,
    /// Other sources found (for validation)
    pub alt_sources: HashMap<String, String>,
    /// Whether sources match (validation flag)
    pub sources_consistent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostSource {
    ConnectRequest,
    ServerNameIndication,
    HttpHostHeader,
    CertificateCN,
    Unknown,
}

impl std::fmt::Display for HostSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostSource::ConnectRequest => write!(f, "CONNECT request"),
            HostSource::ServerNameIndication => write!(f, "TLS SNI"),
            HostSource::HttpHostHeader => write!(f, "HTTP Host header"),
            HostSource::CertificateCN => write!(f, "Certificate CN"),
            HostSource::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Information collected from different sources
#[derive(Debug, Clone, Default)]
pub struct HostSources {
    /// From CONNECT request: host:port
    pub connect_host: Option<(String, u16)>,
    /// From TLS SNI: hostname
    pub sni_host: Option<String>,
    /// From HTTP Host header: host:port
    pub http_host: Option<(String, u16)>,
    /// From Certificate CN: hostname
    pub cert_cn: Option<String>,
}

impl HostIdentification {
    /// Determine host from collected sources using mitmproxy priority
    pub fn from_sources(sources: &HostSources) -> Result<Self> {
        let mut alt_sources = HashMap::new();
        let mut selected_host = None;
        let mut selected_port = 443; // default HTTPS port
        let mut source = HostSource::Unknown;

        // Priority 1: CONNECT request (most authoritative)
        if let Some((host, port)) = &sources.connect_host {
            debug!("Using host from CONNECT request: {}:{}", host, port);
            selected_host = Some(host.clone());
            selected_port = *port;
            source = HostSource::ConnectRequest;
        }

        // Priority 2: TLS SNI (if no CONNECT)
        if selected_host.is_none() {
            if let Some(host) = &sources.sni_host {
                debug!("Using host from SNI: {}", host);
                selected_host = Some(host.clone());
                selected_port = 443;
                source = HostSource::ServerNameIndication;
            }
        }

        // Priority 3: HTTP Host header (if no CONNECT/SNI)
        if selected_host.is_none() {
            if let Some((host, port)) = &sources.http_host {
                debug!("Using host from HTTP Host header: {}:{}", host, port);
                selected_host = Some(host.clone());
                selected_port = *port;
                source = HostSource::HttpHostHeader;
            }
        }

        // Priority 4: Certificate CN (fallback)
        if selected_host.is_none() {
            if let Some(cn) = &sources.cert_cn {
                debug!("Using host from certificate CN: {}", cn);
                selected_host = Some(cn.clone());
                selected_port = 443;
                source = HostSource::CertificateCN;
            }
        }

        let host = selected_host.ok_or_else(|| anyhow!("No host identification source available"))?;

        // Collect all sources for validation
        if let Some((h, p)) = &sources.connect_host {
            alt_sources.insert(
                "connect".to_string(),
                format!("{}:{}", h, p),
            );
        }
        if let Some(h) = &sources.sni_host {
            alt_sources.insert("sni".to_string(), h.clone());
        }
        if let Some((h, p)) = &sources.http_host {
            alt_sources.insert("http_host".to_string(), format!("{}:{}", h, p));
        }
        if let Some(cn) = &sources.cert_cn {
            alt_sources.insert("cert_cn".to_string(), cn.clone());
        }

        // Validate consistency
        let sources_consistent = Self::validate_sources(&host, &sources);
        if !sources_consistent {
            warn!(
                "Host identification sources are inconsistent. \
                 Using {} source: {}",
                source, host
            );
        }

        Ok(HostIdentification {
            host,
            port: selected_port,
            source,
            alt_sources,
            sources_consistent,
        })
    }

    /// Validate that multiple sources agree on the host
    fn validate_sources(primary_host: &str, sources: &HostSources) -> bool {
        let mut consistent = true;

        // Check CONNECT vs SNI
        if let (Some((connect_host, _)), Some(sni)) = (&sources.connect_host, &sources.sni_host) {
            if connect_host != sni {
                warn!(
                    "CONNECT host '{}' != SNI host '{}'",
                    connect_host, sni
                );
                consistent = false;
            }
        }

        // Check CONNECT vs HTTP Host
        if let (Some((connect_host, _)), Some((http_host, _))) =
            (&sources.connect_host, &sources.http_host)
        {
            if connect_host != http_host {
                warn!(
                    "CONNECT host '{}' != HTTP Host '{}'",
                    connect_host, http_host
                );
                consistent = false;
            }
        }

        // Check SNI vs HTTP Host
        if let (Some(sni), Some((http_host, _))) = (&sources.sni_host, &sources.http_host) {
            if sni != http_host {
                warn!("SNI host '{}' != HTTP Host '{}'", sni, http_host);
                consistent = false;
            }
        }

        // Check Certificate CN against primary
        if let Some(cn) = &sources.cert_cn {
            if cn != primary_host {
                warn!(
                    "Primary host '{}' != certificate CN '{}'",
                    primary_host, cn
                );
                consistent = false;
            }
        }

        consistent
    }

    /// Get full address (host:port)
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Log identification details
    pub fn log_identification(&self) {
        info!(
            "Host identification: {} (source: {}, consistent: {})",
            self.address(),
            self.source,
            self.sources_consistent
        );
        if !self.alt_sources.is_empty() {
            for (key, val) in &self.alt_sources {
                debug!("  {} = {}", key, val);
            }
        }
    }
}

/// Extract host from HTTP Host header
pub fn parse_host_header(header_value: &str) -> Result<(String, u16)> {
    // Format: "hostname:port" or just "hostname"
    let parts: Vec<&str> = header_value.split(':').collect();

    match parts.len() {
        1 => {
            // Just hostname, assume HTTPS port
            Ok((parts[0].to_string(), 443))
        }
        2 => {
            // hostname:port
            let host = parts[0].to_string();
            let port = parts[1]
                .parse::<u16>()
                .map_err(|_| anyhow!("Invalid port in Host header: {}", parts[1]))?;
            Ok((host, port))
        }
        _ => Err(anyhow!("Invalid Host header format: {}", header_value)),
    }
}

/// Extract SNI from TLS ClientHello (unused - SNI handled by tokio-rustls)
///
/// Note: SNI extraction is handled by the TLS library (tokio-rustls).
/// This function is kept for documentation but not used in current implementation.
#[allow(dead_code)]
pub fn extract_sni_from_clienthello(data: &[u8]) -> Option<String> {
    // SNI extraction is complex and handled by tokio-rustls
    // which provides SNI info through the TLS connection
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_identification_from_connect() {
        let sources = HostSources {
            connect_host: Some(("example.com".to_string(), 443)),
            sni_host: None,
            http_host: None,
            cert_cn: None,
        };

        let identification = HostIdentification::from_sources(&sources).unwrap();
        assert_eq!(identification.host, "example.com");
        assert_eq!(identification.port, 443);
        assert_eq!(identification.source, HostSource::ConnectRequest);
        assert!(identification.sources_consistent);
    }

    #[test]
    fn test_host_identification_from_sni() {
        let sources = HostSources {
            connect_host: None,
            sni_host: Some("example.com".to_string()),
            http_host: None,
            cert_cn: None,
        };

        let identification = HostIdentification::from_sources(&sources).unwrap();
        assert_eq!(identification.host, "example.com");
        assert_eq!(identification.port, 443);
        assert_eq!(identification.source, HostSource::ServerNameIndication);
    }

    #[test]
    fn test_host_identification_priority() {
        // CONNECT should take priority over SNI
        let sources = HostSources {
            connect_host: Some(("connect.com".to_string(), 443)),
            sni_host: Some("sni.com".to_string()),
            http_host: None,
            cert_cn: None,
        };

        let identification = HostIdentification::from_sources(&sources).unwrap();
        assert_eq!(identification.host, "connect.com");
        assert_eq!(identification.source, HostSource::ConnectRequest);
        assert!(!identification.sources_consistent); // Mismatch
    }

    #[test]
    fn test_parse_host_header_with_port() {
        let (host, port) = parse_host_header("example.com:8080").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_parse_host_header_no_port() {
        let (host, port) = parse_host_header("example.com").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 443);
    }

    #[test]
    fn test_host_identification_fallback_to_cert() {
        let sources = HostSources {
            connect_host: None,
            sni_host: None,
            http_host: None,
            cert_cn: Some("example.com".to_string()),
        };

        let identification = HostIdentification::from_sources(&sources).unwrap();
        assert_eq!(identification.host, "example.com");
        assert_eq!(identification.source, HostSource::CertificateCN);
    }

    #[test]
    fn test_host_identification_no_sources() {
        let sources = HostSources::default();
        assert!(HostIdentification::from_sources(&sources).is_err());
    }

    #[test]
    fn test_address_formatting() {
        let id = HostIdentification {
            host: "example.com".to_string(),
            port: 443,
            source: HostSource::ConnectRequest,
            alt_sources: HashMap::new(),
            sources_consistent: true,
        };

        assert_eq!(id.address(), "example.com:443");
    }
}
