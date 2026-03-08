//! Shared ONVIF `GetServiceCapabilities` request/response wrappers.
//!
//! Multiple ONVIF services (Device, Media, PTZ, Imaging, Events, Analytics)
//! share the same `GetServiceCapabilities` request shape (empty body) and a
//! generic response that wraps service-specific capability structs. This module
//! provides the common envelope types so each service only needs to define its
//! own `Capabilities` struct.

use serde::{Deserialize, Serialize};

/// Shared empty request wrapper used by services that expose `GetServiceCapabilities`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetServiceCapabilities", rename_all = "PascalCase")]
pub struct GetServiceCapabilities {}

/// Shared response wrapper for `GetServiceCapabilities`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetServiceCapabilitiesResponse", rename_all = "PascalCase")]
pub struct GetServiceCapabilitiesResponse<T> {
    /// Service capabilities.
    #[serde(
        rename = "Capabilities",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub capabilities: Option<T>,
}
