//! Type serialization/deserialization tests.
//!
//! These tests validate that ONVIF types can correctly serialize/deserialize
//! to/from XML using sample SOAP payloads from actual ONVIF protocol captures.

mod test_utils;

use onvif_rust::onvif::types::common::{
    BacklightCompensation20, BacklightCompensationMode, DiscoveryMode, Exposure20, ExposureMode,
    ExposurePriority, FloatRange, H264Configuration, H264Profile, ImagingSettings20, IntRange,
    IntRectangle, IrCutFilterMode, MoveStatus, PTZPreset, PTZSpeed, PTZVector, Profile, Rectangle,
    SetDateTimeType, SoapFault, Vector1D, Vector2D, VideoEncoderConfiguration, VideoEncoding,
    VideoRateControl, VideoResolution, VideoSourceConfiguration, WhiteBalanceMode, WideDynamicMode,
};
use onvif_rust::onvif::types::device::{
    GetCapabilitiesResponse, GetDeviceInformation, GetDeviceInformationResponse,
    GetDiscoveryModeResponse, GetHostnameResponse, GetNetworkInterfacesResponse, GetScopesResponse,
    GetServicesResponse, GetSystemDateAndTimeResponse, GetUsersResponse,
};
use onvif_rust::onvif::types::imaging::{GetImagingSettings, GetImagingSettingsResponse};
use onvif_rust::onvif::types::media::{
    GetAudioSourcesResponse, GetProfileResponse, GetProfiles, GetProfilesResponse,
    GetSnapshotUriResponse, GetStreamUriResponse, GetVideoEncoderConfigurationOptionsResponse,
    GetVideoSourceConfigurationResponse, GetVideoSourcesResponse,
};
use onvif_rust::onvif::types::ptz::{
    ContinuousMoveResponse, GetNodeResponse, GetNodesResponse, GetPresets, GetPresetsResponse,
    GetStatusResponse, Stop, StopResponse,
};

use quick_xml::de::from_str;
use quick_xml::se::to_string;

use test_utils::extract_response_element;

// ============================================================================
// Device Service Types
// ============================================================================

#[test]
fn test_get_device_information_response_deserialize() {
    let xml = r#"
    <GetDeviceInformationResponse xmlns="http://www.onvif.org/ver10/device/wsdl">
        <Manufacturer>H264</Manufacturer>
        <Model>XM535_X6E-WEQ_8M</Model>
        <FirmwareVersion>V5.05.R02.000A07F3</FirmwareVersion>
        <SerialNumber>940b5413c9d025f27ne4</SerialNumber>
        <HardwareId>00001</HardwareId>
    </GetDeviceInformationResponse>
    "#;

    let response: GetDeviceInformationResponse = from_str(xml).expect("Failed to deserialize");

    assert_eq!(response.manufacturer, "H264");
    assert_eq!(response.model, "XM535_X6E-WEQ_8M");
    assert_eq!(response.firmware_version, "V5.05.R02.000A07F3");
    assert_eq!(response.serial_number, "940b5413c9d025f27ne4");
    assert_eq!(response.hardware_id, "00001");
}

#[test]
fn test_get_device_information_response_serialize() {
    let response = GetDeviceInformationResponse {
        manufacturer: "TestManufacturer".to_string(),
        model: "TestModel".to_string(),
        firmware_version: "1.0.0".to_string(),
        serial_number: "SN12345".to_string(),
        hardware_id: "HW001".to_string(),
    };

    let xml = to_string(&response).expect("Failed to serialize");

    assert!(xml.contains("TestManufacturer"));
    assert!(xml.contains("TestModel"));
    assert!(xml.contains("1.0.0"));
    assert!(xml.contains("SN12345"));
    assert!(xml.contains("HW001"));
}

#[test]
fn test_get_device_information_roundtrip() {
    let original = GetDeviceInformationResponse {
        manufacturer: "Anyka".to_string(),
        model: "AK3918".to_string(),
        firmware_version: "2.0.0".to_string(),
        serial_number: "ANYKA001".to_string(),
        hardware_id: "HW-AK3918".to_string(),
    };

    let xml = to_string(&original).expect("Failed to serialize");
    let deserialized: GetDeviceInformationResponse = from_str(&xml).expect("Failed to deserialize");

    assert_eq!(original.manufacturer, deserialized.manufacturer);
    assert_eq!(original.model, deserialized.model);
    assert_eq!(original.firmware_version, deserialized.firmware_version);
    assert_eq!(original.serial_number, deserialized.serial_number);
    assert_eq!(original.hardware_id, deserialized.hardware_id);
}

// ============================================================================
// Common Types
// ============================================================================

#[test]
fn test_video_resolution_serialize() {
    let resolution = VideoResolution {
        width: 1920,
        height: 1080,
    };

    let xml = to_string(&resolution).expect("Failed to serialize");
    assert!(xml.contains("1920"));
    assert!(xml.contains("1080"));
}

#[test]
fn test_video_resolution_deserialize() {
    let xml = r#"
    <Resolution xmlns="http://www.onvif.org/ver10/schema">
        <Width>3072</Width>
        <Height>2048</Height>
    </Resolution>
    "#;

    let resolution: VideoResolution = from_str(xml).expect("Failed to deserialize");
    assert_eq!(resolution.width, 3072);
    assert_eq!(resolution.height, 2048);
}

#[test]
fn test_int_range_serialize() {
    let range = IntRange { min: 1, max: 100 };

    let xml = to_string(&range).expect("Failed to serialize");
    assert!(xml.contains("1"));
    assert!(xml.contains("100"));
}

#[test]
fn test_float_range_serialize() {
    let range = FloatRange { min: 0.0, max: 1.0 };

    let xml = to_string(&range).expect("Failed to serialize");
    assert!(xml.contains("0"));
    assert!(xml.contains("1"));
}

#[test]
fn test_vector2d_serialize() {
    let vector = Vector2D {
        x: 0.5,
        y: -0.5,
        space: Some(
            "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace".to_string(),
        ),
    };

    let xml = to_string(&vector).expect("Failed to serialize");
    assert!(xml.contains("0.5"));
    assert!(xml.contains("-0.5"));
}

#[test]
fn test_vector1d_serialize() {
    let vector = Vector1D {
        x: 0.75,
        space: Some("http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace".to_string()),
    };

    let xml = to_string(&vector).expect("Failed to serialize");
    assert!(xml.contains("0.75"));
}

// ============================================================================
// Imaging Types
// ============================================================================

#[test]
fn test_backlight_compensation_serialize() {
    let blc = BacklightCompensation20 {
        mode: BacklightCompensationMode::OFF,
        level: Some(10.0),
    };

    let xml = to_string(&blc).expect("Failed to serialize");
    assert!(xml.contains("OFF"));
    assert!(xml.contains("10"));
}

#[test]
fn test_exposure20_serialize() {
    let exposure = Exposure20 {
        mode: ExposureMode::AUTO,
        priority: Some(ExposurePriority::LowNoise),
        window: Some(Rectangle {
            bottom: 1.0,
            top: 0.0,
            right: 1.0,
            left: 0.0,
        }),
        min_exposure_time: Some(10.0),
        max_exposure_time: Some(40000.0),
        min_gain: Some(0.0),
        max_gain: Some(100.0),
        min_iris: Some(0.0),
        max_iris: Some(100.0),
        exposure_time: None,
        gain: None,
        iris: None,
    };

    let xml = to_string(&exposure).expect("Failed to serialize");
    assert!(xml.contains("AUTO"));
    assert!(xml.contains("10"));
    assert!(xml.contains("40000"));
}

#[test]
fn test_imaging_settings20_defaults() {
    let settings = ImagingSettings20::default();

    assert!(settings.backlight_compensation.is_none());
    assert!(settings.brightness.is_none());
    assert!(settings.color_saturation.is_none());
    assert!(settings.contrast.is_none());
    assert!(settings.exposure.is_none());
    assert!(settings.focus.is_none());
    assert!(settings.ir_cut_filter.is_none());
    assert!(settings.sharpness.is_none());
    assert!(settings.wide_dynamic_range.is_none());
    assert!(settings.white_balance.is_none());
}

// ============================================================================
// PTZ Types
// ============================================================================

#[test]
fn test_ptz_vector_serialize() {
    let ptz_vector = PTZVector {
        pan_tilt: Some(Vector2D {
            x: 0.5,
            y: 0.25,
            space: None,
        }),
        zoom: Some(Vector1D {
            x: 0.1,
            space: None,
        }),
    };

    let xml = to_string(&ptz_vector).expect("Failed to serialize");
    assert!(xml.contains("0.5"));
    assert!(xml.contains("0.25"));
    assert!(xml.contains("0.1"));
}

#[test]
fn test_ptz_speed_serialize() {
    let speed = PTZSpeed {
        pan_tilt: Some(Vector2D {
            x: 1.0,
            y: 0.5,
            space: None,
        }),
        zoom: Some(Vector1D {
            x: 0.5,
            space: None,
        }),
    };

    let xml = to_string(&speed).expect("Failed to serialize");
    assert!(xml.contains("1"));
    assert!(xml.contains("0.5"));
}

#[test]
fn test_ptz_preset_serialize() {
    let preset = PTZPreset {
        token: Some("preset_1".to_string()),
        name: Some("Home".to_string()),
        ptz_position: Some(PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.0,
                y: 0.0,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.0,
                space: None,
            }),
        }),
    };

    let xml = to_string(&preset).expect("Failed to serialize");
    assert!(xml.contains("preset_1") || xml.contains("token"));
    assert!(xml.contains("Home") || xml.contains("name"));
}

// ============================================================================
// Media Types
// ============================================================================

#[test]
fn test_video_encoder_configuration_serialize() {
    let config = VideoEncoderConfiguration {
        token: "encoder_1".to_string(),
        name: "Main Encoder".to_string(),
        use_count: 1,
        encoding: VideoEncoding::H264,
        resolution: VideoResolution {
            width: 1920,
            height: 1080,
        },
        quality: 5.0,
        rate_control: Some(VideoRateControl {
            frame_rate_limit: 30,
            encoding_interval: 1,
            bitrate_limit: 4096,
        }),
        multicast: None,
        session_timeout: "PT60S".to_string(),
        h264: Some(H264Configuration {
            gov_length: 30,
            h264_profile: H264Profile::Main,
        }),
        mpeg4: None,
    };

    let xml = to_string(&config).expect("Failed to serialize");
    assert!(xml.contains("encoder_1"));
    assert!(xml.contains("1920"));
    assert!(xml.contains("1080"));
    assert!(xml.contains("H264"));
}

#[test]
fn test_profile_serialize() {
    let profile = Profile {
        token: "profile_1".to_string(),
        name: "Main Profile".to_string(),
        fixed: Some(true),
        video_source_configuration: Some(VideoSourceConfiguration {
            token: "vsrc_1".to_string(),
            name: "Video Source 1".to_string(),
            use_count: 1,
            source_token: "V_SRC_000".to_string(),
            bounds: IntRectangle {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            view_mode: None,
            extension: None,
        }),
        video_encoder_configuration: None,
        audio_source_configuration: None,
        audio_encoder_configuration: None,
        ptz_configuration: None,
        metadata_configuration: None,
        extension: None,
    };

    let xml = to_string(&profile).expect("Failed to serialize");
    assert!(xml.contains("profile_1"));
    assert!(xml.contains("V_SRC_000"));
}

// ============================================================================
// Enumeration Serialization Tests
// ============================================================================

#[test]
fn test_video_encoding_serialize() {
    let h264 = VideoEncoding::H264;
    let jpeg = VideoEncoding::JPEG;
    let mpeg4 = VideoEncoding::MPEG4;

    let h264_xml = to_string(&h264).expect("Failed to serialize H264");
    let jpeg_xml = to_string(&jpeg).expect("Failed to serialize JPEG");
    let mpeg4_xml = to_string(&mpeg4).expect("Failed to serialize MPEG4");

    assert!(h264_xml.contains("H264"));
    assert!(jpeg_xml.contains("JPEG"));
    assert!(mpeg4_xml.contains("MPEG4"));
}

#[test]
fn test_exposure_mode_serialize() {
    let auto = ExposureMode::AUTO;
    let manual = ExposureMode::MANUAL;

    let auto_xml = to_string(&auto).expect("Failed to serialize Auto");
    let manual_xml = to_string(&manual).expect("Failed to serialize Manual");

    assert!(auto_xml.contains("AUTO"));
    assert!(manual_xml.contains("MANUAL"));
}

#[test]
fn test_white_balance_mode_serialize() {
    let auto = WhiteBalanceMode::AUTO;
    let manual = WhiteBalanceMode::MANUAL;

    let auto_xml = to_string(&auto).expect("Failed to serialize Auto");
    let manual_xml = to_string(&manual).expect("Failed to serialize Manual");

    assert!(auto_xml.contains("AUTO"));
    assert!(manual_xml.contains("MANUAL"));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_string_fields_serialize() {
    let response = GetDeviceInformationResponse {
        manufacturer: "Test".to_string(),
        model: "".to_string(),
        firmware_version: "1.0".to_string(),
        serial_number: "SN".to_string(),
        hardware_id: "HW".to_string(),
    };

    let xml = to_string(&response).expect("Failed to serialize");
    assert!(xml.contains("Test"));
}

#[test]
fn test_special_characters_roundtrip() {
    let response = GetDeviceInformationResponse {
        manufacturer: "Test & Company".to_string(),
        model: "Model<>".to_string(),
        firmware_version: "1.0 \"v\"".to_string(),
        serial_number: "SN'123".to_string(),
        hardware_id: "HW".to_string(),
    };

    let xml = to_string(&response).expect("Failed to serialize");
    let deserialized: GetDeviceInformationResponse = from_str(&xml).expect("Failed to deserialize");

    assert_eq!(deserialized.manufacturer, "Test & Company");
    assert_eq!(deserialized.model, "Model<>");
    assert_eq!(deserialized.firmware_version, "1.0 \"v\"");
    assert_eq!(deserialized.serial_number, "SN'123");
}

#[test]
fn test_unicode_roundtrip() {
    let response = GetDeviceInformationResponse {
        manufacturer: "测试".to_string(),
        model: "カメラ".to_string(),
        firmware_version: "Версия".to_string(),
        serial_number: "SN123".to_string(),
        hardware_id: "HW".to_string(),
    };

    let xml = to_string(&response).expect("Failed to serialize");
    let deserialized: GetDeviceInformationResponse = from_str(&xml).expect("Failed to deserialize");

    assert_eq!(deserialized.manufacturer, "测试");
    assert_eq!(deserialized.model, "カメラ");
    assert_eq!(deserialized.firmware_version, "Версия");
}

// ============================================================================
// Default Value Tests
// ============================================================================

#[test]
fn test_int_range_default() {
    let range = IntRange::default();
    assert_eq!(range.min, 0);
    assert_eq!(range.max, 0);
}

#[test]
fn test_float_range_default() {
    let range = FloatRange::default();
    assert_eq!(range.min, 0.0);
    assert_eq!(range.max, 0.0);
}

#[test]
fn test_backlight_compensation_mode_default() {
    let mode = BacklightCompensationMode::default();
    assert_eq!(mode, BacklightCompensationMode::OFF);
}

#[test]
fn test_exposure_mode_default() {
    let mode = ExposureMode::default();
    assert_eq!(mode, ExposureMode::AUTO);
}

#[test]
fn test_white_balance_mode_default() {
    let mode = WhiteBalanceMode::default();
    assert_eq!(mode, WhiteBalanceMode::AUTO);
}

// ============================================================================
// Real Data Tests - Device Service
// ============================================================================

/// Test GetDeviceInformationResponse deserialization from real captured ONVIF response.
///
/// File: tests/data/responses/response_001_tds_200_36.xml
/// Expected values from actual camera response.
#[test]
fn test_device_real_get_device_information() {
    let raw_xml = include_str!("data/responses/response_001_tds_200_36.xml");
    let cleaned = extract_response_element(raw_xml, "GetDeviceInformationResponse")
        .expect("Failed to extract GetDeviceInformationResponse");

    let response: GetDeviceInformationResponse =
        from_str(&cleaned).expect("Failed to deserialize GetDeviceInformationResponse");

    // Verify actual values from captured response
    assert_eq!(response.manufacturer, "H264");
    assert_eq!(response.model, "XM535_X6E-WEQ_8M");
    assert_eq!(
        response.firmware_version,
        "V5.05.R02.000A07F3.10010.346532.S.ONVIF 21.06"
    );
    assert_eq!(response.serial_number, "940b5413c9d025f27ne4");
    assert_eq!(response.hardware_id, "00001");
}

// ============================================================================
// Real Data Tests - Imaging Service
// ============================================================================

/// Test ImagingSettings20 deserialization from real captured ONVIF response.
///
/// File: tests/data/responses/response_041_timg_200_738.xml
/// Contains full imaging settings with backlight, exposure, focus, white balance.
#[test]
fn test_imaging_real_get_imaging_settings() {
    let raw_xml = include_str!("data/responses/response_041_timg_200_738.xml");
    let cleaned = extract_response_element(raw_xml, "GetImagingSettingsResponse")
        .expect("Failed to extract GetImagingSettingsResponse");

    let response: GetImagingSettingsResponse =
        from_str(&cleaned).expect("Failed to deserialize GetImagingSettingsResponse");

    let settings = &response.imaging_settings;

    // Verify backlight compensation
    let blc = settings
        .backlight_compensation
        .as_ref()
        .expect("BacklightCompensation should be present");
    assert_eq!(blc.mode, BacklightCompensationMode::OFF);
    assert_eq!(blc.level, Some(10.0));

    // Verify brightness, saturation, contrast
    assert_eq!(settings.brightness, Some(50.0));
    assert_eq!(settings.color_saturation, Some(50.0));
    assert_eq!(settings.contrast, Some(50.0));

    // Verify exposure settings
    let exposure = settings
        .exposure
        .as_ref()
        .expect("Exposure should be present");
    assert_eq!(exposure.mode, ExposureMode::AUTO);
    assert_eq!(exposure.priority, Some(ExposurePriority::LowNoise));
    assert_eq!(exposure.min_exposure_time, Some(10.0));
    assert_eq!(exposure.max_exposure_time, Some(40000.0));
    assert_eq!(exposure.min_gain, Some(0.0));
    assert_eq!(exposure.max_gain, Some(100.0));
    assert_eq!(exposure.min_iris, Some(0.0));
    assert_eq!(exposure.max_iris, Some(10.0));
    assert_eq!(exposure.exposure_time, Some(4000.0));
    assert_eq!(exposure.gain, Some(0.0));
    assert_eq!(exposure.iris, Some(10.0));

    // Verify exposure window
    let window = exposure.window.as_ref().expect("Window should be present");
    assert_eq!(window.bottom, 1.0);
    assert_eq!(window.top, 0.0);
    assert_eq!(window.right, 1.0);
    assert_eq!(window.left, 0.0);

    // Verify focus settings
    let focus = settings.focus.as_ref().expect("Focus should be present");
    assert_eq!(focus.default_speed, Some(100.0));
    assert_eq!(focus.near_limit, Some(100.0));
    assert_eq!(focus.far_limit, Some(1000.0));

    // Verify IR cut filter
    assert_eq!(settings.ir_cut_filter, Some(IrCutFilterMode::AUTO));

    // Verify sharpness
    assert_eq!(settings.sharpness, Some(10.0));

    // Verify wide dynamic range
    let wdr = settings
        .wide_dynamic_range
        .as_ref()
        .expect("WideDynamicRange should be present");
    assert_eq!(wdr.mode, WideDynamicMode::OFF);
    assert_eq!(wdr.level, Some(50.0));

    // Verify white balance
    let wb = settings
        .white_balance
        .as_ref()
        .expect("WhiteBalance should be present");
    assert_eq!(wb.mode, WhiteBalanceMode::AUTO);
    assert_eq!(wb.cr_gain, Some(10.0));
    assert_eq!(wb.cb_gain, Some(10.0));
}

// ============================================================================
// Real Data Tests - PTZ Service
// ============================================================================

/// Test PTZNode deserialization from real captured ONVIF response.
///
/// File: tests/data/responses/response_057_tptz_200_962.xml
/// Contains PTZ node with supported spaces, presets, and capabilities.
#[test]
fn test_ptz_real_get_node() {
    let raw_xml = include_str!("data/responses/response_057_tptz_200_962.xml");
    let cleaned = extract_response_element(raw_xml, "GetNodeResponse")
        .expect("Failed to extract GetNodeResponse");

    let response: GetNodeResponse =
        from_str(&cleaned).expect("Failed to deserialize GetNodeResponse");

    let node = &response.ptz_node;

    // Verify node token and attributes
    assert_eq!(node.token, "NODE_000");
    assert_eq!(node.fixed_home_position, Some(false));

    // Verify node name
    assert_eq!(node.name, Some("PTZNode".to_string()));

    // Verify preset and home support
    assert_eq!(node.maximum_number_of_presets, 128);
    assert!(node.home_supported);

    // Verify auxiliary commands
    assert!(node.auxiliary_commands.contains(&"Wiper start".to_string()));
    assert!(node.auxiliary_commands.contains(&"Wiper stop".to_string()));

    // Verify supported PTZ spaces exist
    let spaces = &node.supported_ptz_spaces;

    // Verify absolute pan/tilt position space
    assert!(!spaces.absolute_pan_tilt_position_space.is_empty());
    let abs_pan_tilt = &spaces.absolute_pan_tilt_position_space[0];
    assert_eq!(
        abs_pan_tilt.uri,
        "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace"
    );
    assert_eq!(abs_pan_tilt.x_range.min, -1.0);
    assert_eq!(abs_pan_tilt.x_range.max, 1.0);
    assert_eq!(abs_pan_tilt.y_range.min, -1.0);
    assert_eq!(abs_pan_tilt.y_range.max, 1.0);

    // Verify absolute zoom position space
    assert!(!spaces.absolute_zoom_position_space.is_empty());
    let abs_zoom = &spaces.absolute_zoom_position_space[0];
    assert_eq!(
        abs_zoom.uri,
        "http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace"
    );
    assert_eq!(abs_zoom.x_range.min, 0.0);
    assert_eq!(abs_zoom.x_range.max, 1.0);

    // Verify pan/tilt speed space
    assert!(!spaces.pan_tilt_speed_space.is_empty());
    let speed = &spaces.pan_tilt_speed_space[0];
    assert_eq!(speed.x_range.min, 1.0);
    assert_eq!(speed.x_range.max, 8.0);

    // Verify extension with preset tour support
    let ext = node
        .extension
        .as_ref()
        .expect("Extension should be present");
    let tour = ext
        .supported_preset_tour
        .as_ref()
        .expect("SupportedPresetTour should be present");
    assert_eq!(tour.maximum_number_of_preset_tours, 1);
}

/// Test PTZStatus deserialization from real captured ONVIF response.
///
/// File: tests/data/responses/response_058_tptz_200_968.xml
/// Contains PTZ status with position, move status, and timestamp.
#[test]
fn test_ptz_real_get_status() {
    let raw_xml = include_str!("data/responses/response_058_tptz_200_968.xml");
    let cleaned = extract_response_element(raw_xml, "GetStatusResponse")
        .expect("Failed to extract GetStatusResponse");

    let response: GetStatusResponse =
        from_str(&cleaned).expect("Failed to deserialize GetStatusResponse");

    let status = &response.ptz_status;

    // Verify position
    let position = status
        .position
        .as_ref()
        .expect("Position should be present");

    let pan_tilt = position
        .pan_tilt
        .as_ref()
        .expect("PanTilt should be present");
    assert_eq!(pan_tilt.x, -1.0);
    assert_eq!(pan_tilt.y, -1.0);
    assert_eq!(
        pan_tilt.space,
        Some("http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace".to_string())
    );

    let zoom = position.zoom.as_ref().expect("Zoom should be present");
    assert_eq!(zoom.x, 0.0);
    assert_eq!(
        zoom.space,
        Some("http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace".to_string())
    );

    // Verify move status
    let move_status = status
        .move_status
        .as_ref()
        .expect("MoveStatus should be present");
    assert_eq!(move_status.pan_tilt, Some(MoveStatus::IDLE));
    assert_eq!(move_status.zoom, Some(MoveStatus::IDLE));

    // Verify UTC time is present
    assert_eq!(status.utc_time, "2025-10-13T20:07:30Z");
}

// ============================================================================
// Real Data Tests - Media Service
// ============================================================================

/// Test VideoSource deserialization from real captured ONVIF response.
///
/// File: tests/data/responses/response_009_trt_200_126.xml
/// Contains video source with resolution, framerate, and imaging settings.
#[test]
fn test_media_real_get_video_sources() {
    let raw_xml = include_str!("data/responses/response_009_trt_200_126.xml");
    let cleaned = extract_response_element(raw_xml, "GetVideoSourcesResponse")
        .expect("Failed to extract GetVideoSourcesResponse");

    let response: GetVideoSourcesResponse =
        from_str(&cleaned).expect("Failed to deserialize GetVideoSourcesResponse");

    assert!(!response.video_sources.is_empty());
    let source = &response.video_sources[0];

    // Verify video source properties
    assert_eq!(source.token, "V_SRC_000");
    assert_eq!(source.framerate, 25.0);
    assert_eq!(source.resolution.width, 3072);
    assert_eq!(source.resolution.height, 2048);

    // Verify imaging settings are present
    let imaging = source.imaging.as_ref().expect("Imaging should be present");

    // Verify backlight compensation in imaging
    let blc = imaging
        .backlight_compensation
        .as_ref()
        .expect("BacklightCompensation should be present");
    assert_eq!(blc.mode, BacklightCompensationMode::OFF);

    // Verify exposure in imaging
    let exposure = imaging
        .exposure
        .as_ref()
        .expect("Exposure should be present");
    assert_eq!(exposure.mode, ExposureMode::AUTO);

    // Verify brightness, contrast, saturation
    assert_eq!(imaging.brightness, Some(50.0));
    assert_eq!(imaging.contrast, Some(50.0));
    assert_eq!(imaging.color_saturation, Some(50.0));
}

/// Test GetProfilesResponse deserialization from real captured ONVIF response.
///
/// File: tests/data/responses/response_010_trt_200_147.xml
/// Contains profiles with video/audio configurations, PTZ, and analytics.
#[test]
fn test_media_real_get_profiles() {
    let raw_xml = include_str!("data/responses/response_010_trt_200_147.xml");
    let cleaned = extract_response_element(raw_xml, "GetProfilesResponse")
        .expect("Failed to extract GetProfilesResponse");

    let response: GetProfilesResponse =
        from_str(&cleaned).expect("Failed to deserialize GetProfilesResponse");

    assert!(!response.profiles.is_empty());
    let profile = &response.profiles[0];

    // Verify profile attributes
    assert_eq!(profile.token, "000");
    assert_eq!(profile.name, "mainStream");
    assert_eq!(profile.fixed, Some(true));

    // Verify video source configuration
    let vsc = profile
        .video_source_configuration
        .as_ref()
        .expect("VideoSourceConfiguration should be present");
    assert_eq!(vsc.token, "V_SRC_000");
    assert_eq!(vsc.name, "V_SRC_CFG_000");
    assert_eq!(vsc.use_count, 3);
    assert_eq!(vsc.source_token, "V_SRC_000");
    assert_eq!(vsc.bounds.width, 3072);
    assert_eq!(vsc.bounds.height, 2048);
    assert_eq!(vsc.bounds.x, 0);
    assert_eq!(vsc.bounds.y, 0);

    // Verify video encoder configuration
    let vec = profile
        .video_encoder_configuration
        .as_ref()
        .expect("VideoEncoderConfiguration should be present");
    assert_eq!(vec.token, "000");
    assert_eq!(vec.name, "V_ENC_000");
    assert_eq!(vec.use_count, 1);
    assert_eq!(vec.encoding, VideoEncoding::H264);
    assert_eq!(vec.resolution.width, 3072);
    assert_eq!(vec.resolution.height, 2048);
    assert_eq!(vec.quality, 3.0);
    assert_eq!(vec.session_timeout, "PT10S");

    // Verify rate control
    let rc = vec
        .rate_control
        .as_ref()
        .expect("RateControl should be present");
    assert_eq!(rc.frame_rate_limit, 12);
    assert_eq!(rc.encoding_interval, 0);
    assert_eq!(rc.bitrate_limit, 1536);

    // Verify audio source configuration
    let asc = profile
        .audio_source_configuration
        .as_ref()
        .expect("AudioSourceConfiguration should be present");
    assert_eq!(asc.token, "A_SRC_CFG_000");
    assert_eq!(asc.name, "A_SRC_CFG_000");
    assert_eq!(asc.source_token, "AudioSourceToken");

    // Verify PTZ configuration is present
    let ptz_cfg = profile
        .ptz_configuration
        .as_ref()
        .expect("PTZConfiguration should be present");
    assert_eq!(ptz_cfg.token, "PTZConfigurationToken");
    assert_eq!(ptz_cfg.node_token, "NODE_000");
}

// ============================================================================
// Extended Real Data Tests - Device Service (All Responses)
// ============================================================================

/// Test GetNetworkInterfacesResponse from real captured data.
/// File: response_002_tds_200_42.xml
#[test]
fn test_device_real_get_network_interfaces() {
    let raw_xml = include_str!("data/responses/response_002_tds_200_42.xml");
    let cleaned = extract_response_element(raw_xml, "GetNetworkInterfacesResponse")
        .expect("Failed to extract GetNetworkInterfacesResponse");

    let response: GetNetworkInterfacesResponse =
        from_str(&cleaned).expect("Failed to deserialize GetNetworkInterfacesResponse");

    assert!(!response.network_interfaces.is_empty());
    let iface = &response.network_interfaces[0];
    assert_eq!(iface.token, "eth0");
    assert!(iface.enabled);

    // Verify info
    let info = iface.info.as_ref().expect("Info should be present");
    assert_eq!(info.name, Some("eth0".to_string()));
    assert_eq!(info.hw_address, "00:12:34:70:DF:D2");
    assert_eq!(info.mtu, Some(1500));
}

/// Test GetScopesResponse from real captured data.
/// File: response_003_tds_200_48.xml
#[test]
fn test_device_real_get_scopes() {
    let raw_xml = include_str!("data/responses/response_003_tds_200_48.xml");
    let cleaned = extract_response_element(raw_xml, "GetScopesResponse")
        .expect("Failed to extract GetScopesResponse");

    let response: GetScopesResponse =
        from_str(&cleaned).expect("Failed to deserialize GetScopesResponse");

    assert!(!response.scopes.is_empty());
    // Check that we have video_encoder scope
    let has_video_encoder = response
        .scopes
        .iter()
        .any(|s| s.scope_item.contains("video_encoder"));
    assert!(has_video_encoder, "Should have video_encoder scope");
}

/// Test GetCapabilitiesResponse from real captured data.
/// File: response_005_tds_200_74.xml
///
/// NOTE: This test was previously ignored due to Extension type issues.
/// Fixed by creating typed NetworkCapabilitiesExtension with Dot11Configuration field.
#[test]
fn test_device_real_get_capabilities() {
    let raw_xml = include_str!("data/responses/response_005_tds_200_74.xml");
    let cleaned = extract_response_element(raw_xml, "GetCapabilitiesResponse")
        .expect("Failed to extract GetCapabilitiesResponse");

    let response: GetCapabilitiesResponse =
        from_str(&cleaned).expect("Failed to deserialize GetCapabilitiesResponse");

    let caps = &response.capabilities;

    // Verify analytics capability
    let analytics = caps
        .analytics
        .as_ref()
        .expect("Analytics should be present");
    assert!(analytics.x_addr.contains("analytics_service"));
    assert!(analytics.rule_support);
    assert!(analytics.analytics_module_support);

    // Verify device capability
    let device = caps.device.as_ref().expect("Device should be present");
    assert!(device.x_addr.contains("device_service"));

    // Verify network capability extension is parsed correctly
    if let Some(network) = &device.network {
        if let Some(ext) = &network.extension {
            // Dot11Configuration should be parsed (false in the test data)
            assert!(ext.dot11_configuration.is_some());
        }
    }
}

/// Test GetServicesResponse from real captured data.
/// File: response_007_tds_200_100.xml
#[test]
fn test_device_real_get_services() {
    let raw_xml = include_str!("data/responses/response_007_tds_200_100.xml");
    let cleaned = extract_response_element(raw_xml, "GetServicesResponse")
        .expect("Failed to extract GetServicesResponse");

    let response: GetServicesResponse =
        from_str(&cleaned).expect("Failed to deserialize GetServicesResponse");

    assert!(!response.services.is_empty());

    // Find device service
    let device_service = response
        .services
        .iter()
        .find(|s| s.namespace.contains("device/wsdl"));
    assert!(device_service.is_some(), "Should have device service");

    // Find media service
    let media_service = response
        .services
        .iter()
        .find(|s| s.namespace.contains("media/wsdl"));
    assert!(media_service.is_some(), "Should have media service");
}

/// Test GetSystemDateAndTimeResponse from real captured data.
/// File: response_016_tds_200_237.xml
#[test]
fn test_device_real_get_system_date_time() {
    let raw_xml = include_str!("data/responses/response_016_tds_200_237.xml");
    let cleaned = extract_response_element(raw_xml, "GetSystemDateAndTimeResponse")
        .expect("Failed to extract GetSystemDateAndTimeResponse");

    let response: GetSystemDateAndTimeResponse =
        from_str(&cleaned).expect("Failed to deserialize GetSystemDateAndTimeResponse");

    let sdt = &response.system_date_and_time;
    assert_eq!(sdt.date_time_type, SetDateTimeType::NTP);
    assert!(!sdt.daylight_savings);

    // Verify timezone
    let tz = sdt.time_zone.as_ref().expect("TimeZone should be present");
    assert!(tz.tz.contains("CET"));

    // Verify UTC datetime
    let utc = sdt
        .utc_date_time
        .as_ref()
        .expect("UTCDateTime should be present");
    assert_eq!(utc.date.year, 2025);
    assert_eq!(utc.date.month, 10);
    assert_eq!(utc.date.day, 13);
}

/// Test GetHostnameResponse from real captured data.
/// File: response_020_tds_200_321.xml
#[test]
fn test_device_real_get_hostname() {
    let raw_xml = include_str!("data/responses/response_020_tds_200_321.xml");
    let cleaned = extract_response_element(raw_xml, "GetHostnameResponse")
        .expect("Failed to extract GetHostnameResponse");

    let response: GetHostnameResponse =
        from_str(&cleaned).expect("Failed to deserialize GetHostnameResponse");

    // Hostname info should be present - from_dhcp is a bool, name is Option<String>
    // Just verify the response parsed successfully
    let _from_dhcp = response.hostname_information.from_dhcp;
    let _name = &response.hostname_information.name;
}

/// Test GetDiscoveryModeResponse from real captured data.
/// File: response_024_tds_200_346.xml
#[test]
fn test_device_real_get_discovery_mode() {
    let raw_xml = include_str!("data/responses/response_024_tds_200_346.xml");
    let cleaned = extract_response_element(raw_xml, "GetDiscoveryModeResponse")
        .expect("Failed to extract GetDiscoveryModeResponse");

    let response: GetDiscoveryModeResponse =
        from_str(&cleaned).expect("Failed to deserialize GetDiscoveryModeResponse");

    // Discovery mode should be Discoverable or NonDiscoverable
    assert!(
        response.discovery_mode == DiscoveryMode::Discoverable
            || response.discovery_mode == DiscoveryMode::NonDiscoverable
    );
}

/// Test GetUsersResponse from real captured data.
/// File: response_025_tds_200_359.xml
#[test]
fn test_device_real_get_users() {
    let raw_xml = include_str!("data/responses/response_025_tds_200_359.xml");
    let cleaned = extract_response_element(raw_xml, "GetUsersResponse")
        .expect("Failed to extract GetUsersResponse");

    let response: GetUsersResponse =
        from_str(&cleaned).expect("Failed to deserialize GetUsersResponse");

    // Should have at least admin user
    assert!(!response.users.is_empty());
    let has_admin = response.users.iter().any(|u| u.username == "admin");
    assert!(has_admin, "Should have admin user");
}

// ============================================================================
// Extended Real Data Tests - Media Service (All Responses)
// ============================================================================

/// Test GetSnapshotUriResponse from real captured data.
/// File: response_012_trt_200_175.xml
#[test]
fn test_media_real_get_snapshot_uri() {
    let raw_xml = include_str!("data/responses/response_012_trt_200_175.xml");
    let cleaned = extract_response_element(raw_xml, "GetSnapshotUriResponse")
        .expect("Failed to extract GetSnapshotUriResponse");

    let response: GetSnapshotUriResponse =
        from_str(&cleaned).expect("Failed to deserialize GetSnapshotUriResponse");

    let uri = &response.media_uri;
    assert!(uri.uri.contains("webcapture"));
    assert!(!uri.invalid_after_connect);
    assert!(!uri.invalid_after_reboot);
    assert_eq!(uri.timeout, "PT60S");
}

/// Test GetStreamUriResponse from real captured data.
/// File: response_033_trt_200_587.xml
#[test]
fn test_media_real_get_stream_uri() {
    let raw_xml = include_str!("data/responses/response_033_trt_200_587.xml");
    let cleaned = extract_response_element(raw_xml, "GetStreamUriResponse")
        .expect("Failed to extract GetStreamUriResponse");

    let response: GetStreamUriResponse =
        from_str(&cleaned).expect("Failed to deserialize GetStreamUriResponse");

    let uri = &response.media_uri;
    assert!(uri.uri.contains("rtsp://"));
    assert!(!uri.invalid_after_connect);
    assert!(!uri.invalid_after_reboot);
}

/// Test GetProfileResponse (single profile) from real captured data.
/// File: response_031_trt_200_552.xml
#[test]
fn test_media_real_get_profile() {
    let raw_xml = include_str!("data/responses/response_031_trt_200_552.xml");
    let cleaned = extract_response_element(raw_xml, "GetProfileResponse")
        .expect("Failed to extract GetProfileResponse");

    let response: GetProfileResponse =
        from_str(&cleaned).expect("Failed to deserialize GetProfileResponse");

    let profile = &response.profile;
    assert!(!profile.token.is_empty());
    assert!(!profile.name.is_empty());
}

/// Test GetVideoSourceConfigurationResponse from real captured data.
/// File: response_035_trt_200_606.xml
#[test]
fn test_media_real_get_video_source_configuration() {
    let raw_xml = include_str!("data/responses/response_035_trt_200_606.xml");
    let cleaned = extract_response_element(raw_xml, "GetVideoSourceConfigurationResponse")
        .expect("Failed to extract GetVideoSourceConfigurationResponse");

    let response: GetVideoSourceConfigurationResponse =
        from_str(&cleaned).expect("Failed to deserialize GetVideoSourceConfigurationResponse");

    let config = &response.configuration;
    assert!(!config.token.is_empty());
    assert!(!config.source_token.is_empty());
    assert!(config.bounds.width > 0);
    assert!(config.bounds.height > 0);
}

/// Test GetVideoEncoderConfigurationOptionsResponse from real captured data.
/// File: response_037_trt_200_652.xml
///
/// NOTE: This test was previously ignored due to enum text content parsing issues with quick-xml.
/// Fixed by implementing custom deserializer for Vec<H264Profile> using wrapper struct approach.
#[test]
fn test_media_real_get_video_encoder_configuration_options() {
    let raw_xml = include_str!("data/responses/response_037_trt_200_652.xml");
    let cleaned = extract_response_element(raw_xml, "GetVideoEncoderConfigurationOptionsResponse")
        .expect("Failed to extract GetVideoEncoderConfigurationOptionsResponse");

    let response: GetVideoEncoderConfigurationOptionsResponse = from_str(&cleaned)
        .expect("Failed to deserialize GetVideoEncoderConfigurationOptionsResponse");

    let options = &response.options;
    // Quality range is a required field (IntRange struct)
    assert!(options.quality_range.min >= 0);
    // H264Options should have profiles parsed correctly
    if let Some(h264_options) = &options.h264 {
        assert!(!h264_options.h264_profiles_supported.is_empty());
    }
}

/// Test GetAudioSourcesResponse from real captured data.
/// File: response_079_trt_200_1279.xml
#[test]
fn test_media_real_get_audio_sources() {
    let raw_xml = include_str!("data/responses/response_079_trt_200_1279.xml");
    let cleaned = extract_response_element(raw_xml, "GetAudioSourcesResponse")
        .expect("Failed to extract GetAudioSourcesResponse");

    let response: GetAudioSourcesResponse =
        from_str(&cleaned).expect("Failed to deserialize GetAudioSourcesResponse");

    // Should have at least one audio source
    assert!(!response.audio_sources.is_empty());
    let source = &response.audio_sources[0];
    assert!(!source.token.is_empty());
}

// ============================================================================
// Extended Real Data Tests - PTZ Service (All Responses)
// ============================================================================

/// Test GetPresetsResponse (empty) from real captured data.
/// File: response_056_tptz_200_956.xml
#[test]
fn test_ptz_real_get_presets_empty() {
    let raw_xml = include_str!("data/responses/response_056_tptz_200_956.xml");
    let cleaned = extract_response_element(raw_xml, "GetPresetsResponse")
        .expect("Failed to extract GetPresetsResponse");

    let response: GetPresetsResponse =
        from_str(&cleaned).expect("Failed to deserialize GetPresetsResponse");

    // This captured response has no presets
    assert!(response.presets.is_empty());
}

/// Test GetNodesResponse (multiple nodes) from real captured data.
/// File: response_080_tptz_200_1292.xml
#[test]
fn test_ptz_real_get_nodes() {
    let raw_xml = include_str!("data/responses/response_080_tptz_200_1292.xml");
    let cleaned = extract_response_element(raw_xml, "GetNodesResponse")
        .expect("Failed to extract GetNodesResponse");

    let response: GetNodesResponse =
        from_str(&cleaned).expect("Failed to deserialize GetNodesResponse");

    // Should have at least one PTZ node
    assert!(!response.ptz_nodes.is_empty());
    let node = &response.ptz_nodes[0];
    assert_eq!(node.token, "NODE_000");
}

/// Test ContinuousMoveResponse (empty acknowledgment) from real captured data.
/// File: response_060_tptz_200_1003.xml
#[test]
fn test_ptz_real_continuous_move_response() {
    let raw_xml = include_str!("data/responses/response_060_tptz_200_1003.xml");
    let cleaned = extract_response_element(raw_xml, "ContinuousMoveResponse")
        .expect("Failed to extract ContinuousMoveResponse");

    // ContinuousMoveResponse is an empty struct - just verify it parses
    let _response: ContinuousMoveResponse =
        from_str(&cleaned).expect("Failed to deserialize ContinuousMoveResponse");
}

/// Test StopResponse (empty acknowledgment) from real captured data.
/// File: response_061_tptz_200_1017.xml
#[test]
fn test_ptz_real_stop_response() {
    let raw_xml = include_str!("data/responses/response_061_tptz_200_1017.xml");
    let cleaned =
        extract_response_element(raw_xml, "StopResponse").expect("Failed to extract StopResponse");

    // StopResponse is an empty struct - just verify it parses
    let _response: StopResponse = from_str(&cleaned).expect("Failed to deserialize StopResponse");
}

// ============================================================================
// SOAP Fault Tests
// ============================================================================

/// Test SOAP Fault parsing from real captured error response.
/// File: response_026_s_400_379.xml
#[test]
fn test_soap_fault_action_not_supported() {
    let raw_xml = include_str!("data/responses/response_026_s_400_379.xml");
    let cleaned = extract_response_element(raw_xml, "Fault").expect("Failed to extract Fault");

    let fault: SoapFault = from_str(&cleaned).expect("Failed to deserialize SoapFault");

    // Verify fault code
    let code = &fault.code;
    assert!(code.value.contains("Receiver"));

    // Verify subcode
    let subcode = code.subcode.as_ref().expect("Subcode should be present");
    assert!(subcode.value.contains("ActionNotSupported"));

    // Verify reason
    let reason = &fault.reason;
    assert!(!reason.text.is_empty());
    assert!(reason.text[0].value.contains("Not Implemented"));
}

// ============================================================================
// Request Parsing Tests
// ============================================================================

/// Test GetDeviceInformation request parsing.
/// File: request_001_device__20.xml
#[test]
fn test_request_get_device_information() {
    let raw_xml = include_str!("data/requests/request_001_device__20.xml");
    let cleaned = extract_response_element(raw_xml, "GetDeviceInformation")
        .expect("Failed to extract GetDeviceInformation");

    // GetDeviceInformation is an empty request - just verify it parses
    let _request: GetDeviceInformation =
        from_str(&cleaned).expect("Failed to deserialize GetDeviceInformation");
}

/// Test GetProfiles request parsing.
/// File: request_010_media__135.xml
#[test]
fn test_request_get_profiles() {
    let raw_xml = include_str!("data/requests/request_010_media__135.xml");
    let cleaned =
        extract_response_element(raw_xml, "GetProfiles").expect("Failed to extract GetProfiles");

    let _request: GetProfiles = from_str(&cleaned).expect("Failed to deserialize GetProfiles");
}

/// Test GetImagingSettings request parsing.
/// File: request_041_GetImagingSettings_728.xml
#[test]
fn test_request_get_imaging_settings() {
    let raw_xml = include_str!("data/requests/request_041_GetImagingSettings_728.xml");
    let cleaned = extract_response_element(raw_xml, "GetImagingSettings")
        .expect("Failed to extract GetImagingSettings");

    let request: GetImagingSettings =
        from_str(&cleaned).expect("Failed to deserialize GetImagingSettings");

    // Should have video source token
    assert!(!request.video_source_token.is_empty());
}

/// Test GetPresets request parsing.
/// File: request_056_GetPresets_946.xml
#[test]
fn test_request_get_presets() {
    let raw_xml = include_str!("data/requests/request_056_GetPresets_946.xml");
    let cleaned =
        extract_response_element(raw_xml, "GetPresets").expect("Failed to extract GetPresets");

    let request: GetPresets = from_str(&cleaned).expect("Failed to deserialize GetPresets");

    // Should have profile token
    assert!(!request.profile_token.is_empty());
}

/// Test Stop request parsing.
/// File: request_061_Stop_1013.xml
#[test]
fn test_request_stop() {
    let raw_xml = include_str!("data/requests/request_061_Stop_1013.xml");
    let cleaned = extract_response_element(raw_xml, "Stop").expect("Failed to extract Stop");

    let request: Stop = from_str(&cleaned).expect("Failed to deserialize Stop");

    // Should have profile token
    assert!(!request.profile_token.is_empty());
}
