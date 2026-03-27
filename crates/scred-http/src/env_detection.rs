/// Environment Variable Format Detection
///
/// Analyzes input characteristics to automatically detect if content is:
/// - Environment variable format (KEY=VALUE)
/// - Plain text (regular files, logs)
/// - Binary (refuse to process)
///
/// Returns a detection result with confidence score.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetectionMode {
    /// Likely environment variable format (KEY=VALUE style)
    EnvFormat,
    /// Plain text, use normal pattern redaction
    TextFormat,
    /// Binary content detected, refuse env-mode
    BinaryDetected,
}

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub mode: DetectionMode,
    pub score: f32,
    pub reason: String,
}

const DETECTION_SAMPLE_SIZE: usize = 512; // Only scan first 512 bytes (2-3 lines)

// Secret keywords that often appear at line start in env vars
const SECRET_KEYWORDS: &[&str] = &[
    "API_KEY",
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "CREDENTIAL",
    "PASSPHRASE",
    "AWS_ACCESS",
    "AWS_SECRET",
    "PRIVATE_KEY",
];

// Common environment variable names
const COMMON_ENV_VARS: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "SHELL",
    "LANG",
    "PWD",
    "HOSTNAME",
    "TERM",
];

/// Analyze input and determine if it's environment variable format
pub fn detect_format(input: &[u8]) -> DetectionResult {
    // Limit analysis to first chunk to avoid performance impact
    let sample = if input.len() > DETECTION_SAMPLE_SIZE {
        &input[..DETECTION_SAMPLE_SIZE]
    } else {
        input
    };

    // Check for binary content first (safety check)
    if contains_null_bytes(sample) {
        return DetectionResult {
            mode: DetectionMode::BinaryDetected,
            score: 0.0,
            reason: "Binary content detected (null bytes)".to_string(),
        };
    }

    // Check for high ratio of non-printable characters
    if has_high_non_printable_ratio(sample) {
        return DetectionResult {
            mode: DetectionMode::BinaryDetected,
            score: 0.0,
            reason: "Binary content detected (non-printable chars)".to_string(),
        };
    }

    // Convert to string for analysis
    let sample_str = match std::str::from_utf8(sample) {
        Ok(s) => s,
        Err(_) => {
            return DetectionResult {
                mode: DetectionMode::BinaryDetected,
                score: 0.0,
                reason: "Invalid UTF-8 detected".to_string(),
            };
        }
    };

    // Calculate env-format score
    let score = calculate_env_score(sample_str);

    // Threshold: >=0.45 = env-mode, <0.45 = text-mode
    let (mode, reason) = if score >= 0.45 {
        (
            DetectionMode::EnvFormat,
            format!("Env-format detected (score: {:.2})", score),
        )
    } else {
        (
            DetectionMode::TextFormat,
            format!("Text-format detected (score: {:.2})", score),
        )
    };

    DetectionResult { mode, score, reason }
}

/// Check if content contains null bytes (binary indicator)
fn contains_null_bytes(data: &[u8]) -> bool {
    data.contains(&0)
}

/// Check if high ratio of non-printable characters (binary indicator)
fn has_high_non_printable_ratio(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    let non_printable = data
        .iter()
        .filter(|&&b| {
            // Count bytes that are not:
            // - ASCII printable (32-126)
            // - Tab (9), newline (10), carriage return (13)
            !((32..=126).contains(&b) || b == 9 || b == 10 || b == 13)
        })
        .count();

    let ratio = non_printable as f32 / data.len() as f32;
    ratio > 0.3 // More than 30% non-printable = likely binary
}

/// Calculate environment-format confidence score (0.0 to 1.0)
fn calculate_env_score(content: &str) -> f32 {
    if content.is_empty() {
        return 0.0;
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return 0.0;
    }

    let mut score = 0.0f32;

    // Signal 1: VAR_NAME=VALUE pattern density (0.3 weight)
    let var_equals_lines = lines
        .iter()
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with(';')
                && line.contains('=')
        })
        .count();

    let var_equals_density = var_equals_lines as f32 / lines.len() as f32;
    score += var_equals_density * 0.3;

    // Signal 2: Secret keywords in variable name (0.5 weight)
    let secret_keyword_lines = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim_start();
            let var_name = trimmed.split('=').next().unwrap_or("");
            SECRET_KEYWORDS
                .iter()
                .any(|keyword| var_name.to_uppercase().contains(keyword))
        })
        .count();

    let secret_keyword_ratio = if var_equals_lines > 0 {
        secret_keyword_lines as f32 / var_equals_lines as f32
    } else {
        0.0
    };

    score += (secret_keyword_ratio * 0.8).min(1.0) * 0.5;

    // Signal 3: Common env var names (0.2 weight)
    let common_env_lines = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim_start();
            COMMON_ENV_VARS
                .iter()
                .any(|var| trimmed.starts_with(var) && trimmed.contains('='))
        })
        .count();

    let common_env_ratio = if var_equals_lines > 0 {
        common_env_lines as f32 / var_equals_lines as f32
    } else {
        0.0
    };

    score += (common_env_ratio * 0.8).min(1.0) * 0.2;

    // Clamp to 0.0-1.0 range
    score.min(1.0)
}

