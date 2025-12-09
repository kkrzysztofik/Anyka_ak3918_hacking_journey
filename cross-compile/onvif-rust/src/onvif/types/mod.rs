//! ONVIF Types generated from WSDL/XSD schemas.
//!
//! This module contains hand-crafted Rust types derived from ONVIF WSDL and XSD files.
//! All types use `serde` for XML serialization/deserialization with `quick-xml`.
//!
//! # Modules
//!
//! - [`common`] - Common ONVIF schema types (tt:* namespace)
//! - [`device`] - Device Service types (tds:* namespace)
//! - [`media`] - Media Service types (trt:* namespace)
//! - [`ptz`] - PTZ Service types (tptz:* namespace)
//! - [`imaging`] - Imaging Service types (timg:* namespace)
//!
//! # Namespace URIs
//!
//! All ONVIF namespaces are defined as constants in this module for consistent usage.
//!
//! # Extension Pattern
//!
//! ONVIF types often include extension elements for vendor-specific or future extensions.
//! Types that support extensions include an `extension` field using the [`Extension`] type.

pub mod common;
pub mod device;
pub mod imaging;
pub mod media;
pub mod ptz;

// Re-export common types at module level for convenience
pub use common::*;

// ============================================================================
// ONVIF Namespace Constants
// ============================================================================

/// SOAP 1.2 Envelope namespace.
pub const SOAP_ENVELOPE_NS: &str = "http://www.w3.org/2003/05/soap-envelope";

/// ONVIF Schema namespace (tt:* types).
pub const TT_NS: &str = "http://www.onvif.org/ver10/schema";

/// ONVIF Device Service namespace (tds:* types).
pub const TDS_NS: &str = "http://www.onvif.org/ver10/device/wsdl";

/// ONVIF Media Service namespace (trt:* types).
pub const TRT_NS: &str = "http://www.onvif.org/ver10/media/wsdl";

/// ONVIF PTZ Service namespace (tptz:* types).
pub const TPTZ_NS: &str = "http://www.onvif.org/ver20/ptz/wsdl";

/// ONVIF Imaging Service namespace (timg:* types).
pub const TIMG_NS: &str = "http://www.onvif.org/ver20/imaging/wsdl";

/// WS-Security namespace.
pub const WSSE_NS: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";

/// WS-Security Utility namespace.
pub const WSU_NS: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd";

/// WS-Addressing namespace.
pub const WSA_NS: &str = "http://www.w3.org/2005/08/addressing";

/// WS-Discovery namespace.
pub const WSD_NS: &str = "http://schemas.xmlsoap.org/ws/2005/04/discovery";

// ============================================================================
// Extension Types
// ============================================================================

use serde::{Deserialize, Serialize};

/// Generic extension container for ONVIF extensibility.
///
/// ONVIF types use `xs:any` elements to allow vendor-specific or future extensions.
/// This type captures any unknown XML elements as raw strings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Extension {
    /// Raw XML content of extension elements.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "$value")]
    pub content: Option<String>,
}

impl Extension {
    /// Create an empty extension.
    pub fn empty() -> Self {
        Self { content: None }
    }

    /// Check if the extension is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_none()
    }
}

/// Extension with typed content for specific extension types.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TypedExtension<T> {
    /// Typed extension content.
    #[serde(default, skip_serializing_if = "Option::is_none", flatten)]
    pub inner: Option<T>,
}

impl<T> TypedExtension<T> {
    /// Create an empty typed extension.
    pub fn empty() -> Self {
        Self { inner: None }
    }

    /// Create a typed extension with content.
    pub fn with(value: T) -> Self {
        Self { inner: Some(value) }
    }

    /// Check if the extension is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_none()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Helper to skip serializing empty Option fields.
pub fn is_none<T>(opt: &Option<T>) -> bool {
    opt.is_none()
}

/// Helper to skip serializing empty Vec fields.
pub fn is_empty_vec<T>(vec: &[T]) -> bool {
    vec.is_empty()
}

/// Helper to skip serializing default bool (false).
pub fn is_false(b: &bool) -> bool {
    !*b
}

/// Helper to skip serializing zero values.
pub fn is_zero(n: &i32) -> bool {
    *n == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_empty() {
        let ext = Extension::empty();
        assert!(ext.is_empty());
    }

    #[test]
    fn test_extension_with_content() {
        let ext = Extension {
            content: Some("<vendor:Custom>value</vendor:Custom>".to_string()),
        };
        assert!(!ext.is_empty());
    }

    #[test]
    fn test_typed_extension() {
        let ext: TypedExtension<String> = TypedExtension::empty();
        assert!(ext.is_empty());

        let ext = TypedExtension::with("test".to_string());
        assert!(!ext.is_empty());
    }
}
