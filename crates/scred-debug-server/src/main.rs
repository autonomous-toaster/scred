use anyhow::Result;
use axum::{
    extract::ConnectInfo, http::StatusCode, response::IntoResponse, routing::get, Json, Router,
};
use clap::Parser;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "scred-debug-server")]
#[command(about = "Debug echo server for testing scred-proxy")]
struct Args {
    /// Listen port
    #[arg(short, long, default_value = "8888")]
    port: u16,

    /// Listen address
    #[arg(short, long, default_value = "127.0.0.1")]
    addr: String,

    /// Response size in bytes (default: 500)
    #[arg(short, long, default_value = "500")]
    response_size: usize,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[derive(Clone)]
struct AppState {
    request_count: Arc<AtomicU64>,
    response_size: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    let addr = format!("{}:{}", args.addr, args.port);
    let socket_addr: SocketAddr = addr.parse()?;

    info!(
        "Starting debug echo server on {} (response_size={})",
        socket_addr, args.response_size
    );

    let state = AppState {
        request_count: Arc::new(AtomicU64::new(0)),
        response_size: args.response_size,
    };

    // Build router
    let app = Router::new()
        .route("/", get(handler).post(handler))
        .route("/*path", get(handler).post(handler))
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&socket_addr).await?;
    info!("Listening on {}", socket_addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn handler(
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let request_num = state.request_count.fetch_add(1, Ordering::Relaxed);

    if request_num % 100 == 0 {
        println!("  Request {}: from {}", request_num, peer_addr);
    }

    let response_data = json!({
        "request_num": request_num,
        "client": peer_addr.to_string(),
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
        // Pad response to desired size
        "data": "x".repeat(state.response_size.saturating_sub(200))
    });

    (StatusCode::OK, Json(response_data))
}
