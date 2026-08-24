//! Media Service Capabilities operations.
//!
//! This module provides Media Service capabilities operations.

use crate::onvif::error::OnvifResult;
use crate::onvif::types::media::{
    GetServiceCapabilitiesResponse, MediaServiceCapabilities, ProfileCapabilities,
    StreamingCapabilities,
};

use crate::onvif::media::types::MAX_PROFILES;

/// Handle the ONVIF Media GetServiceCapabilities request.
///
/// Returns the static capabilities of the media service, including profile
/// limits, supported streaming transports, and optional feature flags.
///
/// # Returns
///
/// `GetServiceCapabilitiesResponse` containing `MediaServiceCapabilities` with
/// profile limits (`MAX_PROFILES`), streaming transport flags (RTP/TCP,
/// RTSP/TCP), and optional feature flags (snapshot URI, rotation, OSD).
///
/// # Errors
///
/// This handler is infallible under normal operation.
pub fn get_service_capabilities() -> OnvifResult<GetServiceCapabilitiesResponse> {
    tracing::debug!("GetServiceCapabilities request");

    Ok(GetServiceCapabilitiesResponse {
        capabilities: MediaServiceCapabilities {
            snapshot_uri: Some(true),
            rotation: Some(false),
            video_source_mode: Some(false),
            osd: Some(true),
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
        let caps = &response.capabilities;

        // Top-level capability flags
        assert_eq!(caps.snapshot_uri, Some(true));
        assert_eq!(caps.rotation, Some(false));
        assert_eq!(caps.video_source_mode, Some(false));
        assert_eq!(caps.osd, Some(false));
        assert_eq!(caps.temporary_osd_text, Some(false));
        assert_eq!(caps.exi_compression, Some(false));

        // Profile capabilities
        let profile_caps = caps.profile_capabilities.as_ref().unwrap();
        assert_eq!(
            profile_caps.maximum_number_of_profiles,
            Some(MAX_PROFILES as i32)
        );

        // Streaming capabilities
        let streaming = caps.streaming_capabilities.as_ref().unwrap();
        assert_eq!(streaming.rtp_multicast, Some(false));
        assert_eq!(streaming.rtp_tcp, Some(true));
        assert_eq!(streaming.rtp_rtsp_tcp, Some(true));
        assert_eq!(streaming.non_aggregate_control, Some(false));
        assert_eq!(streaming.no_rtsp_streaming, Some(false));
    }
}
