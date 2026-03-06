//! Media Service Capabilities operations.
//!
//! This module provides Media Service capabilities operations.

use crate::onvif::error::OnvifResult;
use crate::onvif::types::media::{
    GetServiceCapabilitiesResponse, MediaServiceCapabilities, ProfileCapabilities,
    StreamingCapabilities,
};

use crate::onvif::media::types::MAX_PROFILES;

/// Handle GetServiceCapabilities request.
///
/// Returns Media Service capabilities.
pub fn get_service_capabilities() -> OnvifResult<GetServiceCapabilitiesResponse> {
    tracing::debug!("GetServiceCapabilities request");

    Ok(GetServiceCapabilitiesResponse {
        capabilities: MediaServiceCapabilities {
            snapshot_uri: Some(true),
            rotation: Some(false),
            video_source_mode: Some(false),
            osd: Some(false),
            temporary_osd_text: Some(false),
            exi_compression: Some(false),
            profile_capabilities: Some(ProfileCapabilities {
                maximum_number_of_profiles: Some(MAX_PROFILES as i32),
            }),
            streaming_capabilities: Some(StreamingCapabilities {
                rtp_multicast: Some(false),
                rtp_tcp: Some(true),
                rtp_rtsp_tcp: Some(true),
                non_aggregate_control: Some(false),
                no_rtsp_streaming: Some(false),
            }),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_service_capabilities() {
        let result = get_service_capabilities();
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(
            response
                .capabilities
                .profile_capabilities
                .as_ref()
                .unwrap()
                .maximum_number_of_profiles,
            Some(MAX_PROFILES as i32)
        );
    }
}
