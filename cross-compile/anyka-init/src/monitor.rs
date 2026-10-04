//! System resource sampling, replacing `sys_monitor.sh`, plus wifi link health.

use crate::config::MonitorCfg;
use crate::netstat::{self, Action, Health, Policy};
use crate::storm::StormState;
use crate::supervisor_loop::Msg;
use crate::sys::Sys;
use crate::wifi::udhcpc_oneshot_args;
use std::sync::mpsc::Sender;
use std::time::Duration;

const BUSYBOX: &str = "/bin/busybox";

pub fn parse_mem_kb(meminfo: &str) -> Option<u64> {
    let field = |name: &str| -> Option<u64> {
        meminfo
            .lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse().ok())
    };
    field("MemAvailable:").or_else(|| field("MemFree:"))
}

fn sample() {
    let mem = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| parse_mem_kb(&s));
    tracing::info!(mem_avail_kb = mem, "sys");
}

fn sample_link(iface: &str, probe: bool) -> Health {
    let carrier = std::fs::read_to_string(format!("/sys/class/net/{iface}/operstate"))
        .is_ok_and(|s| netstat::parse_operstate(&s))
        && std::fs::read_to_string(format!("/sys/class/net/{iface}/carrier"))
            .is_ok_and(|s| s.trim() == "1");
    let gw = std::fs::read_to_string("/proc/net/route")
        .ok()
        .and_then(|s| netstat::parse_default_route(&s, iface));
    let route = gw.is_some();
    let reachable = match &gw {
        Some(gw) if carrier && probe => netstat::gateway_reachable(gw),
        // Without a probe, treat route presence as reachability so we do not
        // escalate on L3 alone when probing is disabled.
        _ => route,
    };
    Health {
        carrier,
        route,
        reachable,
    }
}

/// Wifi escalation for one tick: updates `ticks`, decides the next `Action`
/// via `netstat::decide`, and applies it. Split out from `tick` so tests can
/// drive the decision/escalation ladder by injecting a `Health` directly,
/// without going through `/sys` or `/proc`.
pub fn apply_wifi_actions(
    sys: &dyn Sys,
    cfg: &MonitorCfg,
    iface: &str,
    state_path: &str,
    tx: &Sender<Msg>,
    h: Health,
    ticks: &mut u32,
) {
    if h.ok() {
        *ticks = 0;
    } else {
        *ticks = ticks.saturating_add(1);
        tracing::warn!(
            carrier = h.carrier,
            route = h.route,
            reachable = h.reachable,
            ticks = *ticks,
            "wifi link unhealthy"
        );
    }

    let storm = StormState::load(state_path);
    let policy = Policy {
        dhcp_after_ticks: cfg.wifi_dhcp_after_ticks,
        supplicant_after_ticks: cfg.wifi_supplicant_after_ticks,
        reboot_after_ticks: cfg.wifi_reboot_after_ticks,
        reboot_cap: cfg.wifi_reboot_cap,
        wifi_reboots_used: storm.wifi_reboots,
    };
    match netstat::decide(h, *ticks, &policy) {
        Action::Nothing => {}
        Action::RunDhcp => {
            tracing::warn!("no default route; re-running udhcpc");
            let _ = sys.run_to_completion(BUSYBOX, &udhcpc_oneshot_args(iface));
            // Do not reset ticks: decide uses absolute thresholds, so
            // clearing here would re-fire the same rung forever.
        }
        Action::RestartSupplicant => {
            let _ = tx.send(Msg::RestartService("wpa_supplicant".into()));
        }
        Action::Reboot => {
            tracing::error!(
                wifi_reboots = storm.wifi_reboots,
                "wifi down past the reboot threshold; rebooting"
            );
            let mut storm = storm;
            storm.wifi_reboots = storm.wifi_reboots.saturating_add(1);
            let _ = storm.save(state_path);
            if let Err(e) = sys.reboot() {
                tracing::error!(error = %e, "reboot() returned without rebooting");
            }
            // Failed reboot: keep ticks so we stay at LogOnly/Reboot
            // rather than dropping back to RunDhcp.
        }
        Action::LogOnly => {
            tracing::error!("wifi down and the reboot budget is exhausted; not rebooting");
        }
    }
}

/// Read the heartbeat counter. `None` means "no signal yet", which is not a
/// stall — see the test.
fn read_heartbeat(path: &str) -> Option<u64> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Video escalation for one tick.
///
/// Compares consecutive counter values rather than checking mtime: P2.5 steps
/// the wall clock by decades, so any mtime-based liveness would either fire
/// instantly or never (see the note in `supervise.rs`).
pub fn apply_video_actions(
    sys: &dyn Sys,
    cfg: &MonitorCfg,
    tx: &Sender<Msg>,
    frames: Option<u64>,
    last: &mut Option<u64>,
    ticks: &mut u32,
) {
    let Some(frames) = frames else {
        *ticks = 0;
        *last = None;
        return;
    };
    if *last != Some(frames) {
        *ticks = 0;
        *last = Some(frames);
    } else {
        *ticks = ticks.saturating_add(1);
        tracing::warn!(frames, ticks = *ticks, "video frames stalled");
    }

    match video_decide(*ticks, cfg) {
        VideoAction::Nothing => {}
        VideoAction::Restart => {
            let _ = tx.send(Msg::RestartService("vendor-daemon".into()));
        }
        VideoAction::Kill => {
            let _ = tx.send(Msg::KillService("vendor-daemon".into()));
        }
        VideoAction::Reboot => {
            tracing::error!(
                ticks = *ticks,
                "video stalled past the reboot threshold; rebooting"
            );
            if let Err(e) = sys.reboot() {
                tracing::error!(error = %e, "reboot() returned without rebooting");
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoAction {
    Nothing,
    Restart,
    Kill,
    Reboot,
}

/// Escalation ladder for a stalled video pipeline.
///
/// Ordered strongest-first, like `netstat::decide`, so absolute thresholds
/// cannot skip a rung. Three rungs rather than two because `RestartService`
/// sends SIGTERM, which a wedged daemon can ignore; the reboot rung does not
/// need the process to die at all.
pub fn video_decide(stalled_ticks: u32, cfg: &MonitorCfg) -> VideoAction {
    if stalled_ticks >= cfg.video_reboot_after_ticks {
        return VideoAction::Reboot;
    }
    if stalled_ticks >= cfg.video_kill_after_ticks {
        return VideoAction::Kill;
    }
    if stalled_ticks >= cfg.video_restart_after_ticks {
        return VideoAction::Restart;
    }
    VideoAction::Nothing
}

/// What `tick` carries from one iteration to the next.
#[derive(Debug, Default)]
pub struct TickState {
    pub reset_done: bool,
    pub wifi_ticks: u32,
    pub video_last: Option<u64>,
    pub video_ticks: u32,
}

/// One iteration of the sampling loop, including the storm-guard reset: once
/// this process has been up longer than the configured threshold, the boot is
/// considered good.
pub fn tick(
    sys: &dyn Sys,
    cfg: &MonitorCfg,
    iface: &str,
    state_path: &str,
    reset_after: Duration,
    tx: &Sender<Msg>,
    st: &mut TickState,
) {
    sample();
    if !st.reset_done && sys.uptime() > reset_after {
        // Reset only the crash-loop counter. wifi_reboots is cleared solely
        // by a successful wifi::bring_up (B4) — uptime alone would wipe it
        // before the link ever recovers.
        let mut storm = StormState::load(state_path);
        storm.fast_reboots = 0;
        match storm.save(state_path) {
            Ok(()) => tracing::info!("boot considered good; storm-guard counter reset"),
            Err(e) => tracing::warn!(error = %e, "failed to reset storm-guard state"),
        }
        st.reset_done = true;
    }

    if cfg.wifi {
        let h = sample_link(iface, cfg.wifi_probe);
        apply_wifi_actions(sys, cfg, iface, state_path, tx, h, &mut st.wifi_ticks);
    }

    if cfg.video {
        let frames = read_heartbeat(&cfg.video_heartbeat_path);
        apply_video_actions(
            sys,
            cfg,
            tx,
            frames,
            &mut st.video_last,
            &mut st.video_ticks,
        );
    }
}

/// Sampling loop. See `tick` for what happens each iteration.
pub fn run(
    sys: &dyn Sys,
    cfg: &MonitorCfg,
    iface: &str,
    state_path: &str,
    reset_after: Duration,
    tx: Sender<Msg>,
) {
    let interval = Duration::from_secs(cfg.interval_sec);
    let mut st = TickState::default();
    loop {
        tick(sys, cfg, iface, state_path, reset_after, &tx, &mut st);
        std::thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::MockSys;
    use std::sync::mpsc;

    #[test]
    fn test_parse_mem_kb_prefers_available() {
        let src = "MemTotal: 100\nMemFree: 10\nMemAvailable: 40\n";
        assert_eq!(parse_mem_kb(src), Some(40));
    }

    #[test]
    fn test_parse_mem_kb_falls_back_to_free() {
        let src = "MemTotal: 100\nMemFree: 10\n";
        assert_eq!(parse_mem_kb(src), Some(10));
    }

    fn healthy() -> Health {
        Health {
            carrier: true,
            route: true,
            reachable: true,
        }
    }

    fn unhealthy() -> Health {
        Health {
            carrier: false,
            route: false,
            reachable: false,
        }
    }

    #[test]
    fn test_apply_wifi_actions_healthy_link_resets_ticks_and_does_nothing() {
        let sys = MockSys::new(); // no expectations: must not touch the OS
        let cfg = MonitorCfg::default();
        let (tx, rx) = mpsc::channel();
        let dir = tempfile::tempdir().expect("tempdir");
        let state_path = dir.path().join("boot.json");
        let mut ticks = 5;

        apply_wifi_actions(
            &sys,
            &cfg,
            "wlan0",
            state_path.to_str().expect("utf8"),
            &tx,
            healthy(),
            &mut ticks,
        );

        assert_eq!(ticks, 0);
        assert!(rx.try_recv().is_err(), "a healthy link must send nothing");
    }

    #[test]
    fn test_apply_wifi_actions_run_dhcp_after_the_dhcp_threshold() {
        let mut sys = MockSys::new();
        sys.expect_run_to_completion()
            .withf(|prog, args| {
                prog == "/bin/busybox" && args.first().map(String::as_str) == Some("udhcpc")
            })
            .times(1)
            .returning(|_, _| Ok(crate::sys::ExitStatus::Code(0)));
        let cfg = MonitorCfg::default(); // wifi_dhcp_after_ticks = 3
        let (tx, rx) = mpsc::channel();
        let dir = tempfile::tempdir().expect("tempdir");
        let state_path = dir.path().join("boot.json");
        let mut ticks = 2;

        let h = Health {
            carrier: true,
            route: false,
            reachable: false,
        };
        apply_wifi_actions(
            &sys,
            &cfg,
            "wlan0",
            state_path.to_str().expect("utf8"),
            &tx,
            h,
            &mut ticks,
        );

        assert_eq!(ticks, 3);
        assert!(
            rx.try_recv().is_err(),
            "RunDhcp must not message the supervisor"
        );
    }

    #[test]
    fn test_apply_wifi_actions_restarts_supplicant_after_the_supplicant_threshold() {
        let sys = MockSys::new(); // RestartSupplicant does not touch Sys at all
        let cfg = MonitorCfg::default(); // wifi_supplicant_after_ticks = 5
        let (tx, rx) = mpsc::channel();
        let dir = tempfile::tempdir().expect("tempdir");
        let state_path = dir.path().join("boot.json");
        let mut ticks = 4;

        apply_wifi_actions(
            &sys,
            &cfg,
            "wlan0",
            state_path.to_str().expect("utf8"),
            &tx,
            unhealthy(),
            &mut ticks,
        );

        assert_eq!(ticks, 5);
        match rx.try_recv() {
            Ok(Msg::RestartService(name)) => assert_eq!(name, "wpa_supplicant"),
            Ok(_) => panic!("expected a RestartService message"),
            Err(e) => panic!("expected a message, got error: {e:?}"),
        }
    }

    #[test]
    fn test_apply_wifi_actions_reboots_past_the_reboot_threshold() {
        let mut sys = MockSys::new();
        sys.expect_reboot().times(1).returning(|| Ok(()));
        let cfg = MonitorCfg::default(); // wifi_reboot_after_ticks = 10, cap = 3
        let (tx, rx) = mpsc::channel();
        let dir = tempfile::tempdir().expect("tempdir");
        let state_path = dir.path().join("boot.json");
        crate::storm::StormState {
            fast_reboots: 0,
            wifi_reboots: 0,
        }
        .save(state_path.to_str().expect("utf8"))
        .expect("seed storm state");
        let mut ticks = 9;

        apply_wifi_actions(
            &sys,
            &cfg,
            "wlan0",
            state_path.to_str().expect("utf8"),
            &tx,
            unhealthy(),
            &mut ticks,
        );

        assert_eq!(ticks, 10);
        assert!(rx.try_recv().is_err());
        let storm = crate::storm::StormState::load(state_path.to_str().expect("utf8"));
        assert_eq!(storm.wifi_reboots, 1, "each fired reboot must persist");
    }

    #[test]
    fn test_apply_wifi_actions_stops_rebooting_past_the_cap() {
        let sys = MockSys::new(); // no expect_reboot(): calling it would panic
        let cfg = MonitorCfg::default(); // reboot_cap = 3
        let (tx, _rx) = mpsc::channel();
        let dir = tempfile::tempdir().expect("tempdir");
        let state_path = dir.path().join("boot.json");
        crate::storm::StormState {
            fast_reboots: 0,
            wifi_reboots: 3,
        }
        .save(state_path.to_str().expect("utf8"))
        .expect("seed storm state");
        let mut ticks = 20;

        apply_wifi_actions(
            &sys,
            &cfg,
            "wlan0",
            state_path.to_str().expect("utf8"),
            &tx,
            unhealthy(),
            &mut ticks,
        );

        let storm = crate::storm::StormState::load(state_path.to_str().expect("utf8"));
        assert_eq!(
            storm.wifi_reboots, 3,
            "LogOnly must not bump the counter further"
        );
    }

    #[test]
    fn test_tick_resets_the_crash_loop_counter_once_uptime_passes_the_threshold() {
        let mut sys = MockSys::new();
        sys.expect_uptime().returning(|| Duration::from_secs(700));
        let cfg = MonitorCfg {
            wifi: false,
            video: false,
            ..MonitorCfg::default()
        };
        let (tx, _rx) = mpsc::channel();
        let dir = tempfile::tempdir().expect("tempdir");
        let state_path = dir.path().join("boot.json");
        crate::storm::StormState {
            fast_reboots: 2,
            wifi_reboots: 1,
        }
        .save(state_path.to_str().expect("utf8"))
        .expect("seed storm state");

        let mut st = TickState::default();
        tick(
            &sys,
            &cfg,
            "wlan0",
            state_path.to_str().expect("utf8"),
            Duration::from_secs(600),
            &tx,
            &mut st,
        );

        assert!(st.reset_done);
        let storm = crate::storm::StormState::load(state_path.to_str().expect("utf8"));
        assert_eq!(storm.fast_reboots, 0, "boot considered good");
        assert_eq!(
            storm.wifi_reboots, 1,
            "only the crash-loop counter resets here"
        );
    }

    #[test]
    fn test_tick_does_not_reset_before_the_uptime_threshold() {
        let mut sys = MockSys::new();
        sys.expect_uptime().returning(|| Duration::from_secs(1));
        let cfg = MonitorCfg {
            wifi: false,
            video: false,
            ..MonitorCfg::default()
        };
        let (tx, _rx) = mpsc::channel();
        let dir = tempfile::tempdir().expect("tempdir");
        let state_path = dir.path().join("boot.json");

        let mut st = TickState::default();
        tick(
            &sys,
            &cfg,
            "wlan0",
            state_path.to_str().expect("utf8"),
            Duration::from_secs(600),
            &tx,
            &mut st,
        );

        assert!(!st.reset_done);
        assert!(
            !state_path.exists(),
            "nothing to persist before the threshold"
        );
    }

    fn cfg_with_video() -> MonitorCfg {
        MonitorCfg {
            wifi: false,
            video: true,
            video_restart_after_ticks: 2,
            video_kill_after_ticks: 3,
            video_reboot_after_ticks: 5,
            ..MonitorCfg::default()
        }
    }

    #[test]
    fn test_apply_video_actions_absent_heartbeat_is_not_a_stall() {
        // Push threads only start on CMD_VENC_START_PUSH from onvif-rust, so
        // there is a legitimate no-frames window at startup and when streaming
        // is off. An absent counter must never escalate.
        let mut ticks = 3;
        let (tx, rx) = mpsc::channel();
        let sys = MockSys::new(); // no reboot expectation: must not be called

        apply_video_actions(&sys, &cfg_with_video(), &tx, None, &mut None, &mut ticks);

        assert_eq!(ticks, 0, "absent heartbeat resets rather than escalates");
        assert!(rx.try_recv().is_err(), "no message must be sent");
    }

    #[test]
    fn test_apply_video_actions_advancing_counter_resets_ticks() {
        let mut ticks = 4;
        let (tx, _rx) = mpsc::channel();
        let sys = MockSys::new();

        apply_video_actions(
            &sys,
            &cfg_with_video(),
            &tx,
            Some(1000),
            &mut None,
            &mut ticks,
        );

        assert_eq!(ticks, 0);
    }

    #[test]
    fn test_apply_video_actions_stalled_counter_escalates_to_restart() {
        let (tx, rx) = mpsc::channel();
        let sys = MockSys::new();
        let cfg = cfg_with_video();
        let mut last = None;
        let mut ticks = 0;

        // Same value three times: first call seeds, next two are stalls.
        apply_video_actions(&sys, &cfg, &tx, Some(500), &mut last, &mut ticks);
        apply_video_actions(&sys, &cfg, &tx, Some(500), &mut last, &mut ticks);
        apply_video_actions(&sys, &cfg, &tx, Some(500), &mut last, &mut ticks);

        assert!(matches!(
            rx.try_recv(),
            Ok(Msg::RestartService(ref s)) if s == "vendor-daemon"
        ));
    }

    #[test]
    fn test_apply_video_actions_reboots_when_the_stall_persists() {
        let (tx, _rx) = mpsc::channel();
        let mut sys = MockSys::new();
        sys.expect_reboot().times(1).returning(|| Ok(()));
        let cfg = cfg_with_video();
        let mut last = Some(500);
        let mut ticks = 5;

        apply_video_actions(&sys, &cfg, &tx, Some(500), &mut last, &mut ticks);
    }

    #[test]
    fn test_video_decide_does_nothing_while_frames_advance() {
        assert_eq!(video_decide(0, &cfg_with_video()), VideoAction::Nothing);
    }

    #[test]
    fn test_video_decide_restarts_at_the_restart_threshold() {
        assert_eq!(video_decide(2, &cfg_with_video()), VideoAction::Restart);
    }

    #[test]
    fn test_video_decide_escalates_to_kill_then_reboot() {
        assert_eq!(video_decide(3, &cfg_with_video()), VideoAction::Kill);
        assert_eq!(video_decide(4, &cfg_with_video()), VideoAction::Kill);
        assert_eq!(video_decide(5, &cfg_with_video()), VideoAction::Reboot);
        assert_eq!(video_decide(50, &cfg_with_video()), VideoAction::Reboot);
    }

    #[test]
    fn test_video_decide_below_the_first_threshold_is_nothing() {
        assert_eq!(video_decide(1, &cfg_with_video()), VideoAction::Nothing);
    }
}
