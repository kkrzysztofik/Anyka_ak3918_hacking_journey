//! Imaging service capabilities operations.
//!
//! This module provides handlers for imaging service capabilities:
//! - GetServiceCapabilities - Retrieve imaging service capabilities

use crate::onvif::error::OnvifResult;
use crate::onvif::types::imaging::{
    GetServiceCapabilities, GetServiceCapabilitiesResponse, ImagingServiceCapabilities,
};

/// Handle GetServiceCapabilities request.
///
/// Returns imaging service capabilities.
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
    }
}
