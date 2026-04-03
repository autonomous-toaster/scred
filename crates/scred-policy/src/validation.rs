//! Secret validation - detect dangerous characters that could enable HTTP tampering

use std::borrow::Cow;

/// Behavior when a secret contains dangerous characters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnInvalid {
    /// Fail startup with error
    #[default]
    Fail,
    /// Log warning, reject secret, continue
    Warn,
    /// Replace dangerous chars with safe alternatives
    Sanitize,
}

/// Validation error types
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Secret contains NUL byte (0x00)")]
    NullByte,

    #[error("Secret contains carriage return (CR, 0x0D)")]
    CarriageReturn,

    #[error("Secret contains line feed (LF, 0x0A)")]
    LineFeed,

    #[error("Secret contains DEL character (0x7F)")]
    DelChar,

    #[error("Secret contains multiple dangerous characters")]
    Multiple,
}

/// Check if a byte is dangerous for HTTP context
#[inline]
fn is_dangerous(b: u8) -> bool {
    matches!(b, 0x00 | 0x0D | 0x0A | 0x7F)
}

/// Validate a secret value for HTTP safety
///
/// # Arguments
/// * `value` - The secret value to validate
/// * `mode` - How to handle invalid characters
///
/// # Returns
/// * `Ok(Cow::Borrowed)` - Value is safe or sanitized (if mode allows)
/// * `Err(ValidationError)` - Value contains dangerous chars and mode is fail/warn
pub fn validate_secret(value: &str, mode: OnInvalid) -> Result<Cow<'_, str>, ValidationError> {
    // Fast path: no dangerous characters
    if !value.bytes().any(is_dangerous) {
        return Ok(Cow::Borrowed(value));
    }

    match mode {
        OnInvalid::Fail | OnInvalid::Warn => {
            // Determine which dangerous chars are present for error message
            let has_null = value.bytes().any(|b| b == 0x00);
            let has_cr = value.bytes().any(|b| b == 0x0D);
            let has_lf = value.bytes().any(|b| b == 0x0A);
            let has_del = value.bytes().any(|b| b == 0x7F);

            let count = [has_null, has_cr, has_lf, has_del]
                .iter()
                .filter(|&&x| x)
                .count();

            let err = if count > 1 {
                ValidationError::Multiple
            } else if has_null {
                ValidationError::NullByte
            } else if has_cr {
                ValidationError::CarriageReturn
            } else if has_lf {
                ValidationError::LineFeed
            } else {
                ValidationError::DelChar
            };

            Err(err)
        }
        OnInvalid::Sanitize => {
            // Replace dangerous chars: CRLF -> space, NUL/DEL -> remove
            let sanitized: String = value
                .bytes()
                .filter(|&b| !is_dangerous(b))
                .map(|b| if b == 0x0D || b == 0x0A { b' ' } else { b })
                .map(|b| b as char)
                .collect();
            Ok(Cow::Owned(sanitized))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_secret() {
        let secret = "sk-proj-abc123xyz";
        assert!(matches!(
            validate_secret(secret, OnInvalid::Fail),
            Ok(Cow::Borrowed(s)) if s == secret
        ));
    }

    #[test]
    fn test_null_byte_fail() {
        let secret = "sk-proj\x00abc";
        assert!(matches!(
            validate_secret(secret, OnInvalid::Fail),
            Err(ValidationError::NullByte)
        ));
    }

    #[test]
    fn test_crlf_fail() {
        let secret = "valid-key\r\nX-Admin: true";
        assert!(matches!(
            validate_secret(secret, OnInvalid::Fail),
            Err(ValidationError::Multiple)
        ));
    }

    #[test]
    fn test_sanitize_crlf() {
        let secret = "key\r\nvalue";
        let result = validate_secret(secret, OnInvalid::Sanitize).unwrap();
        // Both CR and LF are removed, not replaced with space
        assert_eq!(result.as_ref(), "keyvalue");
    }

    #[test]
    fn test_sanitize_null() {
        let secret = "key\x00value";
        let result = validate_secret(secret, OnInvalid::Sanitize).unwrap();
        assert_eq!(result.as_ref(), "keyvalue");
    }

    #[test]
    fn test_sanitize_del() {
        let secret = "key\x7fvalue";
        let result = validate_secret(secret, OnInvalid::Sanitize).unwrap();
        assert_eq!(result.as_ref(), "keyvalue");
    }

    #[test]
    fn test_warn_mode() {
        let secret = "key\rwithCR";
        assert!(matches!(
            validate_secret(secret, OnInvalid::Warn),
            Err(ValidationError::CarriageReturn)
        ));
    }
}
