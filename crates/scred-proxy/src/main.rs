use anyhow::{anyhow, Result};
use rustls::{ClientConfig, RootCertStore, ServerName};
use scred_http::fixed_upstream::FixedUpstream;
use scred_http::streaming_request::{stream_request_to_upstream, StreamingRequestConfig};
use scred_http::streaming_response::{stream_response_to_client, StreamingResponseConfig};
use scred_http::{dns_resolver::DnsResolver, http_line_reader::read_response_line};
use scred_http_redactor::H2Redactor;
use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};
use std::env;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsConnector;
use tracing::{info, debug, warn};
use h2::client;
use http::Request;
use bytes::Bytes;

/// Application mode for redaction
#[derive(Clone, Copy, Debug, PartialEq)]
enum RedactionMode {
    /// Log detected secrets without redacting
    Detect,
    /// Actively redact secrets in requests/responses
    Redact,
    /// Passthrough mode - minimal logging
    Passthrough,
}

#[derive(Clone, Debug)]
struct ProxyConfig {
    listen_addr: String,
    listen_port: u16,
    upstream: FixedUpstream,
    redaction_mode: RedactionMode,
}

impl ProxyConfig {
    fn from_env() -> Result<Self> {
        let listen_port = env::var("SCRED_PROXY_LISTEN_PORT")
            .unwrap_or_else(|_| "9999".to_string())
            .parse::<u16>()?;

        // Upstream URL is required - no default for production safety
        let upstream_url = env::var("SCRED_PROXY_UPSTREAM_URL")
            .map_err(|_| anyhow!(
                "SCRED_PROXY_UPSTREAM_URL environment variable is required. \
                 Example: SCRED_PROXY_UPSTREAM_URL=https://backend.example.com"
            ))?;

        // Parse CLI flags
        let args: Vec<String> = env::args().collect();
        let detect_mode = args.contains(&"--detect".to_string());
        let redact_mode = args.contains(&"--redact".to_string());

        let redaction_mode = if detect_mode {
            RedactionMode::Detect
        } else if redact_mode {
            RedactionMode::Redact
        } else {
            RedactionMode::Passthrough
        };

        Ok(Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port,
            upstream: FixedUpstream::parse(&upstream_url)?,
            redaction_mode,
        })
    }
}

async fn handle_connection(stream: TcpStream, config: Arc<ProxyConfig>) -> Result<()> {
    let peer_addr = stream.peer_addr()?;
    info!("New connection from {}", peer_addr);

    // Create redaction engine based on mode
    let redaction_config = match config.redaction_mode {
        RedactionMode::Detect => {
            // Detection mode: enable detection logging but don't actually redact
            debug!("[{}] Detection mode: secrets will be logged but NOT redacted", peer_addr);
            RedactionConfig {
                enabled: false,  // Don't actually redact
            }
        }
        RedactionMode::Redact => {
            // Active redaction mode
            debug!("[{}] Redaction mode: secrets will be detected AND redacted", peer_addr);
            RedactionConfig {
                enabled: true,   // Actively redact
            }
        }
        RedactionMode::Passthrough => {
            // Passthrough mode: minimal overhead
            debug!("[{}] Passthrough mode: forwarding without redaction", peer_addr);
            RedactionConfig {
                enabled: false,
            }
        }
    };

    let redaction_engine = Arc::new(RedactionEngine::new(redaction_config));
    let redactor = Arc::new(StreamingRedactor::with_defaults(redaction_engine.clone()));

    let (client_read, mut client_write) = stream.into_split();
    let mut client_reader = BufReader::new(client_read);

    // Read first line
    let mut first_line = String::new();
    client_reader.read_line(&mut first_line).await?;

    if first_line.is_empty() {
        info!("[{}] Empty request", peer_addr);
        return Ok(());
    }

    let first_line = first_line.trim().to_string();
    info!("[{}] Request line: {}", peer_addr, first_line);

    let upstream_addr = config.upstream.authority();
    let rewritten_request_line = config.upstream.rewrite_request_line(&first_line)?;
    
    // Extract proxy_host from Host header if available, otherwise use peer address
    // First, peek at headers to find Host
    let mut proxy_host = format!("{}:{}", peer_addr.ip(), config.listen_port);
    
    // Try to read headers to find Host header (this will be consumed by stream_request_to_upstream too)
    // For now, use the peer's IP as fallback - production should use actual Host header
    // This is a limitation of single-pass streaming: headers consumed by stream_request_to_upstream
    // TODO: Implement proper header peeking or use Host header from request line if available
    
    info!("[{}] Using proxy_host for Location rewriting: {}", peer_addr, proxy_host);

    let tcp_stream = DnsResolver::connect_with_retry(&upstream_addr).await?;

    if config.upstream.scheme == "https" {
        let tls_stream = connect_tls_upstream(tcp_stream, &config.upstream.host).await?;
        let mut upstream = tls_stream;

        stream_request_to_upstream(
            &mut client_reader,
            &mut upstream,
            &rewritten_request_line,
            redactor.clone(),
            StreamingRequestConfig::default(),
        )
        .await?;

        let response_line = read_response_line(&mut upstream).await?;
        if response_line.is_empty() {
            return Err(anyhow!("empty response line from upstream"));
        }

        let mut upstream_buf = BufReader::new(upstream);
        stream_response_to_client(
            &mut upstream_buf,
            &mut client_write,
            &response_line,
            redactor,
            StreamingResponseConfig::default(),
            Some(&config.upstream.host),
            Some(&proxy_host),
            Some("http"),  // Clients connect to proxy via HTTP
        )
        .await?;
    } else {
        let mut upstream = tcp_stream;

        stream_request_to_upstream(
            &mut client_reader,
            &mut upstream,
            &rewritten_request_line,
            redactor.clone(),
            StreamingRequestConfig::default(),
        )
        .await?;

        let response_line = read_response_line(&mut upstream).await?;
        if response_line.is_empty() {
            return Err(anyhow!("empty response line from upstream"));
        }

        let mut upstream_buf = BufReader::new(upstream);
        stream_response_to_client(
            &mut upstream_buf,
            &mut client_write,
            &response_line,
            redactor,
            StreamingResponseConfig::default(),
            Some(&config.upstream.host),
            Some(&proxy_host),
            Some("http"),  // Clients connect to proxy via HTTP
        )
        .await?;
    }

    client_write.flush().await?;
    Ok(())
}

async fn handle_h2c_connection(
    client_read: tokio::net::tcp::OwnedReadHalf,
    mut client_write: tokio::net::tcp::OwnedWriteHalf,
    upstream_addr: String,
    redaction_engine: Arc<RedactionEngine>,
    _upstream_host: String,
    _first_line: String,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    use std::io::Cursor;
    
    info!("[H2C] HTTP/2 Cleartext upgrade initiated");
    
    // Send 101 Switching Protocols response
    let response = "HTTP/1.1 101 Switching Protocols\r\nUpgrade: h2c\r\nConnection: Upgrade\r\n\r\n";
    client_write.write_all(response.as_bytes()).await?;
    client_write.flush().await?;
    
    info!("[H2C] Sent 101 response, starting h2 handshake");
    
    // Combine read/write halves into a duplex for h2
    // We'll use a simple wrapper that implements AsyncRead + AsyncWrite
    struct DuplexWrapper {
        read: tokio::net::tcp::OwnedReadHalf,
        write: tokio::net::tcp::OwnedWriteHalf,
    }
    
    impl tokio::io::AsyncRead for DuplexWrapper {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.read).poll_read(cx, buf)
        }
    }
    
    impl tokio::io::AsyncWrite for DuplexWrapper {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            std::pin::Pin::new(&mut self.write).poll_write(cx, buf)
        }
        
        fn poll_flush(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.write).poll_flush(cx)
        }
        
        fn poll_shutdown(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.write).poll_shutdown(cx)
        }
    }
    
    let duplex = DuplexWrapper {
        read: client_read,
        write: client_write,
    };
    
    // Start h2 server with duplex
    let mut h2_conn = h2::server::handshake(duplex).await?;
    info!("[H2C] h2 server handshake complete");
    
    // Connect to upstream and start h2 client
    let upstream_stream = DnsResolver::connect_with_retry(&upstream_addr).await?;
    let (mut send_request, upstream_conn) = h2::client::handshake(upstream_stream).await?;
    
    tokio::spawn(async move {
        if let Err(e) = upstream_conn.await {
            tracing::error!("[H2C] Upstream connection error: {}", e);
        }
    });
    
    // Handle streams from client
    while let Some(result) = h2_conn.accept().await {
        let (request, respond) = result?;
        let engine = redaction_engine.clone();
        let mut sender = send_request.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_h2c_stream(request, respond, sender, engine).await {
                warn!("[H2C] Stream error: {}", e);
            }
        });
    }
    
    info!("[H2C] Connection closed");
    Ok(())
}

async fn handle_h2c_stream(
    request: http::Request<h2::RecvStream>,
    mut respond: h2::server::SendResponse<Bytes>,
    _upstream: h2::client::SendRequest<Bytes>,
    engine: Arc<RedactionEngine>,
) -> Result<()> {
    use http::Response;
    
    let method = request.method().clone();
    let uri = request.uri().clone();
    debug!("[H2C Stream] {} {}", method, uri);
    
    // TODO: Full h2c upstream proxy (phase 1.3 extension)
    // For now, return simple redacted response
    
    let response_body = format!(
        r#"{{"status": "ok", "method": "{}", "uri": "{}", "via": "h2c-proxy"}}"#,
        method, uri
    );
    
    // Apply redaction
    let result = engine.redact(&response_body);
    
    // Send response to client
    let response = Response::builder()
        .status(200)
        .body(())
        .unwrap();
    
    let mut send = respond.send_response(response, false)?;
    send.send_data(Bytes::from(result.redacted.into_bytes()), true)?;
    
    Ok(())
}

async fn connect_tls_upstream(
    stream: TcpStream,
    host: &str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));

    let client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(client_config));
    let server_name = ServerName::try_from(host).map_err(|_| anyhow!("invalid upstream host"))?;
    Ok(connector.connect(server_name, stream).await?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = Arc::new(ProxyConfig::from_env()?);

    // Log redaction mode
    match config.redaction_mode {
        RedactionMode::Detect => {
            info!("🔍 DETECT MODE: Logging all detected secrets (no redaction)");
        }
        RedactionMode::Redact => {
            info!("🔐 REDACT MODE: Actively redacting detected secrets");
        }
        RedactionMode::Passthrough => {
            info!("📊 PASSTHROUGH MODE: Forwarding requests with minimal logging");
        }
    }

    let listen_addr = format!("{}:{}", config.listen_addr, config.listen_port);
    info!("Proxy listening on {}", listen_addr);
    info!("Upstream: {}://{}{}", config.upstream.scheme, config.upstream.authority(), config.upstream.base_path);

    let listener = TcpListener::bind(&listen_addr).await?;

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let config = Arc::clone(&config);

        tokio::spawn(async move {
            match handle_connection(stream, config).await {
                Ok(_) => info!("Connection from {} handled successfully", peer_addr),
                Err(e) => tracing::error!("Error handling connection from {}: {}", peer_addr, e),
            }
        });
    }
}
