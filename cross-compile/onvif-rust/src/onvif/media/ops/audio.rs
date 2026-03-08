//! Media Audio operations.
//!
//! This module provides audio source and encoder management operations including:
//! - GetAudioSources, GetAudioSourceConfigurations, GetAudioEncoderConfigurations
//! - GetAudioSourceConfiguration, GetAudioEncoderConfiguration
//! - SetAudioSourceConfiguration, SetAudioEncoderConfiguration
//! - GetCompatible* configurations

use crate::onvif::error::OnvifResult;
use crate::onvif::types::media::{
    GetAudioEncoderConfiguration, GetAudioEncoderConfigurationOptionsResponse,
    GetAudioEncoderConfigurationResponse, GetAudioEncoderConfigurationsResponse,
    GetAudioSourceConfiguration, GetAudioSourceConfigurationResponse,
    GetAudioSourceConfigurationsResponse, GetAudioSourcesResponse,
    GetCompatibleAudioEncoderConfigurations, GetCompatibleAudioEncoderConfigurationsResponse,
    GetCompatibleAudioSourceConfigurations, GetCompatibleAudioSourceConfigurationsResponse,
    SetAudioEncoderConfiguration, SetAudioEncoderConfigurationResponse,
    SetAudioSourceConfiguration, SetAudioSourceConfigurationResponse,
};

use super::ProfileManagerRef;

/// Handle GetAudioSources request.
///
/// Returns all available audio sources.
pub fn get_audio_sources(pm: &ProfileManagerRef) -> OnvifResult<GetAudioSourcesResponse> {
    tracing::debug!("GetAudioSources request");
    let audio_sources = pm.get_audio_sources();
    Ok(GetAudioSourcesResponse { audio_sources })
}

/// Handle GetAudioSourceConfigurations request.
///
/// Returns all audio source configurations.
pub fn get_audio_source_configurations(
    pm: &ProfileManagerRef,
) -> OnvifResult<GetAudioSourceConfigurationsResponse> {
    tracing::debug!("GetAudioSourceConfigurations request");
    let configurations = pm.get_audio_source_configurations();
    Ok(GetAudioSourceConfigurationsResponse { configurations })
}

/// Handle GetAudioSourceConfiguration request.
///
/// Returns a specific audio source configuration.
pub fn get_audio_source_configuration(
    pm: &ProfileManagerRef,
    request: GetAudioSourceConfiguration,
) -> OnvifResult<GetAudioSourceConfigurationResponse> {
    tracing::debug!(
        "GetAudioSourceConfiguration request for token: {}",
        request.configuration_token
    );
    let configuration = pm.get_audio_source_configuration(&request.configuration_token)?;
    Ok(GetAudioSourceConfigurationResponse { configuration })
}

/// Handle SetAudioSourceConfiguration request.
///
/// Sets an audio source configuration.
pub fn set_audio_source_configuration(
    pm: &ProfileManagerRef,
    request: SetAudioSourceConfiguration,
) -> OnvifResult<SetAudioSourceConfigurationResponse> {
    tracing::debug!(
        "SetAudioSourceConfiguration for token: {}",
        request.configuration.token
    );
    pm.set_audio_source_configuration(request.configuration)?;
    Ok(SetAudioSourceConfigurationResponse {})
}

/// Handle GetAudioEncoderConfigurations request.
///
/// Returns all audio encoder configurations.
pub fn get_audio_encoder_configurations(
    pm: &ProfileManagerRef,
) -> OnvifResult<GetAudioEncoderConfigurationsResponse> {
    tracing::debug!("GetAudioEncoderConfigurations request");
    let configurations = pm.get_audio_encoder_configurations();
    Ok(GetAudioEncoderConfigurationsResponse { configurations })
}

/// Handle GetAudioEncoderConfiguration request.
///
/// Returns a specific audio encoder configuration.
pub fn get_audio_encoder_configuration(
    pm: &ProfileManagerRef,
    request: GetAudioEncoderConfiguration,
) -> OnvifResult<GetAudioEncoderConfigurationResponse> {
    tracing::debug!(
        "GetAudioEncoderConfiguration request for token: {}",
        request.configuration_token
    );
    let configuration = pm.get_audio_encoder_configuration(&request.configuration_token)?;
    Ok(GetAudioEncoderConfigurationResponse { configuration })
}

/// Handle SetAudioEncoderConfiguration request.
///
/// Updates an audio encoder configuration.
pub fn set_audio_encoder_configuration(
    pm: &ProfileManagerRef,
    request: SetAudioEncoderConfiguration,
) -> OnvifResult<SetAudioEncoderConfigurationResponse> {
    tracing::debug!(
        "SetAudioEncoderConfiguration request for token: {}",
        request.configuration.token
    );
    pm.set_audio_encoder_configuration(request.configuration)?;
    Ok(SetAudioEncoderConfigurationResponse {})
}

/// Handle GetAudioEncoderConfigurationOptions request.
///
/// Returns valid options for audio encoder configuration.
///
/// The request payload (`configuration_token`, `profile_token`) is intentionally
/// ignored. The AK3918 is a single-sensor camera with one audio encoder, so the
/// options are identical regardless of which token the client specifies.
pub fn get_audio_encoder_configuration_options(
    pm: &ProfileManagerRef,
) -> OnvifResult<GetAudioEncoderConfigurationOptionsResponse> {
    tracing::debug!("GetAudioEncoderConfigurationOptions request");
    let options = pm.get_audio_encoder_configuration_options();
    Ok(GetAudioEncoderConfigurationOptionsResponse { options })
}

/// Handle GetCompatibleAudioSourceConfigurations request.
///
/// Returns audio source configurations compatible with the given profile.
/// On this single-sensor AK3918 camera all audio source configurations are
/// compatible with every profile, so the full set is returned after validating
/// that the requested profile exists.
pub fn get_compatible_audio_source_configurations(
    pm: &ProfileManagerRef,
    request: GetCompatibleAudioSourceConfigurations,
) -> OnvifResult<GetCompatibleAudioSourceConfigurationsResponse> {
    tracing::debug!(
        "GetCompatibleAudioSourceConfigurations for profile: {}",
        request.profile_token
    );
    // Verify profile exists
    let _ = pm.get_profile(&request.profile_token)?;
    // All audio source configurations are compatible with all profiles
    let configurations = pm.get_audio_source_configurations();
    Ok(GetCompatibleAudioSourceConfigurationsResponse { configurations })
}

/// Handle GetCompatibleAudioEncoderConfigurations request.
///
/// Returns audio encoder configurations compatible with the given profile.
/// On this single-sensor AK3918 camera all audio encoder configurations are
/// compatible with every profile, so the full set is returned after validating
/// that the requested profile exists.
pub fn get_compatible_audio_encoder_configurations(
    pm: &ProfileManagerRef,
    request: GetCompatibleAudioEncoderConfigurations,
) -> OnvifResult<GetCompatibleAudioEncoderConfigurationsResponse> {
    tracing::debug!(
        "GetCompatibleAudioEncoderConfigurations for profile: {}",
        request.profile_token
    );
    // Verify profile exists
    let _ = pm.get_profile(&request.profile_token)?;
    // All audio encoder configurations are compatible with all profiles
    let configurations = pm.get_audio_encoder_configurations();
    Ok(GetCompatibleAudioEncoderConfigurationsResponse { configurations })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::{AudioEncoderConfiguration, AudioEncoding};

    fn create_test_pm() -> ProfileManagerRef {
        crate::onvif::media::ProfileManager::new()
    }

    #[test]
    fn test_get_audio_sources() {
        let pm = create_test_pm();
        let result = get_audio_sources(&pm);
        assert!(result.is_ok());
        assert!(!result.unwrap().audio_sources.is_empty());
    }

    #[test]
    fn test_get_audio_source_configurations() {
        let pm = create_test_pm();
        let result = get_audio_source_configurations(&pm);
        assert!(result.is_ok());
        assert!(!result.unwrap().configurations.is_empty());
    }

    #[test]
    fn test_get_audio_source_configuration() {
        let pm = create_test_pm();
        let result = get_audio_source_configuration(
            &pm,
            GetAudioSourceConfiguration {
                configuration_token: "AudioSourceConfig_0".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_audio_source_configuration_not_found() {
        let pm = create_test_pm();
        let result = get_audio_source_configuration(
            &pm,
            GetAudioSourceConfiguration {
                configuration_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_audio_encoder_configurations() {
        let pm = create_test_pm();
        let result = get_audio_encoder_configurations(&pm);
        assert!(result.is_ok());
        assert!(!result.unwrap().configurations.is_empty());
    }

    #[test]
    fn test_get_audio_encoder_configuration() {
        let pm = create_test_pm();
        let result = get_audio_encoder_configuration(
            &pm,
            GetAudioEncoderConfiguration {
                configuration_token: "AudioEncoderConfig_0".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_audio_encoder_configuration_not_found() {
        let pm = create_test_pm();
        let result = get_audio_encoder_configuration(
            &pm,
            GetAudioEncoderConfiguration {
                configuration_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_set_audio_encoder_configuration() {
        let pm = create_test_pm();
        let result = set_audio_encoder_configuration(
            &pm,
            SetAudioEncoderConfiguration {
                configuration: AudioEncoderConfiguration {
                    token: "AudioEncoderConfig_0".to_string(),
                    name: "TestAudioEncoder".to_string(),
                    use_count: 1,
                    encoding: AudioEncoding::AAC,
                    bitrate: 128000,
                    sample_rate: 48000,
                    multicast: None,
                    session_timeout: "PT60S".to_string(),
                },
                force_persistence: false,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_audio_encoder_configuration_options() {
        let pm = create_test_pm();
        let result = get_audio_encoder_configuration_options(&pm);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_compatible_audio_source_configurations() {
        let pm = create_test_pm();
        let result = get_compatible_audio_source_configurations(
            &pm,
            GetCompatibleAudioSourceConfigurations {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        assert!(!result.unwrap().configurations.is_empty());
    }

    #[test]
    fn test_get_compatible_audio_encoder_configurations() {
        let pm = create_test_pm();
        let result = get_compatible_audio_encoder_configurations(
            &pm,
            GetCompatibleAudioEncoderConfigurations {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        assert!(!result.unwrap().configurations.is_empty());
    }
}
