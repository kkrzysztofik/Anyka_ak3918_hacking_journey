//! Central string length limits for ONVIF inputs.
//!
//! These bounds reduce denial-of-service risk from unbounded allocations on
//! memory-constrained targets. Service-specific validators should use these
//! constants instead of duplicating magic numbers.

/// Maximum username length (Device user management, WS-Security user names).
pub const MAX_USERNAME_CHARS: usize = 64;

/// Maximum ONVIF reference token length (profiles, encoder/source configs, etc.).
///
/// ONVIF `xs:Name` / token types are commonly bounded; this is an explicit
/// embedded-friendly cap beyond which we reject inputs.
pub const MAX_REFERENCE_TOKEN_CHARS: usize = 128;

/// Maximum Device Service scope URI length (`onvif://www.onvif.org/...`).
pub const MAX_SCOPE_URI_CHARS: usize = 256;
