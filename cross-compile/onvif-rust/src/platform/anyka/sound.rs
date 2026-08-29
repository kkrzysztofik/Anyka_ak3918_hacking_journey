//! Event-triggered speaker playback policy.
//!
//! The daemon plays a file; this module owns debounce, event→clip mapping, and
//! drop-when-busy. All of that is host-testable without hardware.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::Instant;
use tracing::debug;

use crate::config::sound::SoundConfig;
use crate::hal::anyka::ipc::AudioPlayStatus;
#[cfg(test)]
use crate::platform::PlatformError;
use crate::platform::PlatformResult;

/// Policy-layer result of `SoundPlayer::play`: what the policy decided, as
/// opposed to `AudioPlayStatus`, which is only what the sink reported.
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

/// Sends a play request to the speaker path (IPC or test fake).
pub trait SoundSink: Send + Sync {
    fn play_file(&self, path: &str, volume: u8) -> PlatformResult<AudioPlayStatus>;
}

impl<T: SoundSink + ?Sized> SoundSink for Arc<T> {
    fn play_file(&self, path: &str, volume: u8) -> PlatformResult<AudioPlayStatus> {
        (**self).play_file(path, volume)
    }
}

/// Plays configured event clips with per-event debounce.
///
/// Debounce reads `tokio::time::Instant`, so paused-clock tests
/// (`#[tokio::test(start_paused = true)]`) drive it without sleeping.
pub struct SoundPlayer<S: SoundSink> {
    config: SoundConfig,
    sink: S,
    last_played: Mutex<HashMap<String, Instant>>,
}

impl<S: SoundSink> SoundPlayer<S> {
    pub fn new(config: SoundConfig, sink: S) -> Self {
        Self {
            config,
            sink,
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

        let now = Instant::now();
        {
            let mut last = lock(&self.last_played);
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
            Ok(AudioPlayStatus::Accepted) => {
                debug!(event, path = %path_str, "sound play accepted");
                Ok(SoundPlayResult::Accepted)
            }
            Ok(AudioPlayStatus::Busy) => {
                debug!(event, path = %path_str, "sound play busy; dropped");
                Ok(SoundPlayResult::Busy)
            }
            Err(e) => {
                rollback_debounce(&mut lock(&self.last_played), event, now);
                Err(e)
            }
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Undo this invocation's debounce stamp after a sink error.
///
/// Only removes the entry when it is still the stamp we wrote: a concurrent
/// retry may already have replaced it with a newer one, and clearing that would
/// let the retry's clip through the debounce window twice.
fn rollback_debounce(last: &mut HashMap<String, Instant>, event: &str, stamp: Instant) {
    if last.get(event) == Some(&stamp) {
        last.remove(event);
    }
}

fn clip_path(clip_dir: &str, clip: &str) -> PathBuf {
    Path::new(clip_dir).join(clip)
}

/// PCM rate of the shipped clip set. **Must match `RATE` in
/// `scripts/make_speech.py`** — the daemon passes this straight to
/// `ak_ao_open()` and nothing validates it against the file, so a mismatch
/// plays at the wrong pitch and speed with no error anywhere.
///
/// 16 kHz rather than 8: Polish sibilants carry most of their energy above
/// 4 kHz, which an 8 kHz anti-alias filter discards wholesale. The DA supports
/// 8/16/32/48 kHz.
const SOUND_SAMPLE_RATE: u32 = 16000;

/// Mono source. The DA is stereo-only and the daemon duplicates each sample to
/// L/R before sending (`sound_dup_mono_to_stereo`); this describes the *file*,
/// which is what `ak_ao_open()` wants — see `ak_ao_demo.c:280`.
const SOUND_CHANNELS: u32 = 1;

impl SoundSink for crate::hal::anyka::ipc::AnykaIpc {
    fn play_file(&self, path: &str, volume: u8) -> PlatformResult<AudioPlayStatus> {
        self.audio_play(path, SOUND_SAMPLE_RATE, SOUND_CHANNELS, i32::from(volume))
    }
}

/// Shared player used by boot / network / upgrade call sites.
pub type SharedSoundPlayer = Arc<SoundPlayer<Arc<crate::hal::anyka::ipc::AnykaIpc>>>;

/// Build a production player: IPC sink, clip directory resolved beside the binary.
///
/// A relative `clip_dir` is resolved against the executable's directory, not the
/// CWD and not `config.toml`'s parent: under the A/B slot layout the binary and
/// its clips live in `.../slots/{a,b}/onvif/` while the config may sit at the
/// flat `/mnt/anyka_hack/onvif/` path.
pub fn build_shared_player(
    mut config: SoundConfig,
    ipc: Arc<crate::hal::anyka::ipc::AnykaIpc>,
) -> SharedSoundPlayer {
    if !Path::new(&config.clip_dir).is_absolute() {
        config.clip_dir = exe_dir().join(&config.clip_dir).display().to_string();
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
    Arc::new(SoundPlayer::new(config, ipc))
}

/// Directory holding the running binary, or the flat payload path if unknowable.
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("/mnt/anyka_hack/onvif"))
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
                    return; // fires once; nothing left to watch
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
        next: Mutex<Vec<PlatformResult<AudioPlayStatus>>>,
    }

    impl FakeSink {
        fn push_outcome(&self, outcome: PlatformResult<AudioPlayStatus>) {
            self.next.lock().unwrap().push(outcome);
        }

        fn calls(&self) -> Vec<(String, u8)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl SoundSink for FakeSink {
        fn play_file(&self, path: &str, volume: u8) -> PlatformResult<AudioPlayStatus> {
            self.calls.lock().unwrap().push((path.to_string(), volume));
            let mut q = self.next.lock().unwrap();
            if q.is_empty() {
                Ok(AudioPlayStatus::Accepted)
            } else {
                q.remove(0)
            }
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

    fn player(config: SoundConfig, sink: Arc<FakeSink>) -> SoundPlayer<Arc<FakeSink>> {
        SoundPlayer::new(config, sink)
    }

    async fn advance(secs: u64) {
        tokio::time::advance(Duration::from_secs(secs)).await;
    }

    #[test]
    fn test_play_disabled_config_plays_nothing() {
        let sink = Arc::new(FakeSink::default());
        let p = player(cfg(false, &[("boot_ready", "boot.raw")]), Arc::clone(&sink));
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Disabled);
        assert!(sink.calls().is_empty());
    }

    #[test]
    fn test_play_unmapped_event_plays_nothing() {
        let sink = Arc::new(FakeSink::default());
        let p = player(cfg(true, &[("boot_ready", "boot.raw")]), Arc::clone(&sink));
        assert_eq!(p.play("network_lost").unwrap(), SoundPlayResult::NoClip);
        assert!(sink.calls().is_empty());
    }

    #[test]
    fn test_play_first_event_is_accepted() {
        let sink = Arc::new(FakeSink::default());
        let p = player(cfg(true, &[("boot_ready", "boot.raw")]), Arc::clone(&sink));
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        assert_eq!(sink.calls(), vec![("sounds/boot.raw".into(), 3)]);
    }

    #[tokio::test(start_paused = true)]
    async fn test_play_repeat_within_debounce_is_dropped() {
        let sink = Arc::new(FakeSink::default());
        let p = player(cfg(true, &[("boot_ready", "boot.raw")]), Arc::clone(&sink));
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        advance(10).await;
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Debounced);
        assert_eq!(sink.calls().len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn test_play_repeat_after_debounce_plays_again() {
        let sink = Arc::new(FakeSink::default());
        let p = player(cfg(true, &[("boot_ready", "boot.raw")]), Arc::clone(&sink));
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        advance(30).await;
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        assert_eq!(sink.calls().len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn test_play_debounce_is_per_event_not_global() {
        let sink = Arc::new(FakeSink::default());
        let p = player(
            cfg(
                true,
                &[("network_up", "ok.raw"), ("upgrade_result", "ok.raw")],
            ),
            Arc::clone(&sink),
        );
        assert_eq!(p.play("network_up").unwrap(), SoundPlayResult::Accepted);
        advance(5).await;
        assert_eq!(p.play("upgrade_result").unwrap(), SoundPlayResult::Accepted);
        assert_eq!(sink.calls().len(), 2);
    }

    #[test]
    fn test_play_busy_response_is_not_an_error() {
        let sink = Arc::new(FakeSink::default());
        sink.push_outcome(Ok(AudioPlayStatus::Busy));
        let p = player(cfg(true, &[("boot_ready", "boot.raw")]), Arc::clone(&sink));
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Busy);
        assert_eq!(sink.calls().len(), 1);
    }

    #[test]
    fn test_clip_path_is_joined_under_clip_dir() {
        let sink = Arc::new(FakeSink::default());
        let mut c = cfg(true, &[("boot_ready", "boot.raw")]);
        c.clip_dir = "/mnt/anyka_hack/onvif/sounds".into();
        let p = player(c, Arc::clone(&sink));
        assert_eq!(p.play("boot_ready").unwrap(), SoundPlayResult::Accepted);
        assert_eq!(sink.calls()[0].0, "/mnt/anyka_hack/onvif/sounds/boot.raw");
    }

    /// The race a slow sink can lose: a retry stamps a newer time while the first
    /// call is still in `play_file`, and the loser's rollback must not erase it.
    #[test]
    fn test_rollback_debounce_keeps_a_newer_stamp() {
        let mine = Instant::now();
        let newer = mine + Duration::from_secs(31);
        let mut last = HashMap::from([("boot_ready".to_string(), newer)]);

        rollback_debounce(&mut last, "boot_ready", mine);
        assert_eq!(last.get("boot_ready"), Some(&newer), "newer stamp survives");

        rollback_debounce(&mut last, "boot_ready", newer);
        assert!(last.is_empty(), "own stamp is rolled back");
    }

    #[test]
    fn test_play_sink_error_is_propagated_and_clears_debounce() {
        let sink = Arc::new(FakeSink::default());
        sink.push_outcome(Err(PlatformError::HardwareFailure("boom".into())));
        sink.push_outcome(Ok(AudioPlayStatus::Accepted));
        let p = player(cfg(true, &[("boot_ready", "boot.raw")]), Arc::clone(&sink));
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

    /// Clips ship beside the binary, so a relative clip_dir must land in the
    /// running slot's directory rather than the CWD.
    #[test]
    fn test_exe_dir_is_the_running_binarys_directory() {
        let exe = std::env::current_exe().expect("current_exe");
        assert_eq!(exe_dir(), exe.parent().unwrap());
    }
}
