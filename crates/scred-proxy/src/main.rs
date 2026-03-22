use anyhow::{anyhow, Result};
use rustls::{ClientConfig, RootCertStore, ServerName};
use scred_http::fixed_upstream::FixedUpstream;
use scred_http::streaming_request::{stream_request_to_upstream, StreamingRequestConfig};
use scred_http::streaming_response::{stream_response_to_client, StreamingResponseConfig};
use scred_http::{dns_resolver::DnsResolver, http_line_reader::read_response_line};
use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};
use std::env;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsConnector;
use tracing::info;

#[derive(Clone, Debug)]
struct ProxyConfig {
    listen_addr: String,
    listen_port: u16,
    upstream: FixedUpstream,
}

impl ProxyConfig {
    fn from_env() -> Result<Self> {
        let listen_port = env::var("SCRED_PROXY_LISTEN_PORT")
            .unwrap_or_else(|_| "9999".to_string())
            .parse::<u16>()?;

        let upstream_url = env::var("SCRED_PROXY_UPSTREAM_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        Ok(Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port,
            upstream: FixedUpstream::parse(&upstream_url)?,
        })
    }
}

async fn handle_connection(stream: TcpStream, config: Arc<ProxyConfig>) -> Result<()> {
    let peer_addr = stream.peer_addr()?;
    info!("New connection from {}", peer_addr);

    let redaction_engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = Arc::new(StreamingRedactor::with_defaults(redaction_engine));

    let (client_read, mut client_write) = stream.into_split();
    let mut client_reader = BufReader::new(client_read);

    let mut first_line = String::new();
    client_reader.read_line(&mut first_line).await?;

    if first_line.is_empty() {
        info!("[{}] Empty request", peer_addr);
        return Ok(());
    }

    let first_line = first_line.trim();
    info!("[{}] Request line: {}", peer_addr, first_line);

    let rewritten_request_line = config.upstream.rewrite_request_line(first_line)?;
    let upstream_addr = config.upstream.authority();
    
    // Use the config listening address as the proxy_host for Location rewriting
    // In real deployments, this should match what clients use to connect
    let proxy_host = format!("localhost:{}", config.listen_port);

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

        stream_response_to_client(
            &mut BufReader::new(&mut upstream),
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

        stream_response_to_client(
            &mut BufReader::new(&mut upstream),
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
