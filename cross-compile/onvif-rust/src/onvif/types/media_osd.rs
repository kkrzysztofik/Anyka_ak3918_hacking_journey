//! ONVIF Media OSD types (subset of ONVIF 24.12 Media service).

use serde::{Deserialize, Serialize};

use super::common::ReferenceToken;

/// Fixed tokens for the two silicon OSD rects.
pub const OSD_TOKEN_NAME: &str = "osd_name";
pub const OSD_TOKEN_DATETIME: &str = "osd_datetime";

/// GetOSDs request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetOSDs")]
pub struct GetOSDs {
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<ReferenceToken>,
}

/// GetOSDs response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetOSDsResponse")]
pub struct GetOSDsResponse {
    #[serde(rename = "trt:OSDs", alias = "OSDs", default)]
    pub osds: Vec<OSDConfiguration>,
}

/// GetOSD request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetOSD")]
pub struct GetOSD {
    #[serde(rename = "OSDToken")]
    pub osd_token: ReferenceToken,
}

/// GetOSD response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetOSDResponse")]
pub struct GetOSDResponse {
    #[serde(rename = "trt:OSD", alias = "OSD")]
    pub osd: OSDConfiguration,
}

/// GetOSDOptions request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetOSDOptions")]
pub struct GetOSDOptions {
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<ReferenceToken>,
}

/// GetOSDOptions response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetOSDOptionsResponse")]
pub struct GetOSDOptionsResponse {
    #[serde(rename = "trt:OSDOptions", alias = "OSDOptions")]
    pub osd_options: OSDConfigurationOptions,
}

/// SetOSD request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetOSD")]
pub struct SetOSD {
    #[serde(rename = "OSD")]
    pub osd: OSDConfiguration,
}

/// SetOSD response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:SetOSDResponse")]
pub struct SetOSDResponse {}

/// CreateOSD request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "CreateOSD")]
pub struct CreateOSD {
    #[serde(rename = "OSD")]
    pub osd: OSDConfiguration,
}

/// CreateOSD response — echoes the fixed token that was enabled.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:CreateOSDResponse")]
pub struct CreateOSDResponse {
    #[serde(rename = "trt:OSDToken", alias = "OSDToken", default)]
    pub osd_token: ReferenceToken,
}

/// DeleteOSD request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "DeleteOSD")]
pub struct DeleteOSD {
    #[serde(rename = "OSDToken")]
    pub osd_token: ReferenceToken,
}

/// DeleteOSD response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:DeleteOSDResponse")]
pub struct DeleteOSDResponse {}

/// One OSD configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tt:OSDConfiguration")]
pub struct OSDConfiguration {
    #[serde(rename = "@token")]
    pub token: ReferenceToken,
    #[serde(
        rename = "tt:VideoSourceConfigurationToken",
        alias = "VideoSourceConfigurationToken"
    )]
    pub video_source_configuration_token: ReferenceToken,
    #[serde(rename = "tt:Type", alias = "Type")]
    pub osd_type: String,
    #[serde(rename = "tt:Position", alias = "Position")]
    pub position: OSDPosConfiguration,
    #[serde(
        rename = "tt:TextString",
        alias = "TextString",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub text_string: Option<OSDTextConfiguration>,
}

/// OSD position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tt:OSDPosConfiguration")]
pub struct OSDPosConfiguration {
    #[serde(rename = "tt:Type", alias = "Type")]
    pub pos_type: String,
}

/// OSD text payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tt:OSDTextConfiguration")]
pub struct OSDTextConfiguration {
    #[serde(rename = "tt:Type", alias = "Type")]
    pub text_type: String,
    #[serde(
        rename = "tt:DateFormat",
        alias = "DateFormat",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub date_format: Option<String>,
    #[serde(
        rename = "tt:TimeFormat",
        alias = "TimeFormat",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub time_format: Option<String>,
    #[serde(
        rename = "tt:FontSize",
        alias = "FontSize",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub font_size: Option<i32>,
    #[serde(
        rename = "tt:FontColor",
        alias = "FontColor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub font_color: Option<OSDColor>,
    #[serde(
        rename = "tt:PlainText",
        alias = "PlainText",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub plain_text: Option<String>,
}

/// Device-global colour + optional alpha (1..=100).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tt:OSDColor")]
pub struct OSDColor {
    #[serde(
        rename = "@Transparent",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub transparent: Option<i32>,
    #[serde(rename = "tt:Color", alias = "Color")]
    pub color: ColorChannels,
}

/// ONVIF colourspace URI for YCbCr, the space the vendor palette is stored in.
pub const COLORSPACE_YCBCR: &str = "http://www.onvif.org/ver10/colorspace/YCbCr";

/// Colour channels, interpreted per `colorspace`.
///
/// We always emit YCbCr with 0..=255 channel values, because that is literally
/// what `def_color_tables[]` in `ak_osd.h` holds — converting to RGB would only
/// lose precision on the way back to a palette index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorChannels {
    #[serde(rename = "@X")]
    pub x: f64,
    #[serde(rename = "@Y")]
    pub y: f64,
    #[serde(rename = "@Z")]
    pub z: f64,
    #[serde(
        rename = "@Colorspace",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub colorspace: Option<String>,
}

/// GetOSDOptions payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tt:OSDConfigurationOptions")]
pub struct OSDConfigurationOptions {
    #[serde(rename = "tt:MaximumNumberOfOSDs", alias = "MaximumNumberOfOSDs")]
    pub maximum_number_of_osds: MaximumNumberOfOSDs,
    #[serde(rename = "tt:Type", alias = "Type")]
    pub osd_type: Vec<String>,
    #[serde(rename = "tt:PositionOption", alias = "PositionOption")]
    pub position_option: Vec<String>,
    #[serde(rename = "tt:TextOption", alias = "TextOption")]
    pub text_option: OSDTextOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaximumNumberOfOSDs {
    #[serde(rename = "@Total")]
    pub total: i32,
    #[serde(
        rename = "@PlainText",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub plain_text: Option<i32>,
    #[serde(
        rename = "@DateAndTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub date_and_time: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OSDTextOptions {
    #[serde(rename = "tt:Type", alias = "Type")]
    pub text_type: Vec<String>,
    #[serde(rename = "tt:FontSizeRange", alias = "FontSizeRange")]
    pub font_size_range: IntRangeXml,
    #[serde(rename = "tt:DateFormat", alias = "DateFormat", default)]
    pub date_format: Vec<String>,
    #[serde(rename = "tt:TimeFormat", alias = "TimeFormat", default)]
    pub time_format: Vec<String>,
    #[serde(
        rename = "tt:FontColor",
        alias = "FontColor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub font_color: Option<OSDColorOptions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntRangeXml {
    #[serde(rename = "tt:Min", alias = "Min")]
    pub min: i32,
    #[serde(rename = "tt:Max", alias = "Max")]
    pub max: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OSDColorOptions {
    #[serde(rename = "tt:Color", alias = "Color", default)]
    pub color: Vec<ColorChannels>,
    #[serde(
        rename = "tt:Transparent",
        alias = "Transparent",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub transparent: Option<IntRangeXml>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osd_configuration_serializes_token_attribute_and_position_child() {
        let osd = OSDConfiguration {
            token: "osd_name".into(),
            video_source_configuration_token: "VideoSourceToken".into(),
            osd_type: "Text".into(),
            position: OSDPosConfiguration {
                pos_type: "UpperLeft".into(),
            },
            text_string: Some(OSDTextConfiguration {
                text_type: "Plain".into(),
                date_format: None,
                time_format: None,
                font_size: Some(16),
                font_color: None,
                plain_text: Some("CAM".into()),
            }),
        };
        let xml = quick_xml::se::to_string(&osd).expect("serialize");
        assert!(
            xml.contains("token=\"osd_name\"") || xml.contains("token='osd_name'"),
            "token must be an attribute: {xml}"
        );
        assert!(xml.contains("Position"), "Position child missing: {xml}");
        let back: OSDConfiguration = quick_xml::de::from_str(&xml).expect("deserialize");
        assert_eq!(back.token, "osd_name");
        assert_eq!(back.position.pos_type, "UpperLeft");
    }
}
