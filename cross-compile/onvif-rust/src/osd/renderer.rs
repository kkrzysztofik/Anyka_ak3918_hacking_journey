//! 1 Hz OSD render planner.
//!
//! Pure decision logic lives here so unit tests never need a tokio timer. The
//! async task that ticks once a second is a thin wrapper around [`RenderState`].

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
use crate::osd::layout::{ChannelDims, Corner, FontMetrics, place, rect_size};

/// Fixed overlay slots on each channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsdRect {
    /// Camera name (rect index 0).
    Name = 0,
    /// Live timestamp (rect index 1).
    DateTime = 1,
}

impl OsdRect {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// What the IPC layer should draw for one rect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawPlan {
    pub channel: u8,
    pub rect: OsdRect,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub glyphs: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RectMemory {
    text: String,
    corner: Corner,
    glyph_len: usize,
}

/// Remembers the last successful draw per channel/rect so unchanged text is
/// skipped and shrinking strings are space-padded.
#[derive(Debug, Default)]
pub struct RenderState {
    /// `[channel][rect]` — only two channels and two rects exist.
    last: [[Option<RectMemory>; 2]; 2],
}

impl RenderState {
    /// Plan a draw for `text` on `channel`/`rect`, or `None` if nothing changed
    /// (or the text is invalid and must not poison state).
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
        if let Some(prev) = slot.as_ref()
            && prev.text == text
            && prev.corner == corner
        {
            return None;
        }

        let previous_len = slot.as_ref().map(|p| p.glyph_len).unwrap_or(0);
        let glyphs = pad_to_erase(glyphs, previous_len);
        let font = FontMetrics::for_channel(channel);
        let placement = place(corner, glyphs.len(), dims, font);
        let (width, height) = rect_size(glyphs.len(), font);

        *slot = Some(RectMemory {
            text: text.to_string(),
            corner,
            glyph_len: glyphs.len(),
        });

        Some(DrawPlan {
            channel,
            rect,
            x: placement.x,
            y: placement.y,
            width,
            height,
            glyphs,
        })
    }

    /// Forget remembered draws — call after daemon reattach / `osd_init`.
    pub fn reset(&mut self) {
        self.last = Default::default();
    }
}

/// Inputs the 1 Hz OSD task needs after `osd_init` succeeds.
pub struct OsdRendererArgs {
    pub ipc: Arc<AnykaIpc>,
    pub vi: Arc<VideoInputHandle>,
    pub dims: [ChannelDims; 2],
    pub config: OsdConfig,
    /// Fallback when `[osd.name].text` is empty.
    pub device_name: String,
    pub shutdown: broadcast::Receiver<()>,
}

/// Spawn the 1 Hz overlay task.
///
/// // ponytail: one shared 1 Hz tick for both rects and both channels. Fine
/// // because only the timestamp changes per second; switch to per-rect timers
/// // only if a sub-second element is ever added.
pub fn spawn_osd_renderer(args: OsdRendererArgs) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        run_osd_renderer(args).await;
    })
}

async fn run_osd_renderer(mut args: OsdRendererArgs) {
    let mut state = RenderState::default();
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Style once at start; colour/alpha are device-global.
    if let Err(e) = args.ipc.osd_set_style(
        i32::from(args.config.color),
        0,
        0,
        i32::from(args.config.alpha),
    ) {
        warn!(error = %e, "osd_set_style failed; continuing without style");
    }

    loop {
        tokio::select! {
            _ = args.shutdown.recv() => {
                debug!("OSD renderer stopping");
                break;
            }
            _ = interval.tick() => {
                tick_once(&mut state, &args);
            }
        }
    }
}

fn tick_once(state: &mut RenderState, args: &OsdRendererArgs) {
    if !args.config.enabled {
        return;
    }

    let name_text = if args.config.name.text.is_empty() {
        args.device_name.as_str()
    } else {
        args.config.name.text.as_str()
    };

    for channel in 0u8..=1 {
        let dims = args.dims[channel as usize];

        if args.config.name.enabled {
            apply_plan(
                state,
                args,
                channel,
                OsdRect::Name,
                name_text,
                args.config.name.position,
                dims,
            );
        }

        if args.config.datetime.enabled {
            let when = Local::now();
            let text = format_datetime(
                when,
                args.config.datetime.date_format,
                args.config.datetime.time_format,
            );
            apply_plan(
                state,
                args,
                channel,
                OsdRect::DateTime,
                &text,
                args.config.datetime.position,
                dims,
            );
        }
    }
}

fn apply_plan(
    state: &mut RenderState,
    args: &OsdRendererArgs,
    channel: u8,
    rect: OsdRect,
    text: &str,
    corner: Corner,
    dims: ChannelDims,
) {
    let Some(plan) = state.plan(channel, rect, text, corner, dims) else {
        return;
    };

    let vi = args.vi.as_ptr();
    if let Err(e) = args.ipc.osd_set_rect(
        vi,
        i32::from(channel),
        plan.rect.as_i32(),
        plan.x,
        plan.y,
        plan.width,
        plan.height,
    ) {
        warn!(error = %e, channel, rect = ?plan.rect, "osd_set_rect failed");
        return;
    }
    let _ = args
        .ipc
        .osd_set_enable(i32::from(channel), plan.rect.as_i32(), true);
    if let Err(e) =
        args.ipc
            .osd_draw_str(i32::from(channel), plan.rect.as_i32(), 0, 0, &plan.glyphs)
    {
        warn!(error = %e, channel, rect = ?plan.rect, "osd_draw_str failed");
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
    fn test_render_plan_skips_unchanged_text() {
        let mut state = RenderState::default();
        let first = state.plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN);
        assert!(first.is_some(), "first render must draw");

        let second = state.plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN);
        assert!(second.is_none(), "unchanged text must not redraw");
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
    fn test_render_plan_redraws_when_the_corner_moves() {
        let mut state = RenderState::default();
        state.plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN);
        let moved = state.plan(0, OsdRect::Name, "CAM1", Corner::LowerRight, MAIN);
        assert!(moved.is_some(), "a position change must redraw");
    }

    #[test]
    fn test_render_plan_rejects_non_ascii_without_poisoning_state() {
        let mut state = RenderState::default();
        assert!(
            state
                .plan(0, OsdRect::Name, "Ogród", Corner::UpperLeft, MAIN)
                .is_none()
        );
        // The bad value must not be recorded as "last drawn".
        assert!(
            state
                .plan(0, OsdRect::Name, "OK", Corner::UpperLeft, MAIN)
                .is_some()
        );
    }
}
