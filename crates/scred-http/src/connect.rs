/// HTTP CONNECT Handler for HTTPS Proxy
/// 
/// The CONNECT method is used to establish a tunnel through the proxy.
/// Client sends: CONNECT example.com:443 HTTP/1.1
/// Proxy responds: HTTP/1.1 200 Connection Established
/// Then bidirectional TLS traffic flows through the proxy

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use std::io;

#[derive(Debug, Clone)]
pub struct ConnectRequest {
    pub host: String,
    pub port: u16,
}

/// Parse HTTP CONNECT request from client
/// 
/// Expected format:
///   CONNECT example.com:443 HTTP/1.1\r\n
///   Host: example.com:443\r\n
///   \r\n
pub async fn parse_connect_request<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> io::Result<ConnectRequest> {
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let line = line.trim();
    if !line.starts_with("CONNECT ") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected CONNECT request",
        ));
    }

    // Parse: CONNECT host:port HTTP/1.1
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid CONNECT request format",
        ));
    }

    let host_port = parts[1];
    let (host, port) = parse_host_port(host_port)?;

    // Read remaining headers until blank line
    let mut header_line = String::new();
    loop {
        header_line.clear();
        reader.read_line(&mut header_line).await?;
        if header_line.trim().is_empty() {
            break;
        }
    }

    Ok(ConnectRequest { host, port })
}

/// Parse "host:port" format
pub fn parse_host_port(host_port: &str) -> io::Result<(String, u16)> {
    let parts: Vec<&str> = host_port.rsplitn(2, ':').collect();
    
    if parts.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected host:port format",
        ));
    }

    let port_str = parts[0];
    let host = parts[1].to_string();

    let port = port_str.parse::<u16>().map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "Invalid port number")
    })?;

    Ok((host, port))
}

/// Send HTTP 200 Connection Established response
pub async fn send_connect_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
) -> io::Result<()> {
    writer.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
    writer.flush().await?;
    Ok(())
}

/// Send HTTP 502 Bad Gateway response
pub async fn send_error_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    status_code: u16,
    reason: &str,
) -> io::Result<()> {
    let response = format!("HTTP/1.1 {} {}\r\nContent-Length: 0\r\n\r\n", status_code, reason);
    writer.write_all(response.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

/// Bidirectional tunnel between client and upstream
/// Copies data in both directions, allowing for interception
pub async fn tunnel(
    mut client: TcpStream,
    mut upstream: TcpStream,
) -> io::Result<()> {
    let (mut client_read, mut client_write) = client.split();
    let (mut upstream_read, mut upstream_write) = upstream.split();

    tokio::select! {
        // Client -> Upstream
        result = tokio::io::copy(&mut client_read, &mut upstream_write) => {
            result?;
        }
        // Upstream -> Client (in parallel)
        result = tokio::io::copy(&mut upstream_read, &mut client_write) => {
            result?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host_port_valid() {
        let (host, port) = parse_host_port("example.com:443").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 443);
    }

    #[test]
    fn test_parse_host_port_with_subdomain() {
        let (host, port) = parse_host_port("api.example.com:8443").unwrap();
        assert_eq!(host, "api.example.com");
        assert_eq!(port, 8443);
    }

    #[test]
    fn test_parse_host_port_localhost() {
        let (host, port) = parse_host_port("localhost:3000").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 3000);
    }

    #[test]
    fn test_parse_host_port_invalid_port() {
        let result = parse_host_port("example.com:invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_host_port_missing_port() {
        let result = parse_host_port("example.com");
        assert!(result.is_err());
    }
}
