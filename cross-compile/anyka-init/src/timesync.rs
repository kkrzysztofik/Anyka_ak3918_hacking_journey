//! SNTP client, replacing the fire-and-forget `ntpd -n -N -p <server> &` in
//! `gergehack.sh:357`.
//!
//! This is not cosmetic. `onvif-rust`'s `ws_security.rs:85` sets
//! `clock_skew_seconds: 300` and `:234-239` rejects any WS-UsernameToken
//! `Created` timestamp outside +/- 5 minutes of now. A camera at the epoch
//! rejects **every** authenticated ONVIF request.
//!
//! The response is unauthenticated UDP from the network and its content sets
//! the system clock, so `parse_response` validates aggressively.

use crate::config::TimeCfg;
use crate::sys::Sys;
use std::io::Read;
use std::net::{ToSocketAddrs, UdpSocket};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Seconds between the NTP epoch (1900-01-01) and the Unix epoch.
pub const NTP_UNIX_OFFSET: u64 = 2_208_988_800;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NtpError {
    #[error("not a server reply (mode {0})")]
    BadMode(u8),
    #[error("server reports itself unsynchronised (LI=3)")]
    Unsynchronised,
    #[error("unusable stratum {0}")]
    BadStratum(u8),
    #[error("originate timestamp does not echo our nonce")]
    NonceMismatch,
    #[error("transmit timestamp is zero")]
    ZeroTimestamp,
    #[error("implausible time: unix {0}")]
    Implausible(u64),
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min_unix: u64,
    pub max_unix: u64,
}

fn be64(pkt: &[u8; 48], off: usize) -> u64 {
    // Offsets are compile-time constants inside a fixed-size array, so this
    // slice is always exactly 8 bytes.
    let mut b = [0u8; 8];
    b.copy_from_slice(&pkt[off..off + 8]);
    u64::from_be_bytes(b)
}

/// Build a client request. `nonce` goes in the transmit-timestamp field; a
/// conformant server echoes it verbatim into the originate field, which is how
/// `parse_response` rejects off-path spoofing.
pub fn build_request(nonce: u64) -> [u8; 48] {
    let mut p = [0u8; 48];
    p[0] = 0b00_100_011; // LI=0, VN=4, Mode=3 (client)
    p[40..48].copy_from_slice(&nonce.to_be_bytes());
    p
}

pub fn parse_response(
    pkt: &[u8; 48],
    sent_nonce: u64,
    bounds: &Bounds,
) -> Result<SystemTime, NtpError> {
    let li = pkt[0] >> 6;
    let mode = pkt[0] & 0b111;
    if mode != 4 {
        return Err(NtpError::BadMode(mode));
    }
    if li == 3 {
        return Err(NtpError::Unsynchronised);
    }
    let stratum = pkt[1];
    if stratum == 0 || stratum > 15 {
        return Err(NtpError::BadStratum(stratum));
    }
    if be64(pkt, 24) != sent_nonce {
        return Err(NtpError::NonceMismatch);
    }
    let transmit = be64(pkt, 40);
    if transmit == 0 {
        return Err(NtpError::ZeroTimestamp);
    }

    let secs_1900 = transmit >> 32;
    let unix = secs_1900
        .checked_sub(NTP_UNIX_OFFSET)
        .ok_or(NtpError::Implausible(0))?;
    if unix < bounds.min_unix || unix > bounds.max_unix {
        return Err(NtpError::Implausible(unix));
    }

    let frac_nanos = (((transmit & 0xFFFF_FFFF) * 1_000_000_000) >> 32) as u32;
    Ok(UNIX_EPOCH + Duration::new(unix, frac_nanos))
}

/// Read a 64-bit nonce from `/dev/urandom`. Falls back to a mixed
/// wall-clock / pid / counter value if unavailable — weaker than urandom,
/// but `Instant::now().elapsed()` is ~0 and must not be used alone.
pub fn random_nonce() -> u64 {
    let mut buf = [0u8; 8];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom")
        && f.read_exact(&mut buf).is_ok()
    {
        return u64::from_be_bytes(buf);
    }
    fallback_nonce()
}

/// Non-urandom nonce: wall clock, pid, and a process-local counter so
/// successive calls differ even when the clock resolution is coarse.
fn fallback_nonce() -> u64 {
    // AtomicU64 is unavailable on ARMv5 (no 64-bit atomics); u32 is enough
    // to keep successive fallbacks distinct when mixed with wall/pid.
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let wall = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = u64::from(std::process::id());
    let n = u64::from(COUNTER.fetch_add(1, Ordering::Relaxed));
    let stack_mix = std::ptr::from_ref(&COUNTER) as u64;
    wall.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(pid.wrapping_shl(32))
        .wrapping_add(n.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(stack_mix)
}

/// `server` is a bare host (the default NTP port 123 is appended) or, for
/// tests that bind an ephemeral port, a `host:port` pair.
/// Resolve `server` to a `host:port` string for UDP.
///
/// Bare hostnames get `:123`. IPv6 literals are bracketed. An already-complete
/// socket address (or `host:port` for IPv4/hostname) is kept as-is so tests can
/// bind an ephemeral loopback port.
fn ntp_socket_target(server: &str) -> String {
    use std::net::{IpAddr, SocketAddr};
    if server.parse::<SocketAddr>().is_ok() {
        return server.to_string();
    }
    if let Ok(ip) = server.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(v4) => format!("{v4}:123"),
            IpAddr::V6(v6) => format!("[{v6}]:123"),
        };
    }
    match server.rsplit_once(':') {
        Some((host, port))
            if !host.is_empty() && !host.contains(':') && port.parse::<u16>().is_ok() =>
        {
            server.to_string()
        }
        _ => format!("{server}:123"),
    }
}

pub fn query(server: &str, timeout: Duration, bounds: &Bounds) -> anyhow::Result<SystemTime> {
    let target = ntp_socket_target(server);
    let addr = target
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("no address for {server}"))?;

    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_read_timeout(Some(timeout))?;
    sock.set_write_timeout(Some(timeout))?;

    let nonce = random_nonce();
    sock.send_to(&build_request(nonce), addr)?;

    let mut buf = [0u8; 48];
    let (n, from) = sock.recv_from(&mut buf)?;
    if n != 48 {
        anyhow::bail!("short NTP reply: {n} bytes");
    }
    if from.ip() != addr.ip() {
        anyhow::bail!("reply from {} but queried {}", from.ip(), addr.ip());
    }
    Ok(parse_response(&buf, nonce, bounds)?)
}

/// `{update_root}/state/ntp.disabled` — the manual-clock marker shared with
/// onvif-rust, which writes it on `SetSystemDateAndTime` Manual mode.
///
/// Deliberately a bare filename, not a parsed file: `update.rs:143` records
/// why structured state on exFAT after a power cut is a hazard here.
pub fn ntp_disabled_marker_path(update_root: &Path) -> PathBuf {
    update_root.join("state/ntp.disabled")
}

/// `{update_root}/state/ntp.status` — advisory sync state for the WebUI.
///
/// A sibling of `ntp.disabled` rather than a key in `anyka.toml`: adding a key
/// there would be a hard parse error for an older anyka-init in the other A/B
/// slot (`deny_unknown_fields`). Unlike the marker this one *is* parsed, which
/// is safe because it is display-only — a garbled line shows "unknown", it
/// never changes behaviour.
pub fn ntp_status_path(update_root: &Path) -> PathBuf {
    update_root.join("state/ntp.status")
}

/// One TSV line: `last_unix \t server \t delta_s \t s1,s2,…`.
/// `last_unix` is 0 when no sync has succeeded yet.
pub fn write_status(update_root: &Path, last: Option<(i64, &str, i64)>, servers: &[String]) {
    let path = ntp_status_path(update_root);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let (unix, server, delta) = last.unwrap_or((0, "", 0));
    let line = format!("{unix}\t{server}\t{delta}\t{}\n", servers.join(","));
    let _ = std::fs::write(&path, line);
}

/// Re-read `[time]` so a `set-ntp` (or a hand-edit) takes effect without a
/// reboot. Any read or parse failure keeps `current` — a config being edited
/// under us must not blank the server list.
pub fn reload_time_cfg(config_path: &Path, current: &TimeCfg) -> TimeCfg {
    match crate::config::Config::load_without_overlay(&config_path.to_string_lossy()) {
        Ok(cfg) => cfg.time,
        Err(e) => {
            tracing::warn!(error = %e, "could not reload [time]; keeping the running value");
            current.clone()
        }
    }
}

/// Current wall-clock time as unix seconds, the same conversion `delta_secs`
/// uses for `sys.realtime()`.
fn now_unix(sys: &dyn Sys) -> i64 {
    sys.realtime()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Query each configured server in turn; step the clock on the first success.
/// Returns `(server, applied delta in seconds)` for the server that answered,
/// or `None` if nothing was applied.
///
/// When `budget` is `Some`, stop before starting a server query that cannot
/// finish within the remaining time (socket timeout is capped to the budget).
/// DNS via `to_socket_addrs` has no caller-controlled deadline, so a single
/// hung resolver can still overrun — the budget still bounds retries and
/// subsequent servers.
pub fn sync_once(
    sys: &dyn Sys,
    cfg: &TimeCfg,
    budget: Option<Duration>,
    ntp_disabled: &Path,
) -> Option<(String, i64)> {
    // Manual clock mode (onvif-rust's `state/ntp.disabled`): do not step the
    // clock over the user's manual setting.
    if ntp_disabled.is_file() {
        tracing::info!(
            marker = %ntp_disabled.display(),
            "NTP disabled by operator (manual clock); not stepping the clock"
        );
        return None;
    }
    let bounds = Bounds {
        min_unix: cfg.min_plausible_unix,
        max_unix: cfg.max_plausible_unix,
    };
    let started = sys.now();
    for server in &cfg.servers {
        let remaining =
            budget.map(|b| b.saturating_sub(sys.now().saturating_duration_since(started)));
        if let Some(left) = remaining
            && left.is_zero()
        {
            tracing::warn!("NTP sync budget exhausted before querying {server}");
            return None;
        }
        let timeout = remaining
            .map(|left| left.min(Duration::from_secs(5)))
            .unwrap_or(Duration::from_secs(5));
        if timeout.is_zero() {
            return None;
        }
        match query(server, timeout, &bounds) {
            Ok(t) => {
                let before = sys.realtime();
                let delta = delta_secs(before, t);
                if delta.unsigned_abs() < cfg.step_threshold_sec {
                    tracing::debug!(server, delta, "clock already within threshold");
                    return Some((server.clone(), 0));
                }
                match sys.set_realtime(t) {
                    Ok(()) => {
                        tracing::info!(server, delta_sec = delta, "stepped system clock");
                        return Some((server.clone(), delta));
                    }
                    Err(e) => tracing::error!(server, error = %e, "clock_settime failed"),
                }
            }
            Err(e) => tracing::warn!(server, error = %e, "NTP query failed"),
        }
    }
    None
}

fn delta_secs(from: SystemTime, to: SystemTime) -> i64 {
    match to.duration_since(from) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

/// P2.5: bounded best-effort first sync. Never blocks boot beyond
/// `first_sync_timeout_sec`.
pub fn first_sync(sys: &dyn Sys, cfg: &TimeCfg, ntp_disabled: &Path) -> bool {
    if !cfg.enabled {
        return false;
    }
    let deadline = sys.now() + Duration::from_secs(cfg.first_sync_timeout_sec);
    loop {
        let remaining = deadline.saturating_duration_since(sys.now());
        if remaining.is_zero() {
            tracing::warn!(
                timeout_sec = cfg.first_sync_timeout_sec,
                "no NTP sync before boot deadline; continuing with a wrong clock. \
                 Authenticated ONVIF requests will fail until the resync thread succeeds."
            );
            return false;
        }
        if sync_once(sys, cfg, Some(remaining), ntp_disabled).is_some() {
            return true;
        }
        let left = deadline.saturating_duration_since(sys.now());
        if left.is_zero() {
            tracing::warn!(
                timeout_sec = cfg.first_sync_timeout_sec,
                "no NTP sync before boot deadline; continuing with a wrong clock. \
                 Authenticated ONVIF requests will fail until the resync thread succeeds."
            );
            return false;
        }
        // Bounded by `deadline` above, so a retry_interval longer than the
        // timeout simply means one attempt.
        sys.sleep(Duration::from_secs(cfg.retry_interval_sec.min(2)).min(left));
    }
}

/// Background resync loop, started after P3.
pub fn resync_loop(
    sys: &dyn Sys,
    cfg: &TimeCfg,
    ntp_disabled: &Path,
    config_path: &Path,
    update_root: &Path,
) {
    // Until the clock has been set once, retry at `retry_interval_sec`, not
    // `resync_interval_sec`. P2.5 gives up after 15s so that boot is not held
    // hostage to the network, which means a slow wifi association routinely
    // lands here with the clock still at the epoch. Sleeping the full 6h resync
    // interval first would leave ws_security.rs:85 (clock_skew_seconds = 300)
    // rejecting every authenticated ONVIF request for those 6 hours.
    let mut cfg = cfg.clone();
    let mut synced = false;
    let mut last: Option<(i64, String, i64)> = None;

    write_status(update_root, None, &cfg.servers);

    loop {
        std::thread::sleep(Duration::from_secs(resync_wait_secs(synced, &cfg)));
        // ponytail: re-parsing the whole config once per resync (6h steady
        // state) is the cheap way to pick up a `set-ntp`. If servers ever need
        // to apply in seconds, share a TimeCfg cell with the supervisor loop
        // instead of shortening this sleep.
        cfg = reload_time_cfg(config_path, &cfg);
        if let Some((server, delta)) = sync_once(sys, &cfg, None, ntp_disabled) {
            synced = true;
            last = Some((now_unix(sys), server, delta));
        }
        let borrowed = last.as_ref().map(|(u, s, d)| (*u, s.as_str(), *d));
        write_status(update_root, borrowed, &cfg.servers);
    }
}

/// How long `resync_loop` waits before its next attempt.
///
/// Split out so the fast-retry-until-first-success rule is testable without
/// running the loop.
pub fn resync_wait_secs(synced: bool, cfg: &TimeCfg) -> u64 {
    if synced {
        cfg.resync_interval_sec
    } else {
        cfg.retry_interval_sec
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    const NONCE: u64 = 0xDEAD_BEEF_CAFE_F00D;
    /// 2026-08-01T00:00:00Z as NTP seconds (unix + 2_208_988_800).
    const GOOD_NTP_SECS: u64 = 1_785_542_400 + NTP_UNIX_OFFSET;

    fn bounds() -> Bounds {
        Bounds {
            min_unix: 1_767_225_600,
            max_unix: 2_524_608_000,
        }
    }

    /// A well-formed server reply: LI=0, VN=4, Mode=4, stratum 2.
    fn good_packet() -> [u8; 48] {
        let mut p = [0u8; 48];
        p[0] = 0b00_100_100;
        p[1] = 2;
        p[24..32].copy_from_slice(&NONCE.to_be_bytes());
        p[40..48].copy_from_slice(&(GOOD_NTP_SECS << 32).to_be_bytes());
        p
    }

    #[test]
    fn test_parse_accepts_well_formed_reply() {
        let t = parse_response(&good_packet(), NONCE, &bounds()).expect("must accept");
        let unix = t
            .duration_since(std::time::UNIX_EPOCH)
            .expect("post-epoch")
            .as_secs();
        assert_eq!(unix, 1_785_542_400);
    }

    #[test]
    fn test_parse_rejects_non_server_mode() {
        let mut p = good_packet();
        p[0] = 0b00_100_011; // mode 3 = client
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::BadMode(3))
        );
    }

    #[test]
    fn test_parse_rejects_leap_indicator_alarm() {
        let mut p = good_packet();
        p[0] |= 0b11_000_000; // LI = 3, server says it is unsynchronised
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Unsynchronised)
        );
    }

    #[test]
    fn test_parse_rejects_kiss_of_death_stratum_zero() {
        let mut p = good_packet();
        p[1] = 0;
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::BadStratum(0))
        );
    }

    #[test]
    fn test_parse_rejects_unsynchronised_stratum_16() {
        let mut p = good_packet();
        p[1] = 16;
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::BadStratum(16))
        );
    }

    #[test]
    fn test_parse_rejects_nonce_mismatch() {
        // This is the anti-spoofing check. Without it any host that can guess
        // our query can set the camera's clock.
        let p = good_packet();
        assert_eq!(
            parse_response(&p, NONCE ^ 1, &bounds()),
            Err(NtpError::NonceMismatch)
        );
    }

    #[test]
    fn test_parse_rejects_zero_transmit_timestamp() {
        let mut p = good_packet();
        p[40..48].copy_from_slice(&0u64.to_be_bytes());
        assert_eq!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::ZeroTimestamp)
        );
    }

    #[test]
    fn test_parse_rejects_time_before_lower_bound() {
        let mut p = good_packet();
        // 2000-01-01, well before min_plausible.
        let secs = 946_684_800u64 + NTP_UNIX_OFFSET;
        p[40..48].copy_from_slice(&(secs << 32).to_be_bytes());
        assert!(matches!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Implausible(_))
        ));
    }

    #[test]
    fn test_parse_rejects_time_after_upper_bound() {
        let mut p = good_packet();
        let secs = 4_000_000_000u64 + NTP_UNIX_OFFSET;
        p[40..48].copy_from_slice(&(secs << 32).to_be_bytes());
        assert!(matches!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Implausible(_))
        ));
    }

    #[test]
    fn test_parse_rejects_pre_1900_timestamp_without_panicking() {
        let mut p = good_packet();
        p[40..48].copy_from_slice(&(1u64 << 32).to_be_bytes());
        assert!(matches!(
            parse_response(&p, NONCE, &bounds()),
            Err(NtpError::Implausible(_))
        ));
    }
}

#[cfg(test)]
mod delta_tests {
    use super::*;

    #[test]
    fn test_delta_secs_positive_when_target_is_later() {
        let a = UNIX_EPOCH + Duration::from_secs(1000);
        let b = UNIX_EPOCH + Duration::from_secs(1090);
        assert_eq!(delta_secs(a, b), 90);
    }

    #[test]
    fn test_delta_secs_negative_when_target_is_earlier() {
        let a = UNIX_EPOCH + Duration::from_secs(1090);
        let b = UNIX_EPOCH + Duration::from_secs(1000);
        assert_eq!(delta_secs(a, b), -90);
    }

    #[test]
    fn test_resync_waits_retry_interval_until_first_success() {
        let cfg = crate::config::TimeCfg::default();
        assert_eq!(
            resync_wait_secs(false, &cfg),
            cfg.retry_interval_sec,
            "before the first sync the clock is wrong and ONVIF auth is down; \
             retry fast, not once per resync interval"
        );
        assert_eq!(resync_wait_secs(true, &cfg), cfg.resync_interval_sec);
        assert!(cfg.retry_interval_sec < cfg.resync_interval_sec);
    }
}

#[cfg(test)]
mod nonce_tests {
    use super::*;

    #[test]
    fn test_fallback_nonce_successive_calls_differ() {
        let a = fallback_nonce();
        let b = fallback_nonce();
        let c = fallback_nonce();
        assert_ne!(a, b, "counter must advance the fallback nonce");
        assert_ne!(b, c, "counter must advance the fallback nonce");
        assert_ne!(a, c);
    }

    #[test]
    fn test_random_nonce_returns_nonzero_entropy() {
        // On the host this usually hits /dev/urandom; either path must not
        // collapse to the old Instant::elapsed() ~0 constant.
        let samples: Vec<u64> = (0..8).map(|_| random_nonce()).collect();
        assert!(
            samples.iter().any(|&n| n != 0),
            "nonce samples were all zero: {samples:?}"
        );
        let unique: std::collections::BTreeSet<_> = samples.iter().copied().collect();
        assert!(
            unique.len() > 1,
            "nonce samples were identical: {samples:?}"
        );
    }
}

#[cfg(test)]
mod marker_tests {
    use super::*;
    use crate::sys::MockSys;

    #[test]
    fn test_sync_once_skips_when_marker_present() {
        let dir = tempfile::tempdir().unwrap();
        let marker = ntp_disabled_marker_path(dir.path());
        std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
        std::fs::write(&marker, b"").unwrap();
        // No expectations: the marker check must return before touching the clock.
        let sys = MockSys::new();
        let cfg = crate::config::TimeCfg::default();
        assert_eq!(sync_once(&sys, &cfg, None, &marker), None);
    }

    /// A marker path that does not exist: sync proceeds normally.
    pub(crate) fn absent_marker() -> PathBuf {
        PathBuf::from("/nonexistent/ntp.disabled")
    }
}

#[cfg(test)]
mod build_request_tests {
    use super::*;

    #[test]
    fn test_build_request_sets_li_vn_mode_and_nonce() {
        let req = build_request(0xDEAD_BEEF_CAFE_F00D);
        assert_eq!(req[0], 0b00_100_011, "LI=0, VN=4, Mode=3 (client)");
        assert_eq!(&req[40..48], &0xDEAD_BEEF_CAFE_F00Du64.to_be_bytes());
        // Everything else is zeroed; only the header byte and the nonce field
        // carry data on the wire.
        assert!(req[1..40].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_ntp_socket_target_keeps_explicit_host_port() {
        assert_eq!(ntp_socket_target("127.0.0.1:4567"), "127.0.0.1:4567");
        assert_eq!(ntp_socket_target("pool.ntp.org:123"), "pool.ntp.org:123");
    }

    #[test]
    fn test_ntp_socket_target_appends_123_for_bare_and_ipv6_hosts() {
        assert_eq!(ntp_socket_target("pool.ntp.org"), "pool.ntp.org:123");
        assert_eq!(ntp_socket_target("2001:db8::1"), "[2001:db8::1]:123");
        assert_eq!(ntp_socket_target("[2001:db8::1]:123"), "[2001:db8::1]:123");
    }
}

/// End-to-end tests against a real UDP socket on loopback: a minimal NTP
/// server that echoes the client's nonce back as the originate timestamp, as
/// `parse_response` requires.
#[cfg(test)]
mod status_tests {
    use super::*;

    /// `[wifi]` is mandatory, so a `[time]`-only file does not parse.
    fn minimal_cfg_with(servers: &str) -> String {
        format!("[wifi]\nssid = \"t\"\npassword = \"p\"\n\n[time]\nservers = [{servers}]\n")
    }

    #[test]
    fn test_reload_time_cfg_picks_up_a_changed_server_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("anyka.toml");
        std::fs::write(&path, minimal_cfg_with("\"new.example\", \"pool.example\"")).unwrap();

        let current = TimeCfg {
            servers: vec!["old.example".into()],
            ..TimeCfg::default()
        };
        let got = reload_time_cfg(&path, &current);
        assert_eq!(
            got.servers,
            vec!["new.example".to_string(), "pool.example".to_string()]
        );
    }

    #[test]
    fn test_reload_time_cfg_keeps_the_current_value_on_a_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("anyka.toml");
        std::fs::write(&path, "this is not toml {{{").unwrap();

        let current = TimeCfg {
            servers: vec!["old.example".into()],
            ..TimeCfg::default()
        };
        let got = reload_time_cfg(&path, &current);
        assert_eq!(
            got.servers,
            vec!["old.example".to_string()],
            "a hand-edit mid-flight must not blank the server list"
        );
    }

    #[test]
    fn test_write_status_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        write_status(
            dir.path(),
            Some((1_760_000_000, "192.168.2.1", -3)),
            &["192.168.2.1".to_string(), "pool.example".to_string()],
        );
        let text = std::fs::read_to_string(dir.path().join("state/ntp.status")).unwrap();
        assert_eq!(
            text.trim_end(),
            "1760000000\t192.168.2.1\t-3\t192.168.2.1,pool.example"
        );
    }

    #[test]
    fn test_write_status_with_no_sync_yet() {
        let dir = tempfile::tempdir().unwrap();
        write_status(dir.path(), None, &["pool.example".to_string()]);
        let text = std::fs::read_to_string(dir.path().join("state/ntp.status")).unwrap();
        assert_eq!(text.trim_end(), "0\t\t0\tpool.example");
    }
}

#[cfg(test)]
mod query_tests {
    use super::*;
    use crate::sys::MockSys;
    use std::net::UdpSocket;

    /// Binds an ephemeral port, replies to exactly one request with a
    /// well-formed server packet (stratum 1, mode 4), and returns the port.
    fn spawn_ntp_echo_server() -> (u16, std::thread::JoinHandle<()>) {
        let sock = UdpSocket::bind("127.0.0.1:0").expect("bind ephemeral");
        let port = sock.local_addr().expect("local_addr").port();
        sock.set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; 48];
            let Ok((n, from)) = sock.recv_from(&mut buf) else {
                return;
            };
            if n != 48 {
                return;
            }
            let mut resp = [0u8; 48];
            resp[0] = 0b00_100_100; // LI=0, VN=4, Mode=4 (server)
            resp[1] = 1; // stratum 1
            resp[24..32].copy_from_slice(&buf[40..48]); // echo client nonce
            let now = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            let ntp_secs = now.as_secs() + NTP_UNIX_OFFSET;
            let frac = (u64::from(now.subsec_nanos()) << 32) / 1_000_000_000;
            resp[40..48].copy_from_slice(&((ntp_secs << 32) | frac).to_be_bytes());
            let _ = sock.send_to(&resp, from);
        });
        (port, handle)
    }

    fn wide_bounds() -> Bounds {
        Bounds {
            min_unix: 0,
            max_unix: u64::MAX,
        }
    }

    fn wide_cfg(server: String) -> crate::config::TimeCfg {
        crate::config::TimeCfg {
            servers: vec![server],
            min_plausible_unix: 0,
            max_plausible_unix: u64::MAX,
            ..crate::config::TimeCfg::default()
        }
    }

    #[test]
    fn test_query_accepts_a_reply_from_a_local_server() {
        let (port, handle) = spawn_ntp_echo_server();
        let result = query(
            &format!("127.0.0.1:{port}"),
            Duration::from_secs(5),
            &wide_bounds(),
        );
        handle.join().expect("server thread");
        assert!(result.is_ok(), "query failed: {result:?}");
    }

    #[test]
    fn test_sync_once_steps_the_clock_when_delta_exceeds_threshold() {
        let (port, handle) = spawn_ntp_echo_server();
        let mut sys = MockSys::new();
        sys.expect_now().returning(std::time::Instant::now);
        // Epoch is far enough from "now" that the delta always exceeds any
        // sane step_threshold_sec.
        sys.expect_realtime().return_const(UNIX_EPOCH);
        sys.expect_set_realtime().times(1).returning(|_| Ok(()));

        let cfg = crate::config::TimeCfg {
            step_threshold_sec: 2,
            ..wide_cfg(format!("127.0.0.1:{port}"))
        };
        let marker = marker_tests::absent_marker();
        let delta = sync_once(&sys, &cfg, None, &marker);
        handle.join().expect("server thread");
        assert!(
            matches!(delta, Some((_, d)) if d > 0),
            "expected a large positive step, got {delta:?}"
        );
    }

    #[test]
    fn test_sync_once_within_threshold_does_not_step_the_clock() {
        let (port, handle) = spawn_ntp_echo_server();
        let mut sys = MockSys::new();
        sys.expect_now().returning(std::time::Instant::now);
        sys.expect_realtime().returning(std::time::SystemTime::now);
        // No expect_set_realtime(): calling it would panic, which is exactly
        // the assertion this test wants.
        let cfg = crate::config::TimeCfg {
            step_threshold_sec: 3600,
            ..wide_cfg(format!("127.0.0.1:{port}"))
        };
        let marker = marker_tests::absent_marker();
        let delta = sync_once(&sys, &cfg, None, &marker);
        handle.join().expect("server thread");
        assert_eq!(delta.map(|(_, d)| d), Some(0));
    }

    #[test]
    fn test_sync_once_returns_none_when_budget_is_exhausted() {
        let mut sys = MockSys::new();
        sys.expect_now().returning(std::time::Instant::now);
        // No expect_realtime()/expect_set_realtime(): the budget check must
        // reject every server before any query is attempted.
        let cfg = wide_cfg("127.0.0.1:1".into());
        let marker = marker_tests::absent_marker();
        let delta = sync_once(&sys, &cfg, Some(Duration::ZERO), &marker);
        assert_eq!(delta, None);
    }

    #[test]
    fn test_sync_once_returns_none_when_every_server_fails() {
        let mut sys = MockSys::new();
        sys.expect_now().returning(std::time::Instant::now);
        // Port 1 has no NTP listener on any test host; bounding the budget
        // keeps the failure fast instead of waiting the full 5s socket
        // timeout.
        let cfg = wide_cfg("127.0.0.1:1".into());
        let marker = marker_tests::absent_marker();
        let delta = sync_once(&sys, &cfg, Some(Duration::from_millis(300)), &marker);
        assert_eq!(delta, None);
    }

    #[test]
    fn test_first_sync_disabled_returns_false() {
        let sys = MockSys::new();
        let cfg = crate::config::TimeCfg {
            enabled: false,
            ..crate::config::TimeCfg::default()
        };
        let marker = marker_tests::absent_marker();
        assert!(!first_sync(&sys, &cfg, &marker));
    }

    #[test]
    fn test_first_sync_returns_true_on_success() {
        let (port, handle) = spawn_ntp_echo_server();
        let mut sys = MockSys::new();
        sys.expect_now().returning(std::time::Instant::now);
        sys.expect_realtime().return_const(UNIX_EPOCH);
        sys.expect_set_realtime().returning(|_| Ok(()));

        let cfg = crate::config::TimeCfg {
            step_threshold_sec: 2,
            first_sync_timeout_sec: 5,
            ..wide_cfg(format!("127.0.0.1:{port}"))
        };
        let marker = marker_tests::absent_marker();
        assert!(first_sync(&sys, &cfg, &marker));
        handle.join().expect("server thread");
    }

    #[test]
    fn test_first_sync_returns_false_when_the_deadline_is_already_past() {
        let mut sys = MockSys::new();
        sys.expect_now().returning(std::time::Instant::now);
        let cfg = crate::config::TimeCfg {
            first_sync_timeout_sec: 0,
            ..wide_cfg("127.0.0.1:1".into())
        };
        let marker = marker_tests::absent_marker();
        assert!(!first_sync(&sys, &cfg, &marker));
    }
}
