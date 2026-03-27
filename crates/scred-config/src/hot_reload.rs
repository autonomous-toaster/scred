//! Hot-reload support for configuration files
//!
//! This module provides functionality to reload configuration files
//! on SIGHUP signal.

use std::sync::Arc;
use std::path::PathBuf;
use tracing::info;
use tokio::sync::Mutex;

/// Configuration hot-reload handler
#[derive(Clone)]
pub struct HotReloadHandler {
    /// Path to the configuration file being watched
    config_path: Arc<Mutex<Option<PathBuf>>>,
    /// Whether hot-reload is enabled
    enabled: bool,
}

impl HotReloadHandler {
    /// Create a new hot-reload handler
    pub fn new(enabled: bool) -> Self {
        HotReloadHandler {
            config_path: Arc::new(Mutex::new(None)),
            enabled,
        }
    }

    /// Set the configuration file path to watch
    pub async fn set_config_path(&self, path: PathBuf) {
        if self.enabled {
            info!("[hot-reload] Configuration file path set: {:?}", &path);
        }
        *self.config_path.lock().await = Some(path);
    }

    /// Get current configuration file path
    pub async fn get_config_path(&self) -> Option<PathBuf> {
        self.config_path.lock().await.clone()
    }

    /// Check if hot-reload is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// SIGHUP signal handler for hot-reload
#[cfg(unix)]
pub async fn setup_sighup_handler<F>(on_reload: F) -> std::io::Result<()>
where
    F: Fn() + Send + Sync + 'static,
{
    let on_reload = Arc::new(on_reload);

    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
            .expect("Failed to create SIGHUP handler");
        
        while sigterm.recv().await.is_some() {
            info!("[hot-reload] SIGHUP received, reloading configuration...");
            on_reload();
        }
    });

    info!("[hot-reload] SIGHUP handler installed");
    Ok(())
}

/// SIGHUP signal handler stub for non-Unix systems
#[cfg(not(unix))]
pub async fn setup_sighup_handler<F>(_on_reload: F) -> std::io::Result<()>
where
    F: Fn() + Send + Sync + 'static,
{
    debug!("[hot-reload] SIGHUP handler not available on this platform");
    Ok(())
}


