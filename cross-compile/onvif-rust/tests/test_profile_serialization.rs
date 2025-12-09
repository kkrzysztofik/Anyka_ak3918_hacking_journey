use onvif_rust::onvif::media::GetProfilesResponse;
use onvif_rust::onvif::types::common::*;

#[test]
fn test_serialize_get_profiles_with_data() {
    // Create a profile similar to what would be created from config
    let profile = Profile {
        token: "Profile_Profile1".to_string(),
        fixed: Some(true),
        name: "Profile1".to_string(),
        video_source_configuration: Some(VideoSourceConfiguration {
            token: "VideoSourceConfig_0".to_string(),
            source_token: "VideoSource_0".to_string(),
            name: "VideoSourceConfig_0".to_string(),
            use_count: 1,
            bounds: IntRectangle {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            view_mode: None,
            extension: None,
        }),
        video_encoder_configuration: Some(VideoEncoderConfiguration {
            token: "VideoEncoderConfig_0".to_string(),
            name: "VideoEncoderConfig_0".to_string(),
            use_count: 1,
            encoding: VideoEncoding::H264,
            resolution: VideoResolution {
                width: 1920,
                height: 1080,
            },
            quality: 80.0,
            rate_control: Some(VideoRateControl {
                frame_rate_limit: 25,
                encoding_interval: 1,
                bitrate_limit: 4000,
            }),
            h264: Some(H264Configuration {
                gov_length: 50,
                h264_profile: H264Profile::Main,
            }),
            mpeg4: None,
            multicast: None,
            session_timeout: "PT60S".to_string(),
        }),
        audio_source_configuration: Some(AudioSourceConfiguration {
            token: "AudioSourceConfig_0".to_string(),
            source_token: "AudioSource_0".to_string(),
            name: "AudioSourceConfig_0".to_string(),
            use_count: 1,
        }),
        audio_encoder_configuration: Some(AudioEncoderConfiguration {
            token: "AudioEncoderConfig_0".to_string(),
            name: "AudioEncoderConfig_0".to_string(),
            use_count: 1,
            encoding: AudioEncoding::G711,
            bitrate: 64,
            sample_rate: 8000,
            multicast: None,
            session_timeout: "PT60S".to_string(),
        }),
        ptz_configuration: Some(PTZConfiguration {
            token: "PTZConfig_0".to_string(),
            name: "PTZConfig_0".to_string(),
            use_count: 1,
            node_token: "PTZNode_0".to_string(),
            default_absolute_pan_tilt_position_space: Some(
                "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace".to_string(),
            ),
            default_absolute_zoom_position_space: Some(
                "http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace".to_string(),
            ),
            default_relative_pan_tilt_translation_space: Some(
                "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace".to_string(),
            ),
            default_relative_zoom_translation_space: Some(
                "http://www.onvif.org/ver10/tptz/ZoomSpaces/TranslationGenericSpace".to_string(),
            ),
            default_continuous_pan_tilt_velocity_space: Some(
                "http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace".to_string(),
            ),
            default_continuous_zoom_velocity_space: Some(
                "http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace".to_string(),
            ),
            default_ptz_speed: None,
            default_ptz_timeout: Some("PT1S".to_string()),
            pan_tilt_limits: None,
            zoom_limits: None,
            move_ramp: None,
            preset_ramp: None,
            preset_tour_ramp: None,
            extension: None,
        }),
        metadata_configuration: None,
        extension: None,
    };

    let response = GetProfilesResponse {
        profiles: vec![profile],
    };

    let xml = quick_xml::se::to_string(&response).expect("Failed to serialize");
    println!("Serialized XML:\n{}", xml);

    // Check that XML contains expected elements
    assert!(xml.contains("trt:Profiles"));
    assert!(xml.contains("Profile1"));
    assert!(xml.contains("VideoSourceConfiguration"));
    assert!(xml.contains("VideoEncoderConfiguration"));
    assert!(xml.contains("H264"));
}
