//! Policy integration for scred-proxy
//!
//! Integrates policy system with the forward proxy.

use std::sync::Arc;
use scred_policy::PolicyEngine;
use scred_config::FileConfig;

/// Initialize policy from config file
///
/// This starts the discovery server if enabled in the config.
pub fn init_policy_from_config(
    file_config: &FileConfig,
) -> Option<Arc<PolicyEngine>> {
    use tracing::{info, warn};

    let policy_cfg = &file_config.policy;

    if !policy_cfg.enabled {
        info!("Policy disabled in config");
        return None;
    }

    // Initialize policy engine
    match PolicyEngine::new(policy_cfg.clone()) {
        Ok(engine) => {
            info!("Policy engine initialized");

            // Start discovery server if enabled
            if policy_cfg.discovery.enabled {
                let port = policy_cfg.discovery.port;
                info!("Discovery server enabled on port {}", port);
                // Note: Discovery server is started by the engine
            }

            Some(Arc::new(engine))
        }
        Err(e) => {
            warn!("Failed to initialize policy engine: {}", e);
            None
        }
    }
}
