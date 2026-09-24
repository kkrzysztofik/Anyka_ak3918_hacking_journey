//! Media Profile operations.
//!
//! This module provides profile management operations including:
//! - GetProfiles, GetProfile, CreateProfile, DeleteProfile
//! - Add/Remove configuration to/from profiles

use crate::onvif::error::OnvifResult;
use crate::onvif::types::media::{
    AddMetadataConfiguration, AddMetadataConfigurationResponse, AddPTZConfiguration,
    AddPTZConfigurationResponse, CreateProfile, CreateProfileResponse, DeleteProfile,
    DeleteProfileResponse, GetCompatibleMetadataConfigurations,
    GetCompatibleMetadataConfigurationsResponse, GetCompatiblePTZConfigurations,
    GetCompatiblePTZConfigurationsResponse, GetMetadataConfiguration,
    GetMetadataConfigurationResponse, GetProfile, GetProfileResponse, GetProfilesResponse,
    RemoveMetadataConfiguration, RemoveMetadataConfigurationResponse, RemovePTZConfiguration,
    RemovePTZConfigurationResponse,
};

use super::ProfileManagerRef;

/// Handle GetProfiles request.
///
/// Returns all configured media profiles.
pub fn get_profiles(pm: &ProfileManagerRef) -> OnvifResult<GetProfilesResponse> {
    tracing::debug!("GetProfiles request");
    let profiles = pm.get_profiles();
    Ok(GetProfilesResponse { profiles })
}

/// Handle GetProfile request.
///
/// Returns a specific profile by token.
pub fn get_profile(pm: &ProfileManagerRef, request: GetProfile) -> OnvifResult<GetProfileResponse> {
    tracing::debug!("GetProfile request for token: {}", request.profile_token);
    let profile = pm.get_profile(&request.profile_token)?;
    Ok(GetProfileResponse { profile })
}

/// Handle CreateProfile request.
///
/// Creates a new media profile.
pub fn create_profile(
    pm: &ProfileManagerRef,
    request: CreateProfile,
) -> OnvifResult<CreateProfileResponse> {
    tracing::debug!("CreateProfile request: name={}", request.name);
    let profile = pm.create_profile(request.name, request.token)?;
    Ok(CreateProfileResponse { profile })
}

/// Handle DeleteProfile request.
///
/// Deletes a media profile.
pub fn delete_profile(
    pm: &ProfileManagerRef,
    request: DeleteProfile,
) -> OnvifResult<DeleteProfileResponse> {
    tracing::debug!("DeleteProfile request for token: {}", request.profile_token);
    pm.delete_profile(&request.profile_token)?;
    Ok(DeleteProfileResponse {})
}

/// Handle AddPTZConfiguration request.
pub fn add_ptz_configuration(
    pm: &ProfileManagerRef,
    request: AddPTZConfiguration,
) -> OnvifResult<AddPTZConfigurationResponse> {
    tracing::debug!(
        "AddPTZConfiguration: profile={} config={}",
        request.profile_token,
        request.configuration_token
    );
    pm.add_ptz_configuration(&request.profile_token, &request.configuration_token)?;
    Ok(AddPTZConfigurationResponse {})
}

/// Handle RemovePTZConfiguration request.
pub fn remove_ptz_configuration(
    pm: &ProfileManagerRef,
    request: RemovePTZConfiguration,
) -> OnvifResult<RemovePTZConfigurationResponse> {
    tracing::debug!("RemovePTZConfiguration: profile={}", request.profile_token);
    pm.remove_ptz_configuration(&request.profile_token)?;
    Ok(RemovePTZConfigurationResponse {})
}

/// Handle GetCompatiblePTZConfigurations request.
pub fn get_compatible_ptz_configurations(
    pm: &ProfileManagerRef,
    request: GetCompatiblePTZConfigurations,
) -> OnvifResult<GetCompatiblePTZConfigurationsResponse> {
    tracing::debug!(
        "GetCompatiblePTZConfigurations for profile: {}",
        request.profile_token
    );
    pm.get_profile(&request.profile_token)?;
    let configurations = pm.get_compatible_ptz_configurations(&request.profile_token);
    Ok(GetCompatiblePTZConfigurationsResponse { configurations })
}

/// Handle AddMetadataConfiguration request.
pub fn add_metadata_configuration(
    pm: &ProfileManagerRef,
    request: AddMetadataConfiguration,
) -> OnvifResult<AddMetadataConfigurationResponse> {
    tracing::debug!(
        "AddMetadataConfiguration: profile={} config={}",
        request.profile_token,
        request.configuration_token
    );
    pm.add_metadata_configuration(&request.profile_token, &request.configuration_token)?;
    Ok(AddMetadataConfigurationResponse {})
}

/// Handle RemoveMetadataConfiguration request.
pub fn remove_metadata_configuration(
    pm: &ProfileManagerRef,
    request: RemoveMetadataConfiguration,
) -> OnvifResult<RemoveMetadataConfigurationResponse> {
    tracing::debug!(
        "RemoveMetadataConfiguration: profile={}",
        request.profile_token
    );
    pm.remove_metadata_configuration(&request.profile_token)?;
    Ok(RemoveMetadataConfigurationResponse {})
}

/// Handle GetCompatibleMetadataConfigurations request.
pub fn get_compatible_metadata_configurations(
    pm: &ProfileManagerRef,
    request: GetCompatibleMetadataConfigurations,
) -> OnvifResult<GetCompatibleMetadataConfigurationsResponse> {
    tracing::debug!(
        "GetCompatibleMetadataConfigurations for profile: {}",
        request.profile_token
    );
    pm.get_profile(&request.profile_token)?;
    let configurations = pm.get_compatible_metadata_configurations(&request.profile_token);
    Ok(GetCompatibleMetadataConfigurationsResponse { configurations })
}

/// Handle GetMetadataConfiguration request.
pub fn get_metadata_configuration(
    pm: &ProfileManagerRef,
    request: GetMetadataConfiguration,
) -> OnvifResult<GetMetadataConfigurationResponse> {
    tracing::debug!(
        "GetMetadataConfiguration for token: {}",
        request.configuration_token
    );
    let configuration = pm.get_metadata_configuration(&request.configuration_token)?;
    Ok(GetMetadataConfigurationResponse { configuration })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::media::types::{METADATA_CONFIG_PREFIX, PTZ_CONFIG_PREFIX};

    fn create_test_pm() -> ProfileManagerRef {
        crate::onvif::media::ProfileManager::new()
    }

    #[test]
    fn test_get_profiles_returns_profiles() {
        let pm = create_test_pm();
        let result = get_profiles(&pm);
        assert!(result.is_ok());
        assert!(!result.unwrap().profiles.is_empty());
    }

    #[test]
    fn test_get_profile_existing() {
        let pm = create_test_pm();
        let result = get_profile(
            &pm,
            GetProfile {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_profile_not_found() {
        let pm = create_test_pm();
        let result = get_profile(
            &pm,
            GetProfile {
                profile_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_create_profile() {
        let pm = create_test_pm();
        let result = create_profile(
            &pm,
            CreateProfile {
                name: "TestProfile".to_string(),
                token: None,
            },
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().profile.name, "TestProfile");
    }

    #[test]
    fn test_delete_profile() {
        let pm = create_test_pm();
        // Create first
        let created = create_profile(
            &pm,
            CreateProfile {
                name: "ToDelete".to_string(),
                token: Some("ToDeleteToken".to_string()),
            },
        )
        .unwrap();
        // Then delete
        let result = delete_profile(
            &pm,
            DeleteProfile {
                profile_token: created.profile.token,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_fixed_profile_fails() {
        let pm = create_test_pm();
        let result = delete_profile(
            &pm,
            DeleteProfile {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_err());
    }

    fn create_test_pm_with_ptz_disabled() -> ProfileManagerRef {
        let config = std::sync::Arc::new(crate::config::ConfigRuntime::new(Default::default()));
        config.write().ptz.enabled = false;
        crate::onvif::media::ProfileManager::with_config(config)
    }

    #[test]
    fn test_add_ptz_configuration_rejects_unknown_profile() {
        let pm = create_test_pm();
        let result = add_ptz_configuration(
            &pm,
            AddPTZConfiguration {
                profile_token: "NoSuchProfile".to_string(),
                configuration_token: format!("{}0", PTZ_CONFIG_PREFIX),
            },
        );
        assert!(
            result.is_err(),
            "an unknown profile must fault, not silently succeed"
        );
    }

    #[test]
    fn test_remove_ptz_configuration_rejects_unknown_profile() {
        let pm = create_test_pm();
        let result = remove_ptz_configuration(
            &pm,
            RemovePTZConfiguration {
                profile_token: "NoSuchProfile".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_compatible_ptz_configurations_empty_when_ptz_disabled() {
        let pm = create_test_pm_with_ptz_disabled();
        let result = get_compatible_ptz_configurations(
            &pm,
            GetCompatiblePTZConfigurations {
                profile_token: "Profile_MainStream".to_string(),
            },
        )
        .unwrap();
        assert!(result.configurations.is_empty());
    }

    #[test]
    fn test_get_compatible_ptz_configurations_rejects_unknown_profile() {
        let pm = create_test_pm();
        let result = get_compatible_ptz_configurations(
            &pm,
            GetCompatiblePTZConfigurations {
                profile_token: "NoSuchProfile".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_metadata_configuration_rejects_unknown_profile() {
        let pm = create_test_pm();
        let result = add_metadata_configuration(
            &pm,
            AddMetadataConfiguration {
                profile_token: "NoSuchProfile".to_string(),
                configuration_token: format!("{}0", METADATA_CONFIG_PREFIX),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_metadata_configuration_rejects_unknown_profile() {
        let pm = create_test_pm();
        let result = remove_metadata_configuration(
            &pm,
            RemoveMetadataConfiguration {
                profile_token: "NoSuchProfile".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_compatible_metadata_configurations_rejects_unknown_profile() {
        let pm = create_test_pm();
        let result = get_compatible_metadata_configurations(
            &pm,
            GetCompatibleMetadataConfigurations {
                profile_token: "NoSuchProfile".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_metadata_configuration_returns_default() {
        let pm = create_test_pm();
        let result = get_metadata_configuration(
            &pm,
            GetMetadataConfiguration {
                configuration_token: format!("{}0", METADATA_CONFIG_PREFIX),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_metadata_configuration_rejects_unknown_token() {
        let pm = create_test_pm();
        let result = get_metadata_configuration(
            &pm,
            GetMetadataConfiguration {
                configuration_token: "NoSuchMetadata".to_string(),
            },
        );
        assert!(result.is_err());
    }
}
