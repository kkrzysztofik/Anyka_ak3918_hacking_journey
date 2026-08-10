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

pub fn parse_loadavg(src: &str) -> Option<f32> {
    src.split_whitespace().next().and_then(|v| v.parse().ok())
}

fn sample() {
    let mem = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| parse_mem_kb(&s));
    let load = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| parse_loadavg(&s));
    tracing::info!(mem_avail_kb = mem, load1 = load, "sys");
}

fn sample_link(iface: &str, probe: bool) -> Health {
    let oper = std::fs::read_to_string(format!("/sys/class/net/{iface}/operstate"))
        .map(|s| netstat::parse_operstate(&s))
        .unwrap_or(false);
    let carrier = std::fs::read_to_string(format!("/sys/class/net/{iface}/carrier"))
        .map(|s| s.trim() == "1")
        .unwrap_or(false);
    let carrier = oper && carrier;
    let route = std::fs::read_to_string("/proc/net/route")
        .ok()
        .and_then(|s| netstat::parse_default_route(&s, iface))
        .is_some();
    let reachable = if carrier && route && probe {
        if let Some(gw) = std::fs::read_to_string("/proc/net/route")
            .ok()
            .and_then(|s| netstat::parse_default_route(&s, iface))
        {
            netstat::gateway_reachable(&gw)
        } else {
            false
        }
    } else {
        // Without a probe, treat route presence as reachability so we do not
        // escalate on L3 alone when probing is disabled.
        route
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

    let policy = VideoPolicy {
        restart_after_ticks: cfg.video_restart_after_ticks,
        kill_after_ticks: cfg.video_kill_after_ticks,
        reboot_after_ticks: cfg.video_reboot_after_ticks,
    };
    match video_decide(*ticks, &policy) {
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
pub struct VideoPolicy {
    pub restart_after_ticks: u32,
    pub kill_after_ticks: u32,
    pub reboot_after_ticks: u32,
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
pub fn video_decide(stalled_ticks: u32, p: &VideoPolicy) -> VideoAction {
    if stalled_ticks >= p.reboot_after_ticks {
        return VideoAction::Reboot;
    }
    if stalled_ticks >= p.kill_after_ticks {
        return VideoAction::Kill;
    }
    if stalled_ticks >= p.restart_after_ticks {
        return VideoAction::Restart;
    }
    VideoAction::Nothing
}

/// One iteration of the sampling loop, including the storm-guard reset: once
/// this process has been up longer than the configured threshold, the boot is
/// considered good.
#[allow(clippy::too_many_arguments)]
pub fn tick(
    sys: &dyn Sys,
    cfg: &MonitorCfg,
    iface: &str,
    state_path: &str,
    reset_after: Duration,
    tx: &Sender<Msg>,
    reset_done: &mut bool,
    ticks: &mut u32,
    video_last: &mut Option<u64>,
    video_ticks: &mut u32,
) {
    sample();
    if !*reset_done && sys.uptime() > reset_after {
        // Reset only the crash-loop counter. wifi_reboots is cleared solely
        // by a successful wifi::bring_up (B4) — uptime alone would wipe it
        // before the link ever recovers.
        let mut storm = StormState::load(state_path);
        storm.fast_reboots = 0;
        match storm.save(state_path) {
            Ok(()) => tracing::info!("boot considered good; storm-guard counter reset"),
            Err(e) => tracing::warn!(error = %e, "failed to reset storm-guard state"),
        }
        *reset_done = true;
    }

    if cfg.wifi {
        let h = sample_link(iface, cfg.wifi_probe);
        apply_wifi_actions(sys, cfg, iface, state_path, tx, h, ticks);
    }

    if cfg.video {
        let frames = read_heartbeat(&cfg.video_heartbeat_path);
        apply_video_actions(sys, cfg, tx, frames, video_last, video_ticks);
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
    let mut reset_done = false;
    let mut ticks: u32 = 0;
    let mut video_last: Option<u64> = None;
    let mut video_ticks: u32 = 0;
    loop {
        tick(
            sys,
            cfg,
            iface,
            state_path,
            reset_after,
            &tx,
            &mut reset_done,
            &mut ticks,
            &mut video_last,
            &mut video_ticks,
        );
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

    #[test]
    fn test_parse_loadavg_first_field() {
        assert_eq!(parse_loadavg("0.42 0.35 0.30 1/100 1234\n"), Some(0.42));
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

        let mut reset_done = false;
        let mut ticks = 0;
        tick(
            &sys,
            &cfg,
            "wlan0",
            state_path.to_str().expect("utf8"),
            Duration::from_secs(600),
            &tx,
            &mut reset_done,
            &mut ticks,
            &mut None,
            &mut 0,
        );

        assert!(reset_done);
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

        let mut reset_done = false;
        let mut ticks = 0;
        tick(
            &sys,
            &cfg,
            "wlan0",
            state_path.to_str().expect("utf8"),
            Duration::from_secs(600),
            &tx,
            &mut reset_done,
            &mut ticks,
            &mut None,
            &mut 0,
        );

        assert!(!reset_done);
        assert!(
            !state_path.exists(),
            "nothing to persist before the threshold"
        );
    }

    fn video_policy() -> VideoPolicy {
        VideoPolicy {
            restart_after_ticks: 2,
            kill_after_ticks: 3,
            reboot_after_ticks: 5,
        }
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
        assert_eq!(video_decide(0, &video_policy()), VideoAction::Nothing);
    }

    #[test]
    fn test_video_decide_restarts_at_the_restart_threshold() {
        assert_eq!(video_decide(2, &video_policy()), VideoAction::Restart);
    }

    #[test]
    fn test_video_decide_escalates_to_kill_then_reboot() {
        assert_eq!(video_decide(3, &video_policy()), VideoAction::Kill);
        assert_eq!(video_decide(4, &video_policy()), VideoAction::Kill);
        assert_eq!(video_decide(5, &video_policy()), VideoAction::Reboot);
        assert_eq!(video_decide(50, &video_policy()), VideoAction::Reboot);
    }

    #[test]
    fn test_video_decide_below_the_first_threshold_is_nothing() {
        assert_eq!(video_decide(1, &video_policy()), VideoAction::Nothing);
    }
}
