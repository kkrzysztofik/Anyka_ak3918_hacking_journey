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
use crate::platform::{PlatformError, PlatformResult};

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

    /// Fire-and-forget play for a named event. Never treats busy as an error.
    pub fn play(&self, event: &str) -> PlatformResult<()> {
        if !self.config.enabled {
            debug!(event, "sound disabled; skip");
            return Ok(());
        }
        let Some(clip) = self.config.clip_for(event) else {
            debug!(event, "no clip mapped; skip");
            return Ok(());
        };

        let now = (self.clock)();
        {
            let mut last = self
                .last_played
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(prev) = last.get(event) {
                if now.duration_since(*prev) < Duration::from_secs(self.config.debounce_secs) {
                    debug!(event, "within debounce; skip");
                    return Ok(());
                }
            }
            last.insert(event.to_string(), now);
        }

        let path = clip_path(&self.config.clip_dir, clip);
        let path_str = path.to_string_lossy();
        match self.sink.play_file(path_str.as_ref(), self.config.volume)? {
            SoundPlayOutcome::Accepted => {
                debug!(event, path = %path_str, "sound play accepted");
                Ok(())
            }
            SoundPlayOutcome::Busy => {
                debug!(event, path = %path_str, "sound play busy; dropped");
                Ok(())
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

    #[test]
    fn disabled_config_plays_nothing() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(false, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        p.play("boot_ready").unwrap();
        assert!(sink.calls().is_empty());
    }

    #[test]
    fn unmapped_event_plays_nothing() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        p.play("network_lost").unwrap();
        assert!(sink.calls().is_empty());
    }

    #[test]
    fn first_event_plays() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        p.play("boot_ready").unwrap();
        assert_eq!(sink.calls(), vec![("sounds/boot.raw".into(), 3)]);
    }

    #[test]
    fn repeat_within_debounce_is_dropped() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        p.play("boot_ready").unwrap();
        clock.advance(10);
        p.play("boot_ready").unwrap();
        assert_eq!(sink.calls().len(), 1);
    }

    #[test]
    fn repeat_after_debounce_plays_again() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        p.play("boot_ready").unwrap();
        clock.advance(30);
        p.play("boot_ready").unwrap();
        assert_eq!(sink.calls().len(), 2);
    }

    #[test]
    fn debounce_is_per_event_not_global() {
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
        p.play("network_up").unwrap();
        clock.advance(5);
        p.play("upgrade_result").unwrap();
        assert_eq!(sink.calls().len(), 2);
    }

    #[test]
    fn busy_response_is_not_an_error() {
        let sink = Arc::new(FakeSink::default());
        sink.push_outcome(Ok(SoundPlayOutcome::Busy));
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        assert!(p.play("boot_ready").is_ok());
        assert_eq!(sink.calls().len(), 1);
    }

    #[test]
    fn clip_path_is_joined_under_clip_dir() {
        let sink = Arc::new(FakeSink::default());
        let clock = TestClock::new();
        let mut c = cfg(true, &[("boot_ready", "boot.raw")]);
        c.clip_dir = "/mnt/anyka_hack/onvif/sounds".into();
        let p = player(c, Arc::clone(&sink), &clock);
        p.play("boot_ready").unwrap();
        assert_eq!(sink.calls()[0].0, "/mnt/anyka_hack/onvif/sounds/boot.raw");
    }

    #[test]
    fn sink_error_is_propagated() {
        let sink = Arc::new(FakeSink::default());
        sink.push_outcome(Err(PlatformError::HardwareFailure("boom".into())));
        let clock = TestClock::new();
        let p = player(
            cfg(true, &[("boot_ready", "boot.raw")]),
            Arc::clone(&sink),
            &clock,
        );
        assert!(p.play("boot_ready").is_err());
    }
}
