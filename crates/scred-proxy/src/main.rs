use anyhow::{anyhow, Result};
use rustls::{ClientConfig, RootCertStore, ServerName};
use scred_config::ConfigLoader;
use scred_http::fixed_upstream::FixedUpstream;
use scred_http::streaming_request::{stream_request_to_upstream, StreamingRequestConfig};
use scred_http::streaming_response::{stream_response_to_client, StreamingResponseConfig};
use scred_http::{dns_resolver::DnsResolver, http_line_reader::read_response_line};
use scred_http::{PatternSelector};
use scred_http_redactor::H2Redactor;
use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor, StreamingConfig};
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

/// Per-path redaction rule
#[derive(Clone, Debug)]
struct PathRedactionRule {
    path_pattern: String,
    should_redact: bool,
    reason: Option<String>,
}

#[derive(Clone, Debug)]
struct ProxyConfig {
    listen_addr: String,
    listen_port: u16,
    upstream: FixedUpstream,
    redaction_mode: RedactionMode,
    detect_selector: PatternSelector,
    redact_selector: PatternSelector,
    per_path_rules: Vec<PathRedactionRule>,
}

impl ProxyConfig {
    /// Load configuration from files and environment variables
    fn from_config_file() -> Result<Self> {
        // Load file-based configuration
        let file_config = ConfigLoader::load()?;
        ConfigLoader::validate(&file_config)?;

        // Extract proxy configuration section
        let proxy_cfg = file_config.scred_proxy
            .ok_or_else(|| anyhow!(
                "No scred-proxy configuration found in config file. \
                 Please configure scred-proxy section in config file (scred.yaml, ~/.scred/config.yaml, or /etc/scred/config.yaml), \
                 or set SCRED_PROXY_UPSTREAM_URL environment variable."
            ))?;

        // Extract listen settings
        let listen_port = proxy_cfg.listen.port
            .or_else(|| env::var("SCRED_PROXY_LISTEN_PORT").ok().and_then(|p| p.parse().ok()))
            .unwrap_or(9999);

        let listen_addr = proxy_cfg.listen.address.unwrap_or_else(|| "0.0.0.0".to_string());

        // Extract upstream URL (required)
        let upstream_url = proxy_cfg.upstream.url
            .or_else(|| env::var("SCRED_PROXY_UPSTREAM_URL").ok())
            .ok_or_else(|| anyhow!(
                "Upstream URL is required. \
                 Set via 'scred_proxy.upstream.url' in config file or \
                 SCRED_PROXY_UPSTREAM_URL environment variable"
            ))?;

        let upstream = FixedUpstream::parse(&upstream_url)?;

        // Parse redaction mode from config
        let mode_str = proxy_cfg.redaction.mode.to_lowercase();
        let redaction_mode = match mode_str.as_str() {
            "detect" => RedactionMode::Detect,
            "redact" => RedactionMode::Redact,
            "passthrough" | "passive" => RedactionMode::Passthrough,
            _ => RedactionMode::Redact,
        };

        // Convert pattern tiers to selectors
        let detect_str = proxy_cfg.redaction.patterns.detect.join(",");
        let redact_str = proxy_cfg.redaction.patterns.redact.join(",");
        
        let detect_selector = match PatternSelector::from_str(&detect_str) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ERROR: Invalid detect patterns in config: '{}'", detect_str);
                eprintln!("Reason: {}", e);
                eprintln!("\nValid tier names: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS");
                std::process::exit(1);
            }
        };
        
        let redact_selector = match PatternSelector::from_str(&redact_str) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ERROR: Invalid redact patterns in config: '{}'", redact_str);
                eprintln!("Reason: {}", e);
                eprintln!("\nValid tier names: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS");
                std::process::exit(1);
            }
        };

        // Convert per-path rules
        let per_path_rules: Vec<PathRedactionRule> = proxy_cfg.rules.iter().map(|rule| {
            PathRedactionRule {
                path_pattern: rule.path.clone(),
                should_redact: rule.redact,
                reason: rule.reason.clone(),
            }
        }).collect();

        info!("Configuration loaded from file");
        info!("Listen: {}:{}", listen_addr, listen_port);
        info!("Upstream: {}", upstream_url);
        info!("Redaction mode: {:?}", redaction_mode);
        if !per_path_rules.is_empty() {
            info!("Per-path rules: {}", per_path_rules.len());
        }

        Ok(Self {
            listen_addr,
            listen_port,
            upstream,
            redaction_mode,
            detect_selector,
            redact_selector,
            per_path_rules,
        })
    }

    /// Check if a path should be redacted based on per-path rules
    fn should_redact_path(&self, path: &str) -> bool {
        // Find matching rule
        for rule in &self.per_path_rules {
            if Self::path_matches(&rule.path_pattern, path) {
                if let Some(reason) = &rule.reason {
                    debug!("Path rule matched '{}': {}", rule.path_pattern, reason);
                }
                return rule.should_redact;
            }
        }
        
        // Default: redact based on redaction mode
        self.redaction_mode == RedactionMode::Redact
    }

    /// Check if path matches pattern (supports * wildcard)
    fn path_matches(pattern: &str, path: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if !pattern.contains('*') {
            return pattern == path;
        }

        // Simple wildcard matching
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut remaining = path;

        for (i, part) in parts.iter().enumerate() {
            if i == 0 {
                // First part must match at start
                if !remaining.starts_with(part) {
                    return false;
                }
                remaining = &remaining[part.len()..];
            } else if i == parts.len() - 1 {
                // Last part must match at end
                if !remaining.ends_with(part) {
                    return false;
                }
            } else {
                // Middle parts must be found in order
                if let Some(pos) = remaining.find(part) {
                    remaining = &remaining[pos + part.len()..];
                } else {
                    return false;
                }
            }
        }

        true
    }

    /// Extract flag value from command line arguments
    fn extract_flag_value(args: &[String], flag: &str) -> Option<String> {
        for (i, arg) in args.iter().enumerate() {
            if arg == flag && i + 1 < args.len() {
                return Some(args[i + 1].clone());
            } else if arg.starts_with(&format!("{}=", flag)) {
                return Some(arg.split('=').nth(1).unwrap_or("").to_string());
            }
        }
        None
    }

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
        let list_tiers = args.contains(&"--list-tiers".to_string());

        // Handle special commands
        if list_tiers {
            println!("SCRED Proxy - Pattern Tiers");
            println!();
            println!("{:<20} {:<10} {}", "Tier", "Risk", "Redact by Default");
            println!("{}", "=".repeat(50));
            let tiers = [
                ("CRITICAL", "95%", "YES"),
                ("API_KEYS", "80%", "YES"),
                ("INFRASTRUCTURE", "60%", "NO"),
                ("SERVICES", "40%", "NO"),
                ("PATTERNS", "30%", "NO"),
            ];
            for (tier, risk, redact) in &tiers {
                println!("{:<20} {:<10} {}", tier, risk, redact);
            }
            std::process::exit(0);
        }

        // Extract pattern tier values
        let detect_flag = Self::extract_flag_value(&args, "--detect");
        let redact_flag = Self::extract_flag_value(&args, "--redact");

        // Get from environment or use defaults
        let detect_env = env::var("SCRED_DETECT_PATTERNS").ok();
        let redact_env = env::var("SCRED_REDACT_PATTERNS").ok();

        let detect_str = detect_flag
            .or_else(|| detect_env.clone())
            .unwrap_or_else(|| "CRITICAL,API_KEYS,INFRASTRUCTURE".to_string());
        let redact_str = redact_flag
            .or_else(|| redact_env.clone())
            .unwrap_or_else(|| "CRITICAL,API_KEYS".to_string());

        // Parse selectors - must not fail silently (consistency with from_config_file)
        let detect_selector = match PatternSelector::from_str(&detect_str) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ERROR: Invalid detect patterns in env: '{}'", detect_str);
                eprintln!("Reason: {}", e);
                eprintln!("\nValid tier names: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS");
                std::process::exit(1);
            }
        };
        let redact_selector = match PatternSelector::from_str(&redact_str) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ERROR: Invalid redact patterns in env: '{}'", redact_str);
                eprintln!("Reason: {}", e);
                eprintln!("\nValid tier names: CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS");
                std::process::exit(1);
            }
        };

        info!("[proxy-config] Detect: {}", detect_selector.description());
        info!("[proxy-config] Redact: {}", redact_selector.description());

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
            detect_selector,
            redact_selector,
            per_path_rules: vec![],  // No per-path rules from env vars
        })
    }

    /// Create a default proxy configuration
    fn from_defaults() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 9999,
            upstream: FixedUpstream::parse("http://localhost:8000").unwrap(),
            redaction_mode: RedactionMode::Redact,
            detect_selector: PatternSelector::default_detect(),
            redact_selector: PatternSelector::default_redact(),
            per_path_rules: vec![],
        }
    }

    /// Merge another config into this one (other config takes precedence)
    /// This allows layering: CLI > ENV > File > Default
    fn merge_from(&mut self, other: ProxyConfig) {
        // Only override if other has non-default values
        if other.listen_addr != "0.0.0.0" {
            self.listen_addr = other.listen_addr;
        }
        if other.listen_port != 9999 {
            self.listen_port = other.listen_port;
        }
        // Always update upstream if it's different
        self.upstream = other.upstream;
        
        // Only update mode if it's not passthrough (the default)
        if other.redaction_mode != RedactionMode::Passthrough {
            self.redaction_mode = other.redaction_mode;
        }
        
        // Update selectors
        self.detect_selector = other.detect_selector;
        self.redact_selector = other.redact_selector;
        
        // Extend per-path rules (don't replace, add to existing)
        self.per_path_rules.extend(other.per_path_rules);
    }
}

async fn handle_connection(stream: TcpStream, config: Arc<ProxyConfig>) -> Result<()> {
    let peer_addr = stream.peer_addr()?;
    info!("New connection from {}", peer_addr);

    // Extract path from request line for per-path rule checking
    // Request line format: "GET /path HTTP/1.1"
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

    // Extract path from request line
    let request_path = if let Some(path_start) = first_line.find(' ') {
        if let Some(path_end) = first_line[path_start + 1..].find(' ') {
            first_line[path_start + 1..path_start + 1 + path_end].to_string()
        } else {
            "/".to_string()
        }
    } else {
        "/".to_string()
    };
    
    // Check if path should be redacted based on per-path rules
    let should_redact = config.should_redact_path(&request_path);
    
    if !should_redact {
        info!("[{}] Per-path rule: SKIPPING redaction for path: {}", peer_addr, request_path);
    } else if !config.per_path_rules.is_empty() {
        info!("[{}] Per-path rule: APPLYING redaction for path: {}", peer_addr, request_path);
    }

    // Create redaction engine based on mode AND per-path rules
    let redaction_config = if !should_redact {
        // Path rule says skip redaction
        debug!("[{}] Redaction disabled by per-path rule for: {}", peer_addr, request_path);
        RedactionConfig {
            enabled: false,
        }
    } else {
        match config.redaction_mode {
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
        }
    };

    let redaction_engine = Arc::new(RedactionEngine::new(redaction_config));
    
    info!("[{}] Detect selector: {}", peer_addr, config.detect_selector.description());
    info!("[{}] Redact selector: {}", peer_addr, config.redact_selector.description());
    
    // Phase 4: Create StreamingRedactor with selector support
    // The selector is now passed through to filter patterns during streaming redaction
    let redactor = Arc::new(StreamingRedactor::with_selector(
        redaction_engine.clone(),
        StreamingConfig::default(),
        config.redact_selector.clone(),
    ));

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
            StreamingRequestConfig {
                debug: false,
                max_headers_size: 64 * 1024,
                redact_selector: Some(config.redact_selector.clone()),
            },
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
            StreamingResponseConfig {
                debug: false,
                add_scred_header: true,
                redact_selector: Some(config.redact_selector.clone()),
            },
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
            StreamingRequestConfig {
                debug: false,
                max_headers_size: 64 * 1024,
                redact_selector: Some(config.redact_selector.clone()),
            },
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
            StreamingResponseConfig {
                debug: false,
                add_scred_header: true,
                redact_selector: Some(config.redact_selector.clone()),
            },
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

    // Implement proper configuration precedence: CLI > ENV > File > Default
    // Step 1: Start with defaults
    let mut config = ProxyConfig::from_defaults();
    info!("[config] Starting with defaults");

    // Step 2: Load and merge file configuration (if present)
    match ProxyConfig::from_config_file() {
        Ok(file_config) => {
            info!("[config] File configuration loaded, merging...");
            config.merge_from(file_config);
        }
        Err(e) => {
            info!("[config] No config file found: {}. Continuing with defaults.", e);
        }
    }

    // Step 3: Load and merge environment variables
    match ProxyConfig::from_env() {
        Ok(env_config) => {
            info!("[config] Environment configuration loaded, merging (overriding file)...");
            config.merge_from(env_config);
        }
        Err(e) => {
            // from_env might fail if SCRED_PROXY_UPSTREAM_URL is not set
            // That's OK if we got it from file or have a default
            if config.upstream.host.is_empty() {
                // No upstream set - this is critical
                eprintln!("ERROR: No upstream URL configured!");
                eprintln!("Provide via: --upstream URL or config file or SCRED_PROXY_UPSTREAM_URL env var");
                std::process::exit(1);
            }
            info!("[config] Env config not fully available ({}), using file/defaults", e);
        }
    }

    let config = Arc::new(config);

    // Log final configuration
    info!("[config] FINAL CONFIGURATION:");
    info!("[config]   Listen: {}:{}", config.listen_addr, config.listen_port);
    info!("[config]   Upstream: {}://{}{}", config.upstream.scheme, config.upstream.authority(), config.upstream.base_path);
    info!("[config]   Mode: {:?}", config.redaction_mode);
    info!("[config]   Detect selector: {}", config.detect_selector.description());
    info!("[config]   Redact selector: {}", config.redact_selector.description());
    if !config.per_path_rules.is_empty() {
        info!("[config]   Per-path rules: {}", config.per_path_rules.len());
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_exact_match() {
        assert!(ProxyConfig::path_matches("/health", "/health"));
        assert!(!ProxyConfig::path_matches("/health", "/healths"));
    }

    #[test]
    fn test_path_wildcard_suffix() {
        assert!(ProxyConfig::path_matches("/api/internal/*", "/api/internal/users"));
        assert!(ProxyConfig::path_matches("/api/internal/*", "/api/internal/settings"));
        assert!(!ProxyConfig::path_matches("/api/internal/*", "/api/public/users"));
    }

    #[test]
    fn test_path_wildcard_both() {
        assert!(ProxyConfig::path_matches("*/logs/*", "/app/logs/error.log"));
        assert!(ProxyConfig::path_matches("*/logs/*", "/sys/logs/info.log"));
    }

    #[test]
    fn test_path_wildcard_all() {
        assert!(ProxyConfig::path_matches("*", "/anything"));
        assert!(ProxyConfig::path_matches("*", "/path/to/resource"));
    }

    #[test]
    fn test_should_redact_path_with_rules() {
        let rules = vec![
            PathRedactionRule {
                path_pattern: "/health".to_string(),
                should_redact: false,
                reason: None,
            },
            PathRedactionRule {
                path_pattern: "/api/internal/*".to_string(),
                should_redact: false,
                reason: None,
            },
        ];

        // Test exact match
        assert!(!ProxyConfig::path_matches(&rules[0].path_pattern, "/health"));
        // Test wildcard match
        assert!(ProxyConfig::path_matches(&rules[1].path_pattern, "/api/internal/users"));
    }
}
