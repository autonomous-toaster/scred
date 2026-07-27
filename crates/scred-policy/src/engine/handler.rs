use super::*;

impl PolicyEngine {
    pub async fn process_request(
        &self,
        body: &mut Vec<u8>,
        host: &str,
    ) -> crate::streaming::ReplacementTracker {
        use crate::streaming::ReplacementTracker;

        let tracker = ReplacementTracker::new();

        // Only process if we have placeholders configured
        if !self.has_placeholders() {
            return tracker;
        }

        let resolved = self.resolve_for_host(host);
        let action = resolved.request_body_action();

        if action == BodyAction::Passthrough {
            return tracker;
        }

        // Replace placeholders with real secrets
        let count = self.replace_placeholders(body);
        if count > 0 {
            tracing::info!(
                "[policy] Replaced {} placeholder(s) in request body for {}",
                count,
                host
            );
        }

        tracker
    }

    /// Process a response body: replace secrets with placeholders
    ///
    /// Uses the tracker from `process_request` to map secrets back
    /// to their placeholders.
    pub async fn process_response(
        &self,
        body: &mut Vec<u8>,
        tracker: &crate::streaming::ReplacementTracker,
    ) -> usize {
        if tracker.replacements().is_empty() {
            return 0;
        }

        let body_str = String::from_utf8_lossy(body).to_string();
        let mut replacements = 0;

        for (secret, (placeholder, _secret_name)) in tracker.replacements() {
            if body_str.contains(secret.as_str()) {
                let new_body = body_str.replace(secret.as_str(), placeholder.as_str());
                *body = new_body.into_bytes();
                replacements += 1;
            }
        }

        if replacements > 0 {
            tracing::info!(
                "[policy] Replaced {} secret(s) with placeholders in response",
                replacements
            );
        }

        replacements
    }
}
