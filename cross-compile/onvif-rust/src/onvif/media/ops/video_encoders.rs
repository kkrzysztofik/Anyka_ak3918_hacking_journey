//! Media Video Encoder operations.
//!
//! This module provides video encoder management operations including:
//! - GetVideoEncoderConfigurations, GetVideoEncoderConfiguration
//! - SetVideoEncoderConfiguration
//! - GetVideoEncoderConfigurationOptions
//! - GetCompatibleVideoEncoderConfigurations

use crate::onvif::error::OnvifResult;
use crate::onvif::types::media::{
    GetCompatibleVideoEncoderConfigurations, GetCompatibleVideoEncoderConfigurationsResponse,
    GetVideoEncoderConfiguration, GetVideoEncoderConfigurationOptionsResponse,
    GetVideoEncoderConfigurationResponse, GetVideoEncoderConfigurationsResponse,
    SetVideoEncoderConfiguration, SetVideoEncoderConfigurationResponse,
};

use super::ProfileManagerRef;

use crate::onvif::media::validation;

/// Handle GetVideoEncoderConfigurations request.
///
/// Returns all video encoder configurations.
pub fn get_video_encoder_configurations(
    pm: &ProfileManagerRef,
) -> OnvifResult<GetVideoEncoderConfigurationsResponse> {
    tracing::debug!("GetVideoEncoderConfigurations request");
    let configurations = pm.get_video_encoder_configurations();
    Ok(GetVideoEncoderConfigurationsResponse { configurations })
}

/// Handle GetVideoEncoderConfiguration request.
///
/// Returns a specific video encoder configuration.
pub fn get_video_encoder_configuration(
    pm: &ProfileManagerRef,
    request: GetVideoEncoderConfiguration,
) -> OnvifResult<GetVideoEncoderConfigurationResponse> {
    tracing::debug!(
        "GetVideoEncoderConfiguration request for token: {}",
        request.configuration_token
    );
    let configuration = pm.get_video_encoder_configuration(&request.configuration_token)?;
    Ok(GetVideoEncoderConfigurationResponse { configuration })
}

/// Handle SetVideoEncoderConfiguration request.
///
/// Updates a video encoder configuration.
/// Validates resolution, quality, frame rate, and bitrate.
pub fn set_video_encoder_configuration(
    pm: &ProfileManagerRef,
    request: SetVideoEncoderConfiguration,
) -> OnvifResult<SetVideoEncoderConfigurationResponse> {
    tracing::debug!(
        "SetVideoEncoderConfiguration request for token: {}",
        request.configuration.token
    );

    // Validate configuration values
    validation::validate_resolution(
        request.configuration.resolution.width,
        request.configuration.resolution.height,
    )?;
    validation::validate_quality(request.configuration.quality)?;

    if let Some(ref rate_control) = request.configuration.rate_control {
        validation::validate_frame_rate(rate_control.frame_rate_limit)?;
        validation::validate_bitrate(rate_control.bitrate_limit)?;
    }

    // TODO: force_persistence is accepted per ONVIF spec but not yet implemented;
    // all configuration changes are currently persisted immediately.
    pm.set_video_encoder_configuration(request.configuration)?;
    Ok(SetVideoEncoderConfigurationResponse {})
}

/// Handle GetVideoEncoderConfigurationOptions request.
///
/// Returns valid options for video encoder configuration.
pub fn get_video_encoder_configuration_options(
    pm: &ProfileManagerRef,
) -> OnvifResult<GetVideoEncoderConfigurationOptionsResponse> {
    tracing::debug!("GetVideoEncoderConfigurationOptions request");
    let options = pm.get_video_encoder_configuration_options();
    Ok(GetVideoEncoderConfigurationOptionsResponse { options })
}

/// Handle GetCompatibleVideoEncoderConfigurations request.
///
/// Returns video encoder configurations compatible with the given profile.
pub fn get_compatible_video_encoder_configurations(
    pm: &ProfileManagerRef,
    request: GetCompatibleVideoEncoderConfigurations,
) -> OnvifResult<GetCompatibleVideoEncoderConfigurationsResponse> {
    tracing::debug!(
        "GetCompatibleVideoEncoderConfigurations for profile: {}",
        request.profile_token
    );
    // Verify profile exists
    let _ = pm.get_profile(&request.profile_token)?;
    // Compatibility: On the AK3918 all video encoder configurations can be
    // bound to any profile because the hardware supports a single sensor with
    // shared encoding pipelines. A multi-sensor device would need to filter
    // by source compatibility here.
    let configurations = pm.get_video_encoder_configurations();
    Ok(GetCompatibleVideoEncoderConfigurationsResponse { configurations })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::{
        H264Configuration, H264Profile, VideoEncoderConfiguration, VideoEncoding, VideoRateControl,
        VideoResolution,
    };

    fn create_test_pm() -> ProfileManagerRef {
        crate::onvif::media::ProfileManager::new()
    }

    #[test]
    fn test_get_video_encoder_configurations() {
        let pm = create_test_pm();
        let result = get_video_encoder_configurations(&pm);
        assert!(result.is_ok());
        assert!(!result.unwrap().configurations.is_empty());
    }

    #[test]
    fn test_get_video_encoder_configuration() {
        let pm = create_test_pm();
        let result = get_video_encoder_configuration(
            &pm,
            GetVideoEncoderConfiguration {
                configuration_token: "VideoEncoderConfig_0".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_video_encoder_configuration_not_found() {
        let pm = create_test_pm();
        let result = get_video_encoder_configuration(
            &pm,
            GetVideoEncoderConfiguration {
                configuration_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_set_video_encoder_configuration() {
        let pm = create_test_pm();
        let result = set_video_encoder_configuration(
            &pm,
            SetVideoEncoderConfiguration {
                configuration: VideoEncoderConfiguration {
                    token: "VideoEncoderConfig_0".to_string(),
                    name: "TestEncoder".to_string(),
                    use_count: 1,
                    encoding: VideoEncoding::H264,
                    resolution: VideoResolution {
                        width: 1920,
                        height: 1080,
                    },
                    quality: 0.5,
                    rate_control: Some(VideoRateControl {
                        frame_rate_limit: 30,
                        bitrate_limit: 2000000,
                        encoding_interval: 1,
                    }),
                    mpeg4: None,
                    h264: Some(H264Configuration {
                        gov_length: 30,
                        h264_profile: H264Profile::Baseline,
                    }),
                    multicast: None,
                    session_timeout: "PT60S".to_string(),
                },
                force_persistence: false,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_video_encoder_configuration_invalid_resolution() {
        let pm = create_test_pm();
        let result = set_video_encoder_configuration(
            &pm,
            SetVideoEncoderConfiguration {
                configuration: VideoEncoderConfiguration {
                    token: "VideoEncoderConfig_0".to_string(),
                    name: "TestEncoder".to_string(),
                    use_count: 1,
                    encoding: VideoEncoding::H264,
                    resolution: VideoResolution {
                        width: 0, // Invalid
                        height: 1080,
                    },
                    quality: 0.5,
                    rate_control: None,
                    mpeg4: None,
                    h264: Some(H264Configuration {
                        gov_length: 30,
                        h264_profile: H264Profile::Baseline,
                    }),
                    multicast: None,
                    session_timeout: "PT60S".to_string(),
                },
                force_persistence: false,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_video_encoder_configuration_options() {
        let pm = create_test_pm();
        let result = get_video_encoder_configuration_options(&pm);
        assert!(result.is_ok());
        let options = result.unwrap().options;
        assert!(options.h264.is_some());
        assert!(options.jpeg.is_some());
    }

    #[test]
    fn test_get_compatible_video_encoder_configurations() {
        let pm = create_test_pm();
        let result = get_compatible_video_encoder_configurations(
            &pm,
            GetCompatibleVideoEncoderConfigurations {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        assert!(!result.unwrap().configurations.is_empty());
    }
}
