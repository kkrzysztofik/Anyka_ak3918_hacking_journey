//! Central string length limits for ONVIF inputs.
//!
//! These bounds reduce denial-of-service risk from unbounded allocations on
//! memory-constrained targets. Service-specific validators should use these
//! constants instead of duplicating magic numbers.

/// Maximum username length (Device user management, WS-Security user names).
pub const MAX_USERNAME_CHARS: usize = 64;

/// Maximum password length (Device user management). Separate from username for clarity and policy.
pub const MAX_PASSWORD_CHARS: usize = 64;

/// Maximum ONVIF reference token length (profiles, encoder/source configs, etc.).
///
/// ONVIF `xs:Name` / token types are commonly bounded; this is an explicit
/// embedded-friendly cap beyond which we reject inputs.
pub const MAX_REFERENCE_TOKEN_CHARS: usize = 128;

/// Maximum Device Service scope URI length (`onvif://www.onvif.org/...`).
pub const MAX_SCOPE_URI_CHARS: usize = 256;

#[cfg(test)]
mod tests {
    use super::{
        MAX_PASSWORD_CHARS, MAX_REFERENCE_TOKEN_CHARS, MAX_SCOPE_URI_CHARS, MAX_USERNAME_CHARS,
    };
    use crate::onvif::common::validation::validate_reference_token;
    use crate::onvif::device::user_types::{validate_password, validate_username};
    use crate::onvif::device::validation::validate_scope;
    use unicode_segmentation::UnicodeSegmentation;

    const SCOPE_PREFIX: &str = "onvif://www.onvif.org/";

    #[test]
    fn limits_username_boundary_matches_validate_username() {
        let max_ok = format!("abc{}", "d".repeat(MAX_USERNAME_CHARS - 3));
        assert_eq!(max_ok.graphemes(true).count(), MAX_USERNAME_CHARS);
        assert!(validate_username(&max_ok).is_ok());

        let too_long = format!("abcd{}", "e".repeat(MAX_USERNAME_CHARS - 3));
        assert!(too_long.graphemes(true).count() > MAX_USERNAME_CHARS);
        assert!(validate_username(&too_long).is_err());
    }

    #[test]
    fn limits_password_boundary_matches_validate_password() {
        let filler_len = MAX_PASSWORD_CHARS.saturating_sub("Abcdef12".len());
        let max_ok = format!("Abcdef12{}", "0".repeat(filler_len));
        assert_eq!(max_ok.chars().count(), MAX_PASSWORD_CHARS);
        assert!(validate_password(Some(&max_ok), true).is_ok());

        let too_long = format!("{}x", max_ok);
        assert!(too_long.chars().count() > MAX_PASSWORD_CHARS);
        assert!(validate_password(Some(&too_long), true).is_err());
    }

    #[test]
    fn limits_reference_token_boundary_matches_validator() {
        let max_ok = "a".repeat(MAX_REFERENCE_TOKEN_CHARS);
        assert!(validate_reference_token(&max_ok, "profile").is_ok());
        let too_long = "a".repeat(MAX_REFERENCE_TOKEN_CHARS + 1);
        assert!(validate_reference_token(&too_long, "profile").is_err());
    }

    #[test]
    fn limits_scope_uri_boundary_matches_validate_scope() {
        let suffix_len = MAX_SCOPE_URI_CHARS.saturating_sub(SCOPE_PREFIX.len());
        let max_ok = format!("{SCOPE_PREFIX}{}", "s".repeat(suffix_len));
        assert_eq!(max_ok.len(), MAX_SCOPE_URI_CHARS);
        assert!(validate_scope(&max_ok).is_ok());

        let too_long = format!("{max_ok}x");
        assert!(too_long.len() > MAX_SCOPE_URI_CHARS);
        assert!(validate_scope(&too_long).is_err());
    }
}
