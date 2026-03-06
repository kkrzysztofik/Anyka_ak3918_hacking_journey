//! Media Profile operations.
//!
//! This module provides profile management operations including:
//! - GetProfiles, GetProfile, CreateProfile, DeleteProfile
//! - Add/Remove configuration to/from profiles

use crate::onvif::error::OnvifResult;
use crate::onvif::types::common::Profile;
use crate::onvif::types::media::{
    CreateProfile, CreateProfileResponse, DeleteProfile, DeleteProfileResponse, GetProfile,
    GetProfileResponse, GetProfilesResponse,
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

/// Get all profiles as a vector.
pub fn get_all_profiles(pm: &ProfileManagerRef) -> Vec<Profile> {
    pm.get_profiles()
}

/// Get a single profile by token.
pub fn get_profile_by_token(pm: &ProfileManagerRef, token: &str) -> OnvifResult<Profile> {
    pm.get_profile(&token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
