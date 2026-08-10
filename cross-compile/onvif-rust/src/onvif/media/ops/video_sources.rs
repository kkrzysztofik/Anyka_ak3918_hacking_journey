//! Media Video Source operations.
//!
//! This module provides video source management operations including:
//! - GetVideoSources, GetVideoSourceConfigurations
//! - GetVideoSourceConfiguration, SetVideoSourceConfiguration
//! - GetVideoSourceConfigurationOptions
//! - GetCompatibleVideoSourceConfigurations

use crate::onvif::error::OnvifResult;
use crate::onvif::types::media::{
    GetCompatibleVideoSourceConfigurations, GetCompatibleVideoSourceConfigurationsResponse,
    GetVideoSourceConfiguration, GetVideoSourceConfigurationOptionsResponse,
    GetVideoSourceConfigurationResponse, GetVideoSourceConfigurationsResponse,
    GetVideoSourcesResponse, SetVideoSourceConfiguration, SetVideoSourceConfigurationResponse,
};

use super::ProfileManagerRef;

/// Handle GetVideoSources request.
///
/// Returns all available video sources.
pub fn get_video_sources(pm: &ProfileManagerRef) -> OnvifResult<GetVideoSourcesResponse> {
    tracing::debug!("GetVideoSources request");
    let video_sources = pm.get_video_sources();
    Ok(GetVideoSourcesResponse { video_sources })
}

/// Handle GetVideoSourceConfigurations request.
///
/// Returns all video source configurations.
pub fn get_video_source_configurations(
    pm: &ProfileManagerRef,
) -> OnvifResult<GetVideoSourceConfigurationsResponse> {
    tracing::debug!("GetVideoSourceConfigurations request");
    let configurations = pm.get_video_source_configurations();
    Ok(GetVideoSourceConfigurationsResponse { configurations })
}

/// Handle GetVideoSourceConfiguration request.
///
/// Returns a specific video source configuration.
pub fn get_video_source_configuration(
    pm: &ProfileManagerRef,
    request: GetVideoSourceConfiguration,
) -> OnvifResult<GetVideoSourceConfigurationResponse> {
    tracing::debug!(
        "GetVideoSourceConfiguration request for token: {}",
        request.configuration_token
    );
    let configuration = pm.get_video_source_configuration(&request.configuration_token)?;
    Ok(GetVideoSourceConfigurationResponse { configuration })
}

/// Handle SetVideoSourceConfiguration request.
///
/// Updates a video source configuration and, if a Rotate extension is
/// present, applies it to the platform live before persisting. `Degree` is
/// restricted to `None`/`180` when `Mode: On` — this hardware's flip+mirror
/// trick can only produce a 180° rotation (see the design doc for why
/// `RotateMode::Auto` isn't modeled at all: it's rejected earlier, at XML
/// deserialization, since the enum has no `Auto` variant to parse into).
pub async fn set_video_source_configuration(
    pm: &ProfileManagerRef,
    platform: Option<&std::sync::Arc<dyn crate::platform::Platform>>,
    request: SetVideoSourceConfiguration,
) -> OnvifResult<SetVideoSourceConfigurationResponse> {
    tracing::debug!(
        "SetVideoSourceConfiguration request for token: {}",
        request.configuration.token
    );

    if let Some(rotate) = request
        .configuration
        .extension
        .as_ref()
        .and_then(|ext| ext.rotate.as_ref())
    {
        if let Some(degree) = rotate.degree
            && degree != 180
        {
            return Err(crate::onvif::error::OnvifError::invalid_arg_val(
                "InvalidDegree",
                format!(
                    "Unsupported rotate degree {}: this device only supports 180",
                    degree
                ),
            ));
        }

        let rotated = rotate.mode == crate::onvif::types::common::RotateMode::On;
        if let Some(platform) = platform
            && let Some(control) = platform.video_control()
        {
            control.set_flip_mirror(rotated).await.map_err(|e| {
                crate::onvif::error::OnvifError::HardwareFailure(format!(
                    "Failed to apply rotation: {}",
                    e
                ))
            })?;
        }
    }

    pm.set_video_source_configuration(request.configuration)?;
    Ok(SetVideoSourceConfigurationResponse {})
}

/// Handle GetVideoSourceConfigurationOptions request.
///
/// Returns valid options for video source configuration.
/// The request payload is intentionally ignored. The AK3918 has a single video
/// sensor, so the options are identical regardless of which token is specified.
pub fn get_video_source_configuration_options(
    pm: &ProfileManagerRef,
) -> OnvifResult<GetVideoSourceConfigurationOptionsResponse> {
    tracing::debug!("GetVideoSourceConfigurationOptions request");
    let options = pm.get_video_source_configuration_options();
    Ok(GetVideoSourceConfigurationOptionsResponse { options })
}

/// Handle GetCompatibleVideoSourceConfigurations request.
///
/// Returns video source configurations compatible with the given profile.
/// On this single-sensor AK3918 camera all video source configurations are
/// compatible with every profile, so the full set is returned after validating
/// that the requested profile exists.
pub fn get_compatible_video_source_configurations(
    pm: &ProfileManagerRef,
    request: GetCompatibleVideoSourceConfigurations,
) -> OnvifResult<GetCompatibleVideoSourceConfigurationsResponse> {
    tracing::debug!(
        "GetCompatibleVideoSourceConfigurations for profile: {}",
        request.profile_token
    );
    // Verify profile exists
    let _ = pm.get_profile(&request.profile_token)?;
    // All video source configurations are compatible with all profiles
    let configurations = pm.get_video_source_configurations();
    Ok(GetCompatibleVideoSourceConfigurationsResponse { configurations })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pm() -> ProfileManagerRef {
        crate::onvif::media::ProfileManager::new()
    }

    #[test]
    fn test_get_video_sources() {
        let pm = create_test_pm();
        let result = get_video_sources(&pm);
        assert!(result.is_ok());
        assert!(!result.unwrap().video_sources.is_empty());
    }

    #[test]
    fn test_get_video_source_configurations() {
        let pm = create_test_pm();
        let result = get_video_source_configurations(&pm);
        assert!(result.is_ok());
        assert!(!result.unwrap().configurations.is_empty());
    }

    #[test]
    fn test_get_video_source_configuration() {
        let pm = create_test_pm();
        let result = get_video_source_configuration(
            &pm,
            GetVideoSourceConfiguration {
                configuration_token: "VideoSourceConfig_0".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_video_source_configuration_not_found() {
        let pm = create_test_pm();
        let result = get_video_source_configuration(
            &pm,
            GetVideoSourceConfiguration {
                configuration_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_video_source_configuration_options() {
        let pm = create_test_pm();
        let result = get_video_source_configuration_options(&pm);
        assert!(result.is_ok());
        assert!(result.unwrap().options.maximum_number_of_profiles.is_some());
    }

    #[test]
    fn test_get_compatible_video_source_configurations() {
        let pm = create_test_pm();
        let result = get_compatible_video_source_configurations(
            &pm,
            GetCompatibleVideoSourceConfigurations {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        assert!(!result.unwrap().configurations.is_empty());
    }

    #[test]
    fn test_get_compatible_video_source_configurations_invalid_profile() {
        let pm = create_test_pm();
        let result = get_compatible_video_source_configurations(
            &pm,
            GetCompatibleVideoSourceConfigurations {
                profile_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    /// Fetch the default test video source configuration and return it ready
    /// to be mutated (e.g. with a `Rotate` extension) and sent back through
    /// `set_video_source_configuration`.
    fn test_configuration(
        pm: &ProfileManagerRef,
    ) -> crate::onvif::types::common::VideoSourceConfiguration {
        get_video_source_configuration(
            pm,
            GetVideoSourceConfiguration {
                configuration_token: "VideoSourceConfig_0".to_string(),
            },
        )
        .unwrap()
        .configuration
    }

    #[tokio::test]
    async fn test_set_video_source_configuration_invalid_degree_rejected() {
        use crate::onvif::error::OnvifError;
        use crate::onvif::types::common::{Rotate, RotateMode, VideoSourceConfigurationExtension};

        let pm = create_test_pm();
        let mut config = test_configuration(&pm);
        config.extension = Some(VideoSourceConfigurationExtension {
            rotate: Some(Rotate {
                mode: RotateMode::On,
                degree: Some(90),
            }),
        });

        let result = set_video_source_configuration(
            &pm,
            None,
            SetVideoSourceConfiguration {
                configuration: config,
                force_persistence: true,
            },
        )
        .await;

        assert!(matches!(result, Err(OnvifError::InvalidArgVal { .. })));

        // Not persisted: re-reading the configuration shows no Rotate extension.
        let after = test_configuration(&pm);
        assert!(after.extension.is_none());
    }

    #[tokio::test]
    async fn test_set_video_source_configuration_rotate_on_no_degree_persists() {
        use crate::onvif::types::common::{Rotate, RotateMode, VideoSourceConfigurationExtension};

        let pm = create_test_pm();
        let mut config = test_configuration(&pm);
        config.extension = Some(VideoSourceConfigurationExtension {
            rotate: Some(Rotate {
                mode: RotateMode::On,
                degree: None,
            }),
        });

        let result = set_video_source_configuration(
            &pm,
            None,
            SetVideoSourceConfiguration {
                configuration: config,
                force_persistence: true,
            },
        )
        .await;
        assert!(result.is_ok());

        let after = test_configuration(&pm);
        let rotate = after.extension.unwrap().rotate.unwrap();
        assert_eq!(rotate.mode, RotateMode::On);
    }

    #[tokio::test]
    async fn test_set_video_source_configuration_rotate_180_accepted() {
        use crate::onvif::types::common::{Rotate, RotateMode, VideoSourceConfigurationExtension};

        let pm = create_test_pm();
        let mut config = test_configuration(&pm);
        config.extension = Some(VideoSourceConfigurationExtension {
            rotate: Some(Rotate {
                mode: RotateMode::On,
                degree: Some(180),
            }),
        });

        let result = set_video_source_configuration(
            &pm,
            None,
            SetVideoSourceConfiguration {
                configuration: config,
                force_persistence: true,
            },
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_video_source_configuration_live_apply_with_platform() {
        use crate::onvif::types::common::{Rotate, RotateMode, VideoSourceConfigurationExtension};
        use crate::platform::Platform;
        use crate::platform::stub::StubPlatform;
        use std::sync::Arc;

        let pm = create_test_pm();
        let platform: Arc<dyn Platform> = Arc::new(StubPlatform::new());

        let mut config = test_configuration(&pm);
        config.extension = Some(VideoSourceConfigurationExtension {
            rotate: Some(Rotate {
                mode: RotateMode::On,
                degree: None,
            }),
        });

        let result = set_video_source_configuration(
            &pm,
            Some(&platform),
            SetVideoSourceConfiguration {
                configuration: config,
                force_persistence: true,
            },
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_video_source_configuration_no_rotate_extension_unchanged() {
        let pm = create_test_pm();
        let config = test_configuration(&pm);
        assert!(config.extension.is_none());

        let result = set_video_source_configuration(
            &pm,
            None,
            SetVideoSourceConfiguration {
                configuration: config,
                force_persistence: true,
            },
        )
        .await;
        assert!(result.is_ok());

        let after = test_configuration(&pm);
        assert!(after.extension.is_none());
    }
}
