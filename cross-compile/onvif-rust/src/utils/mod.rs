//! Utility modules for the ONVIF implementation.
//!
//! This module provides common utilities used throughout the codebase:
//!
//! - [`validation`] - XML security validation (XXE, XML bombs, XSS prevention)
//! - [`memory`] - Memory management utilities (planned)
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::utils::validation::{SecurityValidator, SecurityError};
//!
//! let validator = SecurityValidator::default();
//! validator.validate_xml_payload(xml_bytes)?;
//! ```

pub mod validation;

// Re-exports for security validation
pub use validation::{SecurityError, SecurityValidator};
