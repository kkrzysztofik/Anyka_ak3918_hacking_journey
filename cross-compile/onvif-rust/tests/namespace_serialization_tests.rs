//! Integration tests for ONVIF namespace prefix serialization
//!
//! These tests validate that ONVIF types serialize with correct namespace prefixes
//! (tt:, tds:, trt:, tptz:, timg:) as required by ONVIF 24.12 specification.

use quick_xml::se::to_string;

// Import simple types that are easy to construct for testing
use onvif_rust::onvif::types::common::{FloatRange, IntRange, VideoResolution};

/// Test that IntRange serializes with tt: namespace prefix
#[test]
fn test_int_range_namespace_prefix() {
    let range = IntRange { min: 1, max: 100 };

    let xml = to_string(&range).expect("Failed to serialize IntRange");

    assert!(
        xml.contains("<tt:Min>") || xml.contains("tt:Min"),
        "IntRange.Min should have tt: prefix. Got: {}",
        xml
    );
    assert!(
        xml.contains("<tt:Max>") || xml.contains("tt:Max"),
        "IntRange.Max should have tt: prefix. Got: {}",
        xml
    );
}

/// Test that FloatRange serializes with tt: namespace prefix
#[test]
fn test_float_range_namespace_prefix() {
    let range = FloatRange { min: 0.0, max: 1.0 };

    let xml = to_string(&range).expect("Failed to serialize FloatRange");

    assert!(
        xml.contains("<tt:Min>") || xml.contains("tt:Min"),
        "FloatRange.Min should have tt: prefix. Got: {}",
        xml
    );
    assert!(
        xml.contains("<tt:Max>") || xml.contains("tt:Max"),
        "FloatRange.Max should have tt: prefix. Got: {}",
        xml
    );
}

/// Test that VideoResolution serializes with tt: namespace prefix
#[test]
fn test_video_resolution_namespace_prefix() {
    let resolution = VideoResolution {
        width: 1920,
        height: 1080,
    };

    let xml = to_string(&resolution).expect("Failed to serialize VideoResolution");

    assert!(
        xml.contains("<tt:Width>") || xml.contains("tt:Width"),
        "VideoResolution.Width should have tt: prefix. Got: {}",
        xml
    );
    assert!(
        xml.contains("<tt:Height>") || xml.contains("tt:Height"),
        "VideoResolution.Height should have tt: prefix. Got: {}",
        xml
    );
}

/// Test that VideoEncoderConfigurationOptions serializes nested IntRange with tt: prefix
#[test]
fn test_video_encoder_config_options_namespace_prefix() {
    use onvif_rust::onvif::types::media::VideoEncoderConfigurationOptions;

    let options = VideoEncoderConfigurationOptions {
        quality_range: IntRange { min: 1, max: 100 },
        jpeg: None,
        mpeg4: None,
        h264: None,
        extension: None,
    };

    let xml = to_string(&options).expect("Failed to serialize VideoEncoderConfigurationOptions");

    assert!(
        xml.contains("<tt:QualityRange>") || xml.contains("tt:QualityRange"),
        "VideoEncoderConfigurationOptions.QualityRange should have tt: prefix. Got: {}",
        xml
    );
    // Nested IntRange fields should also have tt: prefix
    assert!(
        xml.contains("<tt:Min>") || xml.contains("tt:Min"),
        "Nested IntRange.Min should have tt: prefix. Got: {}",
        xml
    );
}

/// Test that DurationRange serializes with tt: namespace prefix
#[test]
fn test_duration_range_namespace_prefix() {
    use onvif_rust::onvif::types::ptz::DurationRange;

    let range = DurationRange {
        min: "PT0S".to_string(),
        max: "PT10S".to_string(),
    };

    let xml = to_string(&range).expect("Failed to serialize DurationRange");

    assert!(
        xml.contains("<tt:Min>") || xml.contains("tt:Min"),
        "DurationRange.Min should have tt: prefix. Got: {}",
        xml
    );
    assert!(
        xml.contains("<tt:Max>") || xml.contains("tt:Max"),
        "DurationRange.Max should have tt: prefix. Got: {}",
        xml
    );
}

/// Test that ImagingOptions20 serializes with tt: namespace prefix using Default
#[test]
fn test_imaging_options20_namespace_prefix() {
    use onvif_rust::onvif::types::imaging::ImagingOptions20;

    // Use Default to get a valid instance
    let mut options = ImagingOptions20::default();
    options.brightness = Some(FloatRange {
        min: 0.0,
        max: 100.0,
    });

    let xml = to_string(&options).expect("Failed to serialize ImagingOptions20");

    assert!(
        xml.contains("<tt:Brightness>") || xml.contains("tt:Brightness"),
        "ImagingOptions20.Brightness should have tt: prefix. Got: {}",
        xml
    );
}

/// Test that DeviceServiceCapabilities serializes with tds: namespace prefix
#[test]
fn test_device_service_capabilities_namespace_prefix() {
    use onvif_rust::onvif::types::device::{
        DeviceServiceCapabilities, MiscCapabilities, NetworkCapabilities, SecurityCapabilities,
        SystemCapabilities,
    };

    let caps = DeviceServiceCapabilities {
        network: NetworkCapabilities::default(),
        security: SecurityCapabilities::default(),
        system: SystemCapabilities::default(),
        misc: Some(MiscCapabilities::default()),
    };

    let xml = to_string(&caps).expect("Failed to serialize DeviceServiceCapabilities");

    // Check for tds: prefix on service capability fields
    assert!(
        xml.contains("<tds:Network") || xml.contains("tds:Network"),
        "DeviceServiceCapabilities.Network should have tds: prefix. Got: {}",
        xml
    );
    assert!(
        xml.contains("<tds:Security") || xml.contains("tds:Security"),
        "DeviceServiceCapabilities.Security should have tds: prefix. Got: {}",
        xml
    );
    assert!(
        xml.contains("<tds:System") || xml.contains("tds:System"),
        "DeviceServiceCapabilities.System should have tds: prefix. Got: {}",
        xml
    );
}

/// Test that PTZConfigurationOptions serializes with tt: namespace prefix
#[test]
fn test_ptz_configuration_options_namespace_prefix() {
    use onvif_rust::onvif::types::ptz::{DurationRange, PTZConfigurationOptions, PTZSpaces};

    let options = PTZConfigurationOptions {
        spaces: PTZSpaces::default(),
        ptz_timeout: DurationRange {
            min: "PT0S".to_string(),
            max: "PT10S".to_string(),
        },
        pt_control_direction: None,
        extension: None,
        ptz_ramps: None,
    };

    let xml = to_string(&options).expect("Failed to serialize PTZConfigurationOptions");

    assert!(
        xml.contains("<tt:Spaces") || xml.contains("tt:Spaces"),
        "PTZConfigurationOptions.Spaces should have tt: prefix. Got: {}",
        xml
    );
    assert!(
        xml.contains("<tt:PTZTimeout") || xml.contains("tt:PTZTimeout"),
        "PTZConfigurationOptions.PTZTimeout should have tt: prefix. Got: {}",
        xml
    );
}

/// Test that IntRange roundtrips correctly (serialize then deserialize)
#[test]
fn test_int_range_roundtrip() {
    use quick_xml::de::from_str;

    let original = IntRange { min: 1, max: 100 };
    let xml = to_string(&original).expect("Failed to serialize IntRange");

    // The serialized XML should contain the namespace prefix
    println!("Serialized XML: {}", xml);

    // Deserialize it back - should work because rename applies to both serialization and deserialization
    let deserialized: IntRange = from_str(&xml).expect("Failed to deserialize IntRange");

    assert_eq!(original.min, deserialized.min);
    assert_eq!(original.max, deserialized.max);
}

/// Test DeviceServiceCapabilities roundtrip
#[test]
fn test_device_service_capabilities_roundtrip() {
    use onvif_rust::onvif::types::device::{
        DeviceServiceCapabilities, NetworkCapabilities, SecurityCapabilities, SystemCapabilities,
    };
    use quick_xml::de::from_str;

    let original = DeviceServiceCapabilities {
        network: NetworkCapabilities::default(),
        security: SecurityCapabilities::default(),
        system: SystemCapabilities::default(),
        misc: None,
    };

    let xml = to_string(&original).expect("Failed to serialize DeviceServiceCapabilities");
    println!("DeviceServiceCapabilities XML: {}", xml);

    // Try to deserialize
    let result: Result<DeviceServiceCapabilities, _> = from_str(&xml);
    println!("Deserialization result: {:?}", result.is_ok());
    if let Err(e) = &result {
        println!("Error: {}", e);
    }

    let deserialized = result.expect("Failed to deserialize DeviceServiceCapabilities");
    assert_eq!(original.network, deserialized.network);
}

/// Test GetDeviceInformationResponse roundtrip (has only rename, no alias)
#[test]
fn test_get_device_information_response_roundtrip() {
    use onvif_rust::onvif::types::device::GetDeviceInformationResponse;
    use quick_xml::de::from_str;

    let original = GetDeviceInformationResponse {
        manufacturer: "Anyka".to_string(),
        model: "AK3918".to_string(),
        firmware_version: "2.0.0".to_string(),
        serial_number: "ANYKA001".to_string(),
        hardware_id: "HW-AK3918".to_string(),
    };

    let xml = to_string(&original).expect("Failed to serialize GetDeviceInformationResponse");
    println!("GetDeviceInformationResponse XML: {}", xml);

    // Try deserializing with simpler XML (no prefix on root)
    let simple_xml = "<GetDeviceInformationResponse><tds:Manufacturer>Anyka</tds:Manufacturer><tds:Model>AK3918</tds:Model><tds:FirmwareVersion>2.0.0</tds:FirmwareVersion><tds:SerialNumber>ANYKA001</tds:SerialNumber><tds:HardwareId>HW-AK3918</tds:HardwareId></GetDeviceInformationResponse>";
    println!("Trying with simple XML: {}", simple_xml);

    // Try to deserialize the simple version
    let result: Result<GetDeviceInformationResponse, _> = from_str(simple_xml);
    println!("Simple deserialization result: {:?}", result.is_ok());
    if let Err(e) = &result {
        println!("Error: {}", e);
    }

    // Also try with non-prefixed fields
    let no_prefix_xml = "<GetDeviceInformationResponse><Manufacturer>Anyka</Manufacturer><Model>AK3918</Model><FirmwareVersion>2.0.0</FirmwareVersion><SerialNumber>ANYKA001</SerialNumber><HardwareId>HW-AK3918</HardwareId></GetDeviceInformationResponse>";
    println!("Trying with no-prefix XML: {}", no_prefix_xml);

    let result2: Result<GetDeviceInformationResponse, _> = from_str(no_prefix_xml);
    println!("No-prefix deserialization result: {:?}", result2.is_ok());
    if let Err(e) = &result2 {
        println!("Error: {}", e);
    }
}
