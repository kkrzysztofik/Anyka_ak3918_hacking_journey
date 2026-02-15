//! Integration tests for ONVIF Media Service.
//!
//! Tests the complete Media Service operations including:
//! - Profile management (get, create, delete)
//! - Video/Audio source configurations
//! - Video/Audio encoder configurations
//! - Stream URI generation
//! - Snapshot URI generation

use onvif_rust::onvif::media::MediaService;
use onvif_rust::onvif::types::common::{
    ReferenceToken, StreamSetup, StreamType, Transport, TransportProtocol,
};
use onvif_rust::onvif::types::media::{
    CreateProfile, DeleteProfile, GetAudioEncoderConfigurations, GetAudioSourceConfigurations,
    GetAudioSources, GetProfile, GetProfiles, GetServiceCapabilities, GetSnapshotUri, GetStreamUri,
    GetVideoEncoderConfiguration, GetVideoEncoderConfigurationOptions,
    GetVideoEncoderConfigurations, GetVideoSourceConfiguration, GetVideoSourceConfigurations,
    GetVideoSources,
};

// ============================================================================
// Test Fixtures
// ============================================================================

fn create_test_service() -> MediaService {
    MediaService::new()
}

// ============================================================================
// GetProfiles Tests
// ============================================================================

#[test]
fn test_get_profiles_returns_default_profiles() {
    let service = create_test_service();

    let response = service.handle_get_profiles(GetProfiles {}).unwrap();

    // Should have at least the default profile
    assert!(!response.profiles.is_empty());
}

#[test]
fn test_get_profiles_includes_profile_tokens() {
    let service = create_test_service();

    let response = service.handle_get_profiles(GetProfiles {}).unwrap();

    for profile in &response.profiles {
        assert!(!profile.token.is_empty());
    }
}

// ============================================================================
// GetProfile Tests
// ============================================================================

#[test]
fn test_get_profile_returns_existing_profile() {
    let service = create_test_service();

    // First get all profiles to find a valid token
    let profiles_response = service.handle_get_profiles(GetProfiles {}).unwrap();
    let first_profile = profiles_response.profiles.first().unwrap();

    // Now get that specific profile
    let response = service
        .handle_get_profile(GetProfile {
            profile_token: first_profile.token.clone(),
        })
        .unwrap();

    assert_eq!(response.profile.token, first_profile.token);
}

#[test]
fn test_get_profile_invalid_token_fails() {
    let service = create_test_service();

    let result = service.handle_get_profile(GetProfile {
        profile_token: ReferenceToken::from("invalid_token"),
    });

    assert!(result.is_err());
}

// ============================================================================
// CreateProfile / DeleteProfile Tests
// ============================================================================

#[test]
fn test_create_and_delete_profile() {
    let service = create_test_service();

    // Create a new profile
    let create_response = service
        .handle_create_profile(CreateProfile {
            name: "TestProfile".into(),
            token: None,
        })
        .unwrap();

    // Verify profile was created
    assert!(!create_response.profile.token.is_empty());
    assert_eq!(create_response.profile.name, "TestProfile");

    // Delete the profile
    let delete_result = service.handle_delete_profile(DeleteProfile {
        profile_token: create_response.profile.token.clone(),
    });

    assert!(delete_result.is_ok());

    // Verify profile is gone
    let get_result = service.handle_get_profile(GetProfile {
        profile_token: create_response.profile.token,
    });

    assert!(get_result.is_err());
}

#[test]
fn test_create_profile_with_custom_token() {
    let service = create_test_service();

    let create_response = service
        .handle_create_profile(CreateProfile {
            name: "CustomTokenProfile".into(),
            token: Some(ReferenceToken::from("custom_token_123")),
        })
        .unwrap();

    assert_eq!(
        create_response.profile.token,
        ReferenceToken::from("custom_token_123")
    );

    // Clean up
    let _ = service.handle_delete_profile(DeleteProfile {
        profile_token: create_response.profile.token,
    });
}

// ============================================================================
// GetVideoSources Tests
// ============================================================================

#[test]
fn test_get_video_sources_returns_sources() {
    let service = create_test_service();

    let response = service
        .handle_get_video_sources(GetVideoSources {})
        .unwrap();

    // Should have at least one video source
    assert!(!response.video_sources.is_empty());
}

#[test]
fn test_get_video_sources_have_tokens() {
    let service = create_test_service();

    let response = service
        .handle_get_video_sources(GetVideoSources {})
        .unwrap();

    for source in &response.video_sources {
        assert!(!source.token.is_empty());
    }
}

// ============================================================================
// GetAudioSources Tests
// ============================================================================

#[test]
fn test_get_audio_sources() {
    let service = create_test_service();

    let response = service
        .handle_get_audio_sources(GetAudioSources {})
        .unwrap();

    // May have audio sources (optional)
    let _ = response.audio_sources;
}

// ============================================================================
// GetVideoSourceConfigurations Tests
// ============================================================================

#[test]
fn test_get_video_source_configurations() {
    let service = create_test_service();

    let response = service
        .handle_get_video_source_configurations(GetVideoSourceConfigurations {})
        .unwrap();

    assert!(!response.configurations.is_empty());
}

#[test]
fn test_get_video_source_configuration_by_token() {
    let service = create_test_service();

    // Get all configurations first
    let configs = service
        .handle_get_video_source_configurations(GetVideoSourceConfigurations {})
        .unwrap();

    let first_config = configs.configurations.first().unwrap();

    // Get specific configuration
    let response = service
        .handle_get_video_source_configuration(GetVideoSourceConfiguration {
            configuration_token: first_config.token.clone(),
        })
        .unwrap();

    assert_eq!(response.configuration.token, first_config.token);
}

// ============================================================================
// GetAudioSourceConfigurations Tests
// ============================================================================

#[test]
fn test_get_audio_source_configurations() {
    let service = create_test_service();

    let response = service
        .handle_get_audio_source_configurations(GetAudioSourceConfigurations {})
        .unwrap();

    // May have configurations (optional)
    let _ = response.configurations;
}

// ============================================================================
// GetVideoEncoderConfigurations Tests
// ============================================================================

#[test]
fn test_get_video_encoder_configurations() {
    let service = create_test_service();

    let response = service
        .handle_get_video_encoder_configurations(GetVideoEncoderConfigurations {})
        .unwrap();

    assert!(!response.configurations.is_empty());
}

#[test]
fn test_get_video_encoder_configuration_by_token() {
    let service = create_test_service();

    // Get all configurations first
    let configs = service
        .handle_get_video_encoder_configurations(GetVideoEncoderConfigurations {})
        .unwrap();

    let first_config = configs.configurations.first().unwrap();

    // Get specific configuration
    let response = service
        .handle_get_video_encoder_configuration(GetVideoEncoderConfiguration {
            configuration_token: first_config.token.clone(),
        })
        .unwrap();

    assert_eq!(response.configuration.token, first_config.token);
}

// ============================================================================
// GetVideoEncoderConfigurationOptions Tests
// ============================================================================

#[test]
fn test_get_video_encoder_configuration_options() {
    let service = create_test_service();

    let response = service
        .handle_get_video_encoder_configuration_options(GetVideoEncoderConfigurationOptions {
            configuration_token: None,
            profile_token: None,
        })
        .unwrap();

    // Quality range should have valid min/max
    assert!(response.options.quality_range.min <= response.options.quality_range.max);
}

// ============================================================================
// GetAudioEncoderConfigurations Tests
// ============================================================================

#[test]
fn test_get_audio_encoder_configurations() {
    let service = create_test_service();

    let response = service
        .handle_get_audio_encoder_configurations(GetAudioEncoderConfigurations {})
        .unwrap();

    // May have configurations (optional)
    let _ = response.configurations;
}

// ============================================================================
// GetStreamUri Tests
// ============================================================================

#[test]
fn test_get_stream_uri() {
    let service = create_test_service();

    // Get a valid profile token first
    let profiles = service.handle_get_profiles(GetProfiles {}).unwrap();
    let profile_token = profiles.profiles.first().unwrap().token.clone();

    let response = service
        .handle_get_stream_uri(GetStreamUri {
            stream_setup: StreamSetup {
                stream: StreamType::RtpUnicast,
                transport: Transport {
                    protocol: TransportProtocol::RTSP,
                    tunnel: None,
                },
            },
            profile_token,
        })
        .unwrap();

    // URI should be valid RTSP URL
    assert!(response.media_uri.uri.starts_with("rtsp://"));
}

#[test]
fn test_get_stream_uri_invalid_profile_fails() {
    let service = create_test_service();

    let result = service.handle_get_stream_uri(GetStreamUri {
        stream_setup: StreamSetup {
            stream: StreamType::RtpUnicast,
            transport: Transport {
                protocol: TransportProtocol::RTSP,
                tunnel: None,
            },
        },
        profile_token: ReferenceToken::from("invalid_profile"),
    });

    assert!(result.is_err());
}

// ============================================================================
// GetSnapshotUri Tests
// ============================================================================

#[test]
fn test_get_snapshot_uri() {
    let service = create_test_service();

    // Get a valid profile token first
    let profiles = service.handle_get_profiles(GetProfiles {}).unwrap();
    let profile_token = profiles.profiles.first().unwrap().token.clone();

    let response = service
        .handle_get_snapshot_uri(GetSnapshotUri { profile_token })
        .unwrap();

    // URI should be valid HTTP URL
    assert!(
        response.media_uri.uri.starts_with("http://")
            || response.media_uri.uri.starts_with("https://")
    );
}

// ============================================================================
// GetServiceCapabilities Tests
// ============================================================================

#[test]
fn test_get_service_capabilities() {
    let service = create_test_service();

    let response = service
        .handle_get_service_capabilities(GetServiceCapabilities {})
        .unwrap();

    // Capabilities should exist
    let _ = response.capabilities;
}

// ============================================================================
// Complete Workflow Test
// ============================================================================

#[test]
fn test_complete_media_workflow() {
    let service = create_test_service();

    // 1. Get existing profiles
    let profiles = service.handle_get_profiles(GetProfiles {}).unwrap();
    assert!(!profiles.profiles.is_empty());

    // 2. Create a new profile
    let new_profile = service
        .handle_create_profile(CreateProfile {
            name: "WorkflowTestProfile".into(),
            token: None,
        })
        .unwrap();

    // 3. Get video sources
    let sources = service
        .handle_get_video_sources(GetVideoSources {})
        .unwrap();
    assert!(!sources.video_sources.is_empty());

    // 4. Get stream URI for the new profile
    let stream_uri = service
        .handle_get_stream_uri(GetStreamUri {
            stream_setup: StreamSetup {
                stream: StreamType::RtpUnicast,
                transport: Transport {
                    protocol: TransportProtocol::RTSP,
                    tunnel: None,
                },
            },
            profile_token: new_profile.profile.token.clone(),
        })
        .unwrap();
    assert!(!stream_uri.media_uri.uri.is_empty());

    // 5. Clean up - delete the profile
    let delete_result = service.handle_delete_profile(DeleteProfile {
        profile_token: new_profile.profile.token,
    });
    assert!(delete_result.is_ok());
}
// ============================================================================
// Extended Coverage Tests
// ============================================================================

#[test]
fn test_get_profiles_all_have_names() {
    let service = create_test_service();
    let response = service.handle_get_profiles(GetProfiles {}).unwrap();

    for profile in &response.profiles {
        assert!(!profile.name.is_empty(), "Profile should have a name");
    }
}

#[test]
fn test_get_profiles_have_video_source_config() {
    let service = create_test_service();
    let response = service.handle_get_profiles(GetProfiles {}).unwrap();

    for profile in &response.profiles {
        assert!(profile.video_source_configuration.is_some());
    }
}

#[test]
fn test_get_profiles_have_video_encoder_config() {
    let service = create_test_service();
    let response = service.handle_get_profiles(GetProfiles {}).unwrap();

    for profile in &response.profiles {
        assert!(profile.video_encoder_configuration.is_some());
    }
}

#[test]
fn test_create_profile_with_custom_token_extended() {
    let service = create_test_service();

    let custom_token = "custom_profile_001";
    let response = service
        .handle_create_profile(CreateProfile {
            name: "CustomTokenProfile".into(),
            token: Some(ReferenceToken::from(custom_token)),
        })
        .unwrap();

    assert_eq!(response.profile.token.as_str(), custom_token);

    // Clean up
    let _ = service.handle_delete_profile(DeleteProfile {
        profile_token: response.profile.token,
    });
}

#[test]
fn test_create_profile_auto_generates_token_when_none_provided() {
    let service = create_test_service();

    let response = service
        .handle_create_profile(CreateProfile {
            name: "AutoTokenProfile".into(),
            token: None,
        })
        .unwrap();

    assert!(!response.profile.token.is_empty());

    // Clean up
    let _ = service.handle_delete_profile(DeleteProfile {
        profile_token: response.profile.token,
    });
}

#[test]
fn test_delete_profile_nonexistent_fails() {
    let service = create_test_service();

    let result = service.handle_delete_profile(DeleteProfile {
        profile_token: ReferenceToken::from("nonexistent_profile"),
    });

    assert!(result.is_err());
}

#[test]
fn test_get_video_sources_returns_sources_extended() {
    let service = create_test_service();

    let response = service
        .handle_get_video_sources(GetVideoSources {})
        .unwrap();

    assert!(!response.video_sources.is_empty());
}

#[test]
fn test_get_video_sources_have_tokens_extended() {
    let service = create_test_service();

    let response = service
        .handle_get_video_sources(GetVideoSources {})
        .unwrap();

    for source in &response.video_sources {
        assert!(!source.token.is_empty());
    }
}

#[test]
fn test_get_video_sources_have_resolution() {
    let service = create_test_service();

    let response = service
        .handle_get_video_sources(GetVideoSources {})
        .unwrap();

    for source in &response.video_sources {
        assert!(source.resolution.width > 0);
        assert!(source.resolution.height > 0);
    }
}

#[test]
fn test_get_audio_sources_returns_sources() {
    let service = create_test_service();

    let response = service
        .handle_get_audio_sources(GetAudioSources {})
        .unwrap();

    // May have audio sources or not depending on hardware
    // Just verify it doesn't panic
    let _ = response.audio_sources.len();
}

#[test]
fn test_get_video_source_configurations_not_empty() {
    let service = create_test_service();

    let response = service
        .handle_get_video_source_configurations(GetVideoSourceConfigurations {})
        .unwrap();

    assert!(!response.configurations.is_empty());
}

#[test]
fn test_get_video_encoder_configurations_not_empty() {
    let service = create_test_service();

    let response = service
        .handle_get_video_encoder_configurations(GetVideoEncoderConfigurations {})
        .unwrap();

    assert!(!response.configurations.is_empty());
}

#[test]
fn test_get_video_encoder_configuration_valid_token() {
    let service = create_test_service();

    // Get all configurations first
    let configs_response = service
        .handle_get_video_encoder_configurations(GetVideoEncoderConfigurations {})
        .unwrap();

    let first_config = configs_response.configurations.first().unwrap();

    // Get specific configuration
    let response = service
        .handle_get_video_encoder_configuration(GetVideoEncoderConfiguration {
            configuration_token: first_config.token.clone(),
        })
        .unwrap();

    assert_eq!(response.configuration.token, first_config.token);
}

#[test]
fn test_get_video_source_configuration_valid_token() {
    let service = create_test_service();

    // Get all configurations first
    let configs_response = service
        .handle_get_video_source_configurations(GetVideoSourceConfigurations {})
        .unwrap();

    let first_config = configs_response.configurations.first().unwrap();

    // Get specific configuration
    let response = service
        .handle_get_video_source_configuration(GetVideoSourceConfiguration {
            configuration_token: first_config.token.clone(),
        })
        .unwrap();

    assert_eq!(response.configuration.token, first_config.token);
}

#[test]
fn test_get_audio_source_configurations_returns_list() {
    let service = create_test_service();

    let response = service
        .handle_get_audio_source_configurations(GetAudioSourceConfigurations {})
        .unwrap();

    // May or may not have audio configurations
    let _ = response.configurations.len();
}

#[test]
fn test_get_audio_encoder_configurations_returns_list() {
    let service = create_test_service();

    let response = service
        .handle_get_audio_encoder_configurations(GetAudioEncoderConfigurations {})
        .unwrap();

    // May or may not have audio encoder configurations
    let _ = response.configurations.len();
}

#[test]
fn test_get_video_encoder_configuration_options_returns_options() {
    let service = create_test_service();

    // Get a valid configuration token
    let configs_response = service
        .handle_get_video_encoder_configurations(GetVideoEncoderConfigurations {})
        .unwrap();

    let first_config = configs_response.configurations.first().unwrap();

    let response = service
        .handle_get_video_encoder_configuration_options(GetVideoEncoderConfigurationOptions {
            configuration_token: Some(first_config.token.clone()),
            profile_token: None,
        })
        .unwrap();

    // quality_range is IntRange, verify it has valid bounds
    assert!(response.options.quality_range.min <= response.options.quality_range.max);
}

#[test]
fn test_get_stream_uri_rtsp_protocol() {
    let service = create_test_service();

    let profiles_response = service.handle_get_profiles(GetProfiles {}).unwrap();
    let first_profile = profiles_response.profiles.first().unwrap();

    let response = service
        .handle_get_stream_uri(GetStreamUri {
            stream_setup: StreamSetup {
                stream: StreamType::RtpUnicast,
                transport: Transport {
                    protocol: TransportProtocol::RTSP,
                    tunnel: None,
                },
            },
            profile_token: first_profile.token.clone(),
        })
        .unwrap();

    assert!(response.media_uri.uri.starts_with("rtsp://"));
}

#[test]
#[ignore] // HTTP protocol not implemented in mock service
fn test_get_stream_uri_http_protocol() {
    let service = create_test_service();

    let profiles_response = service.handle_get_profiles(GetProfiles {}).unwrap();
    let first_profile = profiles_response.profiles.first().unwrap();

    let response = service
        .handle_get_stream_uri(GetStreamUri {
            stream_setup: StreamSetup {
                stream: StreamType::RtpUnicast,
                transport: Transport {
                    protocol: TransportProtocol::HTTP,
                    tunnel: None,
                },
            },
            profile_token: first_profile.token.clone(),
        })
        .unwrap();

    assert!(response.media_uri.uri.starts_with("http://"));
}

#[test]
fn test_get_snapshot_uri_returns_valid_uri() {
    let service = create_test_service();

    let profiles_response = service.handle_get_profiles(GetProfiles {}).unwrap();
    let first_profile = profiles_response.profiles.first().unwrap();

    let response = service
        .handle_get_snapshot_uri(GetSnapshotUri {
            profile_token: first_profile.token.clone(),
        })
        .unwrap();

    assert!(!response.media_uri.uri.is_empty());
    assert!(response.media_uri.uri.starts_with("http://"));
}

#[test]
fn test_get_service_capabilities_snapshot_support() {
    let service = create_test_service();

    let response = service
        .handle_get_service_capabilities(GetServiceCapabilities {})
        .unwrap();

    // Should indicate snapshot support
    assert!(response.capabilities.snapshot_uri.unwrap_or(false));
}
