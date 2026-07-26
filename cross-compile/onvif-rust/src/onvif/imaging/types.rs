//! Imaging Service types re-exported from WSDL-generated types.
//!
//! This module re-exports and extends the WSDL-generated imaging types
//! from `crate::onvif::types::imaging`.

// Re-export all imaging types from the generated module
pub use crate::onvif::types::imaging::*;

// Re-export common types used by imaging service
// These are intentionally re-exported for consumers of this module
#[allow(unused_imports)]
pub use crate::onvif::types::common::{
    FloatRange, ImagingSettings20, ImagingStatus20, ReferenceToken,
};

// ============================================================================
// Service Constants
// ============================================================================

/// Imaging service namespace URI.
pub const IMAGING_SERVICE_NAMESPACE: &str = "http://www.onvif.org/ver20/imaging/wsdl";

/// Default video source token.
pub const DEFAULT_VIDEO_SOURCE_TOKEN: &str = "VideoSource_1";
