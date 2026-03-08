use crate::config::ConfigRuntime;
use crate::onvif::types::common::{
    AudioEncoderConfiguration, AudioEncoding, AudioSource, AudioSourceConfiguration, FloatRange,
    H264Configuration, H264Profile, IntRectangle, IpAddress, IpType, MulticastConfiguration,
    PTZConfiguration, PTZSpeed, PanTiltLimits, Profile, Space1DDescription, Space2DDescription,
    Vector1D, Vector2D, VideoEncoderConfiguration, VideoEncoding, VideoRateControl,
    VideoResolution, VideoSource, VideoSourceConfiguration, ZoomLimits,
};
use crate::platform::Resolution;

use super::types::{
    AUDIO_ENCODER_CONFIG_PREFIX, AUDIO_SOURCE_CONFIG_PREFIX, DEFAULT_AUDIO_SOURCE_TOKEN,
    DEFAULT_PTZ_NODE_TOKEN, DEFAULT_VIDEO_SOURCE_TOKEN, PROFILE_TOKEN_PREFIX, PTZ_CONFIG_PREFIX,
    VIDEO_ENCODER_CONFIG_PREFIX, VIDEO_SOURCE_CONFIG_PREFIX,
};

#[derive(Debug, Clone)]
pub(crate) struct DefaultSources {
    pub video_source: VideoSource,
    pub audio_source: AudioSource,
    pub video_source_config: VideoSourceConfiguration,
    pub audio_source_config: AudioSourceConfiguration,
}

#[derive(Debug, Clone)]
pub(crate) struct ProfileConfig {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub bitrate: u32,
    pub encoding_str: String,
    pub profile_str: String,
    pub audio_enabled: bool,
    pub audio_encoding_str: String,
    pub audio_bitrate: u32,
    pub audio_sample_rate: u32,
}

pub(crate) fn create_default_sources(max_sensor_resolution: Resolution) -> DefaultSources {
    DefaultSources {
        video_source: VideoSource {
            token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
            framerate: 30.0,
            resolution: VideoResolution {
                width: max_sensor_resolution.width as i32,
                height: max_sensor_resolution.height as i32,
            },
            imaging: None,
            extension: None,
        },
        audio_source: AudioSource {
            token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
            channels: 1,
        },
        video_source_config: VideoSourceConfiguration {
            token: format!("{}0", VIDEO_SOURCE_CONFIG_PREFIX),
            source_token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
            name: "VideoSourceConfig_0".to_string(),
            use_count: 1,
            view_mode: None,
            bounds: IntRectangle {
                x: 0,
                y: 0,
                width: max_sensor_resolution.width as i32,
                height: max_sensor_resolution.height as i32,
            },
            extension: None,
        },
        audio_source_config: AudioSourceConfiguration {
            token: format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX),
            source_token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
            name: "AudioSourceConfig_0".to_string(),
            use_count: 1,
        },
    }
}

pub(crate) fn is_profile_enabled(config: &ConfigRuntime, profile_num: u32) -> bool {
    config.read().stream_profile(profile_num).enabled
}

pub(crate) fn read_profile_config(config: &ConfigRuntime, profile_num: u32) -> ProfileConfig {
    let c = config.read();
    let sp = c.stream_profile(profile_num);
    ProfileConfig {
        name: if sp.name.is_empty() {
            format!("Stream{}", profile_num)
        } else {
            sp.name.clone()
        },
        width: sp.width,
        height: sp.height,
        framerate: sp.framerate,
        bitrate: sp.bitrate,
        encoding_str: sp.encoding.clone(),
        profile_str: sp.profile.clone(),
        audio_enabled: sp.audio_enabled,
        audio_encoding_str: sp.audio_encoding.clone(),
        audio_bitrate: sp.audio_bitrate,
        audio_sample_rate: sp.audio_sample_rate,
    }
}

pub(crate) fn parse_video_encoding(encoding_str: &str, log_unknown: bool) -> VideoEncoding {
    match encoding_str.to_lowercase().as_str() {
        "h264" => VideoEncoding::H264,
        "mjpeg" | "jpeg" => VideoEncoding::JPEG,
        "mpeg4" => VideoEncoding::MPEG4,
        _ => {
            if log_unknown {
                tracing::warn!(
                    "Unknown video encoding '{}', defaulting to H264",
                    encoding_str
                );
            }
            VideoEncoding::H264
        }
    }
}

pub(crate) fn parse_h264_profile(profile_str: &str, log_unknown: bool) -> H264Profile {
    match profile_str.to_lowercase().as_str() {
        "baseline" => H264Profile::Baseline,
        "main" => H264Profile::Main,
        "high" => H264Profile::High,
        _ => {
            if log_unknown {
                tracing::warn!(
                    "Unknown H.264 profile '{}', defaulting to Main",
                    profile_str
                );
            }
            H264Profile::Main
        }
    }
}

pub(crate) fn parse_audio_encoding(encoding_str: &str, log_unknown: bool) -> AudioEncoding {
    match encoding_str.to_lowercase().as_str() {
        "g711" => AudioEncoding::G711,
        "aac" => AudioEncoding::AAC,
        "g726" => AudioEncoding::G726,
        _ => {
            if log_unknown {
                tracing::warn!(
                    "Unknown audio encoding '{}', defaulting to G711",
                    encoding_str
                );
            }
            AudioEncoding::G711
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_video_encoder_config(
    profile_count: u32,
    name: &str,
    width: u32,
    height: u32,
    framerate: u32,
    bitrate: u32,
    video_encoding: &VideoEncoding,
    h264_profile: H264Profile,
) -> VideoEncoderConfiguration {
    let token = format!("{}{}", VIDEO_ENCODER_CONFIG_PREFIX, profile_count);
    VideoEncoderConfiguration {
        token: token.clone(),
        name: name.to_string(),
        use_count: 1,
        encoding: video_encoding.clone(),
        resolution: VideoResolution {
            width: width as i32,
            height: height as i32,
        },
        quality: 0.8,
        rate_control: Some(VideoRateControl {
            frame_rate_limit: framerate as i32,
            encoding_interval: 1,
            bitrate_limit: bitrate as i32,
        }),
        mpeg4: None,
        h264: if matches!(video_encoding, VideoEncoding::H264) {
            Some(H264Configuration {
                gov_length: framerate as i32,
                h264_profile,
            })
        } else {
            None
        },
        multicast: Some(default_multicast_configuration()),
        session_timeout: "PT60S".to_string(),
    }
}

pub(crate) fn create_audio_encoder_config(
    profile_count: u32,
    bitrate: u32,
    sample_rate: u32,
    encoding: AudioEncoding,
) -> AudioEncoderConfiguration {
    let token = format!("{}{}", AUDIO_ENCODER_CONFIG_PREFIX, profile_count);
    AudioEncoderConfiguration {
        token: token.clone(),
        name: format!("AudioEncoderConfig_{}", profile_count),
        use_count: 1,
        encoding,
        bitrate: bitrate as i32,
        sample_rate: sample_rate as i32,
        multicast: Some(default_multicast_configuration()),
        session_timeout: "PT60S".to_string(),
    }
}

pub(crate) fn create_profile(
    name: &str,
    profile_count: u32,
    video_encoder: Option<VideoEncoderConfiguration>,
    audio_encoder: Option<AudioEncoderConfiguration>,
    ptz_config: &PTZConfiguration,
) -> Profile {
    let profile_token = format!("{}{}", PROFILE_TOKEN_PREFIX, name);
    Profile {
        token: profile_token,
        fixed: Some(true),
        name: name.to_string(),
        video_source_configuration: Some(VideoSourceConfiguration {
            token: format!("{}{}", VIDEO_SOURCE_CONFIG_PREFIX, profile_count),
            source_token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
            name: format!("VideoSourceConfig_{}", profile_count),
            use_count: 1,
            view_mode: None,
            bounds: IntRectangle {
                x: 0,
                y: 0,
                width: video_encoder
                    .as_ref()
                    .map(|c| c.resolution.width)
                    .unwrap_or(1920),
                height: video_encoder
                    .as_ref()
                    .map(|c| c.resolution.height)
                    .unwrap_or(1080),
            },
            extension: None,
        }),
        audio_source_configuration: audio_encoder.as_ref().map(|_| AudioSourceConfiguration {
            token: format!("{}{}", AUDIO_SOURCE_CONFIG_PREFIX, profile_count),
            source_token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
            name: format!("AudioSourceConfig_{}", profile_count),
            use_count: 1,
        }),
        video_encoder_configuration: video_encoder,
        audio_encoder_configuration: audio_encoder,
        ptz_configuration: Some(ptz_config.clone()),
        metadata_configuration: None,
        extension: None,
    }
}

pub(crate) fn create_default_ptz_configuration() -> PTZConfiguration {
    PTZConfiguration {
        token: format!("{}0", PTZ_CONFIG_PREFIX),
        name: "DefaultPTZConfig".to_string(),
        use_count: 2,
        move_ramp: None,
        preset_ramp: None,
        preset_tour_ramp: None,
        node_token: DEFAULT_PTZ_NODE_TOKEN.to_string(),
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
        default_ptz_speed: Some(PTZSpeed {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: 0.5,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.5,
                space: None,
            }),
        }),
        default_ptz_timeout: Some("PT5S".to_string()),
        pan_tilt_limits: Some(PanTiltLimits {
            range: Space2DDescription {
                uri: "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace"
                    .to_string(),
                x_range: FloatRange {
                    min: -1.0,
                    max: 1.0,
                },
                y_range: FloatRange {
                    min: -1.0,
                    max: 1.0,
                },
            },
        }),
        zoom_limits: Some(ZoomLimits {
            range: Space1DDescription {
                uri: "http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace".to_string(),
                x_range: FloatRange { min: 0.0, max: 1.0 },
            },
        }),
        extension: None,
    }
}

fn default_multicast_configuration() -> MulticastConfiguration {
    MulticastConfiguration {
        address: IpAddress {
            address_type: IpType::IPv4,
            ipv4_address: Some("0.0.0.0".to_string()),
            ipv6_address: None,
        },
        port: 0,
        ttl: 0,
        auto_start: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, ConfigRuntime};

    #[test]
    fn test_create_default_sources_uses_sensor_resolution() {
        let defaults = create_default_sources(Resolution::new(1280, 720));

        assert_eq!(defaults.video_source.resolution.width, 1280);
        assert_eq!(defaults.video_source.resolution.height, 720);
        assert_eq!(defaults.video_source_config.bounds.width, 1280);
        assert_eq!(defaults.video_source_config.bounds.height, 720);
        assert_eq!(defaults.audio_source.channels, 1);
        assert_eq!(defaults.audio_source_config.use_count, 1);
    }

    #[test]
    fn test_parse_video_encoding_defaults_to_h264() {
        let encoding = parse_video_encoding("unknown", false);

        assert_eq!(encoding, VideoEncoding::H264);
    }

    #[test]
    fn test_read_profile_config_uses_default_name_for_empty_name() {
        let mut app_config = AppConfig::default();
        app_config.stream_profile_1.name.clear();
        let config = ConfigRuntime::new(app_config);

        let profile = read_profile_config(&config, 1);

        assert_eq!(profile.name, "Stream1");
        assert!(profile.width > 0);
        assert!(profile.height > 0);
    }

    #[test]
    fn test_create_profile_uses_encoder_resolution_for_bounds() {
        let ptz = create_default_ptz_configuration();
        let video_encoder = create_video_encoder_config(
            1,
            "Profile1",
            800,
            600,
            25,
            1024,
            &VideoEncoding::H264,
            H264Profile::Main,
        );

        let profile = create_profile("Profile1", 1, Some(video_encoder), None, &ptz);

        let bounds = profile
            .video_source_configuration
            .expect("video source config should exist")
            .bounds;
        assert_eq!(bounds.width, 800);
        assert_eq!(bounds.height, 600);
        assert!(profile.audio_source_configuration.is_none());
    }
}
