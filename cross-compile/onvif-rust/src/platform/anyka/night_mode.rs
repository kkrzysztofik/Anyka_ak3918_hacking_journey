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

/// One step of a transition plan.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Step {
    Write {
        node: Node,
        value: u8,
    },
    /// Cross the IPC boundary to `ak_vi_switch_mode`.
    IspMode,
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

/// Current AUTO-mode state: what the camera is set to, and when it last moved.
///
/// `current` is `None` until this controller has driven the hardware itself.
/// GPIO survives a restart and the vendor daemon's ISP state does not, so a
/// mode we merely inferred is not something we may act on. See the design doc.
#[derive(Debug, Clone, Copy)]
pub(super) struct AutoState {
    pub(super) current: Option<DayNight>,
    last_change: Option<std::time::Instant>,
}

impl AutoState {
    pub(super) fn new(current: Option<DayNight>) -> Self {
        Self {
            current,
            last_change: None,
        }
    }

    /// Record that a transition has been applied.
    pub(super) fn record_change(&mut self, to: DayNight, at: std::time::Instant) {
        self.current = Some(to);
        self.last_change = Some(at);
    }
}

/// Decide whether to transition, given a reading and the lock window.
///
/// Returns `None` to hold the current mode. A `None` reading (inside the
/// hysteresis band) always holds; so does any reading inside `lock` of the last
/// transition, which is what stops a camera oscillating at dusk.
///
/// A `None` `state.current` never matches, so the first determinate reading
/// after start-up always transitions — that is the ISP reconcile.
pub(super) fn decide(
    state: &AutoState,
    reading: Option<DayNight>,
    now: std::time::Instant,
    lock: Duration,
) -> Option<DayNight> {
    let target = reading?;
    if state.current == Some(target) {
        return None;
    }
    if let Some(last) = state.last_change
        && now.duration_since(last) < lock
    {
        return None;
    }
    Some(target)
}

/// Map a raw `ain0` reading to a day/night conclusion. `None` inside the
/// hysteresis band; the caller must hold its current mode.
pub(super) fn classify(raw: i32, thr: Thresholds) -> Option<DayNight> {
    let (high, low) = if thr.ldr_high_is_day {
        (DayNight::Day, DayNight::Night)
    } else {
        (DayNight::Night, DayNight::Day)
    };

    if raw >= thr.day {
        Some(high)
    } else if raw <= thr.night {
        Some(low)
    } else {
        None
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
    /// Both ircut lines present, so the H-bridge can be driven.
    pub ircut: bool,
    pub ir_led: bool,
    pub white_led: bool,
}

/// Probe for the nodes. One `stat()` per node answers both the wiring
/// question and the ONVIF capability question.
pub(crate) fn probe(paths: &NodePaths) -> Capabilities {
    Capabilities {
        ircut: paths.node(Node::IrCutA).exists() && paths.node(Node::IrCutB).exists(),
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
/// `?`: the solenoid coil is energised between the pulse and the trailing zero
/// writes, and an early return leaves it that way.
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
            Step::IspMode => {}
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

/// Read a GPIO sysfs node as a boolean (nonzero = on).
///
/// Returns `None` when the node is absent or unparseable — the caller must
/// treat that as "unknown", not as off.
fn read_gpio_on(paths: &NodePaths, node: Node) -> Option<bool> {
    let path = paths.node(node);
    let raw = std::fs::read_to_string(path).ok()?;
    // AK3918 user-gpio sysfs nodes are often NUL-padded (e.g. "1\0").
    let cleaned = raw.split('\0').next().unwrap_or("").trim();
    cleaned.parse::<i32>().ok().map(|v| v != 0)
}

#[derive(Debug)]
struct BlockingVisionReadings {
    ain0: Option<i32>,
    ir_led: Option<bool>,
    ircut_a: Option<bool>,
    ircut_b: Option<bool>,
    white_led: Option<bool>,
}

impl BlockingVisionReadings {
    fn empty() -> Self {
        Self {
            ain0: None,
            ir_led: None,
            ircut_a: None,
            ircut_b: None,
            white_led: None,
        }
    }
}

fn read_blocking_vision(paths: &NodePaths, caps: Capabilities) -> BlockingVisionReadings {
    BlockingVisionReadings {
        ain0: read_light_sensor(paths),
        ir_led: caps
            .ir_led
            .then(|| read_gpio_on(paths, Node::IrLed))
            .flatten(),
        ircut_a: caps
            .ircut
            .then(|| read_gpio_on(paths, Node::IrCutA))
            .flatten(),
        ircut_b: caps
            .ircut
            .then(|| read_gpio_on(paths, Node::IrCutB))
            .flatten(),
        white_led: caps
            .white_led
            .then(|| read_gpio_on(paths, Node::WhiteLed))
            .flatten(),
    }
}

/// AUTO poll cadence.
///
/// Every tick is an IPC round-trip through the daemon's single-threaded poll
/// loop. 2s meant 30 round-trips a minute, forever, to read a light level that
/// changes twice a day. Fixed: lock_time dominates responsiveness.
pub(crate) const POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Consecutive AE read failures before falling back to `ain0`.
const AE_FAIL_STREAK_MAX: u32 = 3;

/// Heartbeat cadence for the AUTO sample line when nothing is changing.
///
/// The poll is every 10s; logging each one is ~8,600 lines a day onto an SD
/// card. A change always logs immediately, so this only bounds the quiet case.
const SAMPLE_LOG_INTERVAL: Duration = Duration::from_secs(600);

/// Whether a sample line is due: always on a classification change, otherwise
/// at most once per `interval`.
fn sample_due(
    last: Option<(std::time::Instant, Option<DayNight>)>,
    class: Option<DayNight>,
    now: std::time::Instant,
    interval: Duration,
) -> bool {
    match last {
        None => true,
        Some((at, prev)) => prev != class || now.duration_since(at) >= interval,
    }
}

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
    /// Whether the "never synced" warning has already fired this process.
    unsynced_warned: std::sync::atomic::AtomicBool,
    /// When the last sample line was logged, and what it classified to.
    /// `None` = nothing logged yet. Guards [`SAMPLE_LOG_INTERVAL`].
    sample_log: tokio::sync::Mutex<Option<(std::time::Instant, Option<DayNight>)>>,
    /// A `set_ir_filter` that failed, and what it was trying to reach. GPIO
    /// already advanced, so only the ISP still needs driving.
    isp_pending: tokio::sync::Mutex<Option<DayNight>>,
    /// Best-effort IDR after every day/night apply (forced + AUTO).
    idr_hook: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
}

impl NightModeController {
    /// Build a controller for the configured start-up mode.
    ///
    /// A configured `ON`/`OFF` disables the AUTO loop from the first tick, so a
    /// forced mode survives a restart instead of silently reverting to AUTO.
    /// [`Self::spawn_auto_loop`] applies it to the hardware.
    ///
    /// `idr_hook` is invoked best-effort after every [`Self::apply`]; `None` on
    /// a platform built without a video encoder.
    pub(crate) fn new(
        paths: NodePaths,
        cfg: crate::config::types::NightConfig,
        ffi: std::sync::Arc<dyn crate::hal::common::imaging::ImagingHalTrait>,
        mode: crate::onvif::types::common::IrCutFilterMode,
        idr_hook: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
    ) -> Self {
        use crate::onvif::types::common::IrCutFilterMode;

        let caps = probe(&paths);
        // AUTO starts unknown: this controller has driven nothing yet. GPIO
        // survives a restart but the vendor daemon's ISP state does not, so
        // inferring a mode from the IR_LED node lets AUTO conclude "already
        // correct" and never issue the ISP switch. The first determinate
        // reading reconciles both together. Forced modes keep their fixed
        // start so ON/OFF always lands where the user asked.
        let initial = match mode {
            IrCutFilterMode::OFF => Some(DayNight::Night),
            IrCutFilterMode::ON => Some(DayNight::Day),
            IrCutFilterMode::AUTO => None,
        };
        Self {
            paths,
            caps,
            cfg,
            ffi,
            state: tokio::sync::Mutex::new(AutoState::new(initial)),
            configured: mode,
            auto_enabled: std::sync::atomic::AtomicBool::new(matches!(mode, IrCutFilterMode::AUTO)),
            ae_fail_streak: std::sync::atomic::AtomicU32::new(0),
            unsynced_warned: std::sync::atomic::AtomicBool::new(false),
            sample_log: tokio::sync::Mutex::new(None),
            isp_pending: tokio::sync::Mutex::new(None),
            idr_hook,
        }
    }

    pub(crate) fn capabilities(&self) -> Capabilities {
        self.caps
    }

    /// The day/night state the hardware was last driven to, or `None` if this
    /// controller has not driven it yet.
    ///
    /// While an ISP resync is pending after a failed `set_ir_filter`, returns
    /// `None` so diagnostics do not report a pipeline mode the ISP has not
    /// actually reached yet.
    pub(crate) async fn current_mode(&self) -> Option<DayNight> {
        let state = self.state.lock().await;
        let pending = self.isp_pending.lock().await;
        if pending.is_some() {
            return None;
        }
        state.current
    }

    pub(crate) fn set_auto_enabled(&self, enabled: bool) {
        self.auto_enabled
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
    }

    /// Warn once per process when AUTO has driven nothing and the sensor will
    /// not say which way to go.
    ///
    /// This is what miscalibrated thresholds look like from the outside: the
    /// ISP is still at its power-on day mode, the lamp is wherever it was, and
    /// no reading will ever break the tie.
    fn warn_unsynced_once(&self, raw: i32, src: &'static str) {
        use std::sync::atomic::Ordering;
        if self.unsynced_warned.swap(true, Ordering::SeqCst) {
            return;
        }
        tracing::warn!(
            raw,
            src,
            "night mode has never driven the hardware and the reading is indeterminate; check thresholds"
        );
    }

    /// Log one AUTO sample, rate-limited by [`sample_due`].
    ///
    /// The event name is always `night sample`; the `mode` field is
    /// `Day|Night|Indeterminate`. `info!`, not `debug!`: cameras run at
    /// `level = "error"` and a day/night failure is only diagnosable after the
    /// fact. See the filter directive in `logging::init_logging_impl`.
    async fn log_sample(&self, raw: i32, src: &'static str, class: Option<DayNight>) {
        let now = std::time::Instant::now();
        let mut last = self.sample_log.lock().await;
        if !sample_due(*last, class, now, SAMPLE_LOG_INTERVAL) {
            return;
        }
        *last = Some((now, class));
        match class {
            Some(mode) => tracing::info!(raw, src, ?mode, "night sample"),
            None => tracing::info!(raw, src, mode = "Indeterminate", "night sample"),
        }
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
            self.caps.ircut,
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

        // Split the plan around the ISP step. `plan()` orders the GPIO writes
        // around the ISP switch (lamp on before night, off after day) so no
        // frame is captured dark, and the trailing SETTLE sleep belongs after
        // the switch — executing every GPIO step before the IPC call broke
        // both, because the ISP call happened after the lamp was already off
        // and after the settle had already elapsed.
        let isp_at = steps.iter().position(|s| *s == Step::IspMode);
        let post_isp = isp_at.map_or_else(Vec::new, |i| steps.split_off(i + 1));
        if isp_at.is_some() {
            steps.pop(); // the marker itself
        }
        let pre_isp = steps;

        let paths = self.paths.clone();
        let pre_paths = paths.clone();
        let pre_result = tokio::task::spawn_blocking(move || execute_gpio(&pre_isp, &pre_paths))
            .await
            .map_err(|e| PlatformError::HardwareFailure(format!("join: {e}")))?;
        if let Err(e) = pre_result {
            tracing::warn!(error = %e, ?target, "GPIO transition failed; state not advanced");
            return Err(PlatformError::HardwareFailure(format!("GPIO write: {e}")));
        }

        // The ISP step is the only part that needs the daemon. GPIO is the
        // durable state, so an ISP failure alone still records the transition:
        // the next tick must not re-drive an already-correct GPIO state.
        let isp = self
            .ffi
            .set_ir_filter(matches!(target, DayNight::Night))
            .await;
        if isp != 0 {
            tracing::warn!(isp, "ISP day/night switch failed; GPIO state advanced");
        }

        let post_result = tokio::task::spawn_blocking(move || execute_gpio(&post_isp, &paths))
            .await
            .map_err(|e| PlatformError::HardwareFailure(format!("join: {e}")))?;

        // IDR even when ISP fails: GPIO/filter change still needs a keyframe.
        if let Some(hook) = self.idr_hook.as_ref() {
            hook();
        }

        if let Err(e) = post_result {
            tracing::warn!(error = %e, ?target, "GPIO transition failed; state not advanced");
            return Err(PlatformError::HardwareFailure(format!("GPIO write: {e}")));
        }

        // Only now that the full GPIO plan has succeeded do we update the
        // pending ISP sync: a failed post-ISP GPIO write returns above without
        // leaving a target, so tick never retries the ISP for a transition
        // whose GPIO did not complete. GPIO already moved, so a pending target
        // means a later tick must only re-drive the ISP, never the whole apply
        // (which would re-pulse the coil).
        if isp != 0 {
            *self.isp_pending.lock().await = Some(target);
        } else {
            // A successful ISP call clears any prior pending sync.
            *self.isp_pending.lock().await = None;
        }

        // Logged unconditionally: a silent success and a transition that never
        // happened look identical otherwise, which is what hid the 2026-08-10
        // failure. `isp` is the daemon's ak_vi_switch_mode return.
        tracing::info!(from = ?state.current, to = ?target, isp, "night mode applied");
        state.record_change(target, std::time::Instant::now());
        Ok(())
    }

    /// One AUTO poll: prefer AE luma, fall back to `ain0` after streak failures.
    pub(crate) async fn tick(&self) {
        use std::sync::atomic::Ordering;

        if !self.auto_enabled.load(Ordering::SeqCst) {
            return;
        }

        let (raw, src, reading) = match self.ffi.get_ae_luma().await {
            Some(luma) => {
                self.ae_fail_streak.store(0, Ordering::SeqCst);
                let raw = i32::from(luma);
                let class = classify(
                    raw,
                    Thresholds {
                        day: self.cfg.ae_day_threshold,
                        night: self.cfg.ae_night_threshold,
                        // AE high = bright = day.
                        ldr_high_is_day: true,
                    },
                );
                (raw, "ae", class)
            }
            None => {
                let n = self.ae_fail_streak.fetch_add(1, Ordering::SeqCst) + 1;
                if n < AE_FAIL_STREAK_MAX {
                    return;
                }
                if n == AE_FAIL_STREAK_MAX {
                    tracing::warn!(streak = n, "AE luma unavailable; falling back to ain0");
                }
                let (Some(day), Some(night)) = (self.cfg.day_threshold, self.cfg.night_threshold)
                else {
                    return;
                };
                let Some(raw) = read_light_sensor(&self.paths) else {
                    return;
                };
                let class = classify(
                    raw,
                    Thresholds {
                        day,
                        night,
                        ldr_high_is_day: self.cfg.ldr_high_is_day,
                    },
                );
                (raw, "ain0", class)
            }
        };

        self.log_sample(raw, src, reading).await;

        let (target, unsynced) = {
            let state = self.state.lock().await;
            let target = decide(
                &state,
                reading,
                std::time::Instant::now(),
                Duration::from_millis(self.cfg.lock_time_ms),
            );
            (target, state.current.is_none())
        };

        if unsynced && reading.is_none() {
            self.warn_unsynced_once(raw, src);
        }

        if let Some(target) = target
            && let Err(e) = self.apply(target).await
        {
            tracing::warn!(error = %e, "night-mode transition failed");
        }

        // A failed set_ir_filter must not be left to rot: GPIO already advanced,
        // so we do NOT re-run apply (that would re-pulse the coil). The retry
        // below runs in the same tick, immediately after the failed apply, and
        // on later ticks too if it still fails.
        let mut pending = self.isp_pending.lock().await;
        if let Some(target) = *pending {
            let isp = self
                .ffi
                .set_ir_filter(matches!(target, DayNight::Night))
                .await;
            if isp == 0 {
                *pending = None;
                tracing::info!(to = ?target, isp, "night mode ISP synced");
            }
        }
    }

    /// Write a single lamp GPIO. Used by ONVIF auxiliary commands.
    ///
    /// Takes the transition mutex so a lamp write cannot interleave with an
    /// in-flight [`Self::apply`] driving the same node.
    pub(crate) async fn write_lamp(&self, node: Node, on: bool) -> std::io::Result<()> {
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
        let _guard = self.state.lock().await;
        std::fs::write(self.paths.node(node), if on { "1" } else { "0" })
    }

    /// Apply the configured start-up mode, then run the AUTO poll loop.
    ///
    /// Call once after construction. A configured `ON`/`OFF` is driven onto the
    /// hardware here; `AUTO` leaves the filter where it is until the first tick
    /// has a settled reading.
    ///
    /// `shutdown` is a broadcast subscription: on a notification the loop stops
    /// before its next tick, so it cannot call `set_ir_filter` on a torn-down
    /// IPC client during platform teardown. The returned handle is stored by
    /// the platform and awaited during shutdown.
    pub(crate) fn spawn_auto_loop(
        self: &std::sync::Arc<Self>,
        mut shutdown: tokio::sync::broadcast::Receiver<()>,
    ) -> tokio::task::JoinHandle<()> {
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
                tokio::select! {
                    _ = interval.tick() => this.tick().await,
                    _ = shutdown.recv() => {
                        tracing::debug!("night-mode AUTO loop stopping");
                        return;
                    }
                }
            }
        })
    }

    /// Return a point-in-time snapshot of the vision-switching pipeline state.
    pub(crate) async fn live_diagnostics(&self) -> crate::platform::common::VisionDiagnostics {
        use crate::platform::common::{VisionDiagnostics, VisionSupported};

        let paths = self.paths.clone();
        let caps = self.caps;
        let blocking = tokio::task::spawn_blocking(move || read_blocking_vision(&paths, caps));

        let (ae_luma, mode, gpio) = tokio::join!(
            self.ffi.get_ae_luma(),
            async {
                self.current_mode().await.map(|m| match m {
                    DayNight::Day => "day".to_string(),
                    DayNight::Night => "night".to_string(),
                })
            },
            async {
                match blocking.await {
                    Ok(readings) => readings,
                    Err(e) => {
                        tracing::warn!(error = %e, "vision GPIO sampling task failed");
                        BlockingVisionReadings::empty()
                    }
                }
            }
        );

        VisionDiagnostics {
            mode,
            ae_luma,
            ain0: gpio.ain0,
            ir_led: gpio.ir_led,
            ircut_a: gpio.ircut_a,
            ircut_b: gpio.ircut_b,
            white_led: gpio.white_led,
            supported: VisionSupported {
                ir_led: self.caps.ir_led,
                ircut: self.caps.ircut,
                white_led: self.caps.white_led,
            },
        }
    }
}

/// Build the ordered step list for a transition.
///
/// Ordering follows the vendor reference and is not arbitrary: the lamp turns
/// on before the ISP switches to night and off after it switches to day, so no
/// frame is captured dark. The trailing zero writes de-energise the solenoid
/// coil and are mandatory.
///
/// `ircut` is false on a board without both ircut nodes. The filter steps drop
/// out; the lamp and ISP steps stay, because a board can have an illuminator
/// without a filter.
pub(super) fn plan(target: DayNight, pol: Polarity, ircut: bool) -> Vec<Step> {
    let night_level = u8::from(pol.ircut_high_is_night);
    let ircut_level = match target {
        DayNight::Night => night_level,
        DayNight::Day => 1 - night_level,
    };

    let filter = if ircut {
        vec![
            Step::Write {
                node: Node::IrCutA,
                value: ircut_level,
            },
            Step::Write {
                node: Node::IrCutB,
                value: 1 - ircut_level,
            },
            Step::Sleep(PULSE),
            Step::Write {
                node: Node::IrCutA,
                value: 0,
            },
            Step::Write {
                node: Node::IrCutB,
                value: 0,
            },
        ]
    } else {
        Vec::new()
    };

    let mut steps = Vec::new();
    match target {
        DayNight::Night => {
            steps.push(Step::Write {
                node: Node::IrLed,
                value: 1,
            });
            steps.push(Step::IspMode);
            steps.extend(filter);
        }
        DayNight::Day => {
            steps.extend(filter);
            steps.push(Step::IspMode);
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
            None,
        );
        ctl.apply(DayNight::Night).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(),
            "1"
        );
    }

    #[tokio::test]
    async fn test_apply_invokes_idr_hook_after_transition() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed] {
            std::fs::write(paths.node(n), "9").unwrap();
        }

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_set_ir_filter().returning(|_| 0);

        let calls = std::sync::Arc::new(AtomicUsize::new(0));
        let calls_hook = std::sync::Arc::clone(&calls);
        let ctl = NightModeController::new(
            paths,
            test_config(),
            std::sync::Arc::new(ffi),
            crate::onvif::types::common::IrCutFilterMode::AUTO,
            Some(std::sync::Arc::new(move || {
                calls_hook.fetch_add(1, Ordering::SeqCst);
            })),
        );

        ctl.apply(DayNight::Night).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        ctl.apply(DayNight::Day).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
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
            None,
        );
        let _ = ctl.apply(DayNight::Night).await;

        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(),
            "1"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_live_diagnostics_mode_none_while_apply_holds_state_lock() {
        use std::sync::{Arc, Barrier};

        use crate::hal::common::imaging::MockImagingHalTrait;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed] {
            std::fs::write(paths.node(n), "9").unwrap();
        }

        let barrier = Arc::new(Barrier::new(2));
        let barrier_isp = Arc::clone(&barrier);

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_set_ir_filter().times(1).returning(move |_| {
            barrier_isp.wait();
            -1
        });
        ffi.expect_get_ae_luma().times(1..).returning(|| None);

        let ctl = Arc::new(NightModeController::new(
            paths,
            test_config(),
            Arc::new(ffi),
            crate::onvif::types::common::IrCutFilterMode::AUTO,
            None,
        ));

        let ctl_apply = Arc::clone(&ctl);
        let diag_barrier = Arc::clone(&barrier);
        let diag = tokio::spawn(async move {
            diag_barrier.wait();
            ctl.live_diagnostics().await.mode
        });
        let apply = tokio::spawn(async move { ctl_apply.apply(DayNight::Night).await });

        let mode = diag.await.expect("diagnostics task");
        let _ = apply.await.expect("apply task");

        assert!(
            mode.is_none(),
            "live diagnostics must not report a mode while apply owns state or ISP sync is pending"
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
            None,
        );

        assert_eq!(ctl.current_mode().await, Some(DayNight::Night));

        // The AUTO loop must not undo a configured forced mode.
        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, Some(DayNight::Night));
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
            None,
        );
        assert_eq!(ctl.current_mode().await, None, "AUTO starts unknown");

        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, Some(DayNight::Night));
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

        let cfg = crate::config::types::NightConfig {
            day_threshold: Some(200),
            night_threshold: Some(150),
            lock_time_ms: 0,
            ..Default::default()
        };
        let ctl = NightModeController::new(
            paths,
            cfg,
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
            None,
        );

        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, None, "AUTO starts unknown");
        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, None, "AUTO starts unknown");
        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, Some(DayNight::Night));
    }

    #[tokio::test]
    async fn test_tick_holds_when_ae_fails_and_ain0_is_uncalibrated() {
        // AE unavailable past the streak limit, thresholds unset: the fallback
        // must not run. An unavailable AE reading is not evidence about light,
        // and guessing with another board's numbers put the IR illuminator on
        // at midday.
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        seed_gpio_nodes(&paths);
        std::fs::write(paths.light_sensor(), "100").unwrap();

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_get_ae_luma().times(3).returning(|| None);
        // No set_ir_filter expectation: the fallback must never run.

        let ctl = NightModeController::new(
            paths,
            unlocked_config(), // day_threshold/night_threshold are None
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
            None,
        );

        ctl.tick().await;
        ctl.tick().await;
        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, None, "AUTO starts unknown");
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
        ffi.expect_get_ae_luma().times(5).returning({
            let mut calls = 0u8;
            move || {
                calls += 1;
                match calls {
                    3 => Some(100), // bright AE → day
                    _ => None,
                }
            }
        });
        // The first determinate reading (the AE success on call 3) applies
        // day. The night the ain0 fallback would force must never apply, even
        // after the streak resumes past the cleared state (calls 4-5).
        ffi.expect_set_ir_filter()
            .withf(|enabled| !*enabled)
            .times(1)
            .returning(|_| 0);

        let ctl = NightModeController::new(
            paths,
            unlocked_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
            None,
        );

        ctl.tick().await;
        ctl.tick().await;
        ctl.tick().await;
        ctl.tick().await;
        ctl.tick().await;
        assert_eq!(ctl.current_mode().await, Some(DayNight::Day));
    }

    #[tokio::test]
    async fn test_auto_syncs_the_isp_after_a_restart_with_the_lamp_already_on() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        // Reproduces 192.168.30.121 on 2026-08-10: onvif-rust restarted with a
        // stale IR_LED=1, the vendor daemon restarted alongside it and came up in
        // day mode, and AUTO concluded "already at night" and never called
        // set_ir_filter. The daemon must be told, every time, on the first
        // determinate reading.
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        seed_gpio_nodes(&paths);
        std::fs::write(paths.node(Node::IrLed), "1").unwrap();

        let mut ffi = MockImagingHalTrait::new();
        // Below ae_night_threshold (8): a dark scene, agreeing with the stale lamp.
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
            None,
        );
        assert_eq!(ctl.current_mode().await, None, "nothing driven yet");

        ctl.tick().await;

        assert_eq!(ctl.current_mode().await, Some(DayNight::Night));
    }

    #[tokio::test]
    async fn test_retries_the_isp_after_a_failed_switch() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        // GPIO advances even when the ISP switch fails, but the ISP itself must
        // be retried until the daemon accepts it — otherwise it stays at its
        // restart-default mode while the lamp says night.
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        seed_gpio_nodes(&paths);

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_get_ae_luma().times(3).returning(|| Some(1)); // dark scene
        let mut calls = 0u8;
        ffi.expect_set_ir_filter()
            .withf(|enabled| *enabled)
            .times(2)
            .returning(move |_| {
                calls += 1;
                if calls == 1 { -1 } else { 0 }
            });

        let ctl = NightModeController::new(
            paths,
            unlocked_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
            None,
        );

        ctl.tick().await; // apply fails the ISP (call 1); the same tick retries it (call 2) to 0
        assert_eq!(ctl.current_mode().await, Some(DayNight::Night));

        ctl.tick().await; // decide holds; the retry already resolved back in tick 1
        ctl.tick().await; // re-polls only: set_ir_filter still called exactly twice
    }

    #[tokio::test]
    async fn test_no_isp_retry_when_post_isp_gpio_fails() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        // A failed post-ISP GPIO write must not leave a pending ISP sync:
        // apply returns before recording the mode, so tick must not add an
        // ISP-only retry on top of the normal re-apply. set_ir_filter is called
        // exactly once per tick (by apply), never twice in the same tick.
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        seed_gpio_nodes(&paths);
        // The post-ISP ircut pulse for Night writes ircut_a; make that fail.
        std::fs::remove_file(paths.node(Node::IrCutA)).expect("remove ircut_a");
        std::fs::create_dir(paths.node(Node::IrCutA)).expect("create dir ircut_a");

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_get_ae_luma().times(2).returning(|| Some(1)); // dark scene
        ffi.expect_set_ir_filter()
            .withf(|enabled| *enabled)
            .times(2)
            .returning(|_| -1);

        let ctl = NightModeController::new(
            paths,
            unlocked_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
            None,
        );

        ctl.tick().await; // apply: ISP fails AND post-ISP GPIO fails -> nothing recorded
        ctl.tick().await; // re-apply only: no ISP-only retry (mockall enforces 2 calls)

        assert_eq!(ctl.current_mode().await, None);
    }

    #[test]
    fn test_plan_without_ircut_hardware_still_drives_the_lamp() {
        let steps = plan(DayNight::Night, pol(), false);

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode,
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
            None,
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
            None,
        );

        // No IR_LED node: the filter still moves and no bogus error is raised.
        ctl.apply(DayNight::Night).await.unwrap();
        assert!(!paths.node(Node::IrLed).exists());
    }

    #[tokio::test]
    async fn test_apply_does_not_record_mode_when_gpio_fails() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        // ircut_a is a directory: `probe` sees a node, but the post-ISP GPIO
        // write fails, so the transition must not be recorded.
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::create_dir(paths.node(Node::IrCutA)).unwrap();
        std::fs::create_dir(paths.node(Node::IrCutB)).unwrap();
        std::fs::write(paths.node(Node::IrLed), "9").unwrap();

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_set_ir_filter().returning(|_| 0);

        let ctl = NightModeController::new(
            paths,
            test_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
            None,
        );
        assert_eq!(ctl.current_mode().await, None, "AUTO starts unknown");

        assert!(ctl.apply(DayNight::Night).await.is_err());

        // A failed transition must not be recorded: AUTO would otherwise
        // believe the camera is at Night and never retry the transition.
        assert_eq!(ctl.current_mode().await, None);
    }

    fn thr() -> Thresholds {
        Thresholds {
            day: 1100,
            night: 300,
            ldr_high_is_day: true,
        }
    }

    #[test]
    fn test_probe_reports_ircut_when_both_nodes_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "0").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "0").unwrap();
        std::fs::write(paths.node(Node::IrLed), "0").unwrap();

        let caps = probe(&paths);

        assert!(caps.ircut);
        assert!(caps.ir_led);
    }

    #[test]
    fn test_probe_reports_unsupported_when_a_line_is_missing() {
        // The H-bridge needs both lines; one node alone cannot be driven.
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "0").unwrap();

        let caps = probe(&paths);

        assert!(!caps.ircut);
        assert!(!caps.ir_led);
    }

    #[test]
    fn test_probe_reports_unsupported_when_no_ircut_nodes_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());

        let caps = probe(&paths);

        assert!(!caps.ircut);
    }

    #[test]
    fn test_execute_writes_final_values_to_nodes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "9").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "9").unwrap();
        std::fs::write(paths.node(Node::IrLed), "9").unwrap();

        let steps = plan(DayNight::Night, pol(), true);
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

        let steps = plan(DayNight::Night, pol(), true);
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
    fn test_classify_bright_reading_is_day() {
        assert_eq!(classify(1500, thr()), Some(DayNight::Day));
    }

    #[test]
    fn test_classify_dark_reading_is_night() {
        assert_eq!(classify(120, thr()), Some(DayNight::Night));
    }

    #[test]
    fn test_classify_reading_in_hysteresis_band_is_indeterminate() {
        // 306 is this board's captured ain0 value and sits in the vendor's
        // unhandled 300..1100 dead zone. See design H5.
        assert_eq!(classify(306, thr()), None);
    }

    #[test]
    fn test_classify_respects_inverted_sensor_polarity() {
        let inverted = Thresholds {
            ldr_high_is_day: false,
            ..thr()
        };

        assert_eq!(classify(1500, inverted), Some(DayNight::Night));
        assert_eq!(classify(120, inverted), Some(DayNight::Day));
    }

    #[test]
    fn test_decide_switches_when_reading_differs_and_unlocked() {
        let t0 = std::time::Instant::now();
        let state = AutoState::new(Some(DayNight::Day));

        let target = decide(&state, Some(DayNight::Night), t0, Duration::from_secs(900));

        assert_eq!(target, Some(DayNight::Night));
    }

    #[test]
    fn test_decide_holds_when_reading_matches_current_mode() {
        let t0 = std::time::Instant::now();
        let state = AutoState::new(Some(DayNight::Day));

        let target = decide(&state, Some(DayNight::Day), t0, Duration::from_secs(900));

        assert_eq!(target, None);
    }

    #[test]
    fn test_decide_holds_current_mode_on_indeterminate_reading() {
        let t0 = std::time::Instant::now();
        let state = AutoState::new(Some(DayNight::Day));

        let target = decide(&state, None, t0, Duration::from_secs(900));

        assert_eq!(target, None);
    }

    #[test]
    fn test_decide_refuses_to_switch_inside_the_lock_window() {
        let t0 = std::time::Instant::now();
        let mut state = AutoState::new(Some(DayNight::Day));
        state.record_change(DayNight::Night, t0);

        // One second later, the sensor says day again — a dusk flicker.
        let target = decide(
            &state,
            Some(DayNight::Day),
            t0 + Duration::from_secs(1),
            Duration::from_secs(900),
        );

        assert_eq!(target, None);
    }

    #[test]
    fn test_decide_switches_again_once_the_lock_expires() {
        let t0 = std::time::Instant::now();
        let mut state = AutoState::new(Some(DayNight::Day));
        state.record_change(DayNight::Night, t0);

        let target = decide(
            &state,
            Some(DayNight::Day),
            t0 + Duration::from_secs(901),
            Duration::from_secs(900),
        );

        assert_eq!(target, Some(DayNight::Day));
    }

    #[test]
    fn test_decide_transitions_when_the_current_state_is_unknown() {
        // At start-up in AUTO the controller has driven nothing, so it must act on
        // the first determinate reading rather than assume the hardware agrees.
        let t0 = std::time::Instant::now();
        let state = AutoState::new(None);

        let target = decide(&state, Some(DayNight::Night), t0, Duration::from_secs(900));

        assert_eq!(target, Some(DayNight::Night));
    }

    #[test]
    fn test_sample_due_on_first_call() {
        let t0 = std::time::Instant::now();
        assert!(sample_due(
            None,
            Some(DayNight::Day),
            t0,
            Duration::from_secs(600)
        ));
    }

    #[test]
    fn test_sample_due_when_the_classification_changes() {
        let t0 = std::time::Instant::now();
        let last = Some((t0, Some(DayNight::Day)));

        // One second later, but the reading flipped: log it immediately.
        assert!(sample_due(
            last,
            Some(DayNight::Night),
            t0 + Duration::from_secs(1),
            Duration::from_secs(600),
        ));
    }

    #[test]
    fn test_sample_suppressed_inside_the_window_when_unchanged() {
        let t0 = std::time::Instant::now();
        let last = Some((t0, Some(DayNight::Day)));

        assert!(!sample_due(
            last,
            Some(DayNight::Day),
            t0 + Duration::from_secs(599),
            Duration::from_secs(600),
        ));
    }

    #[test]
    fn test_sample_due_again_once_the_window_expires() {
        let t0 = std::time::Instant::now();
        let last = Some((t0, Some(DayNight::Day)));

        assert!(sample_due(
            last,
            Some(DayNight::Day),
            t0 + Duration::from_secs(600),
            Duration::from_secs(600),
        ));
    }

    #[test]
    fn test_night_mode_plan_lights_lamp_before_isp_switch() {
        let steps = plan(DayNight::Night, pol(), true);

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode,
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
        let steps = plan(DayNight::Day, pol(), true);

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
                Step::IspMode,
                Step::Write {
                    node: Node::IrLed,
                    value: 0
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

        let steps = plan(DayNight::Night, inverted, true);

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode,
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
                Step::Sleep(SETTLE),
            ]
        );
    }

    #[test]
    fn test_read_gpio_on_accepts_nul_padded_sysfs() {
        let dir = tempfile::tempdir().unwrap();
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrLed), "1\0").unwrap();
        std::fs::write(paths.node(Node::WhiteLed), "0\0").unwrap();
        assert_eq!(read_gpio_on(&paths, Node::IrLed), Some(true));
        assert_eq!(read_gpio_on(&paths, Node::WhiteLed), Some(false));
    }

    #[tokio::test]
    async fn test_live_diagnostics_reads_ae_ain0_and_lamps() {
        use crate::hal::common::imaging::MockImagingHalTrait;
        use crate::onvif::types::common::IrCutFilterMode;

        let dir = tempfile::tempdir().unwrap();
        let paths = NodePaths::rooted(dir.path(), dir.path());
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed, Node::WhiteLed] {
            std::fs::write(paths.node(n), "0").unwrap();
        }
        std::fs::write(paths.node(Node::IrLed), "1").unwrap();
        std::fs::write(paths.light_sensor(), "306").unwrap();

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_get_ae_luma().times(1).returning(|| Some(42));

        let ctl = NightModeController::new(
            paths,
            test_config(),
            std::sync::Arc::new(ffi),
            IrCutFilterMode::AUTO,
            None,
        );
        let v = ctl.live_diagnostics().await;
        assert_eq!(v.ae_luma, Some(42));
        assert_eq!(v.ain0, Some(306));
        assert_eq!(v.ir_led, Some(true));
        assert_eq!(v.ircut_a, Some(false));
        assert_eq!(v.ircut_b, Some(false));
        assert_eq!(v.white_led, Some(false));
        assert!(v.supported.ir_led);
        assert!(v.supported.ircut);
        assert!(v.supported.white_led);
        assert_eq!(v.mode, None);
    }
}
