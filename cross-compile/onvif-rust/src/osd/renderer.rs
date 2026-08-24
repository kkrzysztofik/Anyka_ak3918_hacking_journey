//! 1 Hz OSD render planner.
//!
//! Pure decision logic lives here so unit tests never need a tokio timer. The
//! async task that ticks once a second is a thin wrapper around [`RenderState`].
//!
//! This camera's ISP only composites one OSD DMA plane per video channel (the
//! `osd_vpss_wrap` path drops the rect index). Name and datetime therefore
//! share silicon rect 0 as a full-frame canvas; each `draw_str` paints into
//! that canvas without wiping the other string.

use std::ffi::c_void;
use std::sync::Arc;
use std::time::Duration;

use chrono::Local;
use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::config::types::OsdConfig;
use crate::hal::anyka::ipc::AnykaIpc;
use crate::hal::common::video::VideoInputHandle;
use crate::osd::encode::{encode_glyphs, pad_to_erase};
use crate::osd::format::format_datetime;
use crate::osd::layout::{CANVAS_RECT, ChannelDims, Corner, FontMetrics, canvas_rect, place};

/// Logical overlay slots (both map to [`CANVAS_RECT`] on silicon).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsdRect {
    /// Camera name.
    Name = 0,
    /// Live timestamp.
    DateTime = 1,
}

impl OsdRect {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// What the IPC layer should draw for one logical overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawPlan {
    pub channel: u8,
    pub rect: OsdRect,
    pub draw_x: i32,
    pub draw_y: i32,
    pub glyphs: Vec<u16>,
    /// Space-fill at the previous position when the corner moved.
    pub erase: Option<ErasePlan>,
    /// Glyph payload or position differs — needs `draw_str`.
    pub content_changed: bool,
}

/// Erase the previous string when it moves to a new corner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErasePlan {
    pub draw_x: i32,
    pub draw_y: i32,
    pub glyphs: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SlotMemory {
    text: String,
    corner: Corner,
    glyph_len: usize,
    draw_x: i32,
    draw_y: i32,
}

/// Remembers the last successful draw per logical overlay so unchanged text is
/// skipped and shrinking / moved strings are erased.
#[derive(Debug, Default)]
pub struct RenderState {
    /// `[channel][logical overlay]`
    last: [[Option<SlotMemory>; 2]; 2],
    /// Whether the shared full-frame canvas is allocated for each channel.
    canvas_ready: [bool; 2],
}

impl RenderState {
    /// Plan a draw for `text` on `channel`/`rect`, or `None` if the text is
    /// invalid (must not poison state).
    pub fn plan(
        &mut self,
        channel: u8,
        rect: OsdRect,
        text: &str,
        corner: Corner,
        dims: ChannelDims,
    ) -> Option<DrawPlan> {
        if channel > 1 {
            return None;
        }
        let glyphs = match encode_glyphs(text) {
            Ok(g) => g,
            Err(_) => return None,
        };

        let slot = &mut self.last[channel as usize][rect as usize];
        let previous_len = slot.as_ref().map(|p| p.glyph_len).unwrap_or(0);
        let prev = slot.clone();
        let content_changed = match prev.as_ref() {
            None => true,
            Some(p) => p.text != text || p.corner != corner,
        };
        let glyphs = pad_to_erase(glyphs, previous_len);
        let font = FontMetrics::for_channel(channel);
        let placement = place(corner, glyphs.len(), dims, font);

        let erase = match prev.as_ref() {
            Some(p) if p.corner != corner && p.glyph_len > 0 => Some(ErasePlan {
                draw_x: p.draw_x,
                draw_y: p.draw_y,
                glyphs: vec![0x20; p.glyph_len],
            }),
            _ => None,
        };
        let content_changed = content_changed || erase.is_some();

        *slot = Some(SlotMemory {
            text: text.to_string(),
            corner,
            glyph_len: glyphs.len(),
            draw_x: placement.draw_x,
            draw_y: placement.draw_y,
        });

        Some(DrawPlan {
            channel,
            rect,
            draw_x: placement.draw_x,
            draw_y: placement.draw_y,
            glyphs,
            erase,
            content_changed,
        })
    }

    /// Forget remembered draws — call after daemon reattach / `osd_init`.
    pub fn reset(&mut self) {
        self.last = Default::default();
        self.canvas_ready = [false; 2];
    }

    fn ensure_canvas(
        &mut self,
        channel: u8,
        dims: ChannelDims,
        ipc: &AnykaIpc,
        vi: *mut c_void,
    ) -> bool {
        let idx = channel as usize;
        if self.canvas_ready[idx] {
            return true;
        }
        let (x, y, w, h) = canvas_rect(dims);
        if let Err(e) = ipc.osd_set_rect(vi, i32::from(channel), CANVAS_RECT, x, y, w, h) {
            warn!(error = %e, channel, "osd_set_rect (canvas) failed");
            return false;
        }
        let _ = ipc.osd_set_enable(i32::from(channel), CANVAS_RECT, true);
        self.canvas_ready[idx] = true;
        true
    }
}

/// Inputs the 1 Hz OSD task needs after `osd_init` succeeds.
pub struct OsdRendererArgs {
    pub ipc: Arc<AnykaIpc>,
    pub vi: Arc<VideoInputHandle>,
    pub dims: [ChannelDims; 2],
    /// Live settings — updated by ONVIF `SetOSD` / WebUI without restarting the task.
    pub config: Arc<parking_lot::RwLock<OsdConfig>>,
    /// Fallback when `[osd.name].text` is empty.
    pub device_name: String,
    pub shutdown: broadcast::Receiver<()>,
}

/// Spawn the 1 Hz overlay task.
pub fn spawn_osd_renderer(args: OsdRendererArgs) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        run_osd_renderer(args).await;
    })
}

async fn run_osd_renderer(mut args: OsdRendererArgs) {
    let mut state = RenderState::default();
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut last_style: Option<(u8, u8)> = None;

    loop {
        tokio::select! {
            _ = args.shutdown.recv() => {
                debug!("OSD renderer stopping");
                break;
            }
            _ = interval.tick() => {
                tick_once(&mut state, &args, &mut last_style);
            }
        }
    }
}

fn tick_once(state: &mut RenderState, args: &OsdRendererArgs, last_style: &mut Option<(u8, u8)>) {
    let cfg = args.config.read().clone();
    if !cfg.enabled {
        return;
    }

    let style = (cfg.color, cfg.alpha);
    if last_style.as_ref() != Some(&style) {
        if let Err(e) = args
            .ipc
            .osd_set_style(i32::from(cfg.color), 0, 0, i32::from(cfg.alpha))
        {
            warn!(error = %e, "osd_set_style failed; continuing without style");
        } else {
            *last_style = Some(style);
        }
    }

    let name_owned;
    let name_text = if cfg.name.text.is_empty() {
        args.device_name.as_str()
    } else {
        name_owned = cfg.name.text.clone();
        name_owned.as_str()
    };

    for channel in 0u8..=1 {
        let dims = args.dims[channel as usize];
        let mut plans = Vec::new();

        if cfg.name.enabled
            && let Some(plan) =
                state.plan(channel, OsdRect::Name, name_text, cfg.name.position, dims)
        {
            plans.push(plan);
        }

        if cfg.datetime.enabled {
            let when = Local::now();
            let text = format_datetime(when, cfg.datetime.date_format, cfg.datetime.time_format);
            if let Some(plan) =
                state.plan(channel, OsdRect::DateTime, &text, cfg.datetime.position, dims)
            {
                plans.push(plan);
            }
        }

        if plans.is_empty() {
            continue;
        }

        let vi = args.vi.as_ptr();
        if !state.ensure_canvas(channel, dims, &args.ipc, vi) {
            continue;
        }

        // Paint into the shared canvas. Order does not matter for persistence —
        // each draw_str only updates its glyph region — but draw datetime last
        // so the final ISP flush of the tick includes the fresh timestamp.
        plans.sort_by_key(|p| p.rect as i32);
        for plan in &plans {
            if let Some(erase) = &plan.erase
                && let Err(e) = args.ipc.osd_draw_str(
                    i32::from(plan.channel),
                    CANVAS_RECT,
                    erase.draw_x,
                    erase.draw_y,
                    &erase.glyphs,
                )
            {
                warn!(error = %e, channel, "osd_draw_str erase failed");
            }
            if !plan.content_changed {
                continue;
            }
            if let Err(e) = args.ipc.osd_draw_str(
                i32::from(plan.channel),
                CANVAS_RECT,
                plan.draw_x,
                plan.draw_y,
                &plan.glyphs,
            ) {
                warn!(error = %e, channel, rect = ?plan.rect, "osd_draw_str failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: ChannelDims = ChannelDims {
        width: 1280,
        height: 720,
    };

    #[test]
    fn test_render_plan_skips_ipc_when_unchanged() {
        let mut state = RenderState::default();
        let first = state
            .plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN)
            .unwrap();
        assert!(first.content_changed);

        let second = state
            .plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN)
            .unwrap();
        assert!(!second.content_changed, "unchanged text must not draw_str");
        assert!(second.erase.is_none());
    }

    #[test]
    fn test_render_plan_erases_old_corner_when_moving() {
        let mut state = RenderState::default();
        let first = state
            .plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN)
            .unwrap();
        let moved = state
            .plan(0, OsdRect::Name, "CAM1", Corner::LowerRight, MAIN)
            .unwrap();
        assert!(moved.content_changed);
        let erase = moved.erase.expect("corner move must erase old glyphs");
        assert_eq!((erase.draw_x, erase.draw_y), (first.draw_x, first.draw_y));
        assert_eq!(erase.glyphs.len(), first.glyphs.len());
        assert!(erase.glyphs.iter().all(|&g| g == 0x20));
    }

    #[test]
    fn test_render_plan_pads_a_shrinking_string() {
        let mut state = RenderState::default();
        state.plan(0, OsdRect::Name, "LONG NAME", Corner::UpperLeft, MAIN);
        let plan = state
            .plan(0, OsdRect::Name, "AB", Corner::UpperLeft, MAIN)
            .unwrap();
        assert_eq!(plan.glyphs.len(), 9, "must overwrite the previous tail");
        assert_eq!(plan.glyphs[2], 0x20);
    }

    #[test]
    fn test_render_plan_rejects_non_ascii_without_poisoning_state() {
        let mut state = RenderState::default();
        assert!(
            state
                .plan(0, OsdRect::Name, "Ogród", Corner::UpperLeft, MAIN)
                .is_none()
        );
        assert!(
            state
                .plan(0, OsdRect::Name, "OK", Corner::UpperLeft, MAIN)
                .is_some()
        );
    }

    #[test]
    fn test_name_and_datetime_share_independent_slots() {
        let mut state = RenderState::default();
        let name = state
            .plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN)
            .unwrap();
        let dt = state
            .plan(0, OsdRect::DateTime, "2026-08-24 12:00:00", Corner::LowerRight, MAIN)
            .unwrap();
        assert_eq!(name.draw_y, 0);
        assert_eq!(dt.draw_y, 720 - 32);
        // Second datetime tick changes text only — name slot untouched.
        let dt2 = state
            .plan(0, OsdRect::DateTime, "2026-08-24 12:00:01", Corner::LowerRight, MAIN)
            .unwrap();
        assert!(dt2.content_changed);
        let name2 = state
            .plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN)
            .unwrap();
        assert!(!name2.content_changed);
    }
}
