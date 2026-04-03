//! Policy Integration for scred-mitm
//!
//! Integrates the PolicyEngine with the MITM proxy.
//! Combines both placeholder replacement and secret redaction.

use std::sync::Arc;
use scred_config::FileConfig;
use scred_policy::PolicyEngine;
use tracing::{info, error};

/// Initialize policy from config file
///
/// This replaces the separate policy and redaction initialization.
pub fn init_policy(
    file_config: &FileConfig,
) -> Option<Arc<PolicyEngine>> {
    let policy_config = &file_config.policy;

    if !policy_config.enabled {
        info!("Policy disabled in config");
        return None;
    }

    // Initialize policy engine directly from config
    match PolicyEngine::new(policy_config.clone()) {
        Ok(engine) => {
            info!(
                "[policy] Policy engine initialized with {} secrets",
                engine.secrets().len()
            );

            // Log discovery info
            if policy_config.discovery.enabled {
                info!("[policy] Discovery API available at port {}", policy_config.discovery.port);
            }

            Some(Arc::new(engine))
        }
        Err(e) => {
            error!("[policy] Failed to initialize policy engine: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_policy_disabled() {
        let mut config = FileConfig::default();
        config.policy.enabled = false;
        let result = init_policy(&config);
        assert!(result.is_none());
    }
}
