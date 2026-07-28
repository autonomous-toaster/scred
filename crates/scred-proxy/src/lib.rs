pub mod handler;

use anyhow::{anyhow, Result};
use scred_config::ConfigLoader;
use scred_http::fixed_upstream::FixedUpstream;
use std::env;
use tracing::info;

/// Simple proxy configuration
#[derive(Clone, Debug)]
pub struct ProxyConfig {
    pub listen_addr: String,
    pub listen_port: u16,
    pub upstream: FixedUpstream,
}

impl ProxyConfig {
    pub fn from_config_file() -> Result<Self> {
        let mut file_config = ConfigLoader::load()?;
        ConfigLoader::validate(&mut file_config)?;

        let proxy_cfg = file_config.scred_proxy.ok_or_else(|| {
            anyhow!(
                "No scred-proxy configuration found. \
                 Configure scred-proxy section in scred.yaml or set SCRED_PROXY_UPSTREAM_URL"
            )
        })?;

        let listen_port = proxy_cfg
            .listen
            .port
            .or_else(|| {
                env::var("SCRED_PROXY_LISTEN_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
            })
            .unwrap_or(9999);

        let listen_addr = proxy_cfg
            .listen
            .address
            .unwrap_or_else(|| "0.0.0.0".to_string());

        let upstream_url = proxy_cfg
            .upstream
            .url
            .or_else(|| env::var("SCRED_PROXY_UPSTREAM_URL").ok())
            .ok_or_else(|| {
                anyhow!(
                    "Upstream URL required. \
                     Set 'scred_proxy.upstream.url' or SCRED_PROXY_UPSTREAM_URL"
                )
            })?;

        let upstream = FixedUpstream::parse(&upstream_url)?;

        info!(
            "Configuration loaded: listen={}:{}",
            listen_addr, listen_port
        );
        info!(
            "Upstream: {}://{}{}",
            upstream.scheme,
            upstream.authority(),
            upstream.base_path
        );

        Ok(Self {
            listen_addr,
            listen_port,
            upstream,
        })
    }

    pub fn from_env() -> Result<Self> {
        let listen_port = env::var("SCRED_PROXY_LISTEN_PORT")
            .unwrap_or_else(|_| "9999".to_string())
            .parse::<u16>()?;

        let upstream_url = env::var("SCRED_PROXY_UPSTREAM_URL")
            .map_err(|_| anyhow!("SCRED_PROXY_UPSTREAM_URL required when no config file found"))?;

        info!("Environment configuration: listen_port={}", listen_port);

        Ok(Self {
            listen_addr: "0.0.0.0".to_string(),
            listen_port,
            upstream: FixedUpstream::parse(&upstream_url)?,
        })
    }
}
