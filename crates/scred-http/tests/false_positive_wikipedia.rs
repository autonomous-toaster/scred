use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader};

use scred_http::http_line_reader::read_response_line;
use scred_http::streaming_response::{stream_response_to_client, StreamingResponseConfig};
use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};

fn assert_no_obvious_false_positive(body: &str) {
    // Just check for obvious false-positive redaction markers
    let unexpected = [
        "tranxxxxxxxx",  // transitional
        "charxxxx",      // charset
        "Netsxxxxxxxx",  // Netscape
        "Intexxxxxxxx",  // Internet
        "Wikixxxxxxxx",  // Wikipedia
        "Textxxxxxxxx",  // Text
        "tempxxxxxxxx",  // template
    ];

    let mut found_false_positives = Vec::new();
    for marker in unexpected {
        if body.contains(marker) {
            found_false_positives.push(marker);
        }
    }
    
    if !found_false_positives.is_empty() {
        eprintln!("Found false-positive redactions: {:?}", found_false_positives);
        eprintln!("Total body length: {} bytes", body.len());
        
        // Show some context
        for marker in &found_false_positives {
            if let Some(pos) = body.find(marker) {
                let start = pos.saturating_sub(40);
                let end = (pos + marker.len() + 40).min(body.len());
                eprintln!("  '{}' at offset {}: ...{}...", marker, pos, &body[start..end]);
            }
        }
    }
    
    assert!(
        found_false_positives.is_empty(),
        "found {} obvious false positive markers: {:?}",
        found_false_positives.len(),
        found_false_positives
    );
}

async fn fetch_https_body(host: &str, path: &str) -> String {
    use rustls::{ClientConfig, RootCertStore, ServerName};
    use tokio::net::TcpStream;
    use tokio_rustls::TlsConnector;

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
    let tcp = TcpStream::connect(format!("{}:443", host)).await.unwrap();
    let server_name = ServerName::try_from(host).unwrap();
    let mut tls = connector.connect(server_name, tcp).await.unwrap();

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Mozilla/5.0 (compatible; scred-false-positive-test)\r\nAccept: text/html,*/*\r\nConnection: close\r\n\r\n",
        path, host
    );
    tls.write_all(request.as_bytes()).await.unwrap();
    tls.flush().await.unwrap();

    let response_line = read_response_line(&mut tls).await.unwrap();
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = Arc::new(StreamingRedactor::with_defaults(engine));

    let mut sink = Vec::new();
    stream_response_to_client(
        &mut BufReader::new(&mut tls),
        &mut sink,
        &response_line,
        redactor,
        StreamingResponseConfig::default(),
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(sink).unwrap();
    let body = rendered.split("\r\n\r\n").nth(1).unwrap_or("");
    body.to_string()
}

#[tokio::test]
async fn wikipedia_page_should_not_show_obvious_false_positives() {
    let body = fetch_https_body("en.wikipedia.org", "/wiki/HTTP").await;
    assert_no_obvious_false_positive(&body);
}
