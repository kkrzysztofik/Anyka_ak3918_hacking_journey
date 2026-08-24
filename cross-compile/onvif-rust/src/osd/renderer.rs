//! 1 Hz OSD render planner.
//!
//! Pure decision logic lives here so unit tests never need a tokio timer. The
//! async task that ticks once a second is a thin wrapper around [`RenderState`].
//!
//! This camera's ISP only composites one OSD DMA plane per video channel (the
//! `osd_vpss_wrap` path drops the rect index). Name and datetime therefore
//! share silicon rect 0 as a full-frame canvas; each `draw_str` paints into
//! that canvas without wiping the other string.

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
    /// Padded erase length (may exceed the visible glyph count).
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
    /// Permanent give-up for invalid dimensions only (recoverable IPC failures retry).
    canvas_failed: [bool; 2],
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
        let raw_glyphs = match encode_glyphs(text) {
            Ok(g) => g,
            Err(_) => return None,
        };
        let raw_len = raw_glyphs.len();

        let slot = &mut self.last[channel as usize][rect as usize];
        let previous_len = slot.as_ref().map(|p| p.glyph_len).unwrap_or(0);
        let prev = slot.clone();
        let content_changed = match prev.as_ref() {
            None => true,
            Some(p) => p.text != text || p.corner != corner,
        };
        // Pad for erase coverage, but place with the unpadded count so a
        // shrinking right-anchored string stays flush to the corner.
        let glyphs = pad_to_erase(raw_glyphs, previous_len);
        let font = FontMetrics::for_channel(channel);
        let placement = place(corner, raw_len, dims, font);

        let erase = match prev.as_ref() {
            Some(p)
                if p.glyph_len > 0
                    && (p.corner != corner
                        || p.draw_x != placement.draw_x
                        || p.draw_y != placement.draw_y) =>
            {
                Some(ErasePlan {
                    draw_x: p.draw_x,
                    draw_y: p.draw_y,
                    glyphs: vec![0x20; p.glyph_len],
                })
            }
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

    /// Space-fill one overlay's last draw and forget it.
    ///
    /// Needed because both overlays share a single composited canvas: an
    /// overlay that is merely no longer redrawn keeps its last glyphs on screen
    /// forever. Returns `None` when nothing was drawn, so this is idempotent and
    /// safe to call every tick.
    pub fn clear(&mut self, channel: u8, rect: OsdRect) -> Option<ErasePlan> {
        if channel > 1 {
            return None;
        }
        let prev = self.last[channel as usize][rect as usize].take()?;
        if prev.glyph_len == 0 {
            return None;
        }
        Some(ErasePlan {
            draw_x: prev.draw_x,
            draw_y: prev.draw_y,
            glyphs: vec![0x20; prev.glyph_len],
        })
    }

    /// Turn the whole overlay off at the silicon rect.
    ///
    /// Cheaper and more complete than space-filling: an unenabled rect is not
    /// composited at all, so both overlays vanish in one call.
    pub async fn disable_all(&mut self, ipc: &AnykaIpc) {
        for channel in 0u8..=1 {
            let idx = channel as usize;
            if !self.canvas_ready[idx] {
                self.last[idx] = Default::default();
                continue;
            }
            if let Err(e) = ipc
                .osd_set_enable(i32::from(channel), CANVAS_RECT, false)
                .await
            {
                warn!(error = %e, channel, "osd_set_enable(false) failed; overlay may persist");
            }
            self.canvas_ready[idx] = false;
            self.last[idx] = Default::default();
        }
    }

    async fn ensure_canvas(
        &mut self,
        channel: u8,
        dims: ChannelDims,
        ipc: &AnykaIpc,
        vi: u64,
    ) -> bool {
        let idx = channel as usize;
        if self.canvas_ready[idx] {
            return true;
        }
        if self.canvas_failed[idx] {
            return false;
        }
        if dims.width <= 0 || dims.height <= 0 || dims.width > 4096 || dims.height > 4096 {
            warn!(
                channel,
                width = dims.width,
                height = dims.height,
                "osd canvas dims invalid; skipping channel"
            );
            self.canvas_failed[idx] = true;
            return false;
        }
        if let Err(e) = ipc
            .osd_set_rect(vi, i32::from(channel), CANVAS_RECT, canvas_rect(dims))
            .await
        {
            warn!(error = %e, channel, "osd_set_rect (canvas) failed; will retry");
            return false;
        }
        if let Err(e) = ipc
            .osd_set_enable(i32::from(channel), CANVAS_RECT, true)
            .await
        {
            warn!(error = %e, channel, "osd_set_enable(true) failed; will retry");
            return false;
        }
        self.canvas_ready[idx] = true;
        // Replay: forget prior plans so unchanged overlays are redrawn after recovery.
        self.last[idx] = Default::default();
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
                tick_once(&mut state, &args, &mut last_style).await;
            }
        }
    }
}

async fn tick_once(
    state: &mut RenderState,
    args: &OsdRendererArgs,
    last_style: &mut Option<(u8, u8)>,
) {
    let cfg = args.config.read().clone();
    if !cfg.enabled {
        // Not just "stop drawing": the canvas is persistent, so returning early
        // would freeze the last timestamp on the video forever.
        state.disable_all(&args.ipc).await;
        return;
    }

    let style = (cfg.color, cfg.alpha);
    if last_style.as_ref() != Some(&style) {
        if let Err(e) = args
            .ipc
            .osd_set_style(i32::from(cfg.color), 0, 0, i32::from(cfg.alpha))
            .await
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
        if dims.width <= 0 || dims.height <= 0 {
            continue;
        }

        let has_slots =
            state.last[channel as usize][0].is_some() || state.last[channel as usize][1].is_some();
        if !cfg.name.enabled && !cfg.datetime.enabled && !has_slots {
            continue;
        }

        // Gate canvas before plan/clear so a failed setup does not poison SlotMemory.
        let vi = args.vi.as_ptr() as u64;
        if !state.ensure_canvas(channel, dims, &args.ipc, vi).await {
            continue;
        }

        let mut plans = Vec::new();
        // Overlays switched off individually must be wiped from the shared
        // canvas; the other overlay keeps it composited, so `disable_all` is
        // not an option here.
        let mut erases = Vec::new();

        if cfg.name.enabled {
            if let Some(plan) =
                state.plan(channel, OsdRect::Name, name_text, cfg.name.position, dims)
            {
                plans.push(plan);
            }
        } else if let Some(erase) = state.clear(channel, OsdRect::Name) {
            erases.push(erase);
        }

        if cfg.datetime.enabled {
            let when = Local::now();
            let text = format_datetime(when, cfg.datetime.date_format, cfg.datetime.time_format);
            if let Some(plan) = state.plan(
                channel,
                OsdRect::DateTime,
                &text,
                cfg.datetime.position,
                dims,
            ) {
                plans.push(plan);
            }
        } else if let Some(erase) = state.clear(channel, OsdRect::DateTime) {
            erases.push(erase);
        }

        if plans.is_empty() && erases.is_empty() {
            continue;
        }

        // Wipe switched-off overlays first, so a same-tick redraw of the other
        // overlay cannot be clobbered by the erase.
        for erase in &erases {
            if let Err(e) = args
                .ipc
                .osd_draw_str(
                    i32::from(channel),
                    CANVAS_RECT,
                    erase.draw_x,
                    erase.draw_y,
                    &erase.glyphs,
                )
                .await
            {
                warn!(error = %e, channel, "osd_draw_str disable-erase failed");
            }
        }

        // Paint into the shared canvas. Order does not matter for persistence —
        // each draw_str only updates its glyph region — but draw datetime last
        // so the final ISP flush of the tick includes the fresh timestamp.
        plans.sort_by_key(|p| p.rect as i32);
        for plan in &plans {
            if let Some(erase) = &plan.erase
                && let Err(e) = args
                    .ipc
                    .osd_draw_str(
                        i32::from(plan.channel),
                        CANVAS_RECT,
                        erase.draw_x,
                        erase.draw_y,
                        &erase.glyphs,
                    )
                    .await
            {
                warn!(error = %e, channel, "osd_draw_str erase failed");
            }
            if !plan.content_changed {
                continue;
            }
            if let Err(e) = args
                .ipc
                .osd_draw_str(
                    i32::from(plan.channel),
                    CANVAS_RECT,
                    plan.draw_x,
                    plan.draw_y,
                    &plan.glyphs,
                )
                .await
            {
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
    fn test_render_plan_right_anchored_shrink_places_by_unpadded_count() {
        let mut state = RenderState::default();
        let long = state
            .plan(0, OsdRect::Name, "LONG NAME", Corner::UpperRight, MAIN)
            .unwrap();
        let short = state
            .plan(0, OsdRect::Name, "AB", Corner::UpperRight, MAIN)
            .unwrap();
        let font = FontMetrics::for_channel(0);
        let expected = place(Corner::UpperRight, 2, MAIN, font);
        assert_eq!(short.draw_x, expected.draw_x, "flush to the right corner");
        assert!(
            short.draw_x > long.draw_x,
            "shorter text sits further right"
        );
        assert_eq!(short.glyphs.len(), 9, "padded erase length retained");
        assert!(short.erase.is_some(), "old left residue must be erased");
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
    fn test_clear_erases_a_disabled_overlay_at_its_last_position() {
        // Both overlays share one composited canvas, so an overlay that simply
        // stops being redrawn stays on screen. Disabling must space-fill it.
        let mut state = RenderState::default();
        let drawn = state
            .plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN)
            .unwrap();

        let erase = state.clear(0, OsdRect::Name).expect("must erase");
        assert_eq!(erase.glyphs, vec![0x20; 4]);
        assert_eq!((erase.draw_x, erase.draw_y), (drawn.draw_x, drawn.draw_y));
    }

    #[test]
    fn test_clear_is_idempotent_so_it_can_run_every_tick() {
        let mut state = RenderState::default();
        state.plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN);

        assert!(state.clear(0, OsdRect::Name).is_some());
        assert!(
            state.clear(0, OsdRect::Name).is_none(),
            "a cleared slot must not keep re-erasing at 1 Hz"
        );
    }

    #[test]
    fn test_clear_of_a_never_drawn_overlay_is_a_noop() {
        let mut state = RenderState::default();
        assert!(state.clear(0, OsdRect::DateTime).is_none());
    }

    #[tokio::test]
    async fn test_disable_all_turns_the_rect_off_and_forces_a_full_redraw() {
        use crate::hal::anyka::ipc::test_helpers::*;

        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let sink = seen.clone();
        let daemon = FakeDaemon::start(move |_cmd, req| {
            sink.lock().unwrap().push(req.to_vec());
            (0, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let mut state = RenderState::default();
        state.plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN);
        state.canvas_ready[0] = true;

        state.disable_all(&ipc).await;

        // [i32 channel][i32 rect][i32 enable] — enable must be 0.
        let last = seen.lock().unwrap().last().cloned().expect("an IPC call");
        assert_eq!(&last[8..12], &0i32.to_le_bytes(), "must disable the rect");
        assert!(!state.canvas_ready[0], "re-enabling must redo set_rect");
        assert!(
            state.clear(0, OsdRect::Name).is_none(),
            "slot memory must be dropped so the overlay fully redraws"
        );
    }

    #[test]
    fn test_name_and_datetime_share_independent_slots() {
        let mut state = RenderState::default();
        let name = state
            .plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN)
            .unwrap();
        let dt = state
            .plan(
                0,
                OsdRect::DateTime,
                "2026-08-24 12:00:00",
                Corner::LowerRight,
                MAIN,
            )
            .unwrap();
        assert_eq!(name.draw_y, 0);
        assert_eq!(dt.draw_y, 720 - 32);
        // Second datetime tick changes text only — name slot untouched.
        let dt2 = state
            .plan(
                0,
                OsdRect::DateTime,
                "2026-08-24 12:00:01",
                Corner::LowerRight,
                MAIN,
            )
            .unwrap();
        assert!(dt2.content_changed);
        let name2 = state
            .plan(0, OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN)
            .unwrap();
        assert!(!name2.content_changed);
    }

    // ---- tick_once: the integration point of config, state and IPC --------
    //
    // Command IDs are duplicated as literals because the `CMD_OSD_*` consts are
    // private to hal::anyka::ipc. See protocol.h.
    const CMD_SET_RECT: i32 = 23;
    const CMD_DRAW_STR: i32 = 24;
    const CMD_SET_ENABLE: i32 = 25;
    const CMD_SET_STYLE: i32 = 26;

    type Seen = std::sync::Arc<std::sync::Mutex<Vec<(i32, Vec<u8>)>>>;

    /// A daemon that records every command, plus wired-up renderer args.
    ///
    /// The `FakeDaemon` is returned so the caller keeps it alive for the test.
    fn tick_harness(
        cfg: OsdConfig,
    ) -> (
        RenderState,
        OsdRendererArgs,
        Seen,
        crate::hal::anyka::ipc::test_helpers::FakeDaemon,
    ) {
        use crate::hal::anyka::ipc::test_helpers::*;

        let seen: Seen = Default::default();
        let sink = seen.clone();
        let daemon = FakeDaemon::start(move |cmd, req| {
            sink.lock().unwrap().push((cmd, req.to_vec()));
            (0, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let (_tx, rx) = broadcast::channel(1);
        let args = OsdRendererArgs {
            ipc: Arc::new(ipc),
            vi: Arc::new(crate::hal::common::video::VideoInputHandle::test_handle()),
            dims: [
                MAIN,
                ChannelDims {
                    width: 640,
                    height: 360,
                },
            ],
            config: Arc::new(parking_lot::RwLock::new(cfg)),
            device_name: "ipcam".into(),
            shutdown: rx,
        };
        (RenderState::default(), args, seen, daemon)
    }

    fn count_of(seen: &Seen, cmd: i32) -> usize {
        seen.lock()
            .unwrap()
            .iter()
            .filter(|(c, _)| *c == cmd)
            .count()
    }

    #[tokio::test]
    async fn test_tick_draws_both_overlays_on_both_channels() {
        let (mut state, args, seen, _d) = tick_harness(OsdConfig::default());
        let mut style = None;

        tick_once(&mut state, &args, &mut style).await;

        assert_eq!(count_of(&seen, CMD_SET_RECT), 2, "one canvas per channel");
        assert_eq!(count_of(&seen, CMD_DRAW_STR), 4, "2 overlays x 2 channels");
        assert_eq!(count_of(&seen, CMD_SET_STYLE), 1);
    }

    #[tokio::test]
    async fn test_tick_pushes_style_once_not_every_second() {
        let (mut state, args, seen, _d) = tick_harness(OsdConfig::default());
        let mut style = None;

        tick_once(&mut state, &args, &mut style).await;
        tick_once(&mut state, &args, &mut style).await;

        assert_eq!(count_of(&seen, CMD_SET_STYLE), 1, "style is not per-tick");
    }

    #[tokio::test]
    async fn test_tick_disables_the_rect_when_osd_is_switched_off_globally() {
        // The regression this guards: returning early left the canvas enabled
        // with the last timestamp frozen on the video forever.
        let (mut state, args, seen, _d) = tick_harness(OsdConfig::default());
        let mut style = None;
        tick_once(&mut state, &args, &mut style).await;
        seen.lock().unwrap().clear();

        args.config.write().enabled = false;
        tick_once(&mut state, &args, &mut style).await;

        let disables: Vec<Vec<u8>> = seen
            .lock()
            .unwrap()
            .iter()
            .filter(|(c, _)| *c == CMD_SET_ENABLE)
            .map(|(_, r)| r.clone())
            .collect();
        assert_eq!(disables.len(), 2, "one per channel");
        for req in disables {
            assert_eq!(&req[8..12], &0i32.to_le_bytes(), "enable flag must be 0");
        }
    }

    #[tokio::test]
    async fn test_tick_erases_an_individually_disabled_overlay() {
        // The canvas stays composited for the other overlay, so this one has to
        // be painted over with spaces rather than just skipped.
        let (mut state, args, seen, _d) = tick_harness(OsdConfig::default());
        let mut style = None;
        tick_once(&mut state, &args, &mut style).await;
        seen.lock().unwrap().clear();

        args.config.write().name.enabled = false;
        tick_once(&mut state, &args, &mut style).await;

        let all_spaces = seen
            .lock()
            .unwrap()
            .iter()
            .filter(|(c, _)| *c == CMD_DRAW_STR)
            .any(|(_, r)| {
                let count = u16::from_le_bytes([r[16], r[17]]) as usize;
                count > 0 && r[18..18 + count * 2].chunks(2).all(|g| g == [0x20, 0x00])
            });
        assert!(all_spaces, "disabled overlay must be space-filled");
    }

    #[tokio::test]
    async fn test_tick_is_a_noop_once_everything_is_already_off() {
        let (mut state, args, seen, _d) = tick_harness(OsdConfig::default());
        let mut style = None;
        tick_once(&mut state, &args, &mut style).await;
        args.config.write().enabled = false;
        tick_once(&mut state, &args, &mut style).await;
        seen.lock().unwrap().clear();

        tick_once(&mut state, &args, &mut style).await;

        assert!(
            seen.lock().unwrap().is_empty(),
            "a disabled OSD must not chatter at 1 Hz"
        );
    }

    #[tokio::test]
    async fn test_tick_once_redraws_name_after_a_failed_canvas_tick() {
        use crate::hal::anyka::ipc::test_helpers::*;
        use crate::hal::common::AK_FAILED_I32;
        use std::sync::atomic::{AtomicBool, Ordering};

        let fail_rect = std::sync::Arc::new(AtomicBool::new(true));
        let fail_flag = fail_rect.clone();
        let seen: Seen = Default::default();
        let sink = seen.clone();
        let daemon = FakeDaemon::start(move |cmd, req| {
            sink.lock().unwrap().push((cmd, req.to_vec()));
            if cmd == CMD_SET_RECT && fail_flag.swap(false, Ordering::SeqCst) {
                return (AK_FAILED_I32, vec![]);
            }
            (0, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let mut cfg = OsdConfig::default();
        cfg.datetime.enabled = false;
        cfg.name.text = "CAM1".into();

        let (_tx, rx) = broadcast::channel(1);
        let args = OsdRendererArgs {
            ipc: Arc::new(ipc),
            vi: Arc::new(crate::hal::common::video::VideoInputHandle::test_handle()),
            dims: [
                MAIN,
                ChannelDims {
                    width: 0,
                    height: 0,
                },
            ],
            config: Arc::new(parking_lot::RwLock::new(cfg)),
            device_name: "ipcam".into(),
            shutdown: rx,
        };
        let mut state = RenderState::default();
        let mut style = None;

        tick_once(&mut state, &args, &mut style).await;
        assert_eq!(
            count_of(&seen, CMD_DRAW_STR),
            0,
            "failed canvas must skip draws"
        );
        assert!(!state.canvas_ready[0]);
        assert!(!state.canvas_failed[0], "IPC failure must remain retryable");

        tick_once(&mut state, &args, &mut style).await;
        assert!(
            count_of(&seen, CMD_DRAW_STR) >= 1,
            "name must redraw after recovery"
        );
        assert!(state.canvas_ready[0]);
    }
}
