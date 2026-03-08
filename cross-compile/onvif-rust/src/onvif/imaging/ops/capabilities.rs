//! Imaging service capabilities operations.
//!
//! This module provides handlers for imaging service capabilities:
//! - GetServiceCapabilities - Retrieve imaging service capabilities

use crate::onvif::error::OnvifResult;
use crate::onvif::types::imaging::{
    GetServiceCapabilities, GetServiceCapabilitiesResponse, ImagingServiceCapabilities,
};

/// Handle the ONVIF Imaging GetServiceCapabilities request.
///
/// Returns the static capabilities of the imaging service, indicating which
/// optional features (image stabilization, presets) are supported.
///
/// # Arguments
///
/// * `_request` - The `GetServiceCapabilities` request (currently unused; no parameters)
///
/// # Returns
///
/// `GetServiceCapabilitiesResponse` containing an `ImagingServiceCapabilities` struct
/// with flags for image stabilization, presets, and adaptable preset support.
///
/// # Errors
///
/// This handler is infallible under normal operation.
pub fn get_service_capabilities(
    _request: GetServiceCapabilities,
) -> OnvifResult<GetServiceCapabilitiesResponse> {
    tracing::debug!("GetServiceCapabilities request");

    Ok(GetServiceCapabilitiesResponse {
        capabilities: ImagingServiceCapabilities {
            image_stabilization: Some(false),
            presets: Some(false),
            adaptable_preset: Some(false),
        },
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_service_capabilities() {
        let request = GetServiceCapabilities {};
        let result = get_service_capabilities(request);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.capabilities.image_stabilization, Some(false));
        assert_eq!(response.capabilities.presets, Some(false));
        assert_eq!(response.capabilities.adaptable_preset, Some(false));
    }
}
