//! Integration tests for ONVIF profile PTZ and metadata attach/detach.
//!
//! Exercises the Add/Remove/GetCompatible PTZ and metadata verbs at the
//! service level, including the attach → detach → re-attach flow and the
//! token-validation fault when an unknown configuration is supplied.

use onvif_rust::onvif::media::MediaService;
use onvif_rust::onvif::types::media::{
    AddMetadataConfiguration, AddPTZConfiguration, CreateProfile,
    GetCompatibleMetadataConfigurations, GetCompatiblePTZConfigurations, GetProfiles,
    RemoveMetadataConfiguration, RemovePTZConfiguration,
};

fn new_service() -> MediaService {
    MediaService::new()
}

fn create_test_profile(service: &MediaService) {
    service
        .handle_create_profile(CreateProfile {
            name: "TestCam".into(),
            token: Some("Profile_Test".into()),
        })
        .unwrap();
}

fn profile_ptz_token(service: &MediaService, token: &str) -> Option<String> {
    let profiles = service
        .handle_get_profiles(GetProfiles {})
        .unwrap()
        .profiles;
    profiles
        .into_iter()
        .find(|p| p.token == token)
        .and_then(|p| p.ptz_configuration.map(|c| c.token))
}

fn profile_metadata_token(service: &MediaService, token: &str) -> Option<String> {
    let profiles = service
        .handle_get_profiles(GetProfiles {})
        .unwrap()
        .profiles;
    profiles
        .into_iter()
        .find(|p| p.token == token)
        .and_then(|p| p.metadata_configuration.map(|c| c.token))
}

#[test]
fn test_ptz_attach_detach_reattach_flow() {
    let service = new_service();
    create_test_profile(&service);

    // Attach the default PTZ configuration.
    service
        .handle_add_ptz_configuration(AddPTZConfiguration {
            profile_token: "Profile_Test".into(),
            configuration_token: "PTZConfig_0".into(),
        })
        .unwrap();
    assert_eq!(
        profile_ptz_token(&service, "Profile_Test").as_deref(),
        Some("PTZConfig_0")
    );

    // Detach it.
    service
        .handle_remove_ptz_configuration(RemovePTZConfiguration {
            profile_token: "Profile_Test".into(),
        })
        .unwrap();
    assert_eq!(profile_ptz_token(&service, "Profile_Test"), None);

    // Re-attach the same configuration.
    service
        .handle_add_ptz_configuration(AddPTZConfiguration {
            profile_token: "Profile_Test".into(),
            configuration_token: "PTZConfig_0".into(),
        })
        .unwrap();
    assert_eq!(
        profile_ptz_token(&service, "Profile_Test").as_deref(),
        Some("PTZConfig_0")
    );
}

#[test]
fn test_metadata_attach_detach_flow() {
    let service = new_service();
    create_test_profile(&service);

    service
        .handle_add_metadata_configuration(AddMetadataConfiguration {
            profile_token: "Profile_Test".into(),
            configuration_token: "MetadataConfig_0".into(),
        })
        .unwrap();
    assert_eq!(
        profile_metadata_token(&service, "Profile_Test").as_deref(),
        Some("MetadataConfig_0")
    );

    service
        .handle_remove_metadata_configuration(RemoveMetadataConfiguration {
            profile_token: "Profile_Test".into(),
        })
        .unwrap();
    assert_eq!(profile_metadata_token(&service, "Profile_Test"), None);
}

#[test]
fn test_add_ptz_with_unknown_config_fails() {
    let service = new_service();
    create_test_profile(&service);

    let result = service.handle_add_ptz_configuration(AddPTZConfiguration {
        profile_token: "Profile_Test".into(),
        configuration_token: "PTZConfig_9".into(),
    });
    assert!(
        result.is_err(),
        "adding a PTZ config token that is not the default must fault"
    );
}

#[test]
fn test_get_compatible_ptz_and_metadata() {
    let service = new_service();
    create_test_profile(&service);

    let ptz = service
        .handle_get_compatible_ptz_configurations(GetCompatiblePTZConfigurations {
            profile_token: "Profile_Test".into(),
        })
        .unwrap();
    // ptz.enabled defaults to true → exactly the default PTZ configuration.
    assert_eq!(
        ptz.configurations
            .iter()
            .map(|c| c.token.as_str())
            .collect::<Vec<_>>(),
        vec!["PTZConfig_0"]
    );

    let metadata = service
        .handle_get_compatible_metadata_configurations(GetCompatibleMetadataConfigurations {
            profile_token: "Profile_Test".into(),
        })
        .unwrap();
    // Assert the exact seeded token (not merely "non-empty") so a store reset to a
    // different default is caught, matching the PTZ assertion above.
    assert_eq!(
        metadata
            .configurations
            .iter()
            .map(|c| c.token.as_str())
            .collect::<Vec<_>>(),
        vec!["MetadataConfig_0"],
        "the seeded metadata configuration must be offered as compatible"
    );
}
