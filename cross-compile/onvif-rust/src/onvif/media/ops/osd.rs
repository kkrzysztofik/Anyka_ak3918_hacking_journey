//! ONVIF Media OSD operations — two fixed tokens (`osd_name`, `osd_datetime`).

use std::sync::Arc;

use crate::config::ConfigRuntime;
use crate::config::types::{OsdConfig, OsdDateTimeConfig, OsdNameConfig};
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::media_osd::*;
use crate::osd::encode::encode_glyphs;
use crate::osd::format::{DateFormat, TimeFormat};
use crate::osd::layout::Corner;
use crate::platform::Platform;

use super::ProfileManagerRef;

/// Handle GetOSDs — always the two fixed silicon rects.
pub fn get_osds(pm: &ProfileManagerRef, config: &ConfigRuntime) -> OnvifResult<GetOSDsResponse> {
    let vs_token = video_source_token(pm);
    let osd = config.read().osd.clone();
    Ok(GetOSDsResponse {
        osds: vec![
            build_name_osd(&vs_token, &osd),
            build_datetime_osd(&vs_token, &osd),
        ],
    })
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
        OSD_TOKEN_NAME => build_name_osd(&vs_token, &osd),
        OSD_TOKEN_DATETIME => build_datetime_osd(&vs_token, &osd),
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
pub fn set_osd(
    pm: &ProfileManagerRef,
    config: &ConfigRuntime,
    platform: Option<&Arc<dyn Platform>>,
    request: SetOSD,
) -> OnvifResult<SetOSDResponse> {
    let token = request.osd.token.clone();
    validate_osd_for_set(&request.osd)?;

    {
        let mut cfg = config.write();
        match token.as_str() {
            OSD_TOKEN_NAME => apply_name_set(&mut cfg.osd, &request.osd)?,
            OSD_TOKEN_DATETIME => apply_datetime_set(&mut cfg.osd, &request.osd)?,
            other => {
                return Err(OnvifError::InvalidArgVal {
                    subcode: "ter:NoConfig".into(),
                    reason: format!("Unknown OSD token: {other}"),
                });
            }
        }
        if let Some(p) = platform {
            p.apply_osd_config(cfg.osd.clone());
        }
    }

    let _ = pm; // reserved for future VS-token checks
    Ok(SetOSDResponse {})
}

/// CreateOSD is not supported — rects are fixed silicon.
pub fn create_osd(_request: CreateOSD) -> OnvifResult<CreateOSDResponse> {
    Err(OnvifError::ActionNotSupported(
        "CreateOSD: this camera has two fixed OSD rects (osd_name, osd_datetime)".into(),
    ))
}

/// DeleteOSD is not supported — rects are fixed silicon.
pub fn delete_osd(_request: DeleteOSD) -> OnvifResult<DeleteOSDResponse> {
    Err(OnvifError::ActionNotSupported(
        "DeleteOSD: this camera has two fixed OSD rects (osd_name, osd_datetime)".into(),
    ))
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

fn validate_osd_for_set(osd: &OSDConfiguration) -> OnvifResult<()> {
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
    if text.text_type.eq_ignore_ascii_case("Plain")
        && let Some(plain) = text.plain_text.as_deref()
        && !plain.is_empty()
    {
        encode_glyphs(plain).map_err(|e| OnvifError::InvalidArgVal {
            subcode: "ter:InvalidArgVal".into(),
            reason: e,
        })?;
    }
    parse_corner(&osd.position.pos_type)?;
    Ok(())
}

fn apply_name_set(cfg: &mut OsdConfig, osd: &OSDConfiguration) -> OnvifResult<()> {
    let text = osd.text_string.as_ref().expect("validated");
    cfg.name = OsdNameConfig {
        enabled: true,
        position: parse_corner(&osd.position.pos_type)?,
        text: text.plain_text.clone().unwrap_or_default(),
    };
    apply_style(cfg, text);
    Ok(())
}

fn apply_datetime_set(cfg: &mut OsdConfig, osd: &OSDConfiguration) -> OnvifResult<()> {
    let text = osd.text_string.as_ref().expect("validated");
    cfg.datetime = OsdDateTimeConfig {
        enabled: true,
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

fn palette_color(index: u8) -> ColorChannels {
    // Simple distinct greyscale + primary-ish mapping for the 16 vendor slots.
    let t = f64::from(index) / 15.0;
    ColorChannels { x: t, y: t, z: t }
}

fn color_from_palette(index: u8, alpha: u8) -> OSDColor {
    OSDColor {
        transparent: Some(i32::from(alpha)),
        color: palette_color(index.min(15)),
    }
}

fn nearest_palette_index(c: &ColorChannels) -> u8 {
    let mut best = 0u8;
    let mut best_d = f64::MAX;
    for i in 0u8..16 {
        let p = palette_color(i);
        let d = (p.x - c.x).powi(2) + (p.y - c.y).powi(2) + (p.z - c.z).powi(2);
        if d < best_d {
            best_d = d;
            best = i;
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
        let (pm, cfg) = setup();
        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.text_string.as_mut().unwrap().plain_text = Some("Ogród".into());
        let err = set_osd(&pm, cfg.as_ref(), None, SetOSD { osd }).unwrap_err();
        assert!(matches!(err, OnvifError::InvalidArgVal { .. }));
    }

    #[test]
    fn test_set_osd_rejects_a_font_size_other_than_16() {
        let (pm, cfg) = setup();
        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.text_string.as_mut().unwrap().font_size = Some(24);
        let err = set_osd(&pm, cfg.as_ref(), None, SetOSD { osd }).unwrap_err();
        assert!(matches!(err, OnvifError::InvalidArgVal { .. }));
    }

    #[test]
    fn test_set_osd_persists_and_returns_the_stored_value() {
        let (pm, cfg) = setup();
        let mut osd = build_name_osd("VS", &OsdConfig::default());
        osd.position.pos_type = "LowerLeft".into();
        osd.text_string.as_mut().unwrap().plain_text = Some("FRONT".into());
        set_osd(&pm, cfg.as_ref(), None, SetOSD { osd }).unwrap();
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
    fn test_create_osd_returns_action_not_supported() {
        let err = create_osd(CreateOSD {
            osd: build_name_osd("VS", &OsdConfig::default()),
        })
        .unwrap_err();
        assert!(matches!(err, OnvifError::ActionNotSupported(_)));
    }

    #[test]
    fn test_delete_osd_returns_action_not_supported() {
        let err = delete_osd(DeleteOSD {
            osd_token: OSD_TOKEN_NAME.into(),
        })
        .unwrap_err();
        assert!(matches!(err, OnvifError::ActionNotSupported(_)));
    }
}
