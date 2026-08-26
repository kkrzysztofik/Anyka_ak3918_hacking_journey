//! Event-triggered speaker playback policy.
//!
//! The daemon plays a file; this module owns debounce, event→clip mapping, and
//! drop-when-busy. All of that is host-testable without hardware.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::debug;

use crate::config::sound::SoundConfig;
#[cfg(test)]
use crate::platform::PlatformError;
use crate::platform::PlatformResult;

/// Policy-layer result of `SoundPlayer::play` (distinct from sink `SoundPlayOutcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundPlayResult {
    /// Sink accepted the play request.
    Accepted,
    /// Sink reported busy; clip was dropped.
    Busy,
    /// Repeat within debounce window; no sink call.
    Debounced,
    /// Sound config disabled; no sink call.
    Disabled,
    /// No clip mapped for the event; no sink call.
    NoClip,
}

/// Outcome of asking the daemon (or a test sink) to play a clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundPlayOutcome {
    /// Daemon accepted the play (async; not finished).
    Accepted,
    /// A clip is already playing; this one was dropped.
    Busy,
}

/// Sends a play request to the speaker path (IPC or test fake).
pub trait SoundSink: Send + Sync {
    fn play_file(&self, path: &str, volume: u8) -> PlatformResult<SoundPlayOutcome>;
}

/// Plays configured event clips with per-event debounce.
pub struct SoundPlayer<S, C>
where
    S: SoundSink,
    C: Fn() -> Instant + Send + Sync,
{
    config: SoundConfig,
    sink: S,
    clock: C,
    last_played: Mutex<HashMap<String, Instant>>,
}

impl<S, C> SoundPlayer<S, C>
where
    S: SoundSink,
    C: Fn() -> Instant + Send + Sync,
{
    pub fn new(config: SoundConfig, sink: S, clock: C) -> Self {
        Self {
            config,
            sink,
            clock,
            last_played: Mutex::new(HashMap::new()),
        }
    }

    /// Borrow the active sound policy (enabled, events, volume, debounce).
    pub fn config(&self) -> &SoundConfig {
        &self.config
    }

    /// Play a named event clip. Never treats busy/debounce/disabled as an error.
    pub fn play(&self, event: &str) -> PlatformResult<SoundPlayResult> {
        if !self.config.enabled {
            debug!(event, "sound disabled; skip");
            return Ok(SoundPlayResult::Disabled);
        }
        let Some(clip) = self.config.clip_for(event) else {
            debug!(event, "no clip mapped; skip");
            return Ok(SoundPlayResult::NoClip);
        };

        let now = (self.clock)();
        {
            let mut last = self
                .last_played
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(prev) = last.get(event)
                && now.duration_since(*prev) < Duration::from_secs(self.config.debounce_secs)
            {
                debug!(event, "within debounce; skip");
                return Ok(SoundPlayResult::Debounced);
            }
            last.insert(event.to_string(), now);
        }

        let path = clip_path(&self.config.clip_dir, clip);
        let path_str = path.to_string_lossy();
        match self.sink.play_file(path_str.as_ref(), self.config.volume) {
            Ok(SoundPlayOutcome::Accepted) => {
                debug!(event, path = %path_str, "sound play accepted");
                Ok(SoundPlayResult::Accepted)
            }
            Ok(SoundPlayOutcome::Busy) => {
                debug!(event, path = %path_str, "sound play busy; dropped");
                Ok(SoundPlayResult::Busy)
            }
            Err(e) => {
                // Clear debounce only for this invocation — a concurrent retry may
                // have replaced the entry with a newer timestamp.
                let mut last = self
                    .last_played
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if last.get(event) == Some(&now) {
                    last.remove(event);
                }
                Err(e)
            }
        }
    }
}

fn clip_path(clip_dir: &str, clip: &str) -> PathBuf {
    Path::new(clip_dir).join(clip)
}

/// PCM rate verified on hardware for the shipped clip set.
const SOUND_SAMPLE_RATE: u32 = 8000;
const SOUND_CHANNELS: u32 = 1;

impl SoundSink for crate::hal::anyka::ipc::AnykaIpc {
    fn play_file(&self, path: &str, volume: u8) -> PlatformResult<SoundPlayOutcome> {
        use crate::hal::anyka::ipc::AudioPlayStatus;
        match self.audio_play(path, SOUND_SAMPLE_RATE, SOUND_CHANNELS, i32::from(volume))? {
            AudioPlayStatus::Accepted => Ok(SoundPlayOutcome::Accepted),
            AudioPlayStatus::Busy => Ok(SoundPlayOutcome::Busy),
        }
    }
}

impl SoundSink for Arc<crate::hal::anyka::ipc::AnykaIpc> {
    fn play_file(&self, path: &str, volume: u8) -> PlatformResult<SoundPlayOutcome> {
        (**self).play_file(path, volume)
    }
}

/// Shared player used by boot / network / upgrade call sites.
pub type SharedSoundPlayer =
    Arc<SoundPlayer<Arc<crate::hal::anyka::ipc::AnykaIpc>, fn() -> Instant>>;

/// Build a production player: real clock, IPC sink, resolved clip directory.
pub fn build_shared_player(
    mut config: SoundConfig,
    config_dir: &Path,
    ipc: Arc<crate::hal::anyka::ipc::AnykaIpc>,
) -> SharedSoundPlayer {
    if !Path::new(&config.clip_dir).is_absolute() {
        config.clip_dir = config_dir.join(&config.clip_dir).display().to_string();
    }
    if !config.enabled {
        tracing::info!(
            event = "sound_disabled",
            "[sound] enabled=false; event clips silent"
        );
    } else if !Path::new(&config.clip_dir).is_dir() {
        tracing::warn!(
            event = "sound_clip_dir_missing",
            dir = %config.clip_dir,
            "[sound] clip_dir missing; plays will fail until populated"
        );
    }
    Arc::new(SoundPlayer::new(config, ipc, Instant::now))
}

/// Best-effort play: never panics, never blocks the caller on audio policy errors.
pub fn play_event(player: &SharedSoundPlayer, event: &str) {
    if let Err(e) = player.play(event) {
        tracing::warn!(error = %e, event, "sound play failed");
    }
}

/// Edge detector for a boolean link signal.
///
/// The first sample only sets the baseline (no chime for the state at start).
/// Later transitions emit `network_up` / `network_lost`.
#[derive(Debug, Default)]
pub struct LinkEdgeWatcher {
    prev: Option<bool>,
}

impl LinkEdgeWatcher {
    pub fn observe(&mut self, up: bool) -> Option<&'static str> {
        match self.prev {
            None => {
                self.prev = Some(up);
                None
            }
            Some(was) if was == up => None,
            Some(true) => {
                self.prev = Some(false);
                Some("network_lost")
            }
            Some(false) => {
                self.prev = Some(true);
                Some("network_up")
            }
        }
    }
}

/// Fires once when a trial marker disappears after having been seen.
#[derive(Debug, Default)]
pub struct TrialConfirmWatcher {
    saw_marker: bool,
    fired: bool,
}

impl TrialConfirmWatcher {
    /// `true` means play `upgrade_result` once.
    pub fn observe(&mut self, marker_present: bool) -> bool {
        if self.fired {
            return false;
        }
        if marker_present {
            self.saw_marker = true;
            return false;
        }
        if self.saw_marker {
            self.fired = true;
            return true;
        }
        false
    }
}

/// Read `/sys/class/net/<iface>/operstate` — only `up` counts as linked.
pub fn read_link_up(iface: &str) -> bool {
    let path = format!("/sys/class/net/{iface}/operstate");
    std::fs::read_to_string(path)
        .map(|s| s.trim() == "up")
        .unwrap_or(false)
}

/// True if `state/trial-a` or `state/trial-b` exists under the update root.
pub fn trial_marker_present(update_root: &Path) -> bool {
    let state = update_root.join("state");
    state.join("trial-a").exists() || state.join("trial-b").exists()
}

const LINK_POLL: Duration = Duration::from_secs(2);

/// Poll link state until shutdown; play on edges only.
pub async fn run_link_watcher(
    player: SharedSoundPlayer,
    iface: String,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) {
    let mut edge = LinkEdgeWatcher::default();
    let mut ticker = tokio::time::interval(LINK_POLL);
    loop {
        tokio::select! {
            _ = shutdown.recv() => return,
            _ = ticker.tick() => {
                if let Some(event) = edge.observe(read_link_up(&iface)) {
                    play_event(&player, event);
                }
            }
        }
    }
}

/// Poll trial markers until shutdown; play once when a seen marker clears.
pub async fn run_trial_watcher(
    player: SharedSoundPlayer,
    update_root: PathBuf,
    mut shutdown: tokio::sync::broadcast::Receiver<()>,
) {
    let mut watcher = TrialConfirmWatcher::default();
    let mut ticker = tokio::time::interval(LINK_POLL);
    loop {
        tokio::select! {
            _ = shutdown.recv() => return,
            _ = ticker.tick() => {
                if watcher.observe(trial_marker_present(&update_root)) {
                    play_event(&player, "upgrade_result");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeSink {
        calls: Mutex<Vec<(String, u8)>>,
        /// Popped from the front (index 0) so tests can queue outcomes in order.
        next: Mutex<Vec<PlatformResult<SoundPlayOutcome>>>,
    }

    impl FakeSink {
        fn push_outcome(&self, outcome: PlatformResult<SoundPlayOutcome>) {
            self.next.lock().unwrap().push(outcome);
        }

        fn calls(&self) -> Vec<(String, u8)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl SoundSink for FakeSink {
        fn play_file(&self, path: &str, volume: u8) -> PlatformResult<SoundPlayOutcome> {
            self.calls.lock().unwrap().push((path.to_string(), volume));
            let mut q = self.next.lock().unwrap();
            if q.is_empty() {
                Ok(SoundPlayOutcome::Accepted)
            } else {
                q.remove(0)
            }
        }
    }

    impl SoundSink for Arc<FakeSink> {
        fn play_file(&self, path: &str, volume: u8) -> PlatformResult<SoundPlayOutcome> {
            (**self).play_file(path, volume)
        }
    }

    /// Blocks the first `play_file` until `release()` is called, then returns `first_outcome`.
    struct StallingFakeSink {
        calls: Mutex<Vec<(String, u8)>>,
        gate: Arc<(Mutex<bool>, std::sync::Condvar)>,
        first_outcome: Mutex<Option<PlatformResult<SoundPlayOutcome>>>,
        play_count: Mutex<usize>,
        entered: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl StallingFakeSink {
        fn new(
            gate: Arc<(Mutex<bool>, std::sync::Condvar)>,
            first_outcome: PlatformResult<SoundPlayOutcome>,
        ) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                gate,
                first_outcome: Mutex::new(Some(first_outcome)),
                play_count: Mutex::new(0),
                entered: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }
        }

        fn calls(&self) -> Vec<(String, u8)> {
            self.calls.lock().unwrap().clone()
        }

        fn entered(&self) -> Arc<std::sync::atomic::AtomicBool> {
            Arc::clone(&self.entered)
        }

        fn release(gate: &Arc<(Mutex<bool>, std::sync::Condvar)>) {
            *gate.0.lock().unwrap() = true;
            gate.1.notify_all();
        }
    }

    impl SoundSink for StallingFakeSink {
        fn play_file(&self, path: &str, volume: u8) -> PlatformResult<SoundPlayOutcome> {
            self.calls.lock().unwrap().push((path.to_string(), volume));
            let is_first = {
                let mut count = self.play_count.lock().unwrap();
                *count += 1;
                *count == 1
            };
            if is_first {
                self.entered
                    .store(true, std::sync::atomic::Ordering::Release);
                let gate = Arc::clone(&self.gate);
                let mut released = gate.0.lock().unwrap();
                while !*released {
                    released = gate.1.wait(released).unwrap();
                }
                return self
                    .first_outcome
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap_or(Ok(SoundPlayOutcome::Accepted));
            }
            Ok(SoundPlayOutcome::Accepted)
        }
    }

    impl SoundSink for Arc<StallingFakeSink> {
        fn play_file(&self, path: &str, volume: u8) -> PlatformResult<SoundPlayOutcome> {
            (**self).play_file(path, volume)
        }
    }

    struct TestClock {
        now: Mutex<Instant>,
    }

    impl TestClock {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                now: Mutex::new(Instant::now()),
            })
        }

        fn advance(&self, secs: u64) {
            *self.now.lock().unwrap() += Duration::from_secs(secs);
        }

        fn getter(self: &Arc<Self>) -> impl Fn() -> Instant + Send + Sync + 'static {
            let c = Arc::clone(self);
            move || *c.now.lock().unwrap()
        }
    }

    fn cfg(enabled: bool, events: &[(&str, &str)]) -> SoundConfig {
        let mut c = SoundConfig {
            enabled,
            clip_dir: "sounds".into(),
            volume: 3,
            debounce_secs: 30,
            events: Default::default(),
        };
        for (k, v) in events {
            c.events.insert((*k).into(), (*v).into());
        }
        c
    }

    fn player(
        config: SoundConfig,
        sink: Arc<FakeSink>,
        clock: &Arc<TestClock>,
    ) -> SoundPlayer<Arc<FakeSink>, impl Fn() -> Instant + Send + Sync + 'static> {
        SoundPlayer::new(config, sink, clock.getter())
    }

    fn stalling_player(
        config: SoundConfig,
        sink: Arc<StallingFakeSink>,
        clock: &Arc<TestClock>,
    ) -> SoundPlayer<Arc<StallingFakeSink>, impl Fn() -> Instant + Send + Sync + 'static> {
        SoundPlayer::new(config, sink, clock.getter())
    }

    #[test]
    fn test_play_disabled_config_plays_nothing() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(false, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Disabled);
        assert!(sink.calls().is_empty());
    }

    #[test]
    fn test_play_unmapped_event_plays_nothing() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        assert_eq!(p.play("network_lost").unwrap(), SoundPlayResult::NoClip);
        assert!(sink.calls().is_empty());
    }

    #[test]
    fn test_play_first_event_is_accepted() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        assert_eq!(sink.calls(), vec![("sounds/boot.raw".into(), 3)]);
    }

    #[test]
    fn test_play_repeat_within_debounce_is_dropped() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        clock.advance(10);
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Debounced);
        assert_eq!(sink.calls().len(), 1);
    }

    #[test]
    fn test_play_repeat_after_debounce_plays_again() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        clock.advance(30);
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        assert_eq!(sink.calls().len(), 2);
    }

    #[test]
    fn test_play_debounce_is_per_event_not_global() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(
                true,
                &[("network_up", "ok.raw"), ("upgrade_result", "ok.raw")],
            ),
            Arc::clone(&sink),
            &clock,
        );
        assert_eq!(p.play("network_up").unwrap(), SoundPlayResult::Accepted);
        clock.advance(5);
        assert_eq!(p.play("upgrade_result").unwrap(), SoundPlayResult::Accepted);
        assert_eq!(sink.calls().len(), 2);
    }

    #[test]
    fn test_play_busy_response_is_not_an_error() {
        let sink = Arc::new(FakeSink::default());
        sink.push_outcome(Ok(SoundPlayOutcome::Busy));
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Busy);
        assert_eq!(sink.calls().len(), 1);
    }

    #[test]
    fn test_clip_path_is_joined_under_clip_dir() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let mut c = cfg(true, &[("boot_ready", "boot.raw")]);
        c.clip_dir = "/mnt/anyka_hack/onvif/sounds".into();
        let p = player(c, Arc::clone(&sink), &clock);
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        assert_eq!(sink.calls()[0].0, "/mnt/anyka_hack/onvif/sounds/boot.raw");
    }

    #[test]
    fn test_play_sink_error_does_not_clear_newer_debounce_entry() {
        let gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let sink = Arc::new(StallingFakeSink::new(
            Arc::clone(&gate),
            Err(PlatformError::HardwareFailure("stall".into())),
        ));
        let clock = Arc::new(TestClock::new());
        let p = Arc::new(stalling_player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        ));

        let p_stalled = Arc::clone(&p);
        let gate_clone = Arc::clone(&gate);
        let entered = sink.entered();
        let stalled = std::thread::spawn(move || {
            assert!(p_stalled.play("boot_ready").is_err());
        });

        while !entered.load(std::sync::atomic::Ordering::Acquire) {
            std::thread::yield_now();
        }

        clock.advance(31);
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);

        StallingFakeSink::release(&gate_clone);
        stalled.join().unwrap();

        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Debounced);
        assert_eq!(sink.calls().len(), 2);
    }

    #[test]
    fn test_play_sink_error_is_propagated_and_clears_debounce() {
        let sink = Arc::new(FakeSink::default());
        sink.push_outcome(Err(PlatformError::HardwareFailure("boom".into())));
        sink.push_outcome(Ok(SoundPlayOutcome::Accepted));
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        assert!(p.play("boot_ready").is_err());
        // Immediate retry must not be reported as Debounced after a sink failure.
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        assert_eq!(sink.calls().len(), 2);
    }

    #[test]
    fn test_link_edge_ignores_steady_state_and_baseline() {
        let mut w = LinkEdgeWatcher::default();
        assert_eq!(w.observe(true), None); // baseline
        assert_eq!(w.observe(true), None); // steady
        assert_eq!(w.observe(false), Some("network_lost"));
        assert_eq!(w.observe(false), None);
        assert_eq!(w.observe(true), Some("network_up"));
        assert_eq!(w.observe(true), None);
    }

    #[test]
    fn test_trial_confirm_fires_once_on_marker_clear() {
        let mut w = TrialConfirmWatcher::default();
        assert!(!w.observe(false)); // never saw marker
        assert!(!w.observe(true)); // saw it
        assert!(!w.observe(true));
        assert!(w.observe(false)); // cleared → fire
        assert!(!w.observe(false)); // once only
        assert!(!w.observe(true));
        assert!(!w.observe(false));
    }
}
