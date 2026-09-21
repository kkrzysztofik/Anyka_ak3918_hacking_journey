//! Media Streaming operations.
//!
//! This module provides streaming URI operations including:
//! - GetStreamUri, GetSnapshotUri
//! - StartMulticastStreaming, StopMulticastStreaming

use std::sync::Arc;

use crate::onvif::error::{OnvifError, OnvifResult};
#[allow(unused_imports)]
use crate::onvif::types::common::{MediaUri, StreamSetup, StreamType, TransportProtocol};
use crate::onvif::types::media::{
    GetSnapshotUri, GetSnapshotUriResponse, GetStreamUri, GetStreamUriResponse,
    StartMulticastStreaming, StartMulticastStreamingResponse, StopMulticastStreaming,
    StopMulticastStreamingResponse,
};

use super::ProfileManagerRef;
use crate::config::ConfigRuntime;
use crate::onvif::media::types::{
    DEFAULT_RTSP_PORT, DEFAULT_SNAPSHOT_PATH, VIDEO_ENCODER_CONFIG_PREFIX,
};
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

    // Route by the profile's attached video encoder, not by its token. The
    // previous implementation substring-matched the token and handed out the
    // dead "/stream" path for anything not literally named MainStream/SubStream.
    let profile = pm.get_profile(&request.profile_token)?;

    let encoder_token = profile
        .video_encoder_configuration
        .as_ref()
        .map(|c| c.token.as_str())
        .ok_or_else(|| {
            OnvifError::invalid_arg_val(
                "ter:InvalidArgVal",
                format!(
                    "Profile '{}' has no video encoder configuration and cannot be streamed",
                    request.profile_token
                ),
            )
        })?;

    // Build RTSP URI from the encoder-derived channel
    let stream_path = get_stream_path(encoder_token, &request.stream_setup)?;
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

/// Map a video encoder configuration token to its RTSP path.
///
/// Encoder tokens are `VideoEncoderConfig_{n}`, where `n` is the enabled-order
/// index over `stream_profile_1..4`. The hardware exposes exactly two channels
/// ("main" / "sub"), so index 0 is main, 1 is sub, and anything else is a
/// profile we cannot serve.
///
/// Returning an error rather than a fallback is deliberate: the previous
/// implementation matched on the *profile* token and silently handed out
/// "/stream" for anything unrecognised, a path no RTSP server serves.
pub fn get_stream_path(encoder_token: &str, stream_setup: &StreamSetup) -> OnvifResult<String> {
    let index = encoder_token
        .strip_prefix(VIDEO_ENCODER_CONFIG_PREFIX)
        .and_then(|n| n.parse::<u32>().ok());

    let stream_name = match index {
        Some(0) => "main",
        Some(1) => "sub",
        _ => {
            return Err(OnvifError::invalid_arg_val(
                "ter:InvalidArgVal",
                format!(
                    "Video encoder configuration '{}' does not map to an RTSP channel",
                    encoder_token
                ),
            ));
        }
    };

    // Determine stream type suffix
    let type_suffix = match stream_setup.stream {
        StreamType::RtpUnicast => "",
        StreamType::RtpMulticast => "_multicast",
    };

    Ok(format!("/{}{}", stream_name, type_suffix))
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

    fn default_stream_setup() -> StreamSetup {
        StreamSetup {
            stream: StreamType::RtpUnicast,
            transport: crate::onvif::types::common::Transport {
                protocol: TransportProtocol::RTSP,
                tunnel: None,
            },
        }
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
        assert!(
            response.media_uri.uri.ends_with("/main"),
            "got {}",
            response.media_uri.uri
        );
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
        assert!(
            response.media_uri.uri.ends_with("/sub"),
            "got {}",
            response.media_uri.uri
        );
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
    fn test_streaming_get_uri_faults_when_no_encoder_attached() {
        let pm = create_test_pm();
        let config = create_test_config();
        pm.create_profile("Bare".to_string(), Some("Profile_Bare".to_string()))
            .unwrap();

        let result = get_stream_uri(
            &pm,
            &config,
            GetStreamUri {
                stream_setup: default_stream_setup(),
                profile_token: "Profile_Bare".to_string(),
            },
        );
        assert!(
            result.is_err(),
            "a profile with no video encoder must fault, not hand out a dead /stream URI"
        );
    }

    #[test]
    fn test_stream_uri_is_not_fooled_by_a_renamed_profile() {
        // Regression guard: routing must read the profile's encoder, not its
        // token. A profile named to contain neither MainStream nor SubStream
        // still routes by whichever encoder is attached.
        let pm = create_test_pm();
        let config = create_test_config();
        pm.create_profile("Lobby".to_string(), Some("Profile_Lobby".to_string()))
            .unwrap();
        pm.add_video_encoder_configuration(
            &"Profile_Lobby".to_string(),
            &"VideoEncoderConfig_1".to_string(),
        )
        .unwrap();

        let response = get_stream_uri(
            &pm,
            &config,
            GetStreamUri {
                stream_setup: default_stream_setup(),
                profile_token: "Profile_Lobby".to_string(),
            },
        )
        .expect("a custom profile with an encoder must resolve");
        assert!(
            response.media_uri.uri.ends_with("/sub"),
            "got {}",
            response.media_uri.uri
        );
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
