//! Placeholder generation - deterministic, domain-independent placeholders

use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// A generated placeholder with metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placeholder {
    /// The secret name (e.g., "OPENAI_API_KEY")
    pub name: String,
    /// The generated placeholder value
    pub value: String,
    /// The original prefix preserved (e.g., "sk-")
    pub prefix: String,
}

/// Generates deterministic placeholders for secrets
pub struct PlaceholderGenerator {
    /// Stable seed for deterministic generation
    seed: String,
    /// Cache of generated placeholders
    cache: HashMap<String, Placeholder>,
}

impl Clone for PlaceholderGenerator {
    fn clone(&self) -> Self {
        Self {
            seed: self.seed.clone(),
            cache: HashMap::new(), // Start with empty cache in clone
        }
    }
}

impl PlaceholderGenerator {
    /// Create a new generator with the given seed
    pub fn new(seed: &str) -> Self {
        Self {
            seed: seed.to_string(),
            cache: HashMap::new(),
        }
    }

    /// Generate a placeholder for a secret
    ///
    /// # Arguments
    /// * `name` - The secret name (e.g., "OPENAI_API_KEY")
    /// * `value` - The actual secret value
    ///
    /// # Returns
    /// A placeholder that:
    /// - Preserves the original prefix (e.g., "sk-")
    /// - Matches the original length
    /// - Contains only HTTP-safe characters (hex: 0-9, a-f)
    /// - Is deterministic (same name + seed = same placeholder)
    pub fn generate(&mut self, name: &str, value: &str) -> &Placeholder {
        if !self.cache.contains_key(name) {
            let placeholder = self.generate_placeholder(name, value);
            self.cache.insert(name.to_string(), placeholder);
        }
        self.cache.get(name).unwrap()
    }

    fn generate_placeholder(&self, name: &str, value: &str) -> Placeholder {
        // Extract prefix (common patterns)
        let prefix = self.extract_prefix(value);

        // SCRED_MARKER: identifies placeholders for redactor skip
        // Format: {prefix}scred-{hex_chars}
        const SCRED_MARKER: &str = "scrd-";

        // Generate hash input: name + seed
        let hash_input = format!("{}{}", name, self.seed);

        // SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(hash_input.as_bytes());
        let hash = hasher.finalize();

        // Convert to hex
        let hex = hex::encode(&hash);

        // Calculate needed length (excluding prefix and marker)
        let needed_len = value.len().saturating_sub(prefix.len()).saturating_sub(SCRED_MARKER.len());

        // Build placeholder: prefix + "scred-" + hex chars
        let placeholder_value = if needed_len == 0 {
            // Edge case: very short secret, just use prefix + marker (may be truncated)
            format!("{}{}", prefix, SCRED_MARKER)
        } else {
            // prefix + marker + hex (total length matches original)
            let mut result = format!("{}{}", prefix, SCRED_MARKER);
            let hex_chars: Vec<char> = hex.chars().collect();
            for i in 0..needed_len {
                result.push(hex_chars[i % hex_chars.len()]);
            }
            result
        };

        Placeholder {
            name: name.to_string(),
            value: placeholder_value,
            prefix,
        }
    }

    /// Extract prefix using generic rules:
    /// 1. Split on `_` or `-` separator
    /// 2. If first item <= 4 chars, keep it + separator
    /// 3. If first two items both <= 4 chars, keep both + separator (e.g., "sk-proj-")
    /// 4. If no separator and starts with 4 uppercase letters (AKIA, ASIA), keep those
    /// 5. Otherwise, no prefix
    fn extract_prefix(&self, value: &str) -> String {
        if value.is_empty() {
            return String::new();
        }

        // Find separator position
        let sep_pos = value.find(|c| c == '_' || c == '-');

        if let Some(sep_idx) = sep_pos {
            let separator = value.chars().nth(sep_idx).unwrap();
            let first_part = &value[..sep_idx];

            // Check if first part <= 4 chars
            if first_part.len() <= 4 && !first_part.is_empty() {
                // Check for second part after separator
                let rest = &value[sep_idx + 1..];
                let second_sep_pos = rest.find(|c| c == '_' || c == '-');

                if let Some(second_sep_idx) = second_sep_pos {
                    let second_part = &rest[..second_sep_idx];

                    // If second part also <= 4 chars, include both
                    if second_part.len() <= 4 && !second_part.is_empty() {
                        return format!("{}{}{}{}", first_part, separator, second_part, separator);
                    }
                }

                // Just first part + separator
                return format!("{}{}", first_part, separator);
            }
        }

        // Check for 4 uppercase letter prefix (AKIA, ASIA, etc.)
        if value.len() >= 4 {
            let first4 = &value[..4];
            if first4.chars().all(|c| c.is_ascii_uppercase()) {
                return first4.to_string();
            }
        }

        // Check for dot separator (SG. pattern)
        if let Some(dot_idx) = value.find('.') {
            let first_part = &value[..dot_idx];
            if first_part.len() <= 4 && !first_part.is_empty() {
                return format!("{}.", first_part);
            }
        }

        // Default: no prefix
        String::new()
    }

    /// Get a previously generated placeholder
    pub fn get(&self, name: &str) -> Option<&Placeholder> {
        self.cache.get(name)
    }

    /// Get all generated placeholders
    pub fn all(&self) -> &HashMap<String, Placeholder> {
        &self.cache
    }

    /// Check if a value matches a placeholder pattern
    pub fn is_placeholder(&self, value: &str) -> bool {
        // Check for scred- marker: prefix + "scred-" + hex chars
        // This is the standard format for all generated placeholders
        value.contains("scrd-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let mut gen1 = PlaceholderGenerator::new("test-seed");
        let mut gen2 = PlaceholderGenerator::new("test-seed");

        let p1 = gen1.generate("OPENAI_API_KEY", "sk-proj-abc123");
        let p2 = gen2.generate("OPENAI_API_KEY", "sk-proj-abc123");

        assert_eq!(p1.value, p2.value);
    }

    #[test]
    fn test_different_seeds() {
        let mut gen1 = PlaceholderGenerator::new("seed-1");
        let mut gen2 = PlaceholderGenerator::new("seed-2");

        let p1 = gen1.generate("OPENAI_API_KEY", "sk-proj-abc123");
        let p2 = gen2.generate("OPENAI_API_KEY", "sk-proj-abc123");

        assert_ne!(p1.value, p2.value);
    }

    #[test]
    fn test_different_names() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        let p1 = gen.generate("OPENAI_API_KEY", "sk-proj-abc123").clone();
        let p2 = gen.generate("MISTRAL_API_KEY", "sk-proj-xyz789").clone();

        assert_ne!(p1.value, p2.value);
    }

    #[test]
    fn test_prefix_preserved() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        // sk-proj- -> first part "sk" (<=4), second part "proj" (<=4) -> "sk-proj-"
        let p = gen.generate("OPENAI_API_KEY", "sk-proj-abc123xyz");
        assert!(p.value.starts_with("sk-proj-scrd-"));
    }

    #[test]
    fn test_single_part_prefix() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        // sk- -> first part "sk" (<=4) -> "sk-"
        let p = gen.generate("KEY", "sk-abc123");
        assert!(p.value.starts_with("sk-scrd-"));
    }

    #[test]
    fn test_length_preserved() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        let original = "sk-proj-abc123xyz12345678901234567890";
        let p = gen.generate("KEY", original);

        assert_eq!(p.value.len(), original.len());
    }

    #[test]
    fn test_no_prefix() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        // No separator, no 4-uppercase prefix
        let p = gen.generate("SECRET", "abc123xyz456");
        assert!(p.prefix.is_empty());

        // Still length-preserving
        assert_eq!(p.value.len(), "abc123xyz456".len());
    }

    #[test]
    fn test_long_first_part_no_prefix() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        // First part > 4 chars -> no prefix extracted
        let p = gen.generate("KEY", "verylongprefix-abc123");
        assert!(p.prefix.is_empty());
    }

    #[test]
    fn test_cache_used() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        gen.generate("OPENAI_API_KEY", "sk-proj-abc123");
        let p = gen.get("OPENAI_API_KEY").unwrap();

        // Placeholder exists in cache
        assert!(!p.value.is_empty());
    }

    #[test]
    fn test_aws_key_prefix() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        let p = gen.generate("AWS_KEY", "AKIAIOSFODNN7EXAMPLE");
        assert!(p.value.starts_with("AKIAscrd-"));
    }

    #[test]
    fn test_github_token_prefix() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        // ghp_ -> first part "ghp" (<=4) -> "ghp_"
        let p = gen.generate("GITHUB_TOKEN", "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
        assert!(p.value.starts_with("ghp_scrd-"));
    }

    #[test]
    fn test_dot_separator() {
        let mut gen = PlaceholderGenerator::new("test-seed");

        // SG. -> first part "SG" (<=4) -> "SG."
        let p = gen.generate("SENDGRID", "SG.someapikey123");
        assert!(p.value.starts_with("SG.scrd-"));
    }

    #[test]
    fn test_extract_prefix_rules() {
        let gen = PlaceholderGenerator::new("test-seed");

        // Two short parts: sk-proj- -> "sk-proj-"
        assert_eq!(gen.extract_prefix("sk-proj-abc"), "sk-proj-");

        // One short part: sk- -> "sk-"
        assert_eq!(gen.extract_prefix("sk-abc"), "sk-");

        // 4 uppercase: AKIA...
        assert_eq!(gen.extract_prefix("AKIAIOSFODNN"), "AKIA");

        // Underscore: ghp_xxx -> "ghp_"
        assert_eq!(gen.extract_prefix("ghp_xxx"), "ghp_");

        // Dot: SG.xxx -> "SG."
        assert_eq!(gen.extract_prefix("SG.xxx"), "SG.");

        // Long first part: verylong-xxx -> no prefix
        assert_eq!(gen.extract_prefix("verylong-xxx"), "");

        // Second part too long: sk-project-xxx -> "sk-"
        assert_eq!(gen.extract_prefix("sk-project-xxx"), "sk-");

        // Mixed: ab-cd-ef -> "ab-cd-"
        assert_eq!(gen.extract_prefix("ab-cd-ef"), "ab-cd-");
    }
}
