//! Night-mode control: IR cut filter, IR illuminator, white floodlight.
//!
//! The write ordering and the two-line coil-idle pulse are taken from the
//! vendor reference (`anyka_reference/libre_anyka_app/main.c:740-752` and
//! `platform/libplat/src/drv/ak_drv_ir.c:230-255`) and are load-bearing.
//! See `docs/plans/2026-08-02-ir-led-support-design.md`.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Delay after a mode switch, letting the solenoid and ISP settle.
pub(super) const SETTLE: Duration = Duration::from_millis(300);

/// Width of the two-line coil pulse before both lines return to idle.
const PULSE: Duration = Duration::from_millis(10);

/// Day or night target state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DayNight {
    Day,
    Night,
}

/// A GPIO node this module drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Node {
    IrCutA,
    IrCutB,
    IrLed,
    WhiteLed,
}

/// How many lines the IR cut filter solenoid is wired with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineMode {
    /// Single node, level written directly.
    One(Node),
    /// H-bridge: opposed pulse, then both lines back to zero.
    Two,
}

/// One step of a transition plan.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Step {
    Write {
        node: Node,
        value: u8,
    },
    /// Cross the IPC boundary to `ak_vi_switch_mode`.
    IspMode(DayNight),
    Sleep(Duration),
}

/// Board wiring polarity.
#[derive(Debug, Clone, Copy)]
pub(super) struct Polarity {
    /// `true` when writing `1` to the ircut node selects night.
    pub ircut_high_is_night: bool,
}

/// Light-sensor thresholds. Both values must be calibrated per board:
/// the vendor defaults leave this camera's resting reading in an
/// unhandled dead zone.
#[derive(Debug, Clone, Copy)]
pub(super) struct Thresholds {
    /// At or above this raw reading, the sensor is saturated one way.
    pub day: i32,
    /// At or below this raw reading, the sensor is saturated the other way.
    pub night: i32,
    /// `true` when a high raw reading means daylight.
    pub ldr_high_is_day: bool,
}

/// Result of classifying one sensor reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reading {
    Settled(DayNight),
    /// Inside the hysteresis band; the caller must hold its current mode.
    Indeterminate,
}

/// Current AUTO-mode state: what the camera is set to, and when it last moved.
#[derive(Debug, Clone, Copy)]
pub(super) struct AutoState {
    current: DayNight,
    last_change: Option<std::time::Instant>,
}

impl AutoState {
    pub(super) fn new(current: DayNight) -> Self {
        Self {
            current,
            last_change: None,
        }
    }

    pub(super) fn current(&self) -> DayNight {
        self.current
    }

    /// Record that a transition has been applied.
    pub(super) fn record_change(&mut self, to: DayNight, at: std::time::Instant) {
        self.current = to;
        self.last_change = Some(at);
    }
}

/// Decide whether to transition, given a reading and the lock window.
///
/// Returns `None` to hold the current mode. An `Indeterminate` reading always
/// holds; so does any reading inside `lock` of the last transition, which is
/// what stops a camera oscillating at dusk.
pub(super) fn decide(
    state: &AutoState,
    reading: Reading,
    now: std::time::Instant,
    lock: Duration,
) -> Option<DayNight> {
    let Reading::Settled(target) = reading else {
        return None;
    };
    if target == state.current {
        return None;
    }
    if let Some(last) = state.last_change
        && now.duration_since(last) < lock
    {
        return None;
    }
    Some(target)
}

/// Map a raw `ain0` reading to a day/night conclusion.
pub(super) fn classify(raw: i32, thr: Thresholds) -> Reading {
    let (high, low) = if thr.ldr_high_is_day {
        (DayNight::Day, DayNight::Night)
    } else {
        (DayNight::Night, DayNight::Day)
    };

    if raw >= thr.day {
        Reading::Settled(high)
    } else if raw <= thr.night {
        Reading::Settled(low)
    } else {
        Reading::Indeterminate
    }
}

/// Filesystem locations of the nodes this module drives.
///
/// Held as two base directories rather than six paths so tests can point the
/// whole set at a `tempdir` with one call.
#[derive(Debug, Clone)]
pub(crate) struct NodePaths {
    user_gpio: PathBuf,
    kernel_ain: PathBuf,
}

impl Default for NodePaths {
    fn default() -> Self {
        Self {
            user_gpio: PathBuf::from("/sys/user-gpio"),
            kernel_ain: PathBuf::from("/sys/kernel/ain"),
        }
    }
}

impl NodePaths {
    /// Point both trees at explicit roots. Tests pass a `tempdir`.
    pub(crate) fn rooted(user_gpio: &Path, kernel_ain: &Path) -> Self {
        Self {
            user_gpio: user_gpio.to_path_buf(),
            kernel_ain: kernel_ain.to_path_buf(),
        }
    }

    /// Path of a GPIO node. Names are this board's, not the vendor
    /// reference's `gpio-` prefixed ones. See design H1.
    pub(super) fn node(&self, node: Node) -> PathBuf {
        let name = match node {
            Node::IrCutA => "ircut_a",
            Node::IrCutB => "ircut_b",
            Node::IrLed => "IR_LED",
            Node::WhiteLed => "WHITE_LED",
        };
        self.user_gpio.join(name)
    }

    /// Path of the light sensor. `gpio-rf_feed` does not exist on this
    /// board, so the ain fallback is the only source. See design H2.
    pub(super) fn light_sensor(&self) -> PathBuf {
        self.kernel_ain.join("ain0")
    }
}

/// What the hardware actually supports, discovered at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Capabilities {
    /// `None` when no ircut node exists — the filter is unsupported.
    pub line_mode: Option<LineMode>,
    pub ir_led: bool,
    pub white_led: bool,
}

/// Probe for the nodes. One `stat()` per node answers both the wiring
/// question and the ONVIF capability question.
pub(crate) fn probe(paths: &NodePaths) -> Capabilities {
    let a = paths.node(Node::IrCutA).exists();
    let b = paths.node(Node::IrCutB).exists();

    let line_mode = match (a, b) {
        (true, true) => Some(LineMode::Two),
        (true, false) => Some(LineMode::One(Node::IrCutA)),
        (false, true) => Some(LineMode::One(Node::IrCutB)),
        (false, false) => None,
    };

    Capabilities {
        line_mode,
        ir_led: paths.node(Node::IrLed).exists(),
        white_led: paths.node(Node::WhiteLed).exists(),
    }
}

/// Execute the GPIO and sleep steps of a plan.
///
/// [`Step::IspMode`] is skipped here; the caller performs it over IPC, because
/// it needs the vendor daemon's VI handle.
///
/// Every step runs even if an earlier one failed, and the first error is
/// returned at the end. This is deliberate and must not be "cleaned up" into
/// `?`: in [`LineMode::Two`] the solenoid coil is energised between the pulse
/// and the trailing zero writes, and an early return leaves it that way.
pub(super) fn execute_gpio(steps: &[Step], paths: &NodePaths) -> std::io::Result<()> {
    let mut first_err: Option<std::io::Error> = None;

    for step in steps {
        match step {
            Step::Write { node, value } => {
                let path = paths.node(*node);
                if let Err(e) = std::fs::write(&path, value.to_string()) {
                    tracing::warn!(path = %path.display(), value, error = %e, "GPIO write failed");
                    first_err.get_or_insert(e);
                }
            }
            Step::Sleep(d) => std::thread::sleep(*d),
            Step::IspMode(_) => {}
        }
    }

    first_err.map_or(Ok(()), Err)
}

/// Read the raw light-sensor value.
///
/// Returns `None` on any failure. The caller must treat that as "hold the
/// current mode" — never as day, which would switch off night vision the
/// moment the sensor node hiccups.
pub(super) fn read_light_sensor(paths: &NodePaths) -> Option<i32> {
    let path = paths.light_sensor();
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| tracing::warn!(path = %path.display(), error = %e, "light sensor read failed"))
        .ok()?;
    raw.trim().parse::<i32>().ok()
}

/// AUTO poll interval. Fixed: lock_time dominates responsiveness.
pub(crate) const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Consecutive AE read failures before falling back to `ain0`.
const AE_FAIL_STREAK_MAX: u32 = 3;

/// Owns the night-mode state and serialises every transition.
pub(crate) struct NightModeController {
    paths: NodePaths,
    caps: Capabilities,
    cfg: crate::config::types::NightConfig,
    ffi: std::sync::Arc<dyn crate::hal::common::imaging::ImagingHalTrait>,
    state: tokio::sync::Mutex<AutoState>,
    /// Mode from `[imaging] ir_cut_filter`, applied once at start-up.
    configured: crate::onvif::types::common::IrCutFilterMode,
    /// When true, [`Self::tick`] may transition; cleared by forced ON/OFF.
    auto_enabled: std::sync::atomic::AtomicBool,
    /// Consecutive `get_ae_luma` failures; resets on success.
    ae_fail_streak: std::sync::atomic::AtomicU32,
}

impl NightModeController {
    /// Build a controller for the configured start-up mode.
    ///
    /// A configured `ON`/`OFF` disables the AUTO loop from the first tick, so a
    /// forced mode survives a restart instead of silently reverting to AUTO.
    /// [`Self::spawn_auto_loop`] applies it to the hardware.
    pub(crate) fn new(
        paths: NodePaths,
        cfg: crate::config::types::NightConfig,
        ffi: std::sync::Arc<dyn crate::hal::common::imaging::ImagingHalTrait>,
        mode: crate::onvif::types::common::IrCutFilterMode,
    ) -> Self {
        use crate::onvif::types::common::IrCutFilterMode;

        let caps = probe(&paths);
        Self {
            paths,
            caps,
            cfg,
            ffi,
            state: tokio::sync::Mutex::new(AutoState::new(match mode {
                IrCutFilterMode::OFF => DayNight::Night,
                _ => DayNight::Day,
            })),
            configured: mode,
            auto_enabled: std::sync::atomic::AtomicBool::new(matches!(mode, IrCutFilterMode::AUTO)),
            ae_fail_streak: std::sync::atomic::AtomicU32::new(0),
        }
    }

    pub(crate) fn capabilities(&self) -> Capabilities {
        self.caps
    }

    /// The day/night state the hardware was last driven to.
    pub(crate) async fn current_mode(&self) -> DayNight {
        self.state.lock().await.current()
    }

    pub(crate) fn set_auto_enabled(&self, enabled: bool) {
        self.auto_enabled
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
    }

    /// Apply a transition. The mutex makes concurrent transitions
    /// impossible, which matters because the two-line coil must never be
    /// driven by two callers at once.
    pub(crate) async fn apply(
        &self,
        target: DayNight,
    ) -> crate::platform::common::PlatformResult<()> {
        use crate::platform::common::PlatformError;

        let mut state = self.state.lock().await;

        let mut steps = plan(
            target,
            Polarity {
                ircut_high_is_night: self.cfg.ircut_high_is_night,
            },
            self.caps.line_mode,
        );
        // A board without the lamp node would only collect a write error.
        if !self.caps.ir_led {
            steps.retain(|s| {
                !matches!(
                    s,
                    Step::Write {
                        node: Node::IrLed,
                        ..
                    }
                )
            });
        }

        // GPIO first: it is durable and survives a daemon restart, whereas
        // the ISP mode does not. A bounce then costs one tick of
        // wrong-looking image rather than a stuck filter.
        let paths = self.paths.clone();
        let gpio_steps = steps.clone();
        let gpio_result =
            tokio::task::spawn_blocking(move || execute_gpio(&gpio_steps, &paths)).await;

        // The ISP step is the only part that needs the daemon.
        let isp = self
            .ffi
            .set_ir_filter(matches!(target, DayNight::Night))
            .await;

        state.record_change(target, std::time::Instant::now());

        if isp != 0 {
            tracing::warn!(isp, "ISP day/night switch failed; will retry next tick");
        }
        match gpio_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(PlatformError::HardwareFailure(format!("GPIO write: {e}"))),
            Err(e) => Err(PlatformError::HardwareFailure(format!("join: {e}"))),
        }
    }

    /// One AUTO poll: prefer AE luma, fall back to `ain0` after streak failures.
    pub(crate) async fn tick(&self) {
        use std::sync::atomic::Ordering;

        if !self.auto_enabled.load(Ordering::SeqCst) {
            return;
        }

        let reading = match self.ffi.get_ae_luma().await {
            Some(luma) => {
                self.ae_fail_streak.store(0, Ordering::SeqCst);
                classify(
                    i32::from(luma),
                    Thresholds {
                        day: self.cfg.ae_day_threshold,
                        night: self.cfg.ae_night_threshold,
                        // AE high = bright = day.
                        ldr_high_is_day: true,
                    },
                )
            }
            None => {
                let n = self.ae_fail_streak.fetch_add(1, Ordering::SeqCst) + 1;
                if n < AE_FAIL_STREAK_MAX {
                    return;
                }
                if n == AE_FAIL_STREAK_MAX {
                    tracing::warn!(streak = n, "AE luma unavailable; falling back to ain0");
                }
                let Some(raw) = read_light_sensor(&self.paths) else {
                    return;
                };
                classify(
                    raw,
                    Thresholds {
                        day: self.cfg.day_threshold,
                        night: self.cfg.night_threshold,
                        ldr_high_is_day: self.cfg.ldr_high_is_day,
                    },
                )
            }
        };

        let target = {
            let state = self.state.lock().await;
            decide(
                &state,
                reading,
                std::time::Instant::now(),
                Duration::from_millis(self.cfg.lock_time_ms),
            )
        };
        if let Some(target) = target
            && let Err(e) = self.apply(target).await
        {
            tracing::warn!(error = %e, "night-mode transition failed");
        }
    }

    /// Write a single lamp GPIO. Used by ONVIF auxiliary commands.
    pub(crate) fn write_lamp(&self, node: Node, on: bool) -> std::io::Result<()> {
        if matches!(node, Node::IrLed) && !self.caps.ir_led {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "IR_LED node absent",
            ));
        }
        if matches!(node, Node::WhiteLed) && !self.caps.white_led {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "WHITE_LED node absent",
            ));
        }
        std::fs::write(self.paths.node(node), if on { "1" } else { "0" })
    }

    /// Apply the configured start-up mode, then run the AUTO poll loop.
    ///
    /// Call once after construction. A configured `ON`/`OFF` is driven onto the
    /// hardware here; `AUTO` leaves the filter where it is until the first tick
    /// has a settled reading.
    pub(crate) fn spawn_auto_loop(self: &std::sync::Arc<Self>) {
        use crate::onvif::types::common::IrCutFilterMode;

        let this = std::sync::Arc::clone(self);
        tokio::spawn(async move {
            let forced = match this.configured {
                IrCutFilterMode::ON => Some(DayNight::Day),
                IrCutFilterMode::OFF => Some(DayNight::Night),
                IrCutFilterMode::AUTO => None,
            };
            if let Some(target) = forced
                && let Err(e) = this.apply(target).await
            {
                tracing::warn!(error = %e, ?target, "configured start-up night mode failed");
            }

            let mut interval = tokio::time::interval(POLL_INTERVAL);
            loop {
                interval.tick().await;
                this.tick().await;
            }
        });
    }
}

/// Build the ordered step list for a transition.
///
/// Ordering follows the vendor reference and is not arbitrary: the lamp turns
/// on before the ISP switches to night and off after it switches to day, so no
/// frame is captured dark. In [`LineMode::Two`] the trailing zero writes
/// de-energise the solenoid coil and are mandatory.
///
/// `line_mode` is `None` on a board with no ircut node at all. The filter steps
/// drop out; the lamp and ISP steps stay, because a board can have an
/// illuminator without a filter.
pub(super) fn plan(target: DayNight, pol: Polarity, line_mode: Option<LineMode>) -> Vec<Step> {
    let night_level = u8::from(pol.ircut_high_is_night);
    let ircut_level = match target {
        DayNight::Night => night_level,
        DayNight::Day => 1 - night_level,
    };

    let mut ircut = Vec::new();
    match line_mode {
        None => {}
        Some(LineMode::One(node)) => ircut.push(Step::Write {
            node,
            value: ircut_level,
        }),
        Some(LineMode::Two) => {
            ircut.push(Step::Write {
                node: Node::IrCutA,
                value: ircut_level,
            });
            ircut.push(Step::Write {
                node: Node::IrCutB,
                value: 1 - ircut_level,
            });
            ircut.push(Step::Sleep(PULSE));
            ircut.push(Step::Write {
                node: Node::IrCutA,
                value: 0,
            });
            ircut.push(Step::Write {
                node: Node::IrCutB,
                value: 0,
            });
        }
    }

    let mut steps = Vec::new();
    match target {
        DayNight::Night => {
            steps.push(Step::Write {
                node: Node::IrLed,
                value: 1,
            });
            steps.push(Step::IspMode(DayNight::Night));
            steps.extend(ircut);
        }
        DayNight::Day => {
            steps.extend(ircut);
            steps.push(Step::IspMode(DayNight::Day));
            steps.push(Step::Write {
                node: Node::IrLed,
                value: 0,
            });
        }
    }
    steps.push(Step::Sleep(SETTLE));
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pol() -> Polarity {
        Polarity {
            ircut_high_is_night: true,
        }
    }

    fn test_config() -> crate::config::types::NightConfig {
        crate::config::types::NightConfig::default()
    }

    /// AUTO tests need zero lock so a settled reading can switch immediately.
    fn unlocked_config() -> crate::config::types::NightConfig {
        crate::config::types::NightConfig {
            lock_time_ms: 0,
            ..Default::default()
        }
    }

    fn seed_gpio_nodes(paths: &NodePaths) {
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed] {
            std::fs::write(paths.node(n), "9").unwrap();
        }
    }

    #[tokio::test]
    async fn test_apply_night_calls_isp_switch_and_writes_gpios() {
        use crate::hal::common::imaging::MockImagingHalTrait;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed] {
            std::fs::write(paths.node(n), "9").unwrap();
        }

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_set_ir_filter()
            .withf(|enabled| *enabled)
            .times(1)
            .returning(|_| 0);

        let ctl = NightModeController::new(
            paths.clone(),
            test_config(),
            std::sync::Arc::new(ffi),
            crate::onvif::types::common::IrCutFilterMode::AUTO,
        );
        ctl.apply(DayNight::Night).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(),
            "1"
        );
    }

    #[tokio::test]
    async fn test_apply_still_writes_gpios_when_the_isp_call_fails() {
        use crate::hal::common::imaging::MockImagingHalTrait;

        // A daemon restart makes the IPC call fail. GPIO state is durable and
        // must still be correct, so the next tick only has to redo the ISP.
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed] {
            std::fs::write(paths.node(n), "9").unwrap();
        }

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_set_ir_filter().returning(|_| -1);

        let ctl = NightModeController::new(
            paths.clone(),
            test_config(),
            std::sync::Arc::new(ffi),
            crate::onvif::types::common::IrCutFilterMode::AUTO,
        );
        let _ = ctl.apply(DayNight::Night).await;

        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(),
            "1"
        );
    }

    #[tokio::test]
    async fn test_configured_off_mode_starts_at_night_with_auto_disabled() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed] {
            std::fs::write(paths.node(n), "9").unwrap();
        }
        // A sensor reading that would otherwise pull the camera back to day.
        std::fs::write(paths.light_sensor(), "5000").unwrap();

        let ctl = NightModeController::new(
            paths.clone(),
            test_config(),
            std::sync::Arc::new(MockImagingHalTrait::new()),
            IrCutFilterMode::OFF,
        );

        assert_eq!(ctl.current_mode().await, DayNight::Night);

        // The AUTO loop must not undo a configured forced mode.
        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, DayNight::Night);
    }

    #[tokio::test]
    async fn test_tick_uses_ae_luma_when_available() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        seed_gpio_nodes(&paths);
        // ain0 bright enough to be day — AE must win with night luma.
        std::fs::write(paths.light_sensor(), "1500").unwrap();

        let mut ffi = MockImagingHalTrait::new();
        // Below ae_night_threshold (8); ain0=1500 would otherwise be day.
        ffi.expect_get_ae_luma().times(1).returning(|| Some(1));
        ffi.expect_set_ir_filter()
            .withf(|enabled| *enabled)
            .times(1)
            .returning(|_| 0);

        let ctl = NightModeController::new(
            paths,
            unlocked_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
        );
        assert_eq!(ctl.current_mode().await, DayNight::Day);

        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, DayNight::Night);
    }

    #[tokio::test]
    async fn test_tick_falls_back_to_ain0_after_three_ae_failures() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        seed_gpio_nodes(&paths);
        std::fs::write(paths.light_sensor(), "100").unwrap();

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_get_ae_luma().times(3).returning(|| None);
        ffi.expect_set_ir_filter()
            .withf(|enabled| *enabled)
            .times(1)
            .returning(|_| 0);

        let ctl = NightModeController::new(
            paths,
            unlocked_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
        );

        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, DayNight::Day);
        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, DayNight::Day);
        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, DayNight::Night);
    }

    #[tokio::test]
    async fn test_tick_clears_streak_on_ae_success() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        seed_gpio_nodes(&paths);
        // ain0 dark — would force night if streak reached fallback.
        std::fs::write(paths.light_sensor(), "100").unwrap();

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_get_ae_luma().times(3).returning({
            let mut calls = 0u8;
            move || {
                calls += 1;
                if calls < 3 {
                    None
                } else {
                    Some(100) // bright AE → day
                }
            }
        });
        // Must not apply night via ain0 fallback.
        ffi.expect_set_ir_filter().times(0);

        let ctl = NightModeController::new(
            paths,
            unlocked_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
        );

        ctl.tick().await;
        ctl.tick().await;
        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, DayNight::Day);
    }

    #[test]
    fn test_plan_without_ircut_hardware_still_drives_the_lamp() {
        let steps = plan(DayNight::Night, pol(), None);

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode(DayNight::Night),
                Step::Sleep(SETTLE),
            ]
        );
    }

    #[tokio::test]
    async fn test_apply_lights_the_lamp_on_a_board_without_an_ircut_node() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrLed), "9").unwrap();

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_set_ir_filter().returning(|_| 0);

        let ctl = NightModeController::new(
            paths.clone(),
            test_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
        );
        ctl.apply(DayNight::Night).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(),
            "1"
        );
    }

    #[tokio::test]
    async fn test_apply_skips_the_lamp_write_when_the_node_is_absent() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "9").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "9").unwrap();

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_set_ir_filter().returning(|_| 0);

        let ctl = NightModeController::new(
            paths.clone(),
            test_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
        );

        // No IR_LED node: the filter still moves and no bogus error is raised.
        ctl.apply(DayNight::Night).await.unwrap();
        assert!(!paths.node(Node::IrLed).exists());
    }

    fn thr() -> Thresholds {
        Thresholds {
            day: 1100,
            night: 300,
            ldr_high_is_day: true,
        }
    }

    #[test]
    fn test_probe_reports_two_line_when_both_ircut_nodes_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "0").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "0").unwrap();
        std::fs::write(paths.node(Node::IrLed), "0").unwrap();

        let caps = probe(&paths);

        assert_eq!(caps.line_mode, Some(LineMode::Two));
        assert!(caps.ir_led);
    }

    #[test]
    fn test_probe_reports_one_line_when_only_ircut_a_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "0").unwrap();

        let caps = probe(&paths);

        assert_eq!(caps.line_mode, Some(LineMode::One(Node::IrCutA)));
        assert!(!caps.ir_led);
    }

    #[test]
    fn test_probe_reports_unsupported_when_no_ircut_nodes_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());

        let caps = probe(&paths);

        assert_eq!(caps.line_mode, None);
    }

    #[test]
    fn test_execute_writes_final_values_to_nodes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "9").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "9").unwrap();
        std::fs::write(paths.node(Node::IrLed), "9").unwrap();

        let steps = plan(DayNight::Night, pol(), Some(LineMode::Two));
        let outcome = execute_gpio(&steps, &paths);

        assert!(outcome.is_ok());
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(),
            "1"
        );
        // Both coil lines back to idle.
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrCutA)).unwrap(),
            "0"
        );
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrCutB)).unwrap(),
            "0"
        );
    }

    #[test]
    fn test_execute_still_idles_the_coil_when_an_earlier_write_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        // IrLed is a directory so its write fails (sysfs missing nodes also fail;
        // a plain absent path would be created by fs::write on a tempdir).
        std::fs::write(paths.node(Node::IrCutA), "9").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "9").unwrap();
        std::fs::create_dir(paths.node(Node::IrLed)).unwrap();

        let steps = plan(DayNight::Night, pol(), Some(LineMode::Two));
        let outcome = execute_gpio(&steps, &paths);

        assert!(outcome.is_err(), "the failed IrLed write must be reported");
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrCutA)).unwrap(),
            "0",
            "coil must be de-energised even though an earlier step failed"
        );
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrCutB)).unwrap(),
            "0"
        );
    }

    #[test]
    fn test_read_light_sensor_parses_raw_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.light_sensor(), "306\n").unwrap();

        assert_eq!(read_light_sensor(&paths).unwrap(), 306);
    }

    #[test]
    fn test_read_light_sensor_reports_error_when_node_is_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());

        assert!(read_light_sensor(&paths).is_none());
    }

    #[test]
    fn test_read_light_sensor_reports_error_on_unparseable_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.light_sensor(), "not a number").unwrap();

        assert!(read_light_sensor(&paths).is_none());
    }

    #[test]
    fn test_one_line_ircut_b_only_board_plans_write_to_ircut_b() {
        let steps = plan(DayNight::Night, pol(), Some(LineMode::One(Node::IrCutB)));

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode(DayNight::Night),
                Step::Write {
                    node: Node::IrCutB,
                    value: 1
                },
                Step::Sleep(SETTLE),
            ]
        );
    }

    #[test]
    fn test_classify_bright_reading_is_day() {
        assert_eq!(classify(1500, thr()), Reading::Settled(DayNight::Day));
    }

    #[test]
    fn test_classify_dark_reading_is_night() {
        assert_eq!(classify(120, thr()), Reading::Settled(DayNight::Night));
    }

    #[test]
    fn test_classify_reading_in_hysteresis_band_is_indeterminate() {
        // 306 is this board's captured ain0 value and sits in the vendor's
        // unhandled 300..1100 dead zone. See design H5.
        assert_eq!(classify(306, thr()), Reading::Indeterminate);
    }

    #[test]
    fn test_classify_respects_inverted_sensor_polarity() {
        let inverted = Thresholds {
            ldr_high_is_day: false,
            ..thr()
        };

        assert_eq!(classify(1500, inverted), Reading::Settled(DayNight::Night));
        assert_eq!(classify(120, inverted), Reading::Settled(DayNight::Day));
    }

    #[test]
    fn test_decide_switches_when_reading_differs_and_unlocked() {
        let t0 = std::time::Instant::now();
        let state = AutoState::new(DayNight::Day);

        let target = decide(
            &state,
            Reading::Settled(DayNight::Night),
            t0,
            Duration::from_secs(900),
        );

        assert_eq!(target, Some(DayNight::Night));
    }

    #[test]
    fn test_decide_holds_when_reading_matches_current_mode() {
        let t0 = std::time::Instant::now();
        let state = AutoState::new(DayNight::Day);

        let target = decide(
            &state,
            Reading::Settled(DayNight::Day),
            t0,
            Duration::from_secs(900),
        );

        assert_eq!(target, None);
    }

    #[test]
    fn test_decide_holds_current_mode_on_indeterminate_reading() {
        let t0 = std::time::Instant::now();
        let state = AutoState::new(DayNight::Day);

        let target = decide(&state, Reading::Indeterminate, t0, Duration::from_secs(900));

        assert_eq!(target, None);
    }

    #[test]
    fn test_decide_refuses_to_switch_inside_the_lock_window() {
        let t0 = std::time::Instant::now();
        let mut state = AutoState::new(DayNight::Day);
        state.record_change(DayNight::Night, t0);

        // One second later, the sensor says day again — a dusk flicker.
        let target = decide(
            &state,
            Reading::Settled(DayNight::Day),
            t0 + Duration::from_secs(1),
            Duration::from_secs(900),
        );

        assert_eq!(target, None);
    }

    #[test]
    fn test_decide_switches_again_once_the_lock_expires() {
        let t0 = std::time::Instant::now();
        let mut state = AutoState::new(DayNight::Day);
        state.record_change(DayNight::Night, t0);

        let target = decide(
            &state,
            Reading::Settled(DayNight::Day),
            t0 + Duration::from_secs(901),
            Duration::from_secs(900),
        );

        assert_eq!(target, Some(DayNight::Day));
    }

    #[test]
    fn test_night_mode_plan_lights_lamp_before_isp_switch() {
        let steps = plan(DayNight::Night, pol(), Some(LineMode::Two));

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode(DayNight::Night),
                Step::Write {
                    node: Node::IrCutA,
                    value: 1
                },
                Step::Write {
                    node: Node::IrCutB,
                    value: 0
                },
                Step::Sleep(PULSE),
                Step::Write {
                    node: Node::IrCutA,
                    value: 0
                },
                Step::Write {
                    node: Node::IrCutB,
                    value: 0
                },
                Step::Sleep(SETTLE),
            ]
        );
    }

    #[test]
    fn test_day_mode_plan_moves_filter_before_isp_switch() {
        let steps = plan(DayNight::Day, pol(), Some(LineMode::Two));

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrCutA,
                    value: 0
                },
                Step::Write {
                    node: Node::IrCutB,
                    value: 1
                },
                Step::Sleep(PULSE),
                Step::Write {
                    node: Node::IrCutA,
                    value: 0
                },
                Step::Write {
                    node: Node::IrCutB,
                    value: 0
                },
                Step::IspMode(DayNight::Day),
                Step::Write {
                    node: Node::IrLed,
                    value: 0
                },
                Step::Sleep(SETTLE),
            ]
        );
    }

    #[test]
    fn test_one_line_mode_writes_single_node_without_pulse() {
        let steps = plan(DayNight::Night, pol(), Some(LineMode::One(Node::IrCutA)));

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode(DayNight::Night),
                Step::Write {
                    node: Node::IrCutA,
                    value: 1
                },
                Step::Sleep(SETTLE),
            ]
        );
    }

    #[test]
    fn test_inverted_polarity_flips_only_the_ircut_step() {
        let inverted = Polarity {
            ircut_high_is_night: false,
        };

        let steps = plan(DayNight::Night, inverted, Some(LineMode::One(Node::IrCutA)));

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode(DayNight::Night),
                Step::Write {
                    node: Node::IrCutA,
                    value: 0
                },
                Step::Sleep(SETTLE),
            ]
        );
    }
}
