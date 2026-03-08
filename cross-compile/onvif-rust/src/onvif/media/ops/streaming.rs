//! Media Streaming operations.
//!
//! This module provides streaming URI operations including:
//! - GetStreamUri, GetSnapshotUri
//! - StartMulticastStreaming, StopMulticastStreaming

use std::sync::Arc;

use crate::onvif::error::OnvifResult;
#[allow(unused_imports)]
use crate::onvif::types::common::{MediaUri, StreamSetup, StreamType, TransportProtocol};
use crate::onvif::types::media::{
    GetSnapshotUri, GetSnapshotUriResponse, GetStreamUri, GetStreamUriResponse,
    StartMulticastStreaming, StartMulticastStreamingResponse, StopMulticastStreaming,
    StopMulticastStreamingResponse,
};

use super::ProfileManagerRef;
use crate::config::ConfigRuntime;
use crate::onvif::media::types::{DEFAULT_RTSP_PORT, DEFAULT_SNAPSHOT_PATH};
use crate::platform::external_ip;

/// Handle GetStreamUri request.
///
/// Returns the RTSP URI for a profile.
/// Note: `stream_setup.transport.protocol` is not honored -- the URI is always
/// an `rtsp://` address. The AK3918 only exposes an RTSP server, so HTTP and
/// UDP-only transport requests still receive an RTSP URI.
pub fn get_stream_uri(
    pm: &ProfileManagerRef,
    config: &Arc<ConfigRuntime>,
    request: GetStreamUri,
) -> OnvifResult<GetStreamUriResponse> {
    tracing::debug!(
        "GetStreamUri request for profile: {}",
        request.profile_token
    );

    // Validate profile exists
    let _ = pm.get_profile(&request.profile_token)?;

    // Build RTSP URI based on profile
    let stream_path = get_stream_path(&request.profile_token, &request.stream_setup);
    let uri = format!("{}{}", rtsp_url(config), stream_path);

    Ok(GetStreamUriResponse {
        media_uri: MediaUri {
            uri,
            invalid_after_connect: false,
            invalid_after_reboot: false,
            timeout: "PT60S".to_string(),
        },
    })
}

/// Get stream path for a profile.
///
/// Channel routing is currently derived from substring matching on the profile
/// token ("MainStream" / "SubStream"). This works because the fixed profiles
/// use a predictable naming convention, but user-created profiles will fall
/// through to the generic "stream" path.
// TODO: derive channel from the profile's video encoder configuration (or a
// profile-to-channel map) instead of relying on token naming conventions.
pub fn get_stream_path(profile_token: &str, stream_setup: &StreamSetup) -> String {
    // Determine stream name based on profile token substring
    let stream_name = if profile_token.contains("MainStream") {
        "main"
    } else if profile_token.contains("SubStream") {
        "sub"
    } else {
        "stream"
    };

    // Determine stream type suffix
    let type_suffix = match stream_setup.stream {
        StreamType::RtpUnicast => "",
        StreamType::RtpMulticast => "_multicast",
    };

    format!("/{}{}", stream_name, type_suffix)
}

/// Handle GetSnapshotUri request.
///
/// Returns the HTTP URI for a JPEG snapshot.
pub fn get_snapshot_uri(
    pm: &ProfileManagerRef,
    config: &Arc<ConfigRuntime>,
    request: GetSnapshotUri,
) -> OnvifResult<GetSnapshotUriResponse> {
    tracing::debug!(
        "GetSnapshotUri request for profile: {}",
        request.profile_token
    );

    // Validate profile exists
    let _ = pm.get_profile(&request.profile_token)?;

    // Build snapshot URI — read the config path once and avoid cloning when
    // the default is used.
    let cfg = config.read();
    let snapshot_path = if cfg.media.snapshot_path.is_empty() {
        DEFAULT_SNAPSHOT_PATH
    } else {
        &cfg.media.snapshot_path
    };

    let uri = format!(
        "{}{}?profile={}",
        base_url(config),
        snapshot_path,
        request.profile_token
    );

    Ok(GetSnapshotUriResponse {
        media_uri: MediaUri {
            uri,
            invalid_after_connect: false,
            invalid_after_reboot: false,
            timeout: "PT60S".to_string(),
        },
    })
}

/// Handle StartMulticastStreaming request.
///
/// Starts multicast streaming for the given profile.
pub fn start_multicast_streaming(
    pm: &ProfileManagerRef,
    request: StartMulticastStreaming,
) -> OnvifResult<StartMulticastStreamingResponse> {
    tracing::debug!(
        "StartMulticastStreaming for profile: {}",
        request.profile_token
    );

    // Verify profile exists
    let _ = pm.get_profile(&request.profile_token)?;

    // Multicast streaming not supported - return success but do nothing
    // Per ONVIF spec, this is acceptable if multicast is disabled in capabilities
    tracing::warn!(
        "Multicast streaming requested but not supported for profile: {}",
        request.profile_token
    );

    Ok(StartMulticastStreamingResponse {})
}

/// Handle StopMulticastStreaming request.
///
/// Stops multicast streaming for the given profile.
pub fn stop_multicast_streaming(
    pm: &ProfileManagerRef,
    request: StopMulticastStreaming,
) -> OnvifResult<StopMulticastStreamingResponse> {
    tracing::debug!(
        "StopMulticastStreaming for profile: {}",
        request.profile_token
    );

    // Verify profile exists
    let _ = pm.get_profile(&request.profile_token)?;

    // Multicast streaming not supported - return success
    Ok(StopMulticastStreamingResponse {})
}

/// Get the base URL for service addresses.
fn base_url(config: &Arc<ConfigRuntime>) -> String {
    let address = external_ip(config);
    let port = config.read().server.port;
    format!("http://{}:{}", address, port)
}

/// Get the RTSP base URL.
fn rtsp_url(config: &Arc<ConfigRuntime>) -> String {
    let address = external_ip(config);
    let port = {
        let p = config.read().media.rtsp_port;
        if p == 0 { DEFAULT_RTSP_PORT } else { p }
    };
    format!("rtsp://{}:{}", address, port)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pm() -> ProfileManagerRef {
        crate::onvif::media::ProfileManager::new()
    }

    fn create_test_config() -> Arc<ConfigRuntime> {
        Arc::new(ConfigRuntime::new(Default::default()))
    }

    #[test]
    fn test_streaming_get_uri_main_stream_returns_rtsp() {
        let pm = create_test_pm();
        let config = create_test_config();
        let result = get_stream_uri(
            &pm,
            &config,
            GetStreamUri {
                stream_setup: StreamSetup {
                    stream: StreamType::RtpUnicast,
                    transport: crate::onvif::types::common::Transport {
                        protocol: TransportProtocol::RTSP,
                        tunnel: None,
                    },
                },
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.media_uri.uri.contains("rtsp://"));
        assert!(response.media_uri.uri.contains("/main"));
    }

    #[test]
    fn test_streaming_get_uri_sub_stream_returns_sub_path() {
        let pm = create_test_pm();
        let config = create_test_config();
        let result = get_stream_uri(
            &pm,
            &config,
            GetStreamUri {
                stream_setup: StreamSetup {
                    stream: StreamType::RtpUnicast,
                    transport: crate::onvif::types::common::Transport {
                        protocol: TransportProtocol::RTSP,
                        tunnel: None,
                    },
                },
                profile_token: "Profile_SubStream".to_string(),
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.media_uri.uri.contains("/sub"));
    }

    #[test]
    fn test_streaming_get_uri_multicast_returns_multicast_suffix() {
        let pm = create_test_pm();
        let config = create_test_config();
        let result = get_stream_uri(
            &pm,
            &config,
            GetStreamUri {
                stream_setup: StreamSetup {
                    stream: StreamType::RtpMulticast,
                    transport: crate::onvif::types::common::Transport {
                        protocol: TransportProtocol::UDP,
                        tunnel: None,
                    },
                },
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.media_uri.uri.contains("_multicast"));
    }

    #[test]
    fn test_streaming_get_uri_invalid_profile_returns_error() {
        let pm = create_test_pm();
        let config = create_test_config();
        let result = get_stream_uri(
            &pm,
            &config,
            GetStreamUri {
                stream_setup: StreamSetup {
                    stream: StreamType::RtpUnicast,
                    transport: crate::onvif::types::common::Transport {
                        protocol: TransportProtocol::RTSP,
                        tunnel: None,
                    },
                },
                profile_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_get_snapshot_uri_returns_http_url() {
        let pm = create_test_pm();
        let config = create_test_config();
        let result = get_snapshot_uri(
            &pm,
            &config,
            GetSnapshotUri {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.media_uri.uri.contains("http://"));
        assert!(response.media_uri.uri.contains("snapshot"));
    }

    #[test]
    fn test_streaming_get_snapshot_uri_invalid_profile_returns_error() {
        let pm = create_test_pm();
        let config = create_test_config();
        let result = get_snapshot_uri(
            &pm,
            &config,
            GetSnapshotUri {
                profile_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_start_multicast_valid_profile_returns_ok() {
        let pm = create_test_pm();
        let result = start_multicast_streaming(
            &pm,
            StartMulticastStreaming {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        // Should succeed even though multicast is not supported (per ONVIF spec)
        assert!(result.is_ok());
    }

    #[test]
    fn test_streaming_start_multicast_invalid_profile_returns_error() {
        let pm = create_test_pm();
        let result = start_multicast_streaming(
            &pm,
            StartMulticastStreaming {
                profile_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_stop_multicast_valid_profile_returns_ok() {
        let pm = create_test_pm();
        let result = stop_multicast_streaming(
            &pm,
            StopMulticastStreaming {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_streaming_stop_multicast_invalid_profile_returns_error() {
        let pm = create_test_pm();
        let result = stop_multicast_streaming(
            &pm,
            StopMulticastStreaming {
                profile_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }
}
