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
    GetVideoSourceConfiguration, GetVideoSourceConfigurationOptions,
    GetVideoSourceConfigurationOptionsResponse, GetVideoSourceConfigurationResponse,
    GetVideoSourceConfigurations, GetVideoSourceConfigurationsResponse, GetVideoSources,
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
/// Updates a video source configuration.
pub fn set_video_source_configuration(
    pm: &ProfileManagerRef,
    request: SetVideoSourceConfiguration,
) -> OnvifResult<SetVideoSourceConfigurationResponse> {
    tracing::debug!(
        "SetVideoSourceConfiguration request for token: {}",
        request.configuration.token
    );
    pm.set_video_source_configuration(request.configuration)?;
    Ok(SetVideoSourceConfigurationResponse {})
}

/// Handle GetVideoSourceConfigurationOptions request.
///
/// Returns valid options for video source configuration.
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
}
