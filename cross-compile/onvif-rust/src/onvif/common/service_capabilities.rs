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
