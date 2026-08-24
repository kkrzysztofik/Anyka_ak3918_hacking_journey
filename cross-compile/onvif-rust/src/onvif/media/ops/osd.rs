//! ONVIF Media OSD operations — two fixed tokens (`osd_name`, `osd_datetime`).

use std::sync::Arc;

use crate::config::ConfigRuntime;
use crate::config::types::{OsdConfig, OsdDateTimeConfig, OsdNameConfig};
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::media_osd::{
    COLORSPACE_YCBCR, ColorChannels, CreateOSD, CreateOSDResponse, DeleteOSD, DeleteOSDResponse,
    GetOSD, GetOSDOptionsResponse, GetOSDResponse, GetOSDsResponse, IntRangeXml,
    MaximumNumberOfOSDs, OSD_TOKEN_DATETIME, OSD_TOKEN_NAME, OSDColor, OSDColorOptions,
    OSDConfiguration, OSDConfigurationOptions, OSDPosConfiguration, OSDTextConfiguration,
    OSDTextOptions, SetOSD, SetOSDResponse,
};
use crate::osd::encode::encode_glyphs;
use crate::osd::format::{DateFormat, TimeFormat};
use crate::osd::layout::Corner;
use crate::platform::Platform;

use super::ProfileManagerRef;

/// Handle GetOSDs — the enabled subset of the two fixed silicon rects.
///
/// ONVIF has no `Enabled` flag on `OSDConfiguration`, so presence in this list
/// *is* the enabled state, and CreateOSD/DeleteOSD are how a client toggles it.
pub fn get_osds(pm: &ProfileManagerRef, config: &ConfigRuntime) -> OnvifResult<GetOSDsResponse> {
    let vs_token = video_source_token(pm);
    let osd = config.read().osd.clone();
    let mut osds = Vec::with_capacity(2);
    if osd.name.enabled {
        osds.push(build_name_osd(&vs_token, &osd));
    }
    if osd.datetime.enabled {
        osds.push(build_datetime_osd(&vs_token, &osd));
    }
    Ok(GetOSDsResponse { osds })
}

/// Handle GetOSD.
pub fn get_osd(
    pm: &ProfileManagerRef,
    config: &ConfigRuntime,
    request: GetOSD,
) -> OnvifResult<GetOSDResponse> {
    let vs_token = video_source_token(pm);
    let osd = config.read().osd.clone();
    let conf = match request.osd_token.as_str() {
        OSD_TOKEN_NAME if osd.name.enabled => build_name_osd(&vs_token, &osd),
        OSD_TOKEN_DATETIME if osd.datetime.enabled => build_datetime_osd(&vs_token, &osd),
        // A disabled OSD does not exist as far as ONVIF is concerned.
        OSD_TOKEN_NAME | OSD_TOKEN_DATETIME => {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:NoConfig".into(),
                reason: format!("OSD {} is not currently configured", request.osd_token),
            });
        }
        other => {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:NoConfig".into(),
                reason: format!("Unknown OSD token: {other}"),
            });
        }
    };
    Ok(GetOSDResponse { osd: conf })
}

/// Handle GetOSDOptions.
pub fn get_osd_options(_pm: &ProfileManagerRef) -> OnvifResult<GetOSDOptionsResponse> {
    let palette: Vec<ColorChannels> = (0..16).map(palette_color).collect();
    Ok(GetOSDOptionsResponse {
        osd_options: OSDConfigurationOptions {
            maximum_number_of_osds: MaximumNumberOfOSDs {
                total: 2,
                plain_text: Some(1),
                date_and_time: Some(1),
            },
            osd_type: vec!["Text".into()],
            position_option: vec![
                "UpperLeft".into(),
                "UpperRight".into(),
                "LowerLeft".into(),
                "LowerRight".into(),
            ],
            text_option: OSDTextOptions {
                text_type: vec!["Plain".into(), "DateAndTime".into()],
                font_size_range: IntRangeXml { min: 16, max: 16 },
                date_format: vec![
                    "yyyy-MM-dd".into(),
                    "dd/MM/yyyy".into(),
                    "MM/dd/yyyy".into(),
                ],
                time_format: vec!["HH:mm:ss".into(), "hh:mm:ss tt".into()],
                font_color: Some(OSDColorOptions {
                    color: palette,
                    transparent: Some(IntRangeXml { min: 1, max: 100 }),
                }),
            },
        },
    })
}

/// Handle SetOSD — persists into `[osd]` and pushes into the live renderer.
///
/// Leaves the enabled flag alone: saving settings must not silently re-enable
/// an overlay the operator turned off. Use CreateOSD/DeleteOSD for that.
pub fn set_osd(
    config: &ConfigRuntime,
    platform: Option<&Arc<dyn Platform>>,
    request: SetOSD,
) -> OnvifResult<SetOSDResponse> {
    let token = request.osd.token.clone();
    let text = validate_osd_for_set(&request.osd)?;

    let mut cfg = config.write();
    match token.as_str() {
        OSD_TOKEN_NAME => apply_name_set(&mut cfg.osd, &request.osd, text)?,
        OSD_TOKEN_DATETIME => apply_datetime_set(&mut cfg.osd, &request.osd, text)?,
        other => {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:NoConfig".into(),
                reason: format!("Unknown OSD token: {other}"),
            });
        }
    }
    push_to_renderer(&cfg.osd, platform);
    Ok(SetOSDResponse {})
}

/// Handle CreateOSD — enables one of the two fixed rects.
///
/// The rects are fixed silicon, so this cannot mint a new token; it turns an
/// existing one on. Anything else faults, which is what a client attempting a
/// third OSD should see. Position / TextString / colour from the request are
/// applied before enabling so CreateOSD can restore a deleted overlay fully.
pub fn create_osd(
    config: &ConfigRuntime,
    platform: Option<&Arc<dyn Platform>>,
    request: CreateOSD,
) -> OnvifResult<CreateOSDResponse> {
    let token = request.osd.token.clone();
    // TextString is optional on CreateOSD: the WebUI enable path sends token +
    // Position only. When TextString is present, validate and apply it first.
    let text = if request.osd.text_string.is_some() {
        Some(validate_osd_for_set(&request.osd)?)
    } else {
        parse_corner(&request.osd.position.pos_type)?;
        None
    };

    let mut cfg = config.write();
    match token.as_str() {
        OSD_TOKEN_NAME => {
            if let Some(text) = text {
                apply_name_set(&mut cfg.osd, &request.osd, text)?;
            } else {
                cfg.osd.name.position = parse_corner(&request.osd.position.pos_type)?;
            }
            cfg.osd.name.enabled = true;
        }
        OSD_TOKEN_DATETIME => {
            if let Some(text) = text {
                apply_datetime_set(&mut cfg.osd, &request.osd, text)?;
            } else {
                cfg.osd.datetime.position = parse_corner(&request.osd.position.pos_type)?;
            }
            cfg.osd.datetime.enabled = true;
        }
        other => {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:NoConfig".into(),
                reason: format!(
                    "Unknown OSD token {other}: this camera has two fixed rects \
                     ({OSD_TOKEN_NAME}, {OSD_TOKEN_DATETIME})"
                ),
            });
        }
    }
    push_to_renderer(&cfg.osd, platform);
    Ok(CreateOSDResponse { osd_token: token })
}

/// Handle DeleteOSD — disables one of the two fixed rects.
pub fn delete_osd(
    config: &ConfigRuntime,
    platform: Option<&Arc<dyn Platform>>,
    request: DeleteOSD,
) -> OnvifResult<DeleteOSDResponse> {
    set_enabled(config, platform, &request.osd_token, false)?;
    Ok(DeleteOSDResponse {})
}

fn set_enabled(
    config: &ConfigRuntime,
    platform: Option<&Arc<dyn Platform>>,
    token: &str,
    enabled: bool,
) -> OnvifResult<()> {
    let mut cfg = config.write();
    match token {
        OSD_TOKEN_NAME => cfg.osd.name.enabled = enabled,
        OSD_TOKEN_DATETIME => cfg.osd.datetime.enabled = enabled,
        other => {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:NoConfig".into(),
                reason: format!(
                    "Unknown OSD token {other}: this camera has two fixed rects \
                     ({OSD_TOKEN_NAME}, {OSD_TOKEN_DATETIME})"
                ),
            });
        }
    }
    push_to_renderer(&cfg.osd, platform);
    Ok(())
}

fn push_to_renderer(osd: &OsdConfig, platform: Option<&Arc<dyn Platform>>) {
    if let Some(p) = platform {
        p.apply_osd_config(osd.clone());
    }
}

fn video_source_token(pm: &ProfileManagerRef) -> String {
    pm.get_video_source_configurations()
        .into_iter()
        .next()
        .map(|c| c.token)
        .unwrap_or_else(|| "VideoSourceToken".into())
}

fn build_name_osd(vs_token: &str, osd: &OsdConfig) -> OSDConfiguration {
    OSDConfiguration {
        token: OSD_TOKEN_NAME.into(),
        video_source_configuration_token: vs_token.into(),
        osd_type: "Text".into(),
        position: OSDPosConfiguration {
            pos_type: corner_to_onvif(osd.name.position),
        },
        text_string: Some(OSDTextConfiguration {
            text_type: "Plain".into(),
            date_format: None,
            time_format: None,
            font_size: Some(16),
            font_color: Some(color_from_palette(osd.color, osd.alpha)),
            plain_text: Some(osd.name.text.clone()),
        }),
    }
}

fn build_datetime_osd(vs_token: &str, osd: &OsdConfig) -> OSDConfiguration {
    OSDConfiguration {
        token: OSD_TOKEN_DATETIME.into(),
        video_source_configuration_token: vs_token.into(),
        osd_type: "Text".into(),
        position: OSDPosConfiguration {
            pos_type: corner_to_onvif(osd.datetime.position),
        },
        text_string: Some(OSDTextConfiguration {
            text_type: "DateAndTime".into(),
            date_format: Some(date_format_to_onvif(osd.datetime.date_format)),
            time_format: Some(time_format_to_onvif(osd.datetime.time_format)),
            font_size: Some(16),
            font_color: Some(color_from_palette(osd.color, osd.alpha)),
            plain_text: None,
        }),
    }
}

fn validate_osd_for_set(osd: &OSDConfiguration) -> OnvifResult<&OSDTextConfiguration> {
    let Some(text) = osd.text_string.as_ref() else {
        return Err(OnvifError::InvalidArgVal {
            subcode: "ter:InvalidArgVal".into(),
            reason: "OSD TextString is required".into(),
        });
    };
    if let Some(size) = text.font_size
        && size != 16
    {
        return Err(OnvifError::InvalidArgVal {
            subcode: "ter:InvalidArgVal".into(),
            reason: format!("FontSize {size} is not supported; only 16"),
        });
    }
    // Validate any submitted PlainText regardless of TextType so a DateAndTime
    // request cannot smuggle non-encodable text into apply_name_set paths.
    if let Some(plain) = text.plain_text.as_deref()
        && !plain.is_empty()
    {
        encode_glyphs(plain).map_err(|e| OnvifError::InvalidArgVal {
            subcode: "ter:InvalidArgVal".into(),
            reason: e,
        })?;
    }
    parse_corner(&osd.position.pos_type)?;
    Ok(text)
}

fn apply_name_set(
    cfg: &mut OsdConfig,
    osd: &OSDConfiguration,
    text: &OSDTextConfiguration,
) -> OnvifResult<()> {
    cfg.name = OsdNameConfig {
        enabled: cfg.name.enabled,
        position: parse_corner(&osd.position.pos_type)?,
        text: text.plain_text.clone().unwrap_or_default(),
    };
    apply_style(cfg, text);
    Ok(())
}

fn apply_datetime_set(
    cfg: &mut OsdConfig,
    osd: &OSDConfiguration,
    text: &OSDTextConfiguration,
) -> OnvifResult<()> {
    cfg.datetime = OsdDateTimeConfig {
        enabled: cfg.datetime.enabled,
        position: parse_corner(&osd.position.pos_type)?,
        date_format: parse_date_format(text.date_format.as_deref())?,
        time_format: parse_time_format(text.time_format.as_deref())?,
    };
    apply_style(cfg, text);
    Ok(())
}

fn apply_style(cfg: &mut OsdConfig, text: &OSDTextConfiguration) {
    if let Some(fc) = text.font_color.as_ref() {
        cfg.color = nearest_palette_index(&fc.color);
        if let Some(a) = fc.transparent {
            cfg.alpha = a.clamp(1, 100) as u8;
        }
    }
}

fn corner_to_onvif(c: Corner) -> String {
    match c {
        Corner::UpperLeft => "UpperLeft",
        Corner::UpperRight => "UpperRight",
        Corner::LowerLeft => "LowerLeft",
        Corner::LowerRight => "LowerRight",
    }
    .into()
}

fn parse_corner(s: &str) -> OnvifResult<Corner> {
    match s {
        "UpperLeft" => Ok(Corner::UpperLeft),
        "UpperRight" => Ok(Corner::UpperRight),
        "LowerLeft" => Ok(Corner::LowerLeft),
        "LowerRight" => Ok(Corner::LowerRight),
        other => Err(OnvifError::InvalidArgVal {
            subcode: "ter:InvalidArgVal".into(),
            reason: format!("Unsupported OSD position: {other}"),
        }),
    }
}

fn date_format_to_onvif(f: DateFormat) -> String {
    match f {
        DateFormat::Iso => "yyyy-MM-dd",
        DateFormat::European => "dd/MM/yyyy",
        DateFormat::Us => "MM/dd/yyyy",
    }
    .into()
}

fn time_format_to_onvif(f: TimeFormat) -> String {
    match f {
        TimeFormat::H24 => "HH:mm:ss",
        TimeFormat::H12 => "hh:mm:ss tt",
    }
    .into()
}

fn parse_date_format(s: Option<&str>) -> OnvifResult<DateFormat> {
    match s.unwrap_or("yyyy-MM-dd") {
        "yyyy-MM-dd" | "iso" => Ok(DateFormat::Iso),
        "dd/MM/yyyy" => Ok(DateFormat::European),
        "MM/dd/yyyy" => Ok(DateFormat::Us),
        other => Err(OnvifError::InvalidArgVal {
            subcode: "ter:InvalidArgVal".into(),
            reason: format!("Unsupported DateFormat: {other}"),
        }),
    }
}

fn parse_time_format(s: Option<&str>) -> OnvifResult<TimeFormat> {
    match s.unwrap_or("HH:mm:ss") {
        "HH:mm:ss" | "h24" => Ok(TimeFormat::H24),
        "hh:mm:ss tt" | "h12" => Ok(TimeFormat::H12),
        other => Err(OnvifError::InvalidArgVal {
            subcode: "ter:InvalidArgVal".into(),
            reason: format!("Unsupported TimeFormat: {other}"),
        }),
    }
}

/// The vendor's OSD colour table, verbatim from `def_color_tables[]` in
/// `ak_osd.h`, as `(Y, Cb, Cr)`.
///
/// These are YCbCr, not RGB: index 1 is `0xff7f7f` (white) and index 2 is
/// `0x007f7f` (black), which only makes sense with neutral chroma at 0x7f.
/// `ak_osd_set_color` takes an index into this table, so the index is the only
/// thing the hardware understands — the channel values exist purely so ONVIF
/// clients can render a swatch.
pub const VENDOR_PALETTE: [(u8, u8, u8); 16] = [
    (0x00, 0x00, 0x00),
    (0xff, 0x7f, 0x7f),
    (0x00, 0x7f, 0x7f),
    (0x26, 0x6a, 0xc0),
    (0x71, 0x40, 0x8a),
    (0x4b, 0x55, 0x4a),
    (0x59, 0x95, 0x40),
    (0x0e, 0xc0, 0x75),
    (0x34, 0xaa, 0xb5),
    (0x78, 0x60, 0x85),
    (0x2c, 0x8a, 0xa0),
    (0x68, 0xd5, 0x35),
    (0x34, 0xaa, 0x5a),
    (0x43, 0xe9, 0xab),
    (0x4b, 0x55, 0xa5),
    (0x00, 0x80, 0x80),
];

fn palette_color(index: u8) -> ColorChannels {
    let (y, cb, cr) = VENDOR_PALETTE[usize::from(index.min(15))];
    ColorChannels {
        x: f64::from(y),
        y: f64::from(cb),
        z: f64::from(cr),
        colorspace: Some(COLORSPACE_YCBCR.into()),
    }
}

fn color_from_palette(index: u8, alpha: u8) -> OSDColor {
    OSDColor {
        transparent: Some(i32::from(alpha)),
        color: palette_color(index),
    }
}

/// Map an arbitrary client colour back onto a palette index.
///
/// A nearest-neighbour search rather than a lookup because the palette is 16
/// scattered points in YCbCr, and clients with a colour picker (ONVIF Device
/// Manager) will send values that are not in the table.
fn nearest_palette_index(c: &ColorChannels) -> u8 {
    let mut best = 0u8;
    let mut best_d = f64::MAX;
    for (i, &(y, cb, cr)) in VENDOR_PALETTE.iter().enumerate() {
        let d = (f64::from(y) - c.x).powi(2)
            + (f64::from(cb) - c.y).powi(2)
            + (f64::from(cr) - c.z).powi(2);
        if d < best_d {
            best_d = d;
            best = i as u8;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigRuntime;
    use crate::onvif::media::ProfileManager;

    fn setup() -> (ProfileManagerRef, std::sync::Arc<ConfigRuntime>) {
        let shared = std::sync::Arc::new(ConfigRuntime::new(Default::default()));
        let pm = ProfileManager::with_config(std::sync::Arc::clone(&shared));
        (pm, shared)
    }

    #[test]
    fn test_get_osds_returns_exactly_two_fixed_tokens() {
        let (pm, cfg) = setup();
        let resp = get_osds(&pm, cfg.as_ref()).unwrap();
        assert_eq!(resp.osds.len(), 2);
        assert_eq!(resp.osds[0].token, OSD_TOKEN_NAME);
        assert_eq!(resp.osds[1].token, OSD_TOKEN_DATETIME);
    }

    #[test]
    fn test_get_osd_rejects_an_unknown_token() {
        let (pm, cfg) = setup();
        let err = get_osd(
            &pm,
            cfg.as_ref(),
            GetOSD {
                osd_token: "nope".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, OnvifError::InvalidArgVal { .. }));
    }

    #[test]
    fn test_get_osd_options_advertises_only_font_size_16() {
        let (pm, _) = setup();
        let opts = get_osd_options(&pm).unwrap().osd_options;
        assert_eq!(opts.text_option.font_size_range.min, 16);
        assert_eq!(opts.text_option.font_size_range.max, 16);
    }

    #[test]
    fn test_get_osd_options_advertises_sixteen_palette_colours() {
        let (pm, _) = setup();
        let opts = get_osd_options(&pm).unwrap().osd_options;
        assert_eq!(opts.text_option.font_color.unwrap().color.len(), 16);
    }

    #[test]
    fn test_set_osd_rejects_non_ascii_plain_text() {
        let (_pm, cfg) = setup();
        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.text_string.as_mut().unwrap().plain_text = Some("Ogród".into());
        let err = set_osd(cfg.as_ref(), None, SetOSD { osd }).unwrap_err();
        assert!(matches!(err, OnvifError::InvalidArgVal { .. }));
    }

    #[test]
    fn test_set_osd_rejects_non_ascii_plain_text_regardless_of_text_type() {
        let (_pm, cfg) = setup();
        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.text_string.as_mut().unwrap().text_type = "DateAndTime".into();
        osd.text_string.as_mut().unwrap().plain_text = Some("Ogród".into());
        let err = set_osd(cfg.as_ref(), None, SetOSD { osd }).unwrap_err();
        assert!(matches!(err, OnvifError::InvalidArgVal { .. }));
    }

    #[test]
    fn test_set_osd_rejects_a_font_size_other_than_16() {
        let (_pm, cfg) = setup();
        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.text_string.as_mut().unwrap().font_size = Some(24);
        let err = set_osd(cfg.as_ref(), None, SetOSD { osd }).unwrap_err();
        assert!(matches!(err, OnvifError::InvalidArgVal { .. }));
    }

    #[test]
    fn test_set_osd_persists_and_returns_the_stored_value() {
        let (pm, cfg) = setup();
        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.position.pos_type = "LowerLeft".into();
        osd.text_string.as_mut().unwrap().plain_text = Some("FRONT".into());
        set_osd(cfg.as_ref(), None, SetOSD { osd }).unwrap();
        let got = get_osd(
            &pm,
            cfg.as_ref(),
            GetOSD {
                osd_token: OSD_TOKEN_NAME.into(),
            },
        )
        .unwrap();
        assert_eq!(got.osd.position.pos_type, "LowerLeft");
        assert_eq!(
            got.osd.text_string.unwrap().plain_text.as_deref(),
            Some("FRONT")
        );
    }

    #[test]
    fn test_delete_osd_disables_it_and_drops_it_from_get_osds() {
        // ONVIF has no Enabled flag, so absence from GetOSDs is how a client
        // sees "off".
        let (pm, cfg) = setup();
        delete_osd(
            cfg.as_ref(),
            None,
            DeleteOSD {
                osd_token: OSD_TOKEN_DATETIME.into(),
            },
        )
        .unwrap();

        assert!(!cfg.read().osd.datetime.enabled);
        let tokens: Vec<String> = get_osds(&pm, cfg.as_ref())
            .unwrap()
            .osds
            .into_iter()
            .map(|o| o.token)
            .collect();
        assert_eq!(tokens, vec![OSD_TOKEN_NAME.to_string()]);
    }

    #[test]
    fn test_get_osd_on_a_disabled_token_faults() {
        let (pm, cfg) = setup();
        delete_osd(
            cfg.as_ref(),
            None,
            DeleteOSD {
                osd_token: OSD_TOKEN_NAME.into(),
            },
        )
        .unwrap();
        let err = get_osd(
            &pm,
            cfg.as_ref(),
            GetOSD {
                osd_token: OSD_TOKEN_NAME.into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, OnvifError::InvalidArgVal { .. }));
    }

    #[test]
    fn test_create_osd_re_enables_a_deleted_osd() {
        let (_pm, cfg) = setup();
        delete_osd(
            cfg.as_ref(),
            None,
            DeleteOSD {
                osd_token: OSD_TOKEN_NAME.into(),
            },
        )
        .unwrap();

        let resp = create_osd(
            cfg.as_ref(),
            None,
            CreateOSD {
                osd: build_name_osd("VS", &OsdConfig::default()),
            },
        )
        .unwrap();

        assert_eq!(resp.osd_token, OSD_TOKEN_NAME);
        assert!(cfg.read().osd.name.enabled);
    }

    #[test]
    fn test_create_osd_applies_submitted_position() {
        let (_pm, cfg) = setup();
        delete_osd(
            cfg.as_ref(),
            None,
            DeleteOSD {
                osd_token: OSD_TOKEN_NAME.into(),
            },
        )
        .unwrap();

        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.position.pos_type = "LowerLeft".into();
        create_osd(cfg.as_ref(), None, CreateOSD { osd }).unwrap();

        assert_eq!(cfg.read().osd.name.position, Corner::LowerLeft);
        assert!(cfg.read().osd.name.enabled);
    }

    #[test]
    fn test_create_osd_rejects_a_third_token() {
        let (_pm, cfg) = setup();
        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.token = "osd_extra".into();
        let err = create_osd(cfg.as_ref(), None, CreateOSD { osd }).unwrap_err();
        assert!(matches!(err, OnvifError::InvalidArgVal { .. }));
    }

    #[test]
    fn test_set_osd_does_not_silently_re_enable_a_disabled_osd() {
        let (_pm, cfg) = setup();
        delete_osd(
            cfg.as_ref(),
            None,
            DeleteOSD {
                osd_token: OSD_TOKEN_NAME.into(),
            },
        )
        .unwrap();

        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.position.pos_type = "LowerLeft".into();
        set_osd(cfg.as_ref(), None, SetOSD { osd }).unwrap();

        assert!(
            !cfg.read().osd.name.enabled,
            "saving settings must not turn an overlay back on"
        );
    }

    #[test]
    fn test_palette_is_the_vendor_table_in_ycbcr() {
        // Index 1 is 0xff7f7f = white and index 2 is 0x007f7f = black, which
        // only holds if the table is YCbCr. A greyscale ramp would round-trip
        // to the wrong index and the swatch would not match the video.
        assert_eq!(VENDOR_PALETTE[1], (0xff, 0x7f, 0x7f));
        assert_eq!(VENDOR_PALETTE[2], (0x00, 0x7f, 0x7f));

        let c = palette_color(7);
        assert_eq!(c.colorspace.as_deref(), Some(COLORSPACE_YCBCR));
        assert_eq!(nearest_palette_index(&c), 7, "palette must round-trip");
    }

    #[test]
    fn test_nearest_palette_index_snaps_an_arbitrary_colour() {
        // A client colour picker will not send exact table values.
        let near_white = ColorChannels {
            x: 250.0,
            y: 128.0,
            z: 126.0,
            colorspace: Some(COLORSPACE_YCBCR.into()),
        };
        assert_eq!(nearest_palette_index(&near_white), 1);
    }
}
